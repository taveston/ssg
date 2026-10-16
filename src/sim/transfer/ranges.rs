//! What the planner's two sliders range over (PLNR-80, PLNR-81, PLNR-40)
//! and the steps they move in (PLNR-23). Pure functions of plain values
//! and the system snapshot (ARCH-20); the search that samples these
//! ranges is `search.rs`.
//!
//! Two orbits round one primary line up again once every
//! window-to-window time, so a Depart range a little longer than that
//! always holds the next window. A trip between two circular orbits needs
//! least fuel on the half ellipse that touches both, so the Travel range
//! is spread round that ellipse's travel time.
//!
//! Depart moves in whole days from Soonest, the moment the planner last
//! looked at the clock. A day keeps its number as the clock runs, so the
//! slider's positions, and the grid's columns on them, stay put while the
//! range slides on.

use std::f64::consts::PI;

use bevy::prelude::*;
use orbits::{Rail, synodic_period};

use super::ends::{End, Own, departure_end, resolve, target_end};
use super::order::{Departure, Infeasible};
use super::places::Place;
use super::tree::SystemTree;
use crate::sim::tunables::SimTunables;

/// The round steps the Travel time slider may move in, in days, smallest
/// first: 1 and 10 minutes, 1 and 6 hours, 1, 5 and 30 days (PLNR-23).
const SLIDER_STEPS_DAYS: [f64; 7] = [
    1.0 / 1440.0,
    10.0 / 1440.0,
    1.0 / 24.0,
    6.0 / 24.0,
    1.0,
    5.0,
    30.0,
];

/// The steps the Depart slider may move in: 1, 5 and 30 whole days
/// (PLNR-23).
const DEPART_STEPS_DAYS: [f64; 3] = [1.0, 5.0, 30.0];

/// Room for rounding when a span is a whole number of steps, as a
/// fraction of a step.
const STEP_ROUNDING: f64 = 1e-9;

/// Two periods closer than this fraction of each other count as equal:
/// their orbits never line up differently, so there are no windows.
const EQUAL_PERIOD_FRACTION: f64 = 1e-9;

/// A leg whose windows come round more often than this, in days, waits
/// for its window rather than offering it on the Depart slider (PHAS-16,
/// PLNR-80).
pub const WINDOW_WAIT_UNDER_DAYS: f64 = 1.0;

/// PLNR-23: a slider's step: the smallest round duration that leaves at
/// most `max_steps` steps along a range `span_days` long, or the largest
/// if none does.
pub fn slider_step_days(span_days: f64, max_steps: u32) -> f64 {
    smallest_step(&SLIDER_STEPS_DAYS, span_days, max_steps)
}

/// PLNR-23: the Depart slider's step: the smallest of 1, 5 and 30 whole
/// days that leaves at most `max_steps` steps along a range `span_days`
/// long, or 30 days if none does.
pub fn depart_step_days(span_days: f64, max_steps: u32) -> f64 {
    smallest_step(&DEPART_STEPS_DAYS, span_days, max_steps)
}

/// The first of `steps` that leaves at most `max_steps` steps along
/// `span_days`, or the last.
fn smallest_step(steps: &[f64], span_days: f64, max_steps: u32) -> f64 {
    let max_steps = f64::from(max_steps.max(1));
    steps
        .iter()
        .copied()
        .find(|step| span_days / step <= max_steps * (1.0 + STEP_ROUNDING))
        .unwrap_or(steps[steps.len() - 1])
}

/// PLNR-23: the whole days after `after` and up to `end` whose day number
/// is a multiple of `every` days, in order. Day numbers count from day 0,
/// so a day keeps its place as `after` moves on (PLNR-24).
fn whole_days(after: f64, end: f64, every: f64) -> Vec<f64> {
    if !(every > 0.0 && end > after) {
        return Vec::new();
    }
    let first = (after / every).floor() as i64 + 1;
    let last = ((end + STEP_ROUNDING) / every).floor() as i64;
    (first..=last).map(|k| k as f64 * every).collect()
}

/// PLNR-82: how many Depart positions apart the grid's columns are: the
/// least whole number that keeps Soonest and the columns within
/// `max_columns`, wherever the range sits on the day numbers. It depends
/// on the span alone, so it stays the same as the clock runs.
pub fn column_stride(span_days: f64, step_days: f64, max_columns: usize) -> u32 {
    let mut stride = 1u32;
    loop {
        let every = step_days * f64::from(stride);
        // Soonest, and at most one more multiple than the span holds.
        let columns = (span_days / every + STEP_ROUNDING).floor() as usize + 2;
        if columns <= max_columns.max(2) || every >= span_days {
            return stride;
        }
        stride += 1;
    }
}

/// The Travel time slider's settings, in days. Depart has its own type,
/// `DepartDays`.
#[derive(Debug, Clone, PartialEq)]
pub enum Slider {
    /// Any setting from `start` to `end`, in steps (PLNR-23).
    Range { start: f64, end: f64 },
    /// One setting and no slider: a low-thrust drive's travel time
    /// (PLNR-22).
    Fixed(f64),
    /// These settings only, in order: a phasing leg's meetings (PLNR-40).
    Positions(Vec<f64>),
}

impl Slider {
    /// PLNR-82: `count` settings spread evenly over a range, ends
    /// included; the one setting of a fixed slider; every position.
    pub fn samples(&self, count: usize) -> Vec<f64> {
        match self {
            Slider::Range { start, end } => match count {
                0 => Vec::new(),
                1 => vec![*start],
                _ => (0..count)
                    .map(|i| start + (end - start) * i as f64 / (count - 1) as f64)
                    .collect(),
            },
            Slider::Fixed(value) => vec![*value],
            Slider::Positions(positions) => positions.clone(),
        }
    }

    /// The gap between neighbouring samples of a range sampled `count`
    /// times (PLNR-86); zero for a slider that is not a range.
    pub fn spacing(&self, count: usize) -> f64 {
        match self {
            Slider::Range { start, end } if count > 1 => (end - start) / (count - 1) as f64,
            _ => 0.0,
        }
    }

    /// PLNR-23: every setting the slider can take, in order. A range's are
    /// its start and each step on from it, and its end.
    pub fn settings(&self, max_steps: u32) -> Vec<f64> {
        match self {
            Slider::Range { start, end } => {
                let step = slider_step_days(end - start, max_steps);
                let count = ((end - start) / step - STEP_ROUNDING).ceil().max(0.0) as usize;
                (0..=count)
                    .map(|i| {
                        if i == count {
                            *end
                        } else {
                            start + step * i as f64
                        }
                    })
                    .collect()
            }
            Slider::Fixed(value) => vec![*value],
            Slider::Positions(positions) => positions.clone(),
        }
    }

    /// The setting nearest `value`: where a slider set to it rests.
    pub fn snap(&self, value: f64, max_steps: u32) -> f64 {
        nearest_setting(self.settings(max_steps), value)
    }

    /// PLNR-86: the settings within `within` days of `value`, or the
    /// nearest one if none is that close.
    pub fn settings_near(&self, value: f64, within: f64, max_steps: u32) -> Vec<f64> {
        settings_within(self.settings(max_steps), value, within)
    }
}

/// The Depart slider (PLNR-21, PLNR-23): Soonest, the moment the planner
/// last solved it, then each whole day after it up to `end` whose day
/// number is a multiple of `step` days.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepartDays {
    pub soonest: f64,
    pub end: f64,
    pub step: f64,
}

impl DepartDays {
    /// PLNR-82: the grid's depart columns: Soonest, then the positions
    /// whose day number is a multiple of the column stride, so that at
    /// most `count` columns cover the range.
    pub fn samples(&self, count: usize) -> Vec<f64> {
        let stride = column_stride(self.end - self.soonest, self.step, count);
        let mut columns = vec![self.soonest];
        columns.extend(whole_days(
            self.soonest,
            self.end,
            self.step * f64::from(stride),
        ));
        columns
    }

    /// PLNR-86: the gap between the columns of `samples(count)`.
    pub fn spacing(&self, count: usize) -> f64 {
        self.step * f64::from(column_stride(self.end - self.soonest, self.step, count))
    }

    /// PLNR-23: every position, in order: Soonest, then each whole day
    /// after it whose day number is a multiple of the step.
    pub fn settings(&self) -> Vec<f64> {
        let mut settings = vec![self.soonest];
        settings.extend(whole_days(self.soonest, self.end, self.step));
        settings
    }

    /// The position nearest `value`: where the slider set to it rests.
    pub fn snap(&self, value: f64) -> f64 {
        nearest_setting(self.settings(), value)
    }

    /// PLNR-86: the positions within `within` days of `value`, or the
    /// nearest one if none is that close.
    pub fn settings_near(&self, value: f64, within: f64) -> Vec<f64> {
        settings_within(self.settings(), value, within)
    }

    /// PLNR-24: the same span and step from Soonest at `now`.
    pub fn moved_on(&self, now: f64) -> DepartDays {
        DepartDays {
            soonest: now,
            end: now + (self.end - self.soonest),
            step: self.step,
        }
    }
}

/// The one of `settings` nearest `value`, or `value` if there are none.
fn nearest_setting(settings: Vec<f64>, value: f64) -> f64 {
    settings
        .into_iter()
        .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
        .unwrap_or(value)
}

/// Those of `settings` within `within` days of `value`, or the nearest
/// one if none is that close.
fn settings_within(settings: Vec<f64>, value: f64, within: f64) -> Vec<f64> {
    let near: Vec<f64> = settings
        .iter()
        .copied()
        .filter(|setting| (setting - value).abs() <= within * (1.0 + STEP_ROUNDING))
        .collect();
    if near.is_empty() {
        vec![nearest_setting(settings, value)]
    } else {
        near
    }
}

/// Both sliders of one planned leg.
#[derive(Debug, Clone, PartialEq)]
pub struct Sliders {
    /// The order's depart day (PLNR-21, PLNR-80).
    pub depart: DepartDays,
    /// The order's travel time (PLNR-22, PLNR-81, PLNR-40).
    pub travel: Slider,
    /// Whether the Depart range comes from two orbits' periods, so it
    /// holds a window, and the Next window preset can exist (PLNR-85).
    pub windows: bool,
}

impl Sliders {
    /// PLNR-40: a phasing leg's travel times are its meetings.
    pub fn phasing(&self) -> bool {
        matches!(self.travel, Slider::Positions(_))
    }
}

/// One end of a crossing as an orbit round the crossing's primary: its
/// radius, and its period if it goes round at a steady rate with a phase
/// of its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndOrbit {
    pub radius_km: f64,
    /// `None` for a coasting ship, and for a whole ring of the primary,
    /// which any point of will do, so it has no phase to line up.
    pub period_days: Option<f64>,
    /// Whether the end is in a band of the primary itself (BAND-01): a
    /// ship or station in one, or the whole band.
    pub in_band: bool,
}

impl EndOrbit {
    /// A whole band of the primary: any point of it will do.
    fn whole_band(&self) -> bool {
        self.in_band && self.period_days.is_none()
    }
}

/// The window-to-window time of two ends that go round the primary with
/// different periods: 1 ÷ abs(1 ÷ P_a − 1 ÷ P_b), in days (PLNR-80,
/// PHAS-16). `None` when either has no period, or the periods are equal.
pub fn window_interval_days(from: EndOrbit, to: EndOrbit) -> Option<f64> {
    match (from.period_days, to.period_days) {
        (Some(a), Some(b)) if (a - b).abs() > EQUAL_PERIOD_FRACTION * a.max(b) => {
            Some(synodic_period(a, b)).filter(|days| days.is_finite() && *days > 0.0)
        }
        _ => None,
    }
}

/// PHAS-16: the window interval of a leg that leaves for its window
/// rather than at once: an impulsive drive leaving a band of the primary
/// (a ship or station in one) for a target with a position of its own
/// round it, whose windows come round in under a day. `None` for any
/// other leg. A phasing leg never reaches here: it is not a crossing.
pub fn window_wait_interval(impulsive: bool, from: EndOrbit, to: EndOrbit) -> Option<f64> {
    if !(impulsive && from.in_band) {
        return None;
    }
    window_interval_days(from, to).filter(|days| *days < WINDOW_WAIT_UNDER_DAYS)
}

/// PLNR-80 / PLNR-23: the Depart slider, from Soonest at `today`. For an
/// impulsive drive whose two ends go round the primary with different
/// periods it runs one window-to-window time, plus
/// `depart_margin_fraction` of it, so the next window is always on the
/// slider; otherwise, and for windows that come round more than once a
/// day (PHAS-16), `depart_range_days`. The flag says whether it holds a
/// window (PLNR-85).
pub fn depart_range(
    today: f64,
    impulsive: bool,
    from: EndOrbit,
    to: EndOrbit,
    tunables: &SimTunables,
) -> (DepartDays, bool) {
    let window =
        window_interval_days(from, to).filter(|days| impulsive && *days >= WINDOW_WAIT_UNDER_DAYS);
    let span = match window {
        Some(days) => (1.0 + tunables.depart_margin_fraction) * days,
        None => tunables.depart_range_days,
    };
    (days_slider(today, span, tunables), window.is_some())
}

/// PLNR-23 / PLNR-40: the Depart slider over `span_days` from Soonest at
/// `today`, in its whole-day step.
pub fn days_slider(today: f64, span_days: f64, tunables: &SimTunables) -> DepartDays {
    DepartDays {
        soonest: today,
        end: today + span_days,
        step: depart_step_days(span_days, tunables.slider_max_steps),
    }
}

/// PLNR-81: half the period of the ellipse that touches circles of
/// `from_km` and `to_km` round a primary of gravitational parameter `mu`
/// (km³/day²): the travel time of the trip that needs least fuel between
/// them, in days.
pub fn least_fuel_travel_days(mu: f64, from_km: f64, to_km: f64) -> f64 {
    let semi_major_km = (from_km + to_km) / 2.0;
    PI * (semi_major_km.powi(3) / mu).sqrt()
}

/// PLNR-81: the Travel range of an impulsive drive, from
/// `travel_min_factor` to `travel_max_factor` times the least-fuel travel
/// time between the two ends' radii. From a band of the primary to a
/// whole band of it the range starts at the least-fuel time itself: a
/// quicker crossing dips under the Low band (XFER-07).
pub fn travel_range(mu: f64, from: EndOrbit, to: EndOrbit, tunables: &SimTunables) -> Slider {
    let least_fuel = least_fuel_travel_days(mu, from.radius_km, to.radius_km);
    let start = if from.in_band && to.whole_band() {
        least_fuel
    } else {
        tunables.travel_min_factor * least_fuel
    };
    Slider::Range {
        start,
        end: tunables.travel_max_factor * least_fuel,
    }
}

/// A crossing's primary and its two ends as orbits round it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossingOrbits {
    pub primary: Entity,
    pub mu: f64,
    pub from: EndOrbit,
    pub to: EndOrbit,
}

/// XFER-01 / PLNR-80 / PLNR-81: the primary a crossing from `departure`
/// to `destination` goes round, and each end as an orbit round it on
/// `today`: the end's ancestor directly below the primary, or its own
/// orbit if it sits at the primary itself. A coasting ship's radius is
/// the one it has today.
pub fn crossing_orbits(
    tree: &SystemTree,
    departure: &Departure,
    destination: Place,
    together_km: f64,
    today: f64,
) -> Result<CrossingOrbits, Infeasible> {
    let (from, _) = departure_end(tree, departure, together_km)?;
    let to = target_end(&resolve(tree, destination)?);
    let primary = tree
        .lowest_common_ancestor(from.at, to.at)
        .ok_or(Infeasible::UnknownPlace)?;
    let orbit = |end: &End| end_orbit(tree, primary, end, today).ok_or(Infeasible::UnknownPlace);
    Ok(CrossingOrbits {
        primary,
        mu: tree.mu(primary),
        from: orbit(&from)?,
        to: orbit(&to)?,
    })
}

/// One end as an orbit round `primary` on `today`.
pub(super) fn end_orbit(
    tree: &SystemTree,
    primary: Entity,
    end: &End,
    today: f64,
) -> Option<EndOrbit> {
    if let Some(child) = tree.child_below(primary, end.at) {
        let orbit = tree.bodies.get(&child)?.orbit?;
        return Some(EndOrbit {
            radius_km: orbit.radius_km,
            period_days: Some(orbit.period_days),
            in_band: false,
        });
    }
    if end.at != primary {
        return None;
    }
    let in_band = end.band.is_some();
    Some(match end.own {
        Own::Circular(orbit) => EndOrbit {
            radius_km: orbit.radius_km,
            period_days: Some(orbit.period_days),
            in_band,
        },
        Own::Ring(node) => EndOrbit {
            radius_km: node.radius_km,
            period_days: None,
            in_band,
        },
        Own::Conic(conic) => EndOrbit {
            radius_km: conic.state_at(today).radius(),
            period_days: None,
            in_band: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::transfer::places::Band;
    use crate::sim::transfer::test_layout::{layout, within};
    use crate::sim::transfer::tree::ShipState;
    use orbits::{Conic, State, hohmann};

    fn tunables() -> SimTunables {
        SimTunables::default()
    }

    /// PLNR-23: the step is the smallest round duration that keeps a
    /// range within 400 steps: a day for a 180-day range, 10 minutes for
    /// two days, an hour for ten; past 30 days' worth of steps it stays
    /// at 30 days.
    #[test]
    fn a_slider_takes_the_smallest_round_step_that_fits() {
        assert_eq!(slider_step_days(180.0, 400), 1.0);
        assert_eq!(slider_step_days(400.0, 400), 1.0);
        assert_eq!(slider_step_days(401.0, 400), 5.0);
        assert_eq!(slider_step_days(2.0, 400), 10.0 / 1440.0);
        assert_eq!(slider_step_days(10.0, 400), 1.0 / 24.0);
        assert_eq!(slider_step_days(0.2, 400), 1.0 / 1440.0);
        assert_eq!(slider_step_days(1e6, 400), 30.0);
    }

    /// PLNR-23 / PLNR-86: a range's settings run from its start in steps
    /// and end at its end; a setting snaps to the nearest, and the
    /// settings near a value are those within the distance asked.
    #[test]
    fn a_range_moves_in_steps_from_its_start_to_its_end() {
        let slider = Slider::Range {
            start: 10.0,
            end: 12.5,
        };
        let step = slider_step_days(2.5, 400);
        assert_eq!(step, 10.0 / 1440.0);
        let settings = slider.settings(400);
        assert_eq!(settings.first(), Some(&10.0));
        assert_eq!(settings.last(), Some(&12.5));
        assert_eq!(settings.len(), 361);
        let coarse = Slider::Range {
            start: 0.0,
            end: 400.5,
        };
        let settings = coarse.settings(400);
        assert_eq!(settings[1], 5.0);
        assert_eq!(*settings.last().unwrap(), 400.5);
        assert_eq!(coarse.snap(12.4, 400), 10.0);
        assert_eq!(coarse.snap(13.0, 400), 15.0);
        assert_eq!(
            coarse.settings_near(100.0, 11.0, 400),
            vec![90.0, 95.0, 100.0, 105.0, 110.0]
        );
        assert_eq!(coarse.settings_near(101.0, 1.0, 400), vec![100.0]);
        assert_eq!(Slider::Fixed(3.0).settings_near(7.0, 1.0, 400), vec![3.0]);
    }

    /// PLNR-82: a range is sampled evenly with both ends; a fixed slider
    /// has its one setting; positions are all sampled.
    #[test]
    fn samples_spread_evenly_with_both_ends() {
        let slider = Slider::Range {
            start: 0.0,
            end: 90.0,
        };
        assert_eq!(slider.samples(4), vec![0.0, 30.0, 60.0, 90.0]);
        assert_eq!(slider.spacing(4), 30.0);
        assert_eq!(Slider::Fixed(5.0).samples(4), vec![5.0]);
        assert_eq!(Slider::Fixed(5.0).spacing(4), 0.0);
        let meetings = Slider::Positions(vec![1.0, 3.0, 5.0]);
        assert_eq!(meetings.samples(36), vec![1.0, 3.0, 5.0]);
    }

    fn orbit(radius_km: f64, period_days: Option<f64>, in_band: bool) -> EndOrbit {
        EndOrbit {
            radius_km,
            period_days,
            in_band,
        }
    }

    /// PLNR-80 / PLNR-21: two orbits of different periods give a Depart
    /// range from Soonest of one window-to-window time and a tenth; a
    /// low-thrust drive, a coast or a whole band with no phase, equal
    /// periods, and windows that come round more than once a day
    /// (PHAS-16) give 180 days, with no window.
    #[test]
    fn the_depart_range_holds_the_next_window() {
        let t = tunables();
        let a = orbit(1.0, Some(365.25), false);
        let b = orbit(2.0, Some(687.0), false);
        let (range, windows) = depart_range(100.3, true, a, b, &t);
        let synodic = synodic_period(365.25, 687.0);
        assert!(windows);
        let span = 1.1 * synodic;
        assert_eq!(
            range,
            DepartDays {
                soonest: 100.3,
                end: 100.3 + span,
                step: 5.0,
            },
            "{span} days in at most 400 steps: 5-day steps"
        );
        let fixed = DepartDays {
            soonest: 100.3,
            end: 280.3,
            step: 1.0,
        };
        assert_eq!(depart_range(100.3, false, a, b, &t), (fixed, false));
        let whole_band = orbit(2.0, None, true);
        assert_eq!(depart_range(100.3, true, a, whole_band, &t), (fixed, false));
        assert_eq!(depart_range(100.3, true, a, a, &t), (fixed, false));
        let low = orbit(1.0, Some(0.07), true);
        let station = orbit(2.0, Some(1.95), true);
        assert!(window_interval_days(low, station).unwrap() < 1.0);
        assert_eq!(depart_range(100.3, true, low, station, &t), (fixed, false));
    }

    /// PLNR-23 / PLNR-24: the Depart slider's positions are Soonest and
    /// every whole day after it whose day number is a multiple of the
    /// step, so a position keeps its day as Soonest moves on; the step is
    /// the smallest of 1, 5 and 30 days that keeps within 400 steps.
    #[test]
    fn depart_positions_are_soonest_and_whole_days() {
        assert_eq!(depart_step_days(180.0, 400), 1.0);
        assert_eq!(depart_step_days(401.0, 400), 5.0);
        assert_eq!(depart_step_days(1e5, 400), 30.0);
        let slider = DepartDays {
            soonest: 10.4,
            end: 30.4,
            step: 5.0,
        };
        assert_eq!(slider.settings(), vec![10.4, 15.0, 20.0, 25.0, 30.0]);
        let later = DepartDays {
            soonest: 15.0,
            end: 35.0,
            step: 5.0,
        };
        assert_eq!(later.settings(), vec![15.0, 20.0, 25.0, 30.0, 35.0]);
        assert_eq!(slider.snap(12.0), 10.4);
        assert_eq!(slider.snap(13.0), 15.0);
    }

    /// PLNR-82: the grid's depart columns are Soonest and the positions
    /// thinned to every nth, n the least that keeps within 60 columns,
    /// counted from day 0: 180 daily positions give every 4th day, and a
    /// column keeps its day a day later.
    #[test]
    fn depart_columns_are_thinned_by_day_number() {
        assert_eq!(column_stride(180.0, 1.0, 60), 4);
        assert_eq!(column_stride(30.0, 1.0, 60), 1);
        let today = DepartDays {
            soonest: 10.4,
            end: 190.4,
            step: 1.0,
        };
        let columns = today.samples(60);
        assert!(columns.len() <= 60, "{}", columns.len());
        assert_eq!(&columns[..3], &[10.4, 12.0, 16.0]);
        assert_eq!(*columns.last().unwrap(), 188.0);
        assert!(columns[1..].iter().all(|day| day % 4.0 == 0.0));
        assert_eq!(today.spacing(60), 4.0);
        let tomorrow = DepartDays {
            soonest: 11.4,
            end: 191.4,
            step: 1.0,
        };
        assert_eq!(&tomorrow.samples(60)[1..], &columns[1..]);
        let small = DepartDays {
            soonest: 0.5,
            end: 5.5,
            step: 1.0,
        };
        assert_eq!(small.samples(60), vec![0.5, 1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    /// PHAS-16: a leg waits for its window when an impulsive drive leaves
    /// a band of the primary for something with a position of its own,
    /// and the window comes round more than once a day; not from a
    /// planet's orbit, not for a whole band, not for a low-thrust drive,
    /// and not when the window comes round more slowly.
    #[test]
    fn only_a_band_leg_with_sub_day_windows_waits_for_its_window() {
        let high = orbit(57_530.0, Some(1.95), true);
        let station = orbit(6_253.0, Some(0.0696), true);
        let interval = window_wait_interval(true, high, station).unwrap();
        assert!((interval - synodic_period(1.95, 0.0696)).abs() < 1e-12);
        assert!(interval < 0.08);
        assert_eq!(window_wait_interval(false, high, station), None);
        let planet = orbit(57_530.0, Some(1.95), false);
        assert_eq!(window_wait_interval(true, planet, station), None);
        assert_eq!(
            window_wait_interval(true, high, orbit(6_253.0, None, true)),
            None
        );
        let moon = orbit(400_000.0, Some(30.0), false);
        assert_eq!(window_wait_interval(true, high, moon), None, "2.1 days");
    }

    /// PLNR-81: the Travel range runs from 0.3 to 2 times the least-fuel
    /// travel time, the half ellipse touching both radii: about 111 days
    /// from 0.5 to 0.8 AU round the reference star (XFER-12).
    #[test]
    fn the_travel_range_is_spread_round_the_least_fuel_trip() {
        let l = layout();
        let inner = l.tree.bodies[&l.inner].orbit.unwrap();
        let capital = l.tree.bodies[&l.capital].orbit.unwrap();
        let mu = l.tree.mu(l.star);
        let days = least_fuel_travel_days(mu, inner.radius_km, capital.radius_km);
        within(days, 111.0, 0.03, "least-fuel travel");
        let reference = hohmann(mu, inner.radius_km, capital.radius_km).days;
        assert!((days - reference).abs() < 1e-9);
        let end = |radius_km| orbit(radius_km, Some(1.0), false);
        let range = travel_range(
            mu,
            end(inner.radius_km),
            end(capital.radius_km),
            &tunables(),
        );
        assert_eq!(
            range,
            Slider::Range {
                start: 0.3 * days,
                end: 2.0 * days
            }
        );
    }

    /// PLNR-81: from a band of the primary to a whole band of it the
    /// Travel range starts at the least-fuel time; to a station in the
    /// band it starts at 0.3 times it, as elsewhere.
    #[test]
    fn band_to_band_travel_starts_at_the_least_fuel_time() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let t = tunables();
        let low_band = Place::Body {
            body: l.capital,
            band: Band::Low,
        };
        let ends = crossing_orbits(&l.tree, &from, low_band, 10.0, 0.0).unwrap();
        let least = least_fuel_travel_days(ends.mu, ends.from.radius_km, ends.to.radius_km);
        assert_eq!(
            travel_range(ends.mu, ends.from, ends.to, &t),
            Slider::Range {
                start: least,
                end: 2.0 * least
            }
        );
        let station = Place::Station { station: l.station };
        let ends = crossing_orbits(&l.tree, &from, station, 10.0, 0.0).unwrap();
        let Slider::Range { start, .. } = travel_range(ends.mu, ends.from, ends.to, &t) else {
            panic!("a range");
        };
        assert!((start - 0.3 * least).abs() < 1e-9);
    }

    /// XFER-01 / PLNR-80: from the capital's High band to the inner
    /// planet the crossing goes round the star between the two planets'
    /// orbits; to a station in the capital's Low band it goes round the
    /// capital, from the ship's band to the station's own point.
    #[test]
    fn a_crossing_is_read_as_two_orbits_round_its_primary() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let to_inner = Place::Body {
            body: l.inner,
            band: Band::High,
        };
        let orbits = crossing_orbits(&l.tree, &from, to_inner, 10.0, 0.0).unwrap();
        let inner = l.tree.bodies[&l.inner].orbit.unwrap();
        let capital = l.tree.bodies[&l.capital].orbit.unwrap();
        assert_eq!(orbits.primary, l.star);
        assert_eq!(orbits.from.period_days, Some(capital.period_days));
        assert_eq!(orbits.to.radius_km, inner.radius_km);

        let to_station = Place::Station { station: l.station };
        let orbits = crossing_orbits(&l.tree, &from, to_station, 10.0, 0.0).unwrap();
        let high = l.node(l.capital, Band::High);
        let low = l.node(l.capital, Band::Low);
        assert_eq!(orbits.primary, l.capital);
        assert_eq!(orbits.from.period_days, Some(high.period_days));
        assert_eq!(orbits.to.radius_km, low.radius_km);
        assert_eq!(orbits.to.period_days, Some(low.period_days));
    }

    /// PLNR-80 / PLNR-81: the capital's own Low band, as a whole ring, has
    /// no phase to line up with; a coasting target's radius is today's.
    #[test]
    fn a_ring_of_the_primary_and_a_coast_have_no_period() {
        let mut l = layout();
        let from = l.parked(l.capital, Band::High);
        let to_low = Place::Body {
            body: l.capital,
            band: Band::Low,
        };
        let orbits = crossing_orbits(&l.tree, &from, to_low, 10.0, 0.0).unwrap();
        assert_eq!(orbits.to.period_days, None);
        assert_eq!(orbits.to.radius_km, l.node(l.capital, Band::Low).radius_km);

        let mu = l.tree.mu(l.star);
        let start = l.tree.bodies[&l.inner].orbit.unwrap().state_at(0.0);
        let state = State::new(start.position, start.velocity * 1.1);
        let conic = Conic::from_state(mu, state, 0.0, 1e-6).unwrap();
        let ship = l.add_ship(ShipState::Coasting {
            primary: l.star,
            conic,
        });
        let orbits = crossing_orbits(&l.tree, &from, Place::Ship { ship }, 10.0, 30.0).unwrap();
        assert_eq!(orbits.primary, l.star);
        assert_eq!(orbits.to.period_days, None);
        assert!((orbits.to.radius_km - conic.state_at(30.0).radius()).abs() < 1e-6);
    }
}

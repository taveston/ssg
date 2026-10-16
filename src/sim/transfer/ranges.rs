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

use std::f64::consts::PI;

use bevy::prelude::*;
use orbits::{Rail, synodic_period};

use super::ends::{End, Own, departure_end, resolve, target_end};
use super::order::{Departure, Infeasible};
use super::places::Place;
use super::tree::SystemTree;
use crate::sim::tunables::SimTunables;

/// The round steps a slider may move in, in days, smallest first: 1 and
/// 10 minutes, 1 and 6 hours, 1, 5 and 30 days (PLNR-23).
const SLIDER_STEPS_DAYS: [f64; 7] = [
    1.0 / 1440.0,
    10.0 / 1440.0,
    1.0 / 24.0,
    6.0 / 24.0,
    1.0,
    5.0,
    30.0,
];

/// Room for rounding when a span is a whole number of steps, as a
/// fraction of a step.
const STEP_ROUNDING: f64 = 1e-9;

/// Two periods closer than this fraction of each other count as equal:
/// their orbits never line up differently, so there are no windows.
const EQUAL_PERIOD_FRACTION: f64 = 1e-9;

/// PLNR-23: a slider's step: the smallest round duration that leaves at
/// most `max_steps` steps along a range `span_days` long, or the largest
/// if none does.
pub fn slider_step_days(span_days: f64, max_steps: u32) -> f64 {
    let max_steps = f64::from(max_steps.max(1));
    SLIDER_STEPS_DAYS
        .into_iter()
        .find(|step| span_days / step <= max_steps * (1.0 + STEP_ROUNDING))
        .unwrap_or(SLIDER_STEPS_DAYS[SLIDER_STEPS_DAYS.len() - 1])
}

/// One slider's settings, in days.
#[derive(Debug, Clone, PartialEq)]
pub enum Slider {
    /// Any setting from `start` to `end`, in steps (PLNR-23).
    Range { start: f64, end: f64 },
    /// One setting and no slider: a phasing leg's depart day, a
    /// low-thrust drive's travel time (PLNR-22, PLNR-40).
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
    /// times (PLNR-86); zero for a slider that isn't a range.
    pub fn spacing(&self, count: usize) -> f64 {
        match self {
            Slider::Range { start, end } if count > 1 => (end - start) / (count - 1) as f64,
            _ => 0.0,
        }
    }

    /// PLNR-23: the step a range moves in; `None` for a slider that isn't
    /// a range.
    pub fn step(&self, max_steps: u32) -> Option<f64> {
        match self {
            Slider::Range { start, end } => Some(slider_step_days(end - start, max_steps)),
            _ => None,
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
        self.settings(max_steps)
            .into_iter()
            .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
            .unwrap_or(value)
    }

    /// PLNR-86: the settings within `within` days of `value`, or the
    /// nearest one if none is that close.
    pub fn settings_near(&self, value: f64, within: f64, max_steps: u32) -> Vec<f64> {
        let near: Vec<f64> = self
            .settings(max_steps)
            .into_iter()
            .filter(|setting| (setting - value).abs() <= within * (1.0 + STEP_ROUNDING))
            .collect();
        if near.is_empty() {
            vec![self.snap(value, max_steps)]
        } else {
            near
        }
    }
}

/// Both sliders of one planned leg.
#[derive(Debug, Clone, PartialEq)]
pub struct Sliders {
    /// The order's depart day (PLNR-21, PLNR-80).
    pub depart: Slider,
    /// The order's travel time (PLNR-22, PLNR-81, PLNR-40).
    pub travel: Slider,
    /// Whether the Depart range comes from two orbits' periods, so it
    /// holds a window, and the Next window preset can exist (PLNR-85).
    pub windows: bool,
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
}

/// PLNR-80: the Depart range, from `today`. For an impulsive drive whose
/// two ends go round the primary with different periods it runs one
/// window-to-window time, plus `depart_margin_fraction` of it, so the
/// next window is always on the slider; otherwise `depart_range_days`.
/// The flag says which it is (PLNR-85).
pub fn depart_range(
    today: f64,
    impulsive: bool,
    from: EndOrbit,
    to: EndOrbit,
    tunables: &SimTunables,
) -> (Slider, bool) {
    let synodic = match (impulsive, from.period_days, to.period_days) {
        (true, Some(a), Some(b)) if (a - b).abs() > EQUAL_PERIOD_FRACTION * a.max(b) => {
            Some(synodic_period(a, b)).filter(|days| days.is_finite() && *days > 0.0)
        }
        _ => None,
    };
    let span = match synodic {
        Some(days) => (1.0 + tunables.depart_margin_fraction) * days,
        None => tunables.depart_range_days,
    };
    (
        Slider::Range {
            start: today,
            end: today + span,
        },
        synodic.is_some(),
    )
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
/// time between the two ends' radii.
pub fn travel_range(mu: f64, from: EndOrbit, to: EndOrbit, tunables: &SimTunables) -> Slider {
    let least_fuel = least_fuel_travel_days(mu, from.radius_km, to.radius_km);
    Slider::Range {
        start: tunables.travel_min_factor * least_fuel,
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
fn end_orbit(tree: &SystemTree, primary: Entity, end: &End, today: f64) -> Option<EndOrbit> {
    if let Some(child) = tree.child_below(primary, end.at) {
        let orbit = tree.bodies.get(&child)?.orbit?;
        return Some(EndOrbit {
            radius_km: orbit.radius_km,
            period_days: Some(orbit.period_days),
        });
    }
    if end.at != primary {
        return None;
    }
    Some(match end.own {
        Own::Circular(orbit) => EndOrbit {
            radius_km: orbit.radius_km,
            period_days: Some(orbit.period_days),
        },
        Own::Ring(node) => EndOrbit {
            radius_km: node.radius_km,
            period_days: None,
        },
        Own::Conic(conic) => EndOrbit {
            radius_km: conic.state_at(today).radius(),
            period_days: None,
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
        let step = slider.step(400).unwrap();
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

    /// PLNR-80: two orbits of different periods give a Depart range of
    /// one window-to-window time and a tenth; a low-thrust drive, a coast
    /// or a ring with no phase, and equal periods give 180 days.
    #[test]
    fn the_depart_range_holds_the_next_window() {
        let t = tunables();
        let a = EndOrbit {
            radius_km: 1.0,
            period_days: Some(365.25),
        };
        let b = EndOrbit {
            radius_km: 2.0,
            period_days: Some(687.0),
        };
        let (range, windows) = depart_range(100.0, true, a, b, &t);
        let synodic = synodic_period(365.25, 687.0);
        assert!(windows);
        assert_eq!(
            range,
            Slider::Range {
                start: 100.0,
                end: 100.0 + 1.1 * synodic
            }
        );
        let fixed = Slider::Range {
            start: 100.0,
            end: 280.0,
        };
        assert_eq!(depart_range(100.0, false, a, b, &t), (fixed.clone(), false));
        let coast = EndOrbit {
            period_days: None,
            ..b
        };
        assert_eq!(
            depart_range(100.0, true, a, coast, &t),
            (fixed.clone(), false)
        );
        assert_eq!(depart_range(100.0, true, a, a, &t), (fixed, false));
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
        let end = |radius_km| EndOrbit {
            radius_km,
            period_days: None,
        };
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

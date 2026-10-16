//! The planner's three presets (PLNR-83 to PLNR-86): Fastest, Cheapest
//! and Next window, each picked from the grid and then refined at the
//! sliders' own steps. Pure functions of the grid, the sliders and a
//! solve function handed in (ARCH-20); the search that calls them is
//! `search.rs`.
//!
//! The grid is coarse, so a preset picked from it is only near the best
//! setting. Refining solves every slider step within one grid spacing of
//! it and picks again, so the sliders land on a setting they can show.

use std::cmp::Ordering;

use super::grid::{Grid, Sample, Setting, least_delta_v};
use super::ranges::Sliders;
use crate::sim::tunables::SimTunables;

/// The three presets, in the order the panel shows them (PLNR-29).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preset {
    Fastest,
    Cheapest,
    NextWindow,
}

/// What each preset sets the sliders to, if it exists.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Presets {
    pub fastest: Option<Setting>,
    pub cheapest: Option<Setting>,
    pub next_window: Option<Setting>,
}

impl Presets {
    /// PLNR-29: the presets to show, in order: those that exist, less any
    /// whose setting equals an earlier one's.
    pub fn shown(&self) -> Vec<(Preset, Setting)> {
        let mut shown: Vec<(Preset, Setting)> = Vec::new();
        for (preset, setting) in [
            (Preset::Fastest, self.fastest),
            (Preset::Cheapest, self.cheapest),
            (Preset::NextWindow, self.next_window),
        ] {
            if let Some(setting) = setting
                && shown.iter().all(|(_, earlier)| *earlier != setting)
            {
                shown.push((preset, setting));
            }
        }
        shown
    }

    /// The setting the sliders start at: Fastest, or Cheapest when nothing
    /// is affordable (PLNR-13).
    pub fn starting(&self) -> Option<Setting> {
        self.fastest.or(self.cheapest)
    }

    /// PLNR-29: the first shown preset whose setting is `setting`: the one
    /// the sliders sit on.
    pub fn sitting_on(&self, setting: Setting) -> Option<Preset> {
        self.shown()
            .into_iter()
            .find(|(_, at)| *at == setting)
            .map(|(preset, _)| preset)
    }

    /// PLNR-24 / PLNR-87: the presets once Soonest has moved on from
    /// `was_soonest` to `now`. They are not picked again: one at Soonest
    /// stays at Soonest, and one whose depart day has come moves to
    /// Soonest, as a thumb does.
    pub fn moved_on(&self, was_soonest: f64, now: f64) -> Presets {
        let moved = |setting: Option<Setting>| setting.map(|s| moved_on(s, was_soonest, now));
        Presets {
            fastest: moved(self.fastest),
            cheapest: moved(self.cheapest),
            next_window: moved(self.next_window),
        }
    }
}

/// PLNR-24: a setting once Soonest has moved on from `was_soonest` to
/// `now`: at Soonest, or on a day the clock has reached, it departs at
/// the new Soonest; on a later day it keeps its day. The travel time
/// stays.
pub fn moved_on(setting: Setting, was_soonest: f64, now: f64) -> Setting {
    if setting.depart_day == was_soonest || setting.depart_day <= now {
        Setting {
            depart_day: now,
            ..setting
        }
    } else {
        setting
    }
}

/// Earlier arrival first, then less delta-v.
fn by_arrival(a: &Sample, b: &Sample) -> Ordering {
    match (a.cost(), b.cost()) {
        (Some(a), Some(b)) => a
            .arrival_day
            .total_cmp(&b.arrival_day)
            .then(a.delta_v_kms.total_cmp(&b.delta_v_kms)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// PLNR-83: Fastest: of the samples the ship can pay for from
/// `available_kms`, the one that arrives earliest, or of equal arrival
/// the one with less delta-v. `None` when none is affordable.
pub fn fastest<'a>(
    samples: impl Iterator<Item = &'a Sample>,
    available_kms: f64,
) -> Option<Sample> {
    samples
        .filter(|sample| sample.affordable(available_kms))
        .min_by(|a, b| by_arrival(a, b))
        .copied()
}

/// PLNR-84: Cheapest: the sample with least delta-v, counting every
/// sample within `tie_kms` of it as equal and taking the one of those
/// that arrives earliest. It may be over budget.
pub fn cheapest<'a>(
    samples: impl Iterator<Item = &'a Sample> + Clone,
    tie_kms: f64,
) -> Option<Sample> {
    let least = least_delta_v(samples.clone())?.cost()?.delta_v_kms;
    samples
        .filter(|sample| {
            sample
                .cost()
                .is_some_and(|cost| cost.delta_v_kms <= least + tie_kms)
        })
        .min_by(|a, b| by_arrival(a, b))
        .copied()
}

/// PLNR-85: Next window. Each depart day's best is its cheapest trip of
/// any travel time; a day is in a window when its best is within
/// `margin_kms` of the best of all. The earliest such day, with its
/// cheapest trip, is the next window. It may be over budget.
pub fn next_window(grid: &Grid, margin_kms: f64) -> Option<Sample> {
    let per_day = grid.cheapest_per_day();
    let in_window = window_test(&per_day, margin_kms)?;
    per_day.iter().find(|day| in_window(day)).copied().flatten()
}

/// The delta-v of a day's cheapest trip, in km/s; `None` for a day with
/// no trip.
fn best_kms(day: &Option<Sample>) -> Option<f64> {
    day.and_then(|sample| sample.cost())
        .map(|cost| cost.delta_v_kms)
}

/// PLNR-85: given each depart day's cheapest trip, a test of whether a
/// day is in a window: its best is within `margin_kms` of the best of
/// all. `None` when no day has a trip.
fn window_test(
    per_day: &[Option<Sample>],
    margin_kms: f64,
) -> Option<impl Fn(&Option<Sample>) -> bool> {
    let overall = per_day.iter().filter_map(best_kms).min_by(f64::total_cmp)?;
    Some(move |day: &Option<Sample>| best_kms(day).is_some_and(|kms| kms <= overall + margin_kms))
}

/// PLNR-106: whether every sampled depart day is in a window, as
/// `next_window` judges one: each day's cheapest trip is within
/// `margin_kms` of the best of all. A day with no trip at all is not, nor
/// is an empty grid.
pub fn every_day_in_window(grid: &Grid, margin_kms: f64) -> bool {
    let per_day = grid.cheapest_per_day();
    window_test(&per_day, margin_kms).is_some_and(|in_window| per_day.iter().all(in_window))
}

/// What refining a preset picks among the settings near it (PLNR-86).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refine {
    /// The earliest affordable arrival, as Fastest.
    EarliestAffordable,
    /// The least delta-v, as Cheapest and Next window.
    LeastDeltaV,
}

/// PLNR-86: a preset's sample refined: every slider setting within one
/// grid spacing of it, on both sliders, is solved, and the preset picks
/// again among them. If nothing nearby qualifies, the setting nearest the
/// sample is kept. A phasing leg costs the same on every depart day
/// (PLNR-40), so it is refined on its sample's day, Soonest, alone.
fn refine(
    sample: Sample,
    rule: Refine,
    sliders: &Sliders,
    tunables: &SimTunables,
    available_kms: f64,
    solve: &impl Fn(Setting) -> Sample,
) -> Setting {
    let max_steps = tunables.slider_max_steps;
    let depart_within = sliders.depart.spacing(tunables.track_depart_samples);
    let travel_within = sliders.travel.spacing(tunables.track_travel_samples);
    let departs = if sliders.phasing() {
        vec![sample.setting.depart_day]
    } else {
        sliders
            .depart
            .settings_near(sample.setting.depart_day, depart_within)
    };
    let travels =
        sliders
            .travel
            .settings_near(sample.setting.travel_days, travel_within, max_steps);
    let near: Vec<Sample> = departs
        .iter()
        .flat_map(|&depart_day| {
            travels.iter().map(move |&travel_days| Setting {
                depart_day,
                travel_days,
            })
        })
        .map(solve)
        .collect();
    let picked = match rule {
        Refine::EarliestAffordable => fastest(near.iter(), available_kms),
        Refine::LeastDeltaV => least_delta_v(near.iter()),
    };
    picked.map_or(
        Setting {
            depart_day: sliders.depart.snap(sample.setting.depart_day),
            travel_days: sliders.travel.snap(sample.setting.travel_days, max_steps),
        },
        |sample| sample.setting,
    )
}

/// PLNR-83 to PLNR-86: the three presets for a grid, each picked from it
/// and then refined at the sliders' steps by solving with `solve`. Next
/// window exists only when the Depart range holds a window.
pub fn presets(
    grid: &Grid,
    sliders: &Sliders,
    tunables: &SimTunables,
    available_kms: f64,
    solve: impl Fn(Setting) -> Sample,
) -> Presets {
    let refined = |sample: Option<Sample>, rule| {
        sample.map(|sample| refine(sample, rule, sliders, tunables, available_kms, &solve))
    };
    let window = if sliders.windows {
        next_window(grid, tunables.window_margin_kms)
    } else {
        None
    };
    Presets {
        fastest: refined(
            fastest(grid.samples(), available_kms),
            Refine::EarliestAffordable,
        ),
        cheapest: refined(
            cheapest(grid.samples(), tunables.preset_tie_kms),
            Refine::LeastDeltaV,
        ),
        next_window: refined(window, Refine::LeastDeltaV),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::transfer::grid::tests::{rejected, sample};
    use crate::sim::transfer::order::Infeasible;
    use crate::sim::transfer::ranges::{DepartDays, Slider};

    /// PLNR-106: every day is in a window only when each day's cheapest
    /// trip is within the margin of the best; a day with no trip breaks
    /// it, and so does an empty grid.
    #[test]
    fn every_day_is_in_a_window_only_when_all_cost_about_the_same() {
        let grid = |rows: Vec<Vec<Sample>>| Grid { rows };
        let steady = grid(vec![
            vec![sample(0.0, 5.0, 4.0, 5.0), sample(0.0, 9.0, 6.0, 9.0)],
            vec![sample(1.0, 5.0, 4.4, 6.0)],
        ]);
        assert!(every_day_in_window(&steady, 0.5));
        assert!(!every_day_in_window(&steady, 0.3));
        let gap = grid(vec![
            vec![sample(0.0, 5.0, 4.0, 5.0)],
            vec![rejected(1.0, 5.0, Infeasible::NoPath)],
        ]);
        assert!(!every_day_in_window(&gap, 0.5));
        assert!(!every_day_in_window(&grid(vec![]), 0.5));
    }

    /// PLNR-83: Fastest is the earliest affordable arrival, the cheaper on
    /// a tie, and doesn't exist when nothing is affordable.
    #[test]
    fn fastest_is_the_earliest_affordable_arrival() {
        let samples = [
            sample(0.0, 5.0, 9.0, 5.0),
            sample(0.0, 10.0, 6.0, 10.0),
            sample(5.0, 5.0, 5.0, 10.0),
            rejected(0.0, 1.0, Infeasible::NoPath),
        ];
        let pick = fastest(samples.iter(), 8.0).unwrap();
        assert_eq!(pick.setting.depart_day, 5.0, "same arrival, less delta-v");
        assert_eq!(
            fastest(samples.iter(), 10.0).unwrap().setting.travel_days,
            5.0
        );
        assert_eq!(fastest(samples.iter(), 4.0), None);
    }

    /// PLNR-84: Cheapest is the least delta-v, but anything within
    /// 0.05 km/s of it counts as equal and the earliest of those wins;
    /// it may be over budget.
    #[test]
    fn cheapest_takes_the_earliest_of_the_nearly_cheapest() {
        let samples = [
            sample(0.0, 50.0, 4.00, 50.0),
            sample(0.0, 30.0, 4.04, 30.0),
            sample(0.0, 20.0, 4.06, 20.0),
        ];
        let pick = cheapest(samples.iter(), 0.05).unwrap();
        assert_eq!(pick.setting.travel_days, 30.0);
        let none: [Sample; 1] = [rejected(0.0, 1.0, Infeasible::NoPath)];
        assert_eq!(cheapest(none.iter(), 0.05), None);
    }

    /// PLNR-85: a depart day is in a window when its cheapest trip is
    /// within the margin of the cheapest of all; the next window is the
    /// earliest such day, with that day's cheapest travel time.
    #[test]
    fn the_next_window_is_the_first_day_near_the_best() {
        let grid = Grid {
            rows: vec![
                vec![sample(0.0, 10.0, 9.0, 10.0), sample(0.0, 20.0, 8.0, 20.0)],
                vec![sample(5.0, 10.0, 6.4, 15.0), sample(5.0, 20.0, 6.3, 25.0)],
                vec![sample(10.0, 10.0, 6.0, 20.0), sample(10.0, 20.0, 5.9, 30.0)],
            ],
        };
        let window = next_window(&grid, 0.5).unwrap();
        assert_eq!(window.setting.depart_day, 5.0);
        assert_eq!(window.setting.travel_days, 20.0);
        assert_eq!(next_window(&grid, 0.1).unwrap().setting.depart_day, 10.0);
    }

    /// PLNR-29 / PLNR-13: presets that don't exist or repeat an earlier
    /// one's setting are not shown; the sliders start at Fastest, or at
    /// Cheapest when nothing is affordable.
    #[test]
    fn repeated_presets_are_hidden() {
        let a = Setting {
            depart_day: 0.0,
            travel_days: 10.0,
        };
        let b = Setting {
            depart_day: 5.0,
            travel_days: 10.0,
        };
        let presets = Presets {
            fastest: Some(a),
            cheapest: Some(b),
            next_window: Some(b),
        };
        assert_eq!(
            presets.shown(),
            vec![(Preset::Fastest, a), (Preset::Cheapest, b)]
        );
        assert_eq!(presets.starting(), Some(a));
        let over = Presets {
            fastest: None,
            cheapest: Some(b),
            next_window: None,
        };
        assert_eq!(over.shown(), vec![(Preset::Cheapest, b)]);
        assert_eq!(over.starting(), Some(b));
    }

    /// PLNR-86: a preset is refined among the slider's own steps within
    /// one grid spacing of its sample: the cheapest of those for
    /// Cheapest, the earliest affordable for Fastest. Next window exists
    /// only when the Depart range holds a window.
    #[test]
    fn presets_are_refined_at_the_slider_steps() {
        let tunables = SimTunables {
            track_depart_samples: 3,
            track_travel_samples: 3,
            ..SimTunables::default()
        };
        // Depart 0..20 (1-day steps), travel 10..30 (1-day steps); the
        // grid samples them at 0, 10, 20. The true cheapest is at depart
        // 13, travel 17; delta-v grows away from it, twice as fast along
        // travel time. A trip arrives on its depart day plus its travel
        // time.
        let sliders = Sliders {
            depart: DepartDays {
                soonest: 0.0,
                end: 20.0,
                step: 1.0,
            },
            travel: Slider::Range {
                start: 10.0,
                end: 30.0,
            },
            windows: false,
        };
        let solve = |setting: Setting| {
            let kms = 3.0
                + 0.1 * (setting.depart_day - 13.0).abs()
                + 0.2 * (setting.travel_days - 17.0).abs();
            sample(
                setting.depart_day,
                setting.travel_days,
                kms,
                setting.depart_day + setting.travel_days,
            )
        };
        let mut rows = Vec::new();
        for depart in sliders.depart.samples(3) {
            rows.push(
                sliders
                    .travel
                    .samples(3)
                    .into_iter()
                    .map(|travel| {
                        solve(Setting {
                            depart_day: depart,
                            travel_days: travel,
                        })
                    })
                    .collect(),
            );
        }
        let grid = Grid { rows };
        let found = presets(&grid, &sliders, &tunables, 4.0, solve);
        assert_eq!(
            found.cheapest,
            Some(Setting {
                depart_day: 13.0,
                travel_days: 17.0
            })
        );
        // The grid's fastest affordable sample is depart 10, travel 20,
        // arriving on day 30. Nearby, leaving on day 3 for 17 days
        // arrives on day 20 for exactly the 4 km/s available, and nothing
        // affordable arrives earlier.
        assert_eq!(
            found.fastest,
            Some(Setting {
                depart_day: 3.0,
                travel_days: 17.0
            })
        );
        assert_eq!(found.next_window, None);
        let windowed = Sliders {
            windows: true,
            ..sliders
        };
        let found = presets(&grid, &windowed, &tunables, 4.0, solve);
        assert!(found.next_window.is_some());
    }

    /// PLNR-86 / PLNR-40: a phasing leg costs the same on every depart
    /// day, so its presets are refined on Soonest alone.
    #[test]
    fn a_phasing_legs_presets_are_refined_on_soonest_alone() {
        let sliders = Sliders {
            depart: DepartDays {
                soonest: 0.0,
                end: 20.0,
                step: 1.0,
            },
            travel: Slider::Positions(vec![10.0, 20.0, 30.0]),
            windows: false,
        };
        let solve_at = |setting: Setting| {
            sample(
                setting.depart_day,
                setting.travel_days,
                setting.travel_days / 10.0,
                setting.depart_day + setting.travel_days,
            )
        };
        let row = [10.0, 20.0, 30.0].map(|travel| {
            solve_at(Setting {
                depart_day: 0.0,
                travel_days: travel,
            })
        });
        let grid = Grid {
            rows: vec![row.to_vec()],
        };
        let days = std::cell::RefCell::new(Vec::new());
        let found = presets(&grid, &sliders, &SimTunables::default(), 2.5, |setting| {
            days.borrow_mut().push(setting.depart_day);
            solve_at(setting)
        });
        assert!(!days.borrow().is_empty());
        assert!(days.borrow().iter().all(|&day| day == 0.0), "{days:?}");
        assert_eq!(
            found.fastest,
            Some(Setting {
                depart_day: 0.0,
                travel_days: 10.0
            })
        );
    }
}

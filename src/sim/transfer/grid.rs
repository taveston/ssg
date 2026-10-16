//! The planner's grid (PLNR-82): the solver's result for every sampled
//! pair of depart day and travel time, and what the tracks and presets
//! read from it (PLNR-28, PLNR-85). Plain data and pure functions
//! (ARCH-20); the search that fills it is `search.rs`.

use std::mem::discriminant;

use super::order::Infeasible;

/// One setting of the planner's two sliders, in days.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Setting {
    /// The order's depart day, an absolute day (PLNR-24).
    pub depart_day: f64,
    /// The order's travel time: what the slider asks for, which a phasing
    /// leg or a trimmed Intercept may lengthen (PHAS-08, PHAS-15).
    pub travel_days: f64,
}

/// What a setting costs: the leg's delta-v in km/s and the day it ends
/// (XFER-02, XFER-04).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cost {
    pub delta_v_kms: f64,
    pub arrival_day: f64,
}

/// PLNR-82: one setting and what the solver made of it: its cost, or the
/// reason it can't be flown. A setting the ship can't pay for still has a
/// cost; whether it is affordable is a comparison with the budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub setting: Setting,
    pub result: Result<Cost, Infeasible>,
}

impl Sample {
    /// The cost, if the solver could plan the setting.
    pub fn cost(&self) -> Option<Cost> {
        self.result.ok()
    }

    /// PLNR-83: whether the ship can pay for it from `available_kms`.
    pub fn affordable(&self, available_kms: f64) -> bool {
        self.cost()
            .is_some_and(|cost| cost.delta_v_kms <= available_kms)
    }

    /// PLNR-33: whether the solver accepts it. A setting over budget is
    /// accepted: it can be chosen, though not scheduled.
    pub fn accepted(&self) -> bool {
        self.result.is_ok()
    }
}

/// PLNR-33: whether any of `samples` is accepted: a depart day on which
/// some travel time can be flown.
pub fn any_accepted(samples: &[Sample]) -> bool {
    samples.iter().any(Sample::accepted)
}

/// PLNR-92 / PLNR-35: the reason the solver gave most often among
/// `samples`, the first given on a tie; `None` when it gave none.
pub fn commonest_reason<'a>(samples: impl Iterator<Item = &'a Sample>) -> Option<Infeasible> {
    // Reasons carry figures, so they are counted by their kind.
    let mut counts: Vec<(Infeasible, usize)> = Vec::new();
    for reason in samples.filter_map(|sample| sample.result.err()) {
        let kind = discriminant(&reason);
        match counts
            .iter_mut()
            .find(|(seen, _)| discriminant(seen) == kind)
        {
            Some((_, count)) => *count += 1,
            None => counts.push((reason, 1)),
        }
    }
    let most = counts.iter().map(|(_, n)| *n).max()?;
    counts
        .into_iter()
        .find(|(_, n)| *n == most)
        .map(|(reason, _)| reason)
}

/// PLNR-82: the solver's results over the sliders' samples, one row per
/// depart column, each holding every sampled travel time.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub rows: Vec<Vec<Sample>>,
}

impl Grid {
    /// Every sample, a row at a time.
    pub fn samples(&self) -> impl Iterator<Item = &Sample> + Clone {
        self.rows.iter().flatten()
    }

    /// The depart day of each column, in order.
    pub fn depart_days(&self) -> Vec<f64> {
        self.rows
            .iter()
            .filter_map(|row| row.first().map(|s| s.setting.depart_day))
            .collect()
    }

    /// The column of `depart_day`, if the grid has one.
    pub fn column(&self, depart_day: f64) -> Option<&[Sample]> {
        self.rows
            .iter()
            .find(|row| {
                row.first()
                    .is_some_and(|s| s.setting.depart_day == depart_day)
            })
            .map(Vec::as_slice)
    }

    /// PLNR-28 / PLNR-85: for each sampled depart day, its cheapest trip
    /// of any travel time, or `None` when the solver can't plan any.
    pub fn cheapest_per_day(&self) -> Vec<Option<Sample>> {
        self.rows
            .iter()
            .map(|row| least_delta_v(row.iter()))
            .collect()
    }

    /// PLNR-28: the lowest delta-v anywhere on the grid, in km/s.
    pub fn least_delta_v_kms(&self) -> Option<f64> {
        least_delta_v(self.samples()).and_then(|sample| sample.cost().map(|c| c.delta_v_kms))
    }

    /// PLNR-12 / PLNR-92: why nothing on the grid can be planned: the
    /// reason the solver gave most often, the first given on a tie.
    /// `None` when some setting can be planned, or the grid is empty.
    pub fn unplannable_reason(&self) -> Option<Infeasible> {
        if self.samples().any(Sample::accepted) {
            return None;
        }
        commonest_reason(self.samples())
    }
}

/// The sample with least delta-v; the first on a tie. `None` if none can
/// be planned.
pub fn least_delta_v<'a>(samples: impl Iterator<Item = &'a Sample>) -> Option<Sample> {
    samples
        .filter_map(|sample| sample.cost().map(|cost| (sample, cost)))
        .fold(
            None,
            |best: Option<(&Sample, Cost)>, (sample, cost)| match best {
                Some((_, b)) if b.delta_v_kms <= cost.delta_v_kms => best,
                _ => Some((sample, cost)),
            },
        )
        .map(|(sample, _)| *sample)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A sample at `depart` and `travel` costing `kms` and ending on
    /// `arrival`.
    pub fn sample(depart: f64, travel: f64, kms: f64, arrival: f64) -> Sample {
        Sample {
            setting: Setting {
                depart_day: depart,
                travel_days: travel,
            },
            result: Ok(Cost {
                delta_v_kms: kms,
                arrival_day: arrival,
            }),
        }
    }

    /// A sample the solver rejected for `reason`.
    pub fn rejected(depart: f64, travel: f64, reason: Infeasible) -> Sample {
        Sample {
            setting: Setting {
                depart_day: depart,
                travel_days: travel,
            },
            result: Err(reason),
        }
    }

    /// PLNR-82 / PLNR-83: a setting is affordable when the solver could
    /// plan it and its delta-v is within the budget.
    #[test]
    fn a_sample_is_affordable_within_the_budget() {
        assert!(sample(0.0, 10.0, 5.0, 10.0).affordable(5.0));
        assert!(!sample(0.0, 10.0, 5.1, 10.0).affordable(5.0));
        assert!(!rejected(0.0, 10.0, Infeasible::NoPath).affordable(50.0));
    }

    /// PLNR-28 / PLNR-85: each depart day's cheapest trip, and the
    /// grid's lowest delta-v; a day the solver can't plan has none.
    #[test]
    fn each_depart_day_has_its_cheapest_trip() {
        let grid = Grid {
            rows: vec![
                vec![sample(0.0, 10.0, 6.0, 10.0), sample(0.0, 20.0, 4.0, 20.0)],
                vec![
                    rejected(5.0, 10.0, Infeasible::NoPath),
                    rejected(5.0, 20.0, Infeasible::NoPath),
                ],
                vec![sample(10.0, 10.0, 3.0, 20.0), sample(10.0, 20.0, 3.5, 30.0)],
            ],
        };
        let per_day = grid.cheapest_per_day();
        assert_eq!(per_day[0].unwrap().cost().unwrap().delta_v_kms, 4.0);
        assert_eq!(per_day[1], None);
        assert_eq!(per_day[2].unwrap().setting.travel_days, 10.0);
        assert_eq!(grid.least_delta_v_kms(), Some(3.0));
        assert_eq!(grid.unplannable_reason(), None);
    }

    /// PLNR-12 / PLNR-92: a grid with nothing plannable gives the reason
    /// the solver gave most often.
    #[test]
    fn an_unplannable_grid_gives_its_commonest_reason() {
        let periapsis = Infeasible::Periapsis { periapsis_km: 1.0 };
        let grid = Grid {
            rows: vec![vec![
                rejected(0.0, 1.0, Infeasible::NoPath),
                rejected(0.0, 2.0, periapsis),
                rejected(0.0, 3.0, Infeasible::Periapsis { periapsis_km: 2.0 }),
            ]],
        };
        assert_eq!(grid.unplannable_reason(), Some(periapsis));
        assert_eq!(Grid { rows: vec![] }.unplannable_reason(), None);
        assert_eq!(grid.least_delta_v_kms(), None);
    }
}

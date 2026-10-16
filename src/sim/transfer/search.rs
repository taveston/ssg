//! The planner's search (PLNR-80 to PLNR-87, PLNR-40): for one ship and
//! one destination, what the sliders range over, the grid of solver
//! results across them, and the presets. It works on a snapshot of the
//! system, the ship's drive and its budget (`SearchContext`), all plain
//! values (ARCH-20); `Planner::search_context` takes the snapshot.
//!
//! The solver here is run with no budget, so a setting the ship can't pay
//! for still has a delta-v: the tracks show it over budget, and Cheapest
//! may be over budget. Affordability is a comparison with the budget.

use std::f64::consts::TAU;

use orbits::meeting_days;

use super::ends::{Target, resolve};
use super::grid::{Cost, Grid, Sample, Setting};
use super::legs::solve_leg;
use super::order::{Arrival, Departure, Infeasible, Order, SolvedLeg};
use super::phase::gap_rad;
use super::places::Place;
use super::presets::{Presets, presets};
use super::ranges::{Slider, Sliders, crossing_orbits, depart_range, travel_range};
use super::tree::SystemTree;
use crate::sim::ship::Drive;
use crate::sim::tunables::SimTunables;

/// A meeting the solver flies is the one asked for when its travel time
/// matches the request within this fraction (PHAS-08).
const MEETING_MATCH_FRACTION: f64 = 1e-9;

/// What the planner is planning: where to, and how to arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request {
    pub destination: Place,
    pub arrival: Arrival,
}

/// Everything a search reads, as plain values: the system as it stands,
/// the tunables, where the ship sets out from, its drive and what it has
/// to spend, on `today`.
#[derive(Debug, Clone)]
pub struct SearchContext {
    pub tree: SystemTree,
    pub tunables: SimTunables,
    pub departure: Departure,
    pub drive: Drive,
    /// The ship's available delta-v (PLAN-21). While an order is being
    /// changed, the caller adds back what that order reserved (PLNR-27).
    pub available_kms: f64,
    pub today: f64,
}

/// A finished search: the sliders, the grid across them and the presets.
#[derive(Debug, Clone, PartialEq)]
pub struct Search {
    pub sliders: Sliders,
    pub grid: Grid,
    pub presets: Presets,
}

impl SearchContext {
    /// The order a setting makes.
    pub fn order(&self, request: Request, setting: Setting) -> Order {
        Order {
            destination: request.destination,
            arrival: request.arrival,
            depart_day: setting.depart_day,
            travel_days: setting.travel_days,
        }
    }

    /// The leg a setting makes, whatever it costs: for the path and the
    /// readouts of the setting the sliders are at (PLNR-87).
    pub fn solve(&self, request: Request, setting: Setting) -> Result<SolvedLeg, Infeasible> {
        solve_leg(
            &self.tree,
            &self.tunables,
            &self.departure,
            &self.order(request, setting),
            &self.drive,
            f64::INFINITY,
        )
    }

    /// PLNR-82: a setting with the solver's result: its cost, or why it
    /// can't be flown.
    pub fn sample(&self, request: Request, setting: Setting) -> Sample {
        Sample {
            setting,
            result: self.solve(request, setting).map(|leg| Cost {
                delta_v_kms: leg.delta_v_kms,
                arrival_day: leg.arrival_day,
            }),
        }
    }

    /// The named orbit the ship is parked in, if the destination is in
    /// it too: a phasing leg, or already there (PHAS-04, PHAS-05).
    fn own_orbit_target(&self, request: Request) -> Option<Target> {
        let Departure::Parked { orbit, .. } = self.departure else {
            return None;
        };
        let target = resolve(&self.tree, request.destination).ok()?;
        target
            .orbit()
            .is_some_and(|(o, _)| o == orbit)
            .then_some(target)
    }

    /// PLNR-80 / PLNR-81 / PLNR-22 / PLNR-40: what the sliders range
    /// over. A crossing of an impulsive drive has both sliders; a
    /// low-thrust drive's travel time is fixed by the drive; a phasing
    /// leg leaves today and its travel time is one of the meetings the
    /// solver can fly. A destination the ship is already at, or one the
    /// solver can't resolve, has no sliders: the reason is returned.
    pub fn sliders(&self, request: Request) -> Result<Sliders, Infeasible> {
        if let Some(target) = self.own_orbit_target(request) {
            return self.phasing_sliders(request, target);
        }
        let ends = crossing_orbits(
            &self.tree,
            &self.departure,
            request.destination,
            self.tunables.together_km,
            self.today,
        )?;
        let (depart, windows) = depart_range(
            self.today,
            self.drive.impulsive,
            ends.from,
            ends.to,
            &self.tunables,
        );
        let travel = if self.drive.impulsive {
            travel_range(ends.mu, ends.from, ends.to, &self.tunables)
        } else {
            // The drive sets its own travel time, whatever is asked
            // (XFER-06): the slider shows today's.
            let today = Setting {
                depart_day: self.today,
                travel_days: 0.0,
            };
            Slider::Fixed(
                self.solve(request, today)
                    .map_or(0.0, |leg| leg.travel_days),
            )
        };
        Ok(Sliders {
            depart,
            travel,
            windows,
        })
    }

    /// PLNR-40 / PHAS-06 / PHAS-08: a phasing leg leaves today, and its
    /// travel time is one of the meetings t_k the solver can fly, in
    /// order: each k up to `max_phasing_laps` whose own orbit fits. With
    /// none, the reason the solver gives (already there, a drive that
    /// can't, no orbit that fits).
    fn phasing_sliders(&self, request: Request, target: Target) -> Result<Sliders, Infeasible> {
        let (
            Departure::Parked { phase0_rad, .. },
            Target::Point {
                node,
                phase0_rad: at,
                ..
            },
        ) = (self.departure, target)
        else {
            return Err(Infeasible::AlreadyThere);
        };
        let gap_turns = gap_rad(phase0_rad, at) / TAU;
        let mut meetings = Vec::new();
        let mut first_error = None;
        for k in 1..=self.tunables.max_phasing_laps {
            let travel_days = meeting_days(node.period_days, gap_turns, k);
            let setting = Setting {
                depart_day: self.today,
                travel_days,
            };
            match self.solve(request, setting) {
                Ok(leg)
                    if (leg.travel_days - travel_days).abs()
                        <= MEETING_MATCH_FRACTION * travel_days =>
                {
                    meetings.push(travel_days);
                }
                Ok(_) => {}
                Err(reason) => {
                    first_error.get_or_insert(reason);
                }
            }
        }
        if meetings.is_empty() {
            return Err(first_error.unwrap_or(Infeasible::NoPhasingOrbit));
        }
        Ok(Sliders {
            depart: Slider::Fixed(self.today),
            travel: Slider::Positions(meetings),
            windows: false,
        })
    }

    /// PLNR-28 / PLNR-87: the Travel time track on one depart day: the
    /// travel range sampled `track_travel_samples` times, each with its
    /// result; a fixed travel time once; a phasing leg's every meeting.
    pub fn travel_track(
        &self,
        request: Request,
        sliders: &Sliders,
        depart_day: f64,
    ) -> Vec<Sample> {
        sliders
            .travel
            .samples(self.tunables.track_travel_samples)
            .into_iter()
            .map(|travel_days| {
                self.sample(
                    request,
                    Setting {
                        depart_day,
                        travel_days,
                    },
                )
            })
            .collect()
    }

    /// PLNR-82: the grid: `track_depart_samples` depart days spread over
    /// the Depart range, ends included, each with its Travel time track.
    pub fn grid(&self, request: Request, sliders: &Sliders) -> Grid {
        Grid {
            rows: sliders
                .depart
                .samples(self.tunables.track_depart_samples)
                .into_iter()
                .map(|depart_day| self.travel_track(request, sliders, depart_day))
                .collect(),
        }
    }

    /// PLNR-80 to PLNR-86: the sliders, the grid and the refined presets
    /// for a request, or why it has none.
    pub fn search(&self, request: Request) -> Result<Search, Infeasible> {
        let sliders = self.sliders(request)?;
        let grid = self.grid(request, &sliders);
        let presets = presets(
            &grid,
            &sliders,
            &self.tunables,
            self.available_kms,
            |setting| self.sample(request, setting),
        );
        Ok(Search {
            sliders,
            grid,
            presets,
        })
    }
}

/// PLNR-87: what a search depends on. The grid and presets are worked
/// out again only when this changes: the destination and its band,
/// Rendezvous or Intercept, the ship's drive, its available delta-v, or
/// the game day (so at most once a day). Moving a slider doesn't change
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchKey {
    request: Request,
    drive: Drive,
    available_kms: f64,
    day: i64,
}

impl SearchKey {
    pub fn new(request: Request, drive: Drive, available_kms: f64, today: f64) -> Self {
        Self {
            request,
            drive,
            available_kms,
            day: today.floor() as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;
    use crate::sim::transfer::places::Band;
    use crate::sim::transfer::test_layout::{Layout, layout};
    use crate::sim::transfer::tree::ShipState;

    fn context(
        l: &Layout,
        departure: Departure,
        drive: Drive,
        available_kms: f64,
    ) -> SearchContext {
        SearchContext {
            tree: l.tree.clone(),
            tunables: l.tunables.clone(),
            departure,
            drive,
            available_kms,
            today: 10.0,
        }
    }

    fn rendezvous(destination: Place) -> Request {
        Request {
            destination,
            arrival: Arrival::Rendezvous,
        }
    }

    /// PLNR-80 / PLNR-81 / PLNR-82: from the capital's High band to the
    /// inner planet's, an impulsive drive's sliders run from today over
    /// a window-to-window time and a tenth, and round the least-fuel
    /// travel time; the grid has 60 rows of 36 samples, each solved.
    #[test]
    fn an_impulsive_crossing_has_both_sliders_and_a_full_grid() {
        let l = layout();
        let ctx = context(&l, l.parked(l.capital, Band::High), l.ntr, 9.9);
        let request = rendezvous(Place::Body {
            body: l.inner,
            band: Band::High,
        });
        let sliders = ctx.sliders(request).unwrap();
        assert!(sliders.windows);
        let Slider::Range { start, end } = sliders.depart else {
            panic!("a Depart range");
        };
        assert_eq!(start, 10.0);
        let period = |body| l.tree.bodies[&body].orbit.unwrap().period_days;
        let synodic = orbits::synodic_period(period(l.inner), period(l.capital));
        assert!(
            (end - start - 1.1 * synodic).abs() < 1e-9,
            "{}",
            end - start
        );
        let Slider::Range { start, end } = sliders.travel else {
            panic!("a Travel range");
        };
        assert!((end / start - 2.0 / 0.3).abs() < 1e-9);
        let grid = ctx.grid(request, &sliders);
        assert_eq!(grid.rows.len(), 60);
        assert!(grid.rows.iter().all(|row| row.len() == 36));
        assert_eq!(grid.rows[0][0].setting.depart_day, 10.0);
        assert!(grid.samples().filter(|s| s.result.is_ok()).count() > 1000);
    }

    /// PLNR-83 to PLNR-86: the search's presets: Fastest is affordable
    /// and arrives no later than Cheapest, which costs no more; both sit
    /// on slider steps; Next window exists for a range from periods.
    #[test]
    fn the_presets_of_a_crossing_sit_on_slider_steps() {
        let l = layout();
        let ctx = context(&l, l.parked(l.capital, Band::High), l.ntr, 9.9);
        let request = rendezvous(Place::Body {
            body: l.inner,
            band: Band::High,
        });
        let search = ctx.search(request).unwrap();
        let fastest = search.presets.fastest.unwrap();
        let cheapest = search.presets.cheapest.unwrap();
        assert!(search.presets.next_window.is_some());
        let fast = ctx.solve(request, fastest).unwrap();
        let cheap = ctx.solve(request, cheapest).unwrap();
        assert!(fast.delta_v_kms <= 9.9);
        assert!(fast.arrival_day <= cheap.arrival_day + 1e-9);
        assert!(cheap.delta_v_kms <= fast.delta_v_kms + 1e-9);
        let max_steps = ctx.tunables.slider_max_steps;
        for setting in [fastest, cheapest] {
            assert!(
                search
                    .sliders
                    .depart
                    .settings(max_steps)
                    .contains(&setting.depart_day)
            );
            assert!(
                search
                    .sliders
                    .travel
                    .settings(max_steps)
                    .contains(&setting.travel_days)
            );
        }
        let grid_least = search.grid.least_delta_v_kms().unwrap();
        assert!(
            cheap.delta_v_kms <= grid_least + 1e-9,
            "refining finds no worse"
        );
    }

    /// PLNR-83: with too little delta-v nothing is affordable, so there
    /// is no Fastest, but Cheapest still exists, over budget.
    #[test]
    fn with_too_little_delta_v_only_cheapest_exists() {
        let l = layout();
        let ctx = context(&l, l.parked(l.capital, Band::High), l.ntr, 0.5);
        let request = rendezvous(Place::Body {
            body: l.inner,
            band: Band::High,
        });
        let search = ctx.search(request).unwrap();
        assert_eq!(search.presets.fastest, None);
        let cheapest = search.presets.cheapest.unwrap();
        assert!(ctx.solve(request, cheapest).unwrap().delta_v_kms > 0.5);
    }

    /// PLNR-80 / PLNR-82 / PLNR-22: a low-thrust drive has a 180-day
    /// Depart range, one travel time, the drive's own, and no Next
    /// window.
    #[test]
    fn a_low_thrust_drive_has_one_travel_time() {
        let l = layout();
        let ctx = context(&l, l.parked(l.capital, Band::High), l.nep, 24.0);
        let request = rendezvous(Place::Body {
            body: l.inner,
            band: Band::High,
        });
        let sliders = ctx.sliders(request).unwrap();
        assert!(!sliders.windows);
        assert_eq!(
            sliders.depart,
            Slider::Range {
                start: 10.0,
                end: 190.0
            }
        );
        let Slider::Fixed(travel) = sliders.travel else {
            panic!("a fixed travel time");
        };
        let leg = ctx
            .solve(
                request,
                Setting {
                    depart_day: 50.0,
                    travel_days: travel,
                },
            )
            .unwrap();
        assert!((leg.travel_days - travel).abs() < 1e-6);
        let grid = ctx.grid(request, &sliders);
        assert!(grid.rows.iter().all(|row| row.len() == 1));
        assert_eq!(ctx.search(request).unwrap().presets.next_window, None);
    }

    /// PLNR-40 / PLNR-82: a ship half a turn ahead in the ship's own High
    /// band is met by a phasing leg: it leaves today, its travel times are
    /// the meetings the solver can fly (every k, in order), and the grid
    /// has one row with a sample for each.
    #[test]
    fn a_phasing_leg_leaves_today_and_offers_its_meetings() {
        let mut l = layout();
        let orbit = l.band(l.capital, Band::High);
        let target = l.add_ship(ShipState::Parked {
            orbit,
            phase0_rad: 0.2 + PI,
        });
        let ctx = context(&l, l.parked(l.capital, Band::High), l.ntr, 9.9);
        let request = rendezvous(Place::Ship { ship: target });
        let sliders = ctx.sliders(request).unwrap();
        assert_eq!(sliders.depart, Slider::Fixed(10.0));
        let Slider::Positions(meetings) = &sliders.travel else {
            panic!("meetings");
        };
        let period = l.node(l.capital, Band::High).period_days;
        assert_eq!(meetings.len(), ctx.tunables.max_phasing_laps as usize);
        assert!((meetings[0] - 0.5 * period).abs() < 1e-9);
        assert!((meetings[2] - 2.5 * period).abs() < 1e-9);
        let grid = ctx.grid(request, &sliders);
        assert_eq!(grid.rows.len(), 1);
        assert_eq!(grid.rows[0].len(), meetings.len());
        let search = ctx.search(request).unwrap();
        assert_eq!(search.presets.next_window, None);
        let fastest = search.presets.fastest.unwrap();
        assert_eq!(fastest.travel_days, meetings[0]);
    }

    /// PLNR-40 / PLNR-42: in the Low band the first meeting can't be
    /// flown, so it is not offered; a low-thrust drive can't phase at all,
    /// and the ship's own band is already reached.
    #[test]
    fn meetings_that_cannot_be_flown_are_not_offered() {
        let mut l = layout();
        let orbit = l.band(l.capital, Band::Low);
        let target = l.add_ship(ShipState::Parked {
            orbit,
            phase0_rad: 0.2 + PI,
        });
        let ctx = context(&l, l.parked(l.capital, Band::Low), l.ntr, 9.9);
        let request = rendezvous(Place::Ship { ship: target });
        let Slider::Positions(meetings) = ctx.sliders(request).unwrap().travel else {
            panic!("meetings");
        };
        let period = l.node(l.capital, Band::Low).period_days;
        assert!((meetings[0] - 1.5 * period).abs() < 1e-9, "k = 2 first");

        let slow = context(&l, l.parked(l.capital, Band::Low), l.nep, 24.0);
        assert_eq!(
            slow.sliders(request).unwrap_err(),
            Infeasible::PhasingNeedsImpulsive
        );
        let own_band = rendezvous(Place::Body {
            body: l.capital,
            band: Band::Low,
        });
        assert_eq!(ctx.sliders(own_band).unwrap_err(), Infeasible::AlreadyThere);
        assert_eq!(ctx.search(own_band).unwrap_err(), Infeasible::AlreadyThere);
    }

    /// PLNR-87: a search is redone when the destination, the arrival
    /// mode, the drive, the available delta-v or the day changes, and not
    /// within one day.
    #[test]
    fn a_search_is_redone_only_when_its_inputs_or_the_day_change() {
        let l = layout();
        let request = rendezvous(Place::Body {
            body: l.inner,
            band: Band::High,
        });
        let key = SearchKey::new(request, l.ntr, 9.9, 10.2);
        assert_eq!(key, SearchKey::new(request, l.ntr, 9.9, 10.9));
        assert_ne!(key, SearchKey::new(request, l.ntr, 9.9, 11.0));
        assert_ne!(key, SearchKey::new(request, l.ntr, 9.0, 10.2));
        assert_ne!(key, SearchKey::new(request, l.nep, 9.9, 10.2));
        let low = rendezvous(Place::Body {
            body: l.inner,
            band: Band::Low,
        });
        assert_ne!(key, SearchKey::new(low, l.ntr, 9.9, 10.2));
        let intercept = Request {
            arrival: Arrival::Intercept { band: Band::High },
            ..request
        };
        assert_ne!(key, SearchKey::new(intercept, l.ntr, 9.9, 10.2));
    }
}

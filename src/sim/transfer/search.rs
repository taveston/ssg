//! The planner's search (PLNR-80 to PLNR-87, PLNR-40): for one ship and
//! one destination, what the sliders range over, the grid of solver
//! results across them, and the presets. It works on a snapshot of the
//! system, the ship's drive and its budget (`SearchContext`), all plain
//! values (ARCH-20); `Planner::search_context` takes the snapshot.
//!
//! The solver here is run with no budget, so a setting the ship can't pay
//! for still has a delta-v: the tracks show it over budget, and Cheapest
//! may be over budget. Affordability is a comparison with the budget.
//!
//! A search is made once for a request. As the clock runs it is moved on
//! a day at a time (`SearchContext::advance`): Soonest is solved again and
//! the columns that join the Depart range are added, but the columns
//! already solved are kept, because a depart day is an absolute day.

use std::f64::consts::TAU;

use orbits::meeting_days;

use super::ends::{Target, resolve};
use super::grid::{Cost, Grid, Sample, Setting};
use super::legs::{solve_leg, solve_leg_near};
use super::order::{Arrival, Departure, Infeasible, Order, SolvedLeg};
use super::phase::gap_rad;
use super::places::{Band, Place};
use super::presets::{Presets, presets};
use super::ranges::{
    Slider, Sliders, crossing_orbits, days_slider, depart_range, travel_range, window_wait_interval,
};
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
    /// PLNR-30: whether the destination may be intercepted.
    pub intercept_offered: bool,
    /// PHAS-16: for a leg that waits for its window, when the window came
    /// on Soonest, so other days can look for it there.
    pub windows: Option<Windows>,
}

/// PHAS-16: when a leg that waits for its window set out on Soonest, for
/// each travel time of the Travel time track. The two ends stand the same
/// way again once every window interval, so on any other day the window
/// is expected that much later, modulo the interval.
#[derive(Debug, Clone, PartialEq)]
pub struct Windows {
    pub interval_days: f64,
    /// The day each travel time's crossing set out on, before its burn
    /// days; `None` where the leg couldn't be flown.
    pub set_out: Vec<Option<f64>>,
}

impl Windows {
    /// The wait after `depart_day` at which travel time `index`'s window
    /// is expected.
    pub fn wait_on(&self, index: usize, depart_day: f64) -> Option<f64> {
        let set_out = (*self.set_out.get(index)?)?;
        Some((set_out - depart_day).rem_euclid(self.interval_days))
    }
}

impl Search {
    /// PLNR-21: the Soonest setting's depart day: the clock's time when
    /// the search was made or last moved on.
    pub fn soonest(&self) -> f64 {
        match self.sliders.depart {
            Slider::Days { soonest, .. } => soonest,
            _ => self
                .sliders
                .depart
                .settings(1)
                .first()
                .copied()
                .unwrap_or(0.0),
        }
    }
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

    /// PLNR-30: whether the destination may be intercepted. Not in the
    /// ship's own orbit, which is met by a phasing leg, always a
    /// Rendezvous (PHAS-05); nor through the Low band of the crossing's own
    /// primary, which only one travel time could fly (XFER-07).
    pub fn offers_intercept(&self, destination: Place) -> bool {
        let request = Request {
            destination,
            arrival: Arrival::Rendezvous,
        };
        if self.own_orbit_target(request).is_some() {
            return false;
        }
        let Ok(target) = resolve(&self.tree, destination) else {
            return true;
        };
        let Some((_, node)) = target.orbit() else {
            return true;
        };
        let primary = crossing_orbits(
            &self.tree,
            &self.departure,
            destination,
            self.tunables.together_km,
            self.today,
        )
        .map(|ends| ends.primary);
        !(node.band == Some(Band::Low) && primary == Ok(node.body))
    }

    /// PLNR-80 / PLNR-81 / PLNR-22 / PLNR-40: what the sliders range
    /// over. Every leg has a Depart slider in whole days from Soonest. A
    /// crossing of an impulsive drive has a Travel range; a low-thrust
    /// drive's travel time is fixed by the drive; a phasing leg's travel
    /// time is one of the meetings the solver can fly. A destination the
    /// ship is already at, or one the solver can't resolve, has no
    /// sliders: the reason is returned.
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

    /// PLNR-40 / PHAS-06 / PHAS-08: a phasing leg may leave on any day of
    /// `depart_range_days`, and its travel time is one of the meetings t_k
    /// the solver can fly, in order: each k up to `max_phasing_laps` whose
    /// own orbit fits. The gap never changes, so the meetings are the same
    /// on every day. With none, the reason the solver gives (already
    /// there, a drive that can't, no orbit that fits).
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
            depart: days_slider(self.today, self.tunables.depart_range_days, &self.tunables),
            travel: Slider::Positions(meetings),
            windows: false,
        })
    }

    /// PHAS-16: the window interval of a leg that waits for its window,
    /// or `None`.
    fn window_interval(&self, request: Request) -> Option<f64> {
        if self.own_orbit_target(request).is_some() {
            return None;
        }
        let ends = crossing_orbits(
            &self.tree,
            &self.departure,
            request.destination,
            self.tunables.together_km,
            self.today,
        )
        .ok()?;
        window_wait_interval(self.drive.impulsive, ends.from, ends.to)
    }

    /// PLNR-28 / PLNR-87 / PHAS-16: the Travel time track on one depart
    /// day, with, for each travel time, the day its crossing set out on.
    /// Where `windows` says when the windows came on another day, each
    /// leg looks for its window there first.
    fn track_near(
        &self,
        request: Request,
        sliders: &Sliders,
        depart_day: f64,
        windows: Option<&Windows>,
    ) -> (Vec<Sample>, Vec<Option<f64>>) {
        sliders
            .travel
            .samples(self.tunables.track_travel_samples)
            .into_iter()
            .enumerate()
            .map(|(j, travel_days)| {
                let setting = Setting {
                    depart_day,
                    travel_days,
                };
                let hint = windows.and_then(|w| w.wait_on(j, depart_day));
                let leg = solve_leg_near(
                    &self.tree,
                    &self.tunables,
                    &self.departure,
                    &self.order(request, setting),
                    &self.drive,
                    f64::INFINITY,
                    hint,
                );
                let set_out = leg
                    .as_ref()
                    .ok()
                    .map(|leg| depart_day + leg.departure_wait_days);
                let sample = Sample {
                    setting,
                    result: leg.map(|leg| Cost {
                        delta_v_kms: leg.delta_v_kms,
                        arrival_day: leg.arrival_day,
                    }),
                };
                (sample, set_out)
            })
            .unzip()
    }

    /// PLNR-28 / PLNR-87: the Travel time track of a search's request on
    /// one depart day: the travel range sampled `track_travel_samples`
    /// times, each with its result; a fixed travel time once; a phasing
    /// leg's every meeting. Each leg looks for its window where the search
    /// found it on Soonest (PHAS-16).
    pub fn search_track(&self, request: Request, search: &Search, depart_day: f64) -> Vec<Sample> {
        self.track_near(
            request,
            &search.sliders,
            depart_day,
            search.windows.as_ref(),
        )
        .0
    }

    /// PLNR-82: the grid's depart columns: Soonest and the Depart
    /// positions, thinned to at most `track_depart_samples`. A phasing
    /// leg's meetings are the same on every day, so it has Soonest alone
    /// (PLNR-40).
    pub fn depart_columns(&self, sliders: &Sliders) -> Vec<f64> {
        let columns = sliders.depart.samples(self.tunables.track_depart_samples);
        if sliders.phasing() {
            columns.into_iter().take(1).collect()
        } else {
            columns
        }
    }

    /// PLNR-82 / PHAS-16: the Soonest column, solved in full, and for a
    /// leg that waits for its window, when the window came on it.
    fn soonest_column(
        &self,
        request: Request,
        sliders: &Sliders,
        soonest: f64,
    ) -> (Vec<Sample>, Option<Windows>) {
        let (row, set_out) = self.track_near(request, sliders, soonest, None);
        let windows = self.window_interval(request).map(|interval_days| Windows {
            interval_days,
            set_out,
        });
        (row, windows)
    }

    /// PLNR-82: the grid: each depart column with its Travel time track,
    /// and when the windows came on Soonest. A leg that waits for its
    /// window (PHAS-16) costs the same whenever the two ends stand the
    /// same way, which they do again once every window interval, so the
    /// other columns look for their windows where Soonest found them.
    pub fn grid_and_windows(&self, request: Request, sliders: &Sliders) -> (Grid, Option<Windows>) {
        let columns = self.depart_columns(sliders);
        let Some((&soonest, later)) = columns.split_first() else {
            return (Grid { rows: Vec::new() }, None);
        };
        let (first, windows) = self.soonest_column(request, sliders, soonest);
        let mut rows = vec![first];
        rows.extend(
            later
                .iter()
                .map(|&day| self.track_near(request, sliders, day, windows.as_ref()).0),
        );
        (Grid { rows }, windows)
    }

    /// PLNR-24 / PLNR-87: a search moved on to `now`, once a game day at
    /// most. Soonest becomes `now` and its column is solved again; the
    /// columns whose day the clock has reached leave the grid, and those
    /// that now fall in the Depart range join it; every other column is
    /// kept as it was solved. The presets are not picked again, only
    /// moved on as thumbs are. Returns the depart days solved.
    pub fn advance(&mut self, request: Request, search: &mut Search, now: f64) -> Vec<f64> {
        let was_soonest = search.soonest();
        if let Slider::Days { soonest, end, step } = search.sliders.depart {
            search.sliders.depart = Slider::Days {
                soonest: now,
                end: now + (end - soonest),
                step,
            };
        }
        self.today = now;
        let mut kept = std::mem::take(&mut search.grid.rows);
        let (first, windows) = self.soonest_column(request, &search.sliders, now);
        search.windows = windows;
        search.grid.rows.push(first);
        let mut solved = vec![now];
        for &day in self.depart_columns(&search.sliders).iter().skip(1) {
            let old = kept
                .iter()
                .position(|row| row.first().is_some_and(|s| s.setting.depart_day == day));
            let row = match old {
                Some(index) => kept.swap_remove(index),
                None => {
                    solved.push(day);
                    self.search_track(request, search, day)
                }
            };
            search.grid.rows.push(row);
        }
        search.presets = search.presets.moved_on(was_soonest, now);
        solved
    }

    /// PLNR-80 to PLNR-86: the sliders, the grid and the refined presets
    /// for a request, or why it has none.
    pub fn search(&self, request: Request) -> Result<Search, Infeasible> {
        let sliders = self.sliders(request)?;
        let (grid, windows) = self.grid_and_windows(request, &sliders);
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
            intercept_offered: self.offers_intercept(request.destination),
            windows,
        })
    }
}

/// PLNR-87: what a search depends on. The grid and presets are worked
/// out again only when this changes: the destination and its band,
/// Rendezvous or Intercept, the ship's drive or its available delta-v.
/// Neither a slider move nor the clock changes it: the clock only moves
/// the search on (`SearchContext::advance`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchKey {
    request: Request,
    drive: Drive,
    available_kms: f64,
}

impl SearchKey {
    pub fn new(request: Request, drive: Drive, available_kms: f64) -> Self {
        Self {
            request,
            drive,
            available_kms,
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
    /// inner planet's, an impulsive drive's sliders run from Soonest over
    /// a window-to-window time and a tenth, and round the least-fuel
    /// travel time; the grid has up to 60 columns of 36 samples, each
    /// solved, Soonest first.
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
        let Slider::Days {
            soonest: start,
            end,
            ..
        } = sliders.depart
        else {
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
        let grid = ctx.grid_and_windows(request, &sliders).0;
        assert!((40..=60).contains(&grid.rows.len()), "{}", grid.rows.len());
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
            Slider::Days {
                soonest: 10.0,
                end: 190.0,
                step: 1.0,
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
        let grid = ctx.grid_and_windows(request, &sliders).0;
        assert!(grid.rows.iter().all(|row| row.len() == 1));
        assert_eq!(ctx.search(request).unwrap().presets.next_window, None);
    }

    /// PLNR-40 / PLNR-82: a ship half a turn ahead in the ship's own High
    /// band is met by a phasing leg: it may leave on any day of 180, its
    /// travel times are the meetings the solver can fly (every k, in
    /// order), and the grid has one column, Soonest, with a sample for
    /// each: every day costs the same. It is never an Intercept.
    #[test]
    fn a_phasing_leg_may_leave_any_day_and_offers_its_meetings() {
        let mut l = layout();
        let orbit = l.band(l.capital, Band::High);
        let target = l.add_ship(ShipState::Parked {
            orbit,
            phase0_rad: 0.2 + PI,
        });
        let ctx = context(&l, l.parked(l.capital, Band::High), l.ntr, 9.9);
        let request = rendezvous(Place::Ship { ship: target });
        let sliders = ctx.sliders(request).unwrap();
        assert_eq!(
            sliders.depart,
            Slider::Days {
                soonest: 10.0,
                end: 190.0,
                step: 1.0
            }
        );
        let Slider::Positions(meetings) = &sliders.travel else {
            panic!("meetings");
        };
        let period = l.node(l.capital, Band::High).period_days;
        assert_eq!(meetings.len(), ctx.tunables.max_phasing_laps as usize);
        assert!((meetings[0] - 0.5 * period).abs() < 1e-9);
        assert!((meetings[2] - 2.5 * period).abs() < 1e-9);
        let grid = ctx.grid_and_windows(request, &sliders).0;
        assert_eq!(grid.rows.len(), 1);
        assert_eq!(grid.rows[0].len(), meetings.len());
        let search = ctx.search(request).unwrap();
        assert_eq!(search.presets.next_window, None);
        let fastest = search.presets.fastest.unwrap();
        assert_eq!(fastest.travel_days, meetings[0]);
        assert!(!search.intercept_offered);
        let later = ctx
            .solve(
                request,
                Setting {
                    depart_day: 40.0,
                    ..fastest
                },
            )
            .unwrap();
        let now = ctx.solve(request, fastest).unwrap();
        assert!((later.delta_v_kms - now.delta_v_kms).abs() < 1e-9);
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
    /// mode, the drive or the available delta-v changes; the clock is not
    /// part of it.
    #[test]
    fn a_search_is_redone_only_when_its_inputs_change() {
        let l = layout();
        let request = rendezvous(Place::Body {
            body: l.inner,
            band: Band::High,
        });
        let key = SearchKey::new(request, l.ntr, 9.9);
        assert_eq!(key, SearchKey::new(request, l.ntr, 9.9));
        assert_ne!(key, SearchKey::new(request, l.ntr, 9.0));
        assert_ne!(key, SearchKey::new(request, l.nep, 9.9));
        let low = rendezvous(Place::Body {
            body: l.inner,
            band: Band::Low,
        });
        assert_ne!(key, SearchKey::new(low, l.ntr, 9.9));
        let intercept = Request {
            arrival: Arrival::Intercept { band: Band::High },
            ..request
        };
        assert_ne!(key, SearchKey::new(intercept, l.ntr, 9.9));
    }

    /// PLNR-24 / PLNR-87: moving a search on a day solves only Soonest
    /// and the columns that join the range; every other column keeps its
    /// day and its results, and the columns whose day came are gone.
    /// The presets keep their settings, one on a day that came moving to
    /// Soonest.
    #[test]
    fn moving_on_a_day_solves_only_the_new_columns() {
        let l = layout();
        let mut ctx = context(&l, l.parked(l.capital, Band::High), l.nep, 24.0);
        let request = rendezvous(Place::Body {
            body: l.inner,
            band: Band::High,
        });
        let mut search = ctx.search(request).unwrap();
        let before = search.grid.clone();
        // A daily step over 180 days: every 4th day is a column.
        assert_eq!(&before.depart_days()[..3], &[10.0, 12.0, 16.0]);
        search.presets.cheapest = Some(Setting {
            depart_day: 12.0,
            travel_days: 1.0,
        });
        let solved = ctx.advance(request, &mut search, 12.25);
        assert_eq!(ctx.today, 12.25);
        assert_eq!(search.soonest(), 12.25);
        assert_eq!(solved, vec![12.25, 192.0]);
        let days = search.grid.depart_days();
        assert_eq!(&days[..3], &[12.25, 16.0, 20.0]);
        assert_eq!(*days.last().unwrap(), 192.0);
        for day in &days[1..days.len() - 1] {
            assert_eq!(
                search.grid.column(*day),
                before.column(*day),
                "day {day} kept"
            );
        }
        assert_eq!(search.grid.column(12.0), None, "its day has come");
        assert_eq!(
            search.presets.cheapest,
            Some(Setting {
                depart_day: 12.25,
                travel_days: 1.0
            })
        );
        let Slider::Days { soonest, end, .. } = search.sliders.depart else {
            panic!("a Depart slider");
        };
        assert_eq!((soonest, end), (12.25, 192.25));
    }

    /// PHAS-16 / PLNR-82: to a station in the capital's Low band the leg
    /// waits for its window, so the grid finds each travel time's window
    /// in full on Soonest, and on the other days looks for it where the
    /// ends stand the same way. What it finds is what the solver finds in
    /// full, but for a few settings, and never dearer.
    #[test]
    fn a_window_legs_grid_looks_for_its_windows_where_soonest_found_them() {
        let l = layout();
        let ctx = context(&l, l.parked(l.capital, Band::High), l.ntr, 9.9);
        let request = rendezvous(Place::Station { station: l.station });
        let search = ctx.search(request).unwrap();
        assert!(!search.sliders.windows, "no Next window");
        let windows = search.windows.clone().expect("a leg that waits");
        assert!(windows.interval_days < 1.0 / 12.0);
        let wait = windows.wait_on(0, 12.0).unwrap();
        assert!((0.0..windows.interval_days).contains(&wait));
        let mut differ = 0;
        for sample in search.grid.samples() {
            let full = ctx.sample(request, sample.setting);
            if full != *sample {
                differ += 1;
                if let (Ok(full), Ok(found)) = (full.result, sample.result) {
                    assert!(found.delta_v_kms <= full.delta_v_kms + 1e-9);
                }
            }
        }
        let cells = search.grid.samples().count();
        assert!(differ * 20 < cells, "{differ} of {cells} differ");
    }

    /// PLNR-30: an Intercept is offered at another body and at a station
    /// in another body's band, but not through the Low band of the
    /// crossing's own primary: its whole band or a station in it.
    #[test]
    fn no_intercept_through_the_primarys_own_low_band() {
        let l = layout();
        let ctx = context(&l, l.parked(l.capital, Band::High), l.ntr, 9.9);
        let inner_low = Place::Body {
            body: l.inner,
            band: Band::Low,
        };
        assert!(ctx.offers_intercept(inner_low));
        assert!(ctx.offers_intercept(Place::Station {
            station: l.far_station
        }));
        assert!(!ctx.offers_intercept(Place::Body {
            body: l.capital,
            band: Band::Low
        }));
        assert!(!ctx.offers_intercept(Place::Station { station: l.station }));
        let from_low = context(&l, l.parked(l.capital, Band::Low), l.ntr, 9.9);
        assert!(from_low.offers_intercept(Place::Body {
            body: l.capital,
            band: Band::High
        }));
    }
}

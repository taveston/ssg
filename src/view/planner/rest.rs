//! Where the planner's thumbs may come to rest (PLNR-24, PLNR-33,
//! PLNR-34, PLNR-87): a setting the solver rejects can't be chosen, so a
//! thumb sent onto one moves to the nearest setting that isn't rejected;
//! and as the clock runs the search is moved on a day at a time, the
//! thumbs with it.
//!
//! Whether a setting is rejected is read from what the search already
//! holds wherever it can: the grid's columns on the Depart track, the
//! Travel time track on the current depart day. A setting between two
//! samples that are both rejected is rejected too, as the track draws it;
//! anything else is solved, and a Travel time track solved for a depart
//! day off the grid is kept for the session.

use super::slider::{nearest_accepted, nearest_index, step_accepted};
use super::state::PlannerSession;
use crate::sim::transfer::grid::{Sample, Setting, any_accepted};
use crate::sim::transfer::presets::moved_on;
use crate::sim::transfer::search::{Request, Search, SearchContext};

/// What deciding where a thumb rests reads: the search, the snapshot it
/// solves against, the request, and the tracks solved so far.
pub struct Rest<'a> {
    context: &'a SearchContext,
    search: &'a Search,
    request: Request,
    tracks: &'a mut Vec<Vec<Sample>>,
}

/// The rest rules for an open planner's search; `None` until it has one.
pub fn rest_of(session: &mut PlannerSession) -> Option<Rest<'_>> {
    let PlannerSession {
        found,
        tracks,
        request,
        ..
    } = session;
    let found = found.as_ref()?;
    Some(Rest {
        context: found.context.as_ref()?,
        search: found.result.as_ref().ok()?,
        request: (*request)?,
        tracks,
    })
}

/// The settings of the two samples either side of `value`, of those
/// that exist.
fn neighbours(values: &[f64], value: f64) -> [Option<f64>; 2] {
    [
        values
            .iter()
            .copied()
            .filter(|v| *v < value)
            .reduce(f64::max),
        values
            .iter()
            .copied()
            .filter(|v| *v > value)
            .reduce(f64::min),
    ]
}

impl Rest<'_> {
    fn max_steps(&self) -> u32 {
        self.context.tunables.slider_max_steps
    }

    /// PLNR-34: whether anything on the grid is accepted. With nothing,
    /// the thumbs stay where they are.
    fn anything_accepted(&self) -> bool {
        self.search.grid.samples().any(Sample::accepted)
    }

    /// PLNR-28 / PLNR-87: the Travel time track on `depart_day`: the
    /// grid's column there, one solved before, or one solved now and
    /// kept.
    pub fn track_on(&mut self, depart_day: f64) -> Vec<Sample> {
        if let Some(column) = self.search.grid.column(depart_day) {
            return column.to_vec();
        }
        let solved = |row: &&Vec<Sample>| {
            row.first()
                .is_some_and(|s| s.setting.depart_day == depart_day)
        };
        if let Some(track) = self.tracks.iter().find(solved) {
            return track.clone();
        }
        let track = self
            .context
            .search_track(self.request, self.search, depart_day);
        self.tracks.push(track.clone());
        track
    }

    /// PLNR-33: whether some travel time is accepted on `day`. A phasing
    /// leg's meetings are the same on every day (PLNR-40).
    fn depart_accepted(&mut self, day: f64) -> bool {
        let search = self.search;
        let grid = &search.grid;
        if let Some(column) = grid.column(day) {
            return any_accepted(column);
        }
        if search.sliders.phasing() {
            return grid.rows.first().is_some_and(|row| any_accepted(row));
        }
        let rejected = |d: f64| grid.column(d).is_none_or(|c| !any_accepted(c));
        let around = neighbours(&grid.depart_days(), day);
        if around.iter().any(Option::is_some) && around.iter().flatten().all(|d| rejected(*d)) {
            return false;
        }
        any_accepted(&self.track_on(day))
    }

    /// PLNR-33: whether the leg is accepted on `depart_day` with
    /// `travel_days`.
    fn travel_accepted(&mut self, depart_day: f64, travel_days: f64) -> bool {
        let track = self.track_on(depart_day);
        if let Some(sample) = track.iter().find(|s| s.setting.travel_days == travel_days) {
            return sample.accepted();
        }
        let values: Vec<f64> = track.iter().map(|s| s.setting.travel_days).collect();
        let rejected = |t: f64| {
            track
                .iter()
                .find(|s| s.setting.travel_days == t)
                .is_none_or(|s| !s.accepted())
        };
        let around = neighbours(&values, travel_days);
        if around.iter().any(Option::is_some) && around.iter().flatten().all(|t| rejected(*t)) {
            return false;
        }
        let setting = Setting {
            depart_day,
            travel_days,
        };
        self.context.sample(self.request, setting).accepted()
    }

    /// PLNR-33: where the Depart thumb rests when sent to `value`: the
    /// nearest Depart position on which some travel time is accepted.
    /// `None` when there is none, so it stays.
    pub fn depart_at(&mut self, value: f64) -> Option<f64> {
        if !self.anything_accepted() {
            return None;
        }
        let settings = self.search.sliders.depart.settings(self.max_steps());
        let index = nearest_index(&settings, value);
        nearest_accepted(settings.len(), index, |i| self.depart_accepted(settings[i]))
            .map(|i| settings[i])
    }

    /// PLNR-33: where the Travel time thumb rests on `depart_day` when
    /// sent to `value`. `None` when nothing is accepted, so it stays.
    pub fn travel_at(&mut self, depart_day: f64, value: f64) -> Option<f64> {
        if !self.anything_accepted() {
            return None;
        }
        let settings = self.search.sliders.travel.settings(self.max_steps());
        let index = nearest_index(&settings, value);
        nearest_accepted(settings.len(), index, |i| {
            self.travel_accepted(depart_day, settings[i])
        })
        .map(|i| settings[i])
    }

    /// PLNR-23 / PLNR-33: a key step of the Depart thumb from `from`: on
    /// to the next accepted position that way, or `None` to stay.
    pub fn depart_step(&mut self, from: f64, steps: i32) -> Option<f64> {
        if !self.anything_accepted() {
            return None;
        }
        let settings = self.search.sliders.depart.settings(self.max_steps());
        let index = nearest_index(&settings, from);
        step_accepted(settings.len(), index, steps, |i| {
            self.depart_accepted(settings[i])
        })
        .map(|i| settings[i])
    }

    /// PLNR-23 / PLNR-33: a key step of the Travel time thumb from
    /// `from` on `depart_day`.
    pub fn travel_step(&mut self, depart_day: f64, from: f64, steps: i32) -> Option<f64> {
        if !self.anything_accepted() {
            return None;
        }
        let settings = self.search.sliders.travel.settings(self.max_steps());
        let index = nearest_index(&settings, from);
        step_accepted(settings.len(), index, steps, |i| {
            self.travel_accepted(depart_day, settings[i])
        })
        .map(|i| settings[i])
    }

    /// PLNR-33: a setting with each thumb moved off a rejected setting:
    /// the depart day first, then the travel time on it.
    pub fn settle(&mut self, setting: Setting) -> Setting {
        if !self.anything_accepted() {
            return setting;
        }
        let depart_day = if self.depart_accepted(setting.depart_day) {
            setting.depart_day
        } else {
            self.depart_at(setting.depart_day)
                .unwrap_or(setting.depart_day)
        };
        let travel_days = if self.travel_accepted(depart_day, setting.travel_days) {
            setting.travel_days
        } else {
            self.travel_at(depart_day, setting.travel_days)
                .unwrap_or(setting.travel_days)
        };
        Setting {
            depart_day,
            travel_days,
        }
    }
}

/// PLNR-33: the session's setting with each thumb moved off a rejected
/// setting.
pub fn settle(session: &mut PlannerSession) {
    let Some(setting) = session.setting else {
        return;
    };
    if let Some(mut rest) = rest_of(session) {
        let settled = rest.settle(setting);
        session.setting = Some(settled);
    }
}

/// PLNR-24 / PLNR-87: the search moved on to `now`. Soonest becomes
/// `now`; the grid gains the columns that now fall in the range and
/// loses those whose day has come; a thumb at Soonest, or on a day that
/// has come, moves to Soonest with its travel time kept; and, when
/// `settle` is set, a thumb left on a rejected setting moves off it
/// (PLNR-33). The leg and the Travel time track are solved again.
pub fn advance_session(session: &mut PlannerSession, now: f64, settle_thumbs: bool) {
    let Some(request) = session.request else {
        return;
    };
    let Some(found) = session.found.as_mut() else {
        return;
    };
    let (Some(context), Ok(search)) = (found.context.as_mut(), found.result.as_mut()) else {
        return;
    };
    let was_soonest = search.soonest();
    context.advance(request, search, now);
    session.setting = session.setting.map(|s| moved_on(s, was_soonest, now));
    session
        .tracks
        .retain(|track| track.first().is_some_and(|s| s.setting.depart_day > now));
    session.solved_for = None;
    session.travel_track_day = None;
    if settle_thumbs {
        settle(session);
    }
}

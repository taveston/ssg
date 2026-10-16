//! The planner's working out (PLNR-11 to PLNR-13, PLNR-24, PLNR-30,
//! PLNR-33, PLNR-34, PLNR-87): what a click picks, the search for the
//! request, when it is run again and when it is moved on with the clock,
//! the leg of the current setting, the Travel time track, and the
//! preview of a hovered place. The rules are the simulation's
//! (`sim/transfer/`, reached through `Planner`); these systems decide only
//! when to ask, and keep the answers in `PlannerState` for the panel and
//! the map.

use bevy::prelude::*;

use super::pick::Pickables;
use super::rest::{advance_session, rest_of, settle};
use super::state::{Found, HoverOutcome, HoverPreview, PickTarget, PlannerSession, PlannerState};
use crate::sim::clock::GameClock;
use crate::sim::named_orbit::NamedOrbit;
use crate::sim::transfer::grid::{Sample, Setting};
use crate::sim::transfer::order::{Arrival, Infeasible};
use crate::sim::transfer::places::Place;
use crate::sim::transfer::plan::Planner;
use crate::sim::transfer::ranges::{Slider, Sliders, crossing_orbits};
use crate::sim::transfer::search::{Request, Search, SearchContext, SearchKey};
use crate::view::hovered::HoveredBodies;

/// PLNR-11: what can't be picked takes no hover mark and no click, so
/// the set is kept up to date while the planner is open.
pub fn mark_unpickable(mut state: ResMut<PlannerState>, pickables: Pickables) {
    let Some(ship) = state.session.as_ref().map(|s| s.ship) else {
        return;
    };
    state.unpickable = pickables.unpickable(ship).collect();
}

/// PLNR-11 / PLNR-13: a click that hit a place starts tuning it, as a
/// Rendezvous, with the sliders at its starting preset.
pub fn resolve_clicks(mut state: ResMut<PlannerState>, pickables: Pickables) {
    let Some(target) = state.clicked.take() else {
        return;
    };
    let Some(session) = state.session.as_mut() else {
        return;
    };
    if let Some(destination) = pickables.place(target, session.ship) {
        session.pick(Request {
            destination,
            arrival: Arrival::Rendezvous,
        });
    }
}

/// PLNR-87: the search's key for `request`: the request, the ship's drive
/// and its available delta-v (with what an order being changed
/// reserved). `None` for a ship with no drive.
fn search_key(
    planner: &Planner,
    ship: Entity,
    request: Request,
    changing: f64,
) -> Option<SearchKey> {
    let (drive, available) = planner.budget(ship)?;
    Some(SearchKey::new(request, drive, available + changing))
}

/// PLNR-30: the request as it can be planned: an Intercept the
/// destination doesn't offer is a Rendezvous.
fn offered(context: &SearchContext, request: Request) -> Request {
    if request.arrival.is_intercept() && !context.offers_intercept(request.destination) {
        Request {
            arrival: Arrival::Rendezvous,
            ..request
        }
    } else {
        request
    }
}

/// PLNR-80 to PLNR-86: search for `request` from the system as it stands,
/// counting what an order being changed reserved as available (PLNR-27).
/// Returns the search and the request it was made for, which is a
/// Rendezvous where the destination offers no Intercept (PLNR-30).
fn run_search(
    planner: &Planner,
    ship: Entity,
    request: Request,
    changing: f64,
) -> (Found, Request) {
    match planner.search_context(ship) {
        Ok(mut context) => {
            context.available_kms += changing;
            let request = offered(&context, request);
            let key = search_key(planner, ship, request, changing);
            let result = context.search(request);
            let primary = crossing_orbits(
                &context.tree,
                &context.departure,
                request.destination,
                context.tunables.together_km,
                context.today,
            )
            .ok()
            .map(|ends| ends.primary);
            let found = Found {
                key,
                context: Some(context),
                result,
                primary,
            };
            (found, request)
        }
        Err(reason) => {
            let found = Found {
                key: search_key(planner, ship, request, changing),
                context: None,
                result: Err(reason),
                primary: None,
            };
            (found, request)
        }
    }
}

/// PLNR-13: where the sliders start: the Fastest setting, or Cheapest if
/// nothing is affordable, or the start of both ranges if nothing can be
/// planned at all (so the reason shows).
pub fn starting_setting(search: &Search, max_steps: u32) -> Setting {
    search.presets.starting().unwrap_or_else(|| Setting {
        depart_day: search.sliders.depart.settings(max_steps)[0],
        travel_days: search.sliders.travel.settings(max_steps)[0],
    })
}

/// PLNR-13 / PLNR-40 / PLNR-22: a setting kept across a new search: on
/// the nearest Depart position, within the Travel range, on a meeting for
/// a phasing leg, and the fixed value where there is no slider.
pub fn fitted(setting: Setting, sliders: &Sliders, max_steps: u32) -> Setting {
    let fit = |slider: &Slider, value: f64| match slider {
        Slider::Range { start, end } => value.clamp(*start, end.max(*start)),
        _ => slider.snap(value, max_steps),
    };
    Setting {
        depart_day: fit(&sliders.depart, setting.depart_day),
        travel_days: fit(&sliders.travel, setting.travel_days),
    }
}

/// PLNR-87: whether the clock has moved into a later game day than the
/// one the search was last moved on to.
fn day_has_passed(session: &PlannerSession, now: f64) -> bool {
    session
        .search()
        .is_some_and(|search| now.floor() > search.soonest().floor())
}

/// PLNR-87: the grid and presets are worked out again when the request,
/// the ship's drive or its available delta-v changes; a slider move never
/// does it, and nor does the clock, which only moves the search on, once
/// a game day (PLNR-24). The setting is kept across a new search, with a
/// thumb on a rejected setting moved off it (PLNR-33), or starts at its
/// preset after a pick (PLNR-13).
pub fn refresh_search(mut state: ResMut<PlannerState>, planner: Planner, clock: Res<GameClock>) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    let Some(request) = session.request else {
        return;
    };
    let now = clock.elapsed_days();
    let changing = session.changing.unwrap_or(0.0);
    let key = search_key(&planner, session.ship, request, changing);
    if session.found.as_ref().is_some_and(|found| found.key == key) {
        if day_has_passed(session, now) {
            advance_session(session, now, true);
        }
        return;
    }
    let (found, request) = run_search(&planner, session.ship, request, changing);
    session.request = Some(request);
    let mut kept = false;
    if let (Ok(search), Some(context)) = (&found.result, &found.context) {
        let max_steps = context.tunables.slider_max_steps;
        session.setting = Some(match session.setting {
            Some(setting) => {
                kept = true;
                fitted(setting, &search.sliders, max_steps)
            }
            None => starting_setting(search, max_steps),
        });
    }
    session.found = Some(found);
    session.solved_for = None;
    session.travel_track_day = None;
    session.tracks.clear();
    if kept {
        settle(session);
    }
}

/// PLNR-28 / PLNR-87: the Travel time track on `depart_day`: the grid's
/// column there, or one solved for it.
fn track_on(session: &mut PlannerSession, depart_day: f64) -> Vec<Sample> {
    rest_of(session).map_or_else(Vec::new, |mut rest| rest.track_on(depart_day))
}

/// PLNR-87 / PLNR-34: the leg of the setting is solved when the setting
/// changes, and the Travel time track when the depart day does. With
/// nothing on the grid accepted there is no leg, only the grid's reason.
pub fn solve_current(session: &mut PlannerSession) {
    let Some(found) = session.found.as_ref() else {
        return;
    };
    let (Some(context), Ok(search)) = (&found.context, &found.result) else {
        session.leg = found.result.as_ref().err().map(|why| Err(*why));
        return;
    };
    let (Some(request), Some(setting)) = (session.request, session.setting) else {
        return;
    };
    if session.solved_for != Some(setting) {
        session.leg = Some(match search.grid.unplannable_reason() {
            Some(why) => Err(why),
            None => context.solve(request, setting),
        });
        session.solved_for = Some(setting);
    }
    if session.travel_track_day != Some(setting.depart_day) {
        session.travel_track = track_on(session, setting.depart_day);
        session.travel_track_day = Some(setting.depart_day);
    }
}

/// PLNR-87: solves what the current setting needs, each frame it has
/// changed.
pub fn solve_setting(mut state: ResMut<PlannerState>) {
    if let Some(session) = state.session.as_mut() {
        solve_current(session);
    }
}

/// PLNR-12: what hovering a place shows: its Fastest leg, or what the
/// cheapest trip needs when nothing is affordable, or why it can't be
/// planned.
pub fn hover_outcome(context: &SearchContext, request: Request) -> HoverOutcome {
    let search = match context.search(request) {
        Ok(search) => search,
        Err(why) => return HoverOutcome::Unplannable(why),
    };
    if let Some(fastest) = search.presets.fastest {
        return match context.solve(request, fastest) {
            Ok(leg) => HoverOutcome::Fastest(Box::new(leg)),
            Err(why) => HoverOutcome::Unplannable(why),
        };
    }
    let cheapest = search
        .presets
        .cheapest
        .and_then(|setting| context.solve(request, setting).ok());
    match cheapest {
        Some(leg) => HoverOutcome::CantReach {
            cheapest_kms: leg.delta_v_kms,
        },
        None => HoverOutcome::Unplannable(
            search
                .grid
                .unplannable_reason()
                .unwrap_or(Infeasible::NoPath),
        ),
    }
}

/// The place under the pointer that the planner could pick: the first
/// hovered body or named orbit, in a steady order, that resolves to one.
fn hovered_place(
    hovered: &HoveredBodies,
    orbits: &Query<(), With<NamedOrbit>>,
    pickables: &Pickables,
    ship: Entity,
) -> Option<Place> {
    let mut entities: Vec<Entity> = hovered.0.iter().copied().collect();
    entities.sort_by_key(|e| e.index_u32());
    entities.into_iter().find_map(|entity| {
        let target = if orbits.contains(entity) {
            PickTarget::Orbit(entity)
        } else {
            PickTarget::Body(entity)
        };
        pickables.place(target, ship)
    })
}

/// The place under the pointer that the planner could pick, found apart
/// from the search because the two read the same ships differently.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct HoveredPlace(pub Option<Place>);

/// PLNR-12: while choosing, which hovered place could be picked.
pub fn find_hovered_place(
    state: Res<PlannerState>,
    hovered: Res<HoveredBodies>,
    orbits: Query<(), With<NamedOrbit>>,
    pickables: Pickables,
    mut place: ResMut<HoveredPlace>,
) {
    let now = state
        .session
        .as_ref()
        .filter(|session| session.request.is_none())
        .and_then(|session| hovered_place(&hovered, &orbits, &pickables, session.ship));
    if place.0 != now {
        place.0 = now;
    }
}

/// PLNR-12: while choosing, the hovered place's Fastest setting is worked
/// out once, and again only when the place or its search's key changes.
pub fn preview_hover(
    mut state: ResMut<PlannerState>,
    hovered: Res<HoveredPlace>,
    planner: Planner,
    clock: Res<GameClock>,
) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    let Some(place) = hovered.0.filter(|_| session.request.is_none()) else {
        session.hover = None;
        return;
    };
    let request = Request {
        destination: place,
        arrival: Arrival::Rendezvous,
    };
    let key = search_key(&planner, session.ship, request, 0.0);
    let day = clock.elapsed_days().floor() as i64;
    if session
        .hover
        .as_ref()
        .is_some_and(|hover| hover.place == place && hover.key == key && hover.day == day)
    {
        return;
    }
    let (outcome, available_kms) = match planner.search_context(session.ship) {
        Ok(context) => (hover_outcome(&context, request), context.available_kms),
        Err(why) => (HoverOutcome::Unplannable(why), 0.0),
    };
    session.hover = Some(HoverPreview {
        place,
        key,
        day,
        outcome,
        available_kms,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sliders() -> Sliders {
        Sliders {
            depart: Slider::Days {
                soonest: 10.5,
                end: 40.5,
                step: 1.0,
            },
            travel: Slider::Positions(vec![3.0, 7.0, 12.0]),
            windows: false,
        }
    }

    /// PLNR-13 / PLNR-23 / PLNR-40: a setting kept across a new search
    /// lands on the nearest Depart position, on a meeting, or on the
    /// fixed travel time.
    #[test]
    fn a_kept_setting_is_fitted_to_the_new_sliders() {
        let setting = Setting {
            depart_day: 55.0,
            travel_days: 8.0,
        };
        assert_eq!(
            fitted(setting, &sliders(), 400),
            Setting {
                depart_day: 40.0,
                travel_days: 7.0
            }
        );
        let early = Setting {
            depart_day: 10.6,
            travel_days: 1.0,
        };
        assert_eq!(fitted(early, &sliders(), 400).depart_day, 10.5);
        let fixed = Sliders {
            travel: Slider::Fixed(30.0),
            ..sliders()
        };
        assert_eq!(fitted(early, &fixed, 400).travel_days, 30.0);
    }
}

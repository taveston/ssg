//! The planner's working out (PLNR-11 to PLNR-13, PLNR-24, PLNR-87): what
//! a click picks, the search for the request and when it is run again,
//! the leg of the current setting, the Travel time track, and the
//! preview of a hovered place. The rules are the simulation's
//! (`sim/transfer/`, reached through `Planner`); these systems decide only
//! when to ask, and keep the answers in `PlannerState` for the panel and
//! the map.

use bevy::prelude::*;

use super::pick::Pickables;
use crate::sim::clock::GameClock;
use crate::sim::named_orbit::NamedOrbit;
use crate::sim::transfer::grid::Setting;
use crate::sim::transfer::order::{Arrival, Infeasible};
use crate::sim::transfer::places::Place;
use crate::sim::transfer::plan::Planner;
use crate::sim::transfer::ranges::{Slider, Sliders, crossing_orbits};
use crate::sim::transfer::search::{Request, Search, SearchContext, SearchKey};
use crate::view::hovered::HoveredBodies;
use crate::view::planner::state::{Found, HoverOutcome, HoverPreview, PickTarget, PlannerState};

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
/// and its available delta-v (with what an order being changed reserved)
/// and the day. `None` for a ship with no drive.
fn search_key(
    planner: &Planner,
    ship: Entity,
    request: Request,
    changing: f64,
    today: f64,
) -> Option<SearchKey> {
    let (drive, available) = planner.budget(ship)?;
    Some(SearchKey::new(request, drive, available + changing, today))
}

/// PLNR-80 to PLNR-86: search for `request` from the system as it stands,
/// counting what an order being changed reserved as available (PLNR-27).
fn run_search(
    planner: &Planner,
    ship: Entity,
    request: Request,
    changing: f64,
    key: Option<SearchKey>,
) -> Found {
    match planner.search_context(ship) {
        Ok(mut context) => {
            context.available_kms += changing;
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
            Found {
                key,
                context: Some(context),
                result,
                primary,
            }
        }
        Err(reason) => Found {
            key,
            context: None,
            result: Err(reason),
            primary: None,
        },
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

/// PLNR-13 / PLNR-24 / PLNR-40 / PLNR-22: a setting kept across a new
/// search: within the ranges, on a meeting for a phasing leg, the fixed
/// value where there is no slider, and never departing before today.
pub fn fitted(setting: Setting, sliders: &Sliders, today: f64) -> Setting {
    let fit = |slider: &Slider, value: f64| match slider {
        Slider::Range { start, end } => value.clamp(*start, end.max(*start)),
        Slider::Fixed(fixed) => *fixed,
        Slider::Positions(positions) => positions
            .iter()
            .copied()
            .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
            .unwrap_or(value),
    };
    Setting {
        depart_day: fit(&sliders.depart, setting.depart_day).max(today),
        travel_days: fit(&sliders.travel, setting.travel_days),
    }
}

/// PLNR-87: the grid and presets are worked out again when the request,
/// the ship's drive or available delta-v, or the game day changes; a
/// slider move never does it. The setting is kept across a new search,
/// or starts at its preset after a pick (PLNR-13).
pub fn refresh_search(mut state: ResMut<PlannerState>, planner: Planner, clock: Res<GameClock>) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    let Some(request) = session.request else {
        return;
    };
    let today = clock.elapsed_days();
    let changing = session.changing.unwrap_or(0.0);
    let key = search_key(&planner, session.ship, request, changing, today);
    if session.found.as_ref().is_some_and(|found| found.key == key) {
        return;
    }
    let found = run_search(&planner, session.ship, request, changing, key);
    if let (Ok(search), Some(context)) = (&found.result, &found.context) {
        let max_steps = context.tunables.slider_max_steps;
        session.setting = Some(match session.setting {
            Some(setting) => fitted(setting, &search.sliders, today),
            None => starting_setting(search, max_steps),
        });
    }
    session.found = Some(found);
    session.solved_for = None;
    session.travel_track_day = None;
}

/// PLNR-24 / PLNR-87: the depart day never falls before today; the leg
/// of the setting is solved when the setting changes, and the Travel time
/// track when the depart day does.
pub fn solve_setting(mut state: ResMut<PlannerState>, clock: Res<GameClock>) {
    let today = clock.elapsed_days();
    let Some(session) = state.session.as_mut() else {
        return;
    };
    let Some(found) = session.found.as_ref() else {
        return;
    };
    let (Some(context), Ok(search)) = (&found.context, &found.result) else {
        session.leg = found.result.as_ref().err().map(|why| Err(*why));
        return;
    };
    let (Some(request), Some(mut setting)) = (session.request, session.setting) else {
        return;
    };
    if setting.depart_day < today {
        // The clock moving on is not a slider move: the track stays.
        setting.depart_day = today;
        session.setting = Some(setting);
        if session.travel_track_day.is_some() {
            session.travel_track_day = Some(today);
        }
    }
    if session.solved_for != Some(setting) {
        session.leg = Some(context.solve(request, setting));
        session.solved_for = Some(setting);
    }
    if session.travel_track_day != Some(setting.depart_day) {
        session.travel_track = context.travel_track(request, &search.sliders, setting.depart_day);
        session.travel_track_day = Some(setting.depart_day);
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
    let today = clock.elapsed_days();
    let key = search_key(&planner, session.ship, request, 0.0, today);
    if session
        .hover
        .as_ref()
        .is_some_and(|hover| hover.place == place && hover.key == key)
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
        outcome,
        available_kms,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sliders() -> Sliders {
        Sliders {
            depart: Slider::Range {
                start: 10.0,
                end: 40.0,
            },
            travel: Slider::Positions(vec![3.0, 7.0, 12.0]),
            windows: false,
        }
    }

    /// PLNR-13 / PLNR-24 / PLNR-40: a setting kept across a new search
    /// stays inside the ranges, lands on a meeting, and never departs
    /// before today.
    #[test]
    fn a_kept_setting_is_fitted_to_the_new_sliders() {
        let setting = Setting {
            depart_day: 55.0,
            travel_days: 8.0,
        };
        assert_eq!(
            fitted(setting, &sliders(), 10.0),
            Setting {
                depart_day: 40.0,
                travel_days: 7.0
            }
        );
        let early = Setting {
            depart_day: 12.0,
            travel_days: 1.0,
        };
        assert_eq!(fitted(early, &sliders(), 15.5).depart_day, 15.5);
        let fixed = Sliders {
            travel: Slider::Fixed(30.0),
            ..sliders()
        };
        assert_eq!(fitted(early, &fixed, 10.0).travel_days, 30.0);
    }
}

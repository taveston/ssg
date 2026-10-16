//! The transfer planner's shared state (PLNR-02, PLNR-03, PLNR-11,
//! PLNR-53): whether a planner is open, for which ship, what it is
//! planning, and what the search and the solver found for it. Shared, so
//! the click handlers, the inset, the outline, the hover marks, the
//! camera's keys and the map's paths read it without importing the
//! planner's panel (ARCH-03). The planner (`view/planner/`) keeps it up
//! to date.
//!
//! A planner is *choosing* until it has a request (PLNR-10), then
//! *tuning* (PLNR-20). The map draws from it: `PlannerSession::leg` is the
//! leg of the current setting, and `PlannerSession::hover` the Fastest leg
//! of the place hovered while choosing (PLNR-12, PLNR-62).

use std::collections::HashSet;

use bevy::prelude::*;

use crate::sim::transfer::grid::{Sample, Setting};
use crate::sim::transfer::order::{Infeasible, Order, SolvedLeg};
use crate::sim::transfer::places::Place;
use crate::sim::transfer::search::{Request, Search, SearchContext, SearchKey};

/// What a click on the map or in the outline hit while the planner is
/// open (PLNR-11): a body (marker, disc, pill, entry or a lone object's
/// ring) or a named orbit (a shared ring or an orbit row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickTarget {
    Body(Entity),
    Orbit(Entity),
}

/// The planner's two sliders (PLNR-21, PLNR-22).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SliderKind {
    Depart,
    Travel,
}

/// PLNR-80 to PLNR-87: a search, with what it was run on and for.
#[derive(Debug, Clone)]
pub struct Found {
    /// What the search depends on; a new key means a new search (PLNR-87).
    /// `None` for a ship with no drive.
    pub key: Option<SearchKey>,
    /// The snapshot it ran on; slider moves solve against it too. `None`
    /// when the ship can't be planned for at all.
    pub context: Option<SearchContext>,
    /// The sliders, grid and presets, or why the request has none.
    pub result: Result<Search, Infeasible>,
    /// The body the crossing goes round, for the reasons that name it
    /// (PLNR-92).
    pub primary: Option<Entity>,
}

/// PLNR-12: what hovering a place while choosing shows.
#[derive(Debug, Clone, PartialEq)]
pub enum HoverOutcome {
    /// The leg of its Fastest setting (PLNR-83), which the map draws.
    Fastest(Box<SolvedLeg>),
    /// No setting is affordable: the cheapest needs this much.
    CantReach { cheapest_kms: f64 },
    /// The solver can't plan it at all.
    Unplannable(Infeasible),
}

/// PLNR-12: the hovered place's preview, kept until the place or what
/// its search depends on changes.
#[derive(Debug, Clone, PartialEq)]
pub struct HoverPreview {
    pub place: Place,
    pub key: Option<SearchKey>,
    pub outcome: HoverOutcome,
    /// The ship's available delta-v the preview was judged against.
    pub available_kms: f64,
}

/// One open planner.
#[derive(Debug, Clone)]
pub struct PlannerSession {
    /// The ship being planned for.
    pub ship: Entity,
    /// Where to and how; `None` while choosing (PLNR-10).
    pub request: Option<Request>,
    /// Where the sliders are; `None` until the search sets them (PLNR-13).
    pub setting: Option<Setting>,
    /// PLNR-53: while an order is being changed, the delta-v it reserved,
    /// which counts as available (PLNR-27).
    pub changing: Option<f64>,
    /// The search for `request` (PLNR-87).
    pub found: Option<Found>,
    /// The leg of `setting`, solved with no budget limit (PLNR-87).
    pub leg: Option<Result<SolvedLeg, Infeasible>>,
    /// The setting `leg` was solved for.
    pub solved_for: Option<Setting>,
    /// PLNR-28: the Travel time track on the current depart day, and the
    /// depart day it was sampled on.
    pub travel_track: Vec<Sample>,
    pub travel_track_day: Option<f64>,
    /// PLNR-12: the hovered place's preview, while choosing.
    pub hover: Option<HoverPreview>,
}

impl PlannerSession {
    /// PLNR-02: a planner that starts by choosing a destination.
    pub fn choosing(ship: Entity) -> Self {
        Self {
            ship,
            request: None,
            setting: None,
            changing: None,
            found: None,
            leg: None,
            solved_for: None,
            travel_track: Vec::new(),
            travel_track_day: None,
            hover: None,
        }
    }

    /// PLNR-53: a planner tuning an existing order, set as it is.
    pub fn changing(ship: Entity, order: &Order, reserved_kms: f64) -> Self {
        Self {
            request: Some(Request {
                destination: order.destination,
                arrival: order.arrival,
            }),
            setting: Some(Setting {
                depart_day: order.depart_day,
                travel_days: order.travel_days,
            }),
            changing: Some(reserved_kms),
            ..Self::choosing(ship)
        }
    }

    /// PLNR-13: picking a destination starts tuning it from scratch: the
    /// sliders go to the new search's starting preset.
    pub fn pick(&mut self, request: Request) {
        self.request = Some(request);
        self.setting = None;
        self.found = None;
        self.leg = None;
        self.solved_for = None;
        self.travel_track.clear();
        self.travel_track_day = None;
        self.hover = None;
    }

    /// The search's result, when there is one.
    pub fn search(&self) -> Option<&Search> {
        self.found.as_ref()?.result.as_ref().ok()
    }

    /// The leg of the current setting, when it can be flown at all.
    pub fn solved_leg(&self) -> Option<&SolvedLeg> {
        self.leg.as_ref()?.as_ref().ok()
    }
}

/// The planner, open or not, and what it shares with the rest of the view.
#[derive(Resource, Debug, Default)]
pub struct PlannerState {
    pub session: Option<PlannerSession>,
    /// PLNR-03: a click that picked a place this frame, for the planner to
    /// resolve (PLNR-11).
    pub clicked: Option<PickTarget>,
    /// PLNR-11: bodies and named orbits that can't be picked while the
    /// planner is open. They take no hover mark.
    pub unpickable: HashSet<Entity>,
    /// PLNR-53: the line shown at the top of the inset after an order
    /// departed or failed while it was being changed.
    pub notice: Option<&'static str>,
    /// PLNR-23: the slider with keyboard focus, which takes Left and Right
    /// in place of the camera's pan.
    pub slider_focus: Option<SliderKind>,
}

impl PlannerState {
    /// Whether a planner is open.
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    /// Whether a planner is open for `ship`.
    pub fn is_open_for(&self, ship: Entity) -> bool {
        self.session.as_ref().is_some_and(|s| s.ship == ship)
    }

    /// PLNR-02: open the planner for `ship`, choosing.
    pub fn open(&mut self, ship: Entity) {
        self.start(PlannerSession::choosing(ship));
    }

    /// PLNR-53: open the planner on an existing order.
    pub fn start(&mut self, session: PlannerSession) {
        self.session = Some(session);
        self.clicked = None;
        self.notice = None;
        self.slider_focus = None;
    }

    /// PLNR-02: end the planner and drop its settings. Returns whether
    /// one was open.
    pub fn end(&mut self) -> bool {
        self.clicked = None;
        self.unpickable.clear();
        self.slider_focus = None;
        self.session.take().is_some()
    }

    /// PLNR-03: while the planner is open, a click that hits a place
    /// picks it instead of selecting it. Returns whether the click was
    /// taken; a click on something that can't be picked is taken and does
    /// nothing (PLNR-11).
    pub fn pick(&mut self, target: PickTarget) -> bool {
        if !self.is_open() {
            return false;
        }
        let entity = match target {
            PickTarget::Body(entity) | PickTarget::Orbit(entity) => entity,
        };
        if !self.unpickable.contains(&entity) {
            self.clicked = Some(target);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PLNR-03 / PLNR-11: with no planner open a click is not taken; with
    /// one open it is, and a pick of something that can't be picked does
    /// nothing.
    #[test]
    fn an_open_planner_takes_clicks_and_ignores_what_cant_be_picked() {
        let mut world = World::new();
        let [ship, place, other] = [(); 3].map(|_| world.spawn_empty().id());
        let mut state = PlannerState::default();
        assert!(!state.pick(PickTarget::Body(place)));
        state.open(ship);
        assert!(state.is_open_for(ship));
        state.unpickable.insert(other);
        assert!(state.pick(PickTarget::Body(other)));
        assert_eq!(state.clicked, None);
        assert!(state.pick(PickTarget::Orbit(place)));
        assert_eq!(state.clicked, Some(PickTarget::Orbit(place)));
        assert!(state.end());
        assert!(!state.end(), "already ended");
        assert_eq!(state.clicked, None);
    }
}

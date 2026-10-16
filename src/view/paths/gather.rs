//! Which paths the map draws this frame (PLNR-62 to PLNR-67): the leg
//! being planned and the hovered place's Fastest leg while the planner is
//! open, and each own ship's order as `look.rs` says, each with its ghosts,
//! its arrival label and the coast after an Intercept. Gathered into
//! `MapPaths`, which the strokes and the marks draw from.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use orbits::Conic;

use super::look::{Emphasis, OrderFacts, order_look};
use crate::sim::body::{Affiliation, DisplayName};
use crate::sim::clock::GameClock;
use crate::sim::rail::ShipTrajectory;
use crate::sim::ship::InFlight;
use crate::sim::track::ShipRecord;
use crate::sim::transfer::coast::{CoastEvent, CoastOf, coast_event};
use crate::sim::transfer::order::{Arrival, SolvedLeg};
use crate::sim::transfer::places::Place;
use crate::sim::transfer::plan::{Plan, Status};
use crate::sim::transfer::tree::{SystemTree, TreeQuery};
use crate::sim::tunables::SimTunables;
use crate::sim::units::KM_PER_AU;
use crate::view::hovered::HoveredBodies;
use crate::view::planner::state::{HoverOutcome, PlannerState};
use crate::view::planner::words::{LegWords, coast_label};
use crate::view::selection::Selection;

/// A ghost to draw: the object it stands for, and its label (PLNR-63,
/// PLNR-65).
#[derive(Debug, Clone, PartialEq)]
pub struct Ghost {
    pub object: Entity,
    pub label: Option<String>,
}

/// The coast after an Intercept, up to its event (PLNR-67): the conic,
/// the body it goes round, its days and its label.
#[derive(Debug, Clone, PartialEq)]
pub struct CoastDraw {
    pub primary: Entity,
    pub conic: Conic,
    pub from_day: f64,
    pub to_day: f64,
    pub label: String,
}

/// One path to draw: whose it is, its rail and crossing days, how
/// strongly it shows, its ghosts and its coast.
#[derive(Debug, Clone, PartialEq)]
pub struct PathDraw {
    pub ship: Entity,
    pub trajectory: ShipTrajectory,
    pub crossing_start_day: f64,
    pub crossing_end_day: f64,
    pub emphasis: Emphasis,
    /// Drawn only while the order is being planned (PLNR-63).
    pub departure_ghost: Option<Ghost>,
    pub arrival_ghost: Option<Ghost>,
    pub coast: Option<CoastDraw>,
}

/// The paths drawn this frame.
#[derive(Resource, Default, Debug)]
pub struct MapPaths(pub Vec<PathDraw>);

/// The figures of a leg, solved or ordered, that its path is drawn from.
struct LegFigures<'a> {
    trajectory: &'a ShipTrajectory,
    crossing_start_day: f64,
    crossing_end_day: f64,
    arrival_day: f64,
    coast: CoastOf<'a>,
    destination: Place,
    arrival: Arrival,
    origin: Option<Place>,
    arrival_speed_kms: Option<f64>,
    turn_rad: Option<f64>,
}

impl<'a> LegFigures<'a> {
    fn solved(leg: &'a SolvedLeg, destination: Place, arrival: Arrival) -> Self {
        Self {
            trajectory: &leg.trajectory,
            crossing_start_day: leg.crossing_start_day,
            crossing_end_day: leg.crossing_end_day,
            arrival_day: leg.arrival_day,
            coast: CoastOf::solved(leg, destination),
            destination,
            arrival,
            origin: leg.origin,
            arrival_speed_kms: leg.arrival_speed_kms,
            turn_rad: leg.turn_rad,
        }
    }

    fn ordered(leg: &'a crate::sim::transfer::plan::Leg) -> Self {
        Self {
            trajectory: &leg.trajectory,
            crossing_start_day: leg.crossing_start_day,
            crossing_end_day: leg.crossing_end_day,
            arrival_day: leg.arrival_day,
            coast: CoastOf::ordered(leg),
            destination: leg.destination,
            arrival: leg.arrival,
            origin: leg.origin,
            arrival_speed_kms: leg.arrival_speed_kms,
            turn_rad: leg.turn_rad,
        }
    }

    fn intercept(&self) -> bool {
        self.arrival.is_intercept()
    }
}

/// The ships whose orders may show, with what decides it.
type OrderShips<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Plan,
        &'static Affiliation,
        Option<&'static ShipRecord>,
        Has<InFlight>,
    ),
>;

/// What the paths are gathered from.
#[derive(SystemParam)]
pub struct PathSources<'w, 's> {
    state: Res<'w, PlannerState>,
    selection: Res<'w, Selection>,
    hovered: Res<'w, HoveredBodies>,
    clock: Res<'w, GameClock>,
    sim_tunables: Res<'w, SimTunables>,
    ships: OrderShips<'w, 's>,
    names: Query<'w, 's, &'static DisplayName>,
    tree: TreeQuery<'w, 's>,
}

impl PathSources<'_, '_> {
    fn name(&self, entity: Option<Entity>) -> String {
        entity
            .and_then(|e| self.names.get(e).ok())
            .map(|n| n.0.clone())
            .unwrap_or_default()
    }

    /// PLNR-65: the arrival ghost's label, at the day the leg ends.
    fn ghost_label(&self, leg: &LegFigures) -> String {
        let destination = self.name(leg.destination.entity());
        let words = LegWords {
            intercept: leg.intercept(),
            to_ship: matches!(leg.destination, Place::Ship { .. }),
            destination: &destination,
        };
        let end = if leg.intercept() {
            leg.crossing_end_day
        } else {
            leg.arrival_day
        };
        words.ghost_label(end, leg.arrival_speed_kms, leg.turn_rad)
    }

    /// PLNR-67 / PLNR-88: the coast after an Intercept round the star, up
    /// to its event while that is still ahead, with its label.
    fn coast(&self, leg: &LegFigures, tree: &SystemTree) -> Option<CoastDraw> {
        let today = self.clock.elapsed_days();
        let lost_km = self.sim_tunables.lost_beyond_au * KM_PER_AU;
        let coast = leg.coast;
        let (primary, conic) = coast.conic()?;
        let (body, days) = match coast_event(tree, lost_km, coast, today)? {
            CoastEvent::Crosses { body, days } => (Some(body), days),
            CoastEvent::Leaves { days } => (None, days),
        };
        (days > 0.0).then(|| CoastDraw {
            primary,
            conic,
            from_day: leg.crossing_end_day,
            to_day: today + days,
            label: coast_label(body.map(|b| self.name(Some(b))).as_deref(), days),
        })
    }

    /// PLNR-63: the departure ghost stands for what the path starts from:
    /// the departure's ancestor directly below the crossing's primary, or
    /// the ship itself when it sets out from an orbit of the primary.
    fn departure_object(&self, ship: Entity, leg: &LegFigures, tree: &SystemTree) -> Entity {
        let primary = leg
            .trajectory
            .segments()
            .iter()
            .find_map(|s| crossing_primary(&s.kind));
        let origin = leg.origin.and_then(Place::entity);
        match (primary, origin) {
            (Some(primary), Some(origin)) => tree.child_below(primary, origin).unwrap_or(ship),
            _ => ship,
        }
    }

    /// One path, with the ghosts and labels asked for.
    fn path(
        &self,
        ship: Entity,
        leg: &LegFigures,
        (emphasis, planning, arrival_ghost): (Emphasis, bool, bool),
        tree: &SystemTree,
    ) -> PathDraw {
        PathDraw {
            ship,
            trajectory: leg.trajectory.clone(),
            crossing_start_day: leg.crossing_start_day,
            crossing_end_day: leg.crossing_end_day,
            emphasis,
            departure_ghost: planning.then(|| Ghost {
                object: self.departure_object(ship, leg, tree),
                label: None,
            }),
            arrival_ghost: arrival_ghost
                .then(|| leg.destination.entity())
                .flatten()
                .map(|object| Ghost {
                    object,
                    label: Some(self.ghost_label(leg)),
                }),
            coast: self.coast(leg, tree),
        }
    }

    /// PLNR-62: the leg being planned and, while choosing, the hovered
    /// place's Fastest leg (PLNR-12), each with both ghosts.
    fn planning(&self, tree: &SystemTree) -> Vec<PathDraw> {
        let Some(session) = &self.state.session else {
            return Vec::new();
        };
        let planned = session
            .request
            .zip(session.solved_leg())
            .map(|(request, leg)| LegFigures::solved(leg, request.destination, request.arrival));
        let hovered = session
            .hover
            .as_ref()
            .and_then(|preview| match &preview.outcome {
                HoverOutcome::Fastest(leg) => {
                    Some(LegFigures::solved(leg, preview.place, Arrival::Rendezvous))
                }
                _ => None,
            });
        planned
            .into_iter()
            .chain(hovered)
            .map(|leg| self.path(session.ship, &leg, (Emphasis::Full, true, true), tree))
            .collect()
    }

    /// PLNR-62: each own ship's order, as `order_look` says. A ship being
    /// planned for shows the plan in the making instead.
    fn orders(&self, tree: &SystemTree) -> Vec<PathDraw> {
        let today = self.clock.elapsed_days();
        let mut paths = Vec::new();
        for (ship, plan, affiliation, record, in_flight) in &self.ships {
            if self.state.is_open_for(ship) {
                continue;
            }
            let leg = plan.leg();
            let figures = LegFigures::ordered(leg);
            let facts = OrderFacts {
                commanded: *affiliation == Affiliation::Own && record.is_some_and(|r| r.commanded),
                status: plan.status,
                coasting: plan.status == Status::Done && figures.intercept() && in_flight,
                left: today >= leg.leave_day(),
                selected: self.selection.is_selected(ship),
                hovered: self.hovered.contains(ship),
            };
            if let Some(look) = order_look(facts) {
                let ask = (look.emphasis, false, look.arrival_ghost);
                paths.push(self.path(ship, &figures, ask, tree));
            }
        }
        paths
    }
}

/// The body a crossing segment goes round; `None` for a parked one.
fn crossing_primary(kind: &crate::sim::rail::SegmentKind) -> Option<Entity> {
    use crate::sim::rail::SegmentKind;
    match kind {
        SegmentKind::Conic { primary, .. } | SegmentKind::Spiral { primary, .. } => Some(*primary),
        SegmentKind::Parked { .. } => None,
    }
}

/// PLNR-62: gather this frame's paths. The system snapshot the ghosts and
/// coasts are worked out on is taken only when there is a path to draw.
pub fn gather_paths(sources: PathSources, mut paths: ResMut<MapPaths>) {
    paths.0.clear();
    let anything = sources.state.session.is_some() || !sources.ships.is_empty();
    if !anything {
        return;
    }
    let tree = sources.tree.snapshot(sources.clock.elapsed_days());
    paths.0.extend(sources.planning(&tree));
    paths.0.extend(sources.orders(&tree));
}

//! Orders and status (PLAN-01 to PLAN-30): a plan is the order a ship was
//! given, the leg solved from it, and where it has got to. The order is
//! the truth; the leg's trajectory is derived from it and can be rebuilt
//! (PLAN-03). The transitions are a pure function of the plan and the day
//! (`advance`, ARCH-20); `run_plans` applies what they ask for to the ship.
//!
//! `Planner` is the simulation-owned way to give orders (ARCH-14): it
//! solves the leg against the system as it stands, reserves the delta-v
//! and puts the plan on the ship.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use orbits::{CircularOrbit, Segment};

use super::legs::solve_leg;
use super::order::{
    Arrival, Departure, Infeasible, Landing, LegBurns, Order, SolvedLeg, TargetLock,
};
use super::phase::{gap_rad, together};
use super::places::{Place, is_escaping};
use super::search::SearchContext;
use super::tree::{ShipState, TreeQuery};
use crate::sim::body::BodyClass;
use crate::sim::clock::GameClock;
use crate::sim::named_orbit::InNamedOrbit;
use crate::sim::orbit::{Orbit, OrbitPositionKm};
use crate::sim::rail::{SegmentKind, ShipTrajectory};
use crate::sim::ship::{DeltaV, Drive, InFlight, ShipRail};
use crate::sim::tunables::SimTunables;
use crate::sim::units::KM_PER_AU;

/// PLAN-02: where a plan has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Scheduled,
    InFlight,
    /// Told to Hold mid-flight: coasting, with its later burns dropped
    /// (PLAN-13).
    Holding,
    Done,
    Failed,
}

/// PLAN-01: one leg: the order and what was solved from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Leg {
    pub destination: Place,
    pub arrival: Arrival,
    pub depart_day: f64,
    pub travel_days: f64,
    pub trajectory: ShipTrajectory,
    /// PLAN-12: where the ship set out from; `None` from a coasting state.
    pub origin: Option<Place>,
    pub primary: Entity,
    pub delta_v_kms: f64,
    pub burns: LegBurns,
    pub crossing_start_day: f64,
    pub crossing_end_day: f64,
    pub arrival_day: f64,
    pub landing: Landing,
    pub intercept_speed_kms: Option<f64>,
    /// XFER-14 / XFER-10: an Intercept's closing speed at its target.
    pub closing_speed_kms: Option<f64>,
    /// PLNR-26: the speed the arrival row and the ghost's label show.
    pub arrival_speed_kms: Option<f64>,
    /// XFER-13: the turn of an Intercept through another body's Low band.
    pub turn_rad: Option<f64>,
    /// PHAS-03: days waited in the departure orbit for the exit point.
    pub departure_wait_days: f64,
    /// PHAS-10: days waited for the target to reach the capture point.
    pub arrival_wait_days: f64,
    /// PHAS-11: an Intercept's closest-approach phase.
    pub closest_approach_rad: Option<f64>,
    /// PHAS-15: days added so an Intercept meets its target.
    pub trim_days: Option<f64>,
    /// PLAN-11: the parked target ship the leg was solved against.
    pub target_lock: Option<TargetLock>,
}

impl Leg {
    pub fn new(order: &Order, solved: SolvedLeg) -> Self {
        Self {
            destination: order.destination,
            arrival: order.arrival,
            depart_day: order.depart_day,
            travel_days: solved.travel_days,
            trajectory: solved.trajectory,
            origin: solved.origin,
            primary: solved.primary,
            delta_v_kms: solved.delta_v_kms,
            burns: solved.burns,
            crossing_start_day: solved.crossing_start_day,
            crossing_end_day: solved.crossing_end_day,
            arrival_day: solved.arrival_day,
            landing: solved.landing,
            intercept_speed_kms: solved.intercept_speed_kms,
            closing_speed_kms: solved.closing_speed_kms,
            arrival_speed_kms: solved.arrival_speed_kms,
            turn_rad: solved.turn_rad,
            departure_wait_days: solved.departure_wait_days,
            arrival_wait_days: solved.arrival_wait_days,
            closest_approach_rad: solved.closest_approach_rad,
            trim_days: solved.trim_days,
            target_lock: solved.target_lock,
        }
    }

    /// The named orbit the leg assumes the ship departs from (PLAN-11).
    pub fn departure_orbit(&self) -> Option<Entity> {
        self.trajectory.segments().first()?.kind.parked_in()
    }
}

/// Which of a leg's burns have been made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BurnsMade {
    start: bool,
    end: bool,
}

/// PLAN-01 / PLAN-02: a ship's plan. As a component on the ship there is
/// at most one.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Plan {
    pub ship: Entity,
    pub legs: Vec<Leg>,
    pub status: Status,
    /// What this plan still holds reserved on the ship's account.
    reserved_kms: f64,
    burned: BurnsMade,
}

impl Plan {
    /// PLAN-01: a plan of one leg; more legs are rejected until multi-leg
    /// plans are specced.
    pub fn new(ship: Entity, legs: Vec<Leg>) -> Result<Self, Infeasible> {
        if legs.len() != 1 {
            return Err(Infeasible::MultiLeg);
        }
        let reserved_kms = legs[0].delta_v_kms;
        Ok(Self {
            ship,
            legs,
            status: Status::Scheduled,
            reserved_kms,
            burned: BurnsMade::default(),
        })
    }

    /// PLAN-12: the last leg, which describes the ship's place.
    pub fn leg(&self) -> &Leg {
        self.legs.last().expect("a plan has a leg")
    }

    /// PLAN-02: Scheduled or InFlight, so no other plan may be given.
    pub fn is_active(&self) -> bool {
        matches!(self.status, Status::Scheduled | Status::InFlight)
    }

    /// What the plan still has reserved (PLAN-21).
    pub fn reserved_kms(&self) -> f64 {
        self.reserved_kms
    }
}

/// What a transition asks the ship to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Move this much from reserved to spent (PLAN-21).
    Burn(f64),
    /// Give this much reserved back (PLAN-21).
    Release(f64),
    /// Start following the leg's trajectory.
    Launch(ShipTrajectory),
    /// Leave the named orbit and fly round `primary` (RAIL-09).
    LeaveOrbit { primary: Entity },
    /// Capture into a named orbit of `body` on `path` (BAND-06).
    JoinOrbit {
        orbit: Entity,
        body: Entity,
        path: CircularOrbit,
    },
    /// Cut the rail at its current segment, extended without end
    /// (PLAN-13).
    Truncate,
}

/// PLAN-10 / PLAN-11 / PLAN-21: the transitions a plan makes on `day`:
/// Scheduled becomes InFlight on the depart day if everything the leg
/// was solved against still holds (`as_solved`: the ship in the orbit
/// assumed, a target ship where it was) and the ship can still pay
/// (`available_kms` counts what this plan reserved), else Failed; in
/// flight, the burns are made as their days pass, the ship leaves and
/// joins orbits, and the plan is Done on arrival (Rendezvous) or at the
/// crossing's end (Intercept).
pub fn advance(plan: &mut Plan, day: f64, as_solved: bool, available_kms: f64) -> Vec<Effect> {
    let mut effects = Vec::new();
    let leg = plan.legs.last().cloned().expect("a plan has a leg");
    if plan.status == Status::Scheduled && day >= leg.depart_day {
        let can_pay = available_kms + plan.reserved_kms >= leg.delta_v_kms - 1e-9;
        if as_solved && can_pay {
            plan.status = Status::InFlight;
            effects.push(Effect::Launch(leg.trajectory.clone()));
        } else {
            plan.status = Status::Failed;
            effects.push(Effect::Release(plan.reserved_kms));
            plan.reserved_kms = 0.0;
            return effects;
        }
    }
    if plan.status != Status::InFlight {
        return effects;
    }
    if day >= leg.crossing_start_day && !plan.burned.start {
        plan.burned.start = true;
        plan.reserved_kms -= leg.burns.start_kms;
        effects.push(Effect::Burn(leg.burns.start_kms));
        effects.push(Effect::LeaveOrbit {
            primary: leg.primary,
        });
    }
    if day >= leg.crossing_end_day && !plan.burned.end {
        plan.burned.end = true;
        match leg.landing {
            Landing::Orbit {
                orbit,
                body,
                phase0_rad,
            } => {
                plan.reserved_kms -= leg.burns.end_kms;
                effects.push(Effect::Burn(leg.burns.end_kms));
                let path = parked_path(&leg.trajectory, orbit, phase0_rad);
                effects.push(Effect::JoinOrbit { orbit, body, path });
            }
            Landing::Coast => {
                plan.reserved_kms -= leg.burns.end_kms;
                effects.push(Effect::Burn(leg.burns.end_kms));
            }
            Landing::Intercept => plan.status = Status::Done,
        }
    }
    if plan.status == Status::InFlight
        && leg.arrival == Arrival::Rendezvous
        && day >= leg.arrival_day
    {
        plan.status = Status::Done;
    }
    effects
}

/// The circular path the leg's last segment parks the ship on.
fn parked_path(trajectory: &ShipTrajectory, orbit: Entity, phase0_rad: f64) -> CircularOrbit {
    trajectory
        .segments()
        .iter()
        .rev()
        .find_map(|segment| match segment.kind {
            SegmentKind::Parked { orbit: o, path, .. } if o == orbit => Some(path),
            _ => None,
        })
        .unwrap_or(CircularOrbit {
            radius_km: 0.0,
            period_days: f64::INFINITY,
            phase0_rad,
        })
}

/// PLAN-13: Hold. An InFlight plan keeps its current segment without end,
/// drops its later burns and is Holding; anything else is left alone (a
/// parked ship holds already).
pub fn hold(plan: &mut Plan) -> Vec<Effect> {
    if plan.status != Status::InFlight || !plan.burned.start {
        return Vec::new();
    }
    plan.status = Status::Holding;
    plan.burned.end = true;
    let released = plan.reserved_kms.max(0.0);
    plan.reserved_kms = 0.0;
    vec![Effect::Truncate, Effect::Release(released)]
}

/// PLAN-22: cancelling a Scheduled plan gives back what it reserved and
/// leaves the ship as it was; an InFlight plan can't be cancelled.
pub fn cancel(plan: &mut Plan) -> Option<Effect> {
    if plan.status != Status::Scheduled {
        return None;
    }
    plan.status = Status::Failed;
    let released = plan.reserved_kms;
    plan.reserved_kms = 0.0;
    Some(Effect::Release(released))
}

/// PLAN-13: `rail` cut at the segment followed on `day`, which then goes
/// on without end.
pub fn truncated(rail: &ShipTrajectory, day: f64) -> Option<ShipTrajectory> {
    let keep = rail
        .segments()
        .iter()
        .position(|segment| segment.contains(day))?;
    let mut segments: Vec<Segment<SegmentKind>> = rail.segments()[..=keep].to_vec();
    segments[keep].end_day = None;
    ShipTrajectory::new(segments).ok()
}

/// The simulation's API for orders (ARCH-14).
#[derive(SystemParam)]
pub struct Planner<'w, 's> {
    tree: TreeQuery<'w, 's>,
    tunables: Res<'w, SimTunables>,
    clock: Res<'w, GameClock>,
    ships: Query<
        'w,
        's,
        (
            &'static Drive,
            &'static mut DeltaV,
            Option<&'static mut Plan>,
        ),
    >,
    commands: Commands<'w, 's>,
}

impl Planner<'_, '_> {
    /// What a search for `ship` reads (PLNR-04): the system as it stands
    /// today, where the ship sets out from, its drive and its available
    /// delta-v, as plain values the planner's rules work on.
    pub fn search_context(&self, ship: Entity) -> Result<SearchContext, Infeasible> {
        let (drive, account, _) = self.ships.get(ship).map_err(|_| Infeasible::NoDrive)?;
        let today = self.clock.elapsed_days();
        let tree = self.tree.snapshot(today);
        let departure = match tree.ships.get(&ship).ok_or(Infeasible::UnknownPlace)? {
            ShipState::Parked { orbit, phase0_rad } => Departure::Parked {
                orbit: *orbit,
                phase0_rad: *phase0_rad,
            },
            ShipState::Coasting { primary, conic } => Departure::Coasting {
                primary: *primary,
                conic: *conic,
            },
            ShipState::InFlight => return Err(Infeasible::ShipInFlight),
        };
        Ok(SearchContext {
            tree,
            tunables: self.tunables.clone(),
            departure,
            drive: *drive,
            available_kms: account.available_kms(),
            today,
        })
    }

    /// What a search for `ship` depends on besides its request (PLNR-87):
    /// its drive and its available delta-v (PLAN-21), without taking a
    /// snapshot of the system.
    pub fn budget(&self, ship: Entity) -> Option<(Drive, f64)> {
        let (drive, account, _) = self.ships.get(ship).ok()?;
        Some((*drive, account.available_kms()))
    }

    /// The leg `order` would give `ship` today, without scheduling it.
    pub fn preview(&self, ship: Entity, order: &Order) -> Result<SolvedLeg, Infeasible> {
        let context = self.search_context(ship)?;
        solve_leg(
            &context.tree,
            &context.tunables,
            &context.departure,
            order,
            &context.drive,
            context.available_kms,
        )
    }

    /// PLAN-01 / PLAN-21: give `ship` a one-leg plan for `order`, reserving
    /// its delta-v. A Hold order holds instead (PLAN-13), and an order to
    /// where the ship already is (PHAS-04) is done at once: like Hold on a
    /// parked ship, it costs nothing and changes nothing.
    pub fn schedule(&mut self, ship: Entity, order: &Order) -> Result<(), Infeasible> {
        if order.destination == Place::Hold {
            return self.hold(ship);
        }
        if self
            .ships
            .get(ship)
            .is_ok_and(|(_, _, plan)| plan.is_some_and(|plan| plan.is_active()))
        {
            return Err(Infeasible::Busy);
        }
        let solved = match self.preview(ship, order) {
            Err(Infeasible::AlreadyThere) => return Ok(()),
            solved => solved?,
        };
        let plan = Plan::new(ship, vec![Leg::new(order, solved)])?;
        let (_, mut account, _) = self.ships.get_mut(ship).map_err(|_| Infeasible::NoDrive)?;
        account.reserve(plan.reserved_kms());
        self.commands.entity(ship).insert(plan);
        Ok(())
    }

    /// PLAN-13: Hold: a ship in flight keeps its current segment without
    /// end; a parked or scheduled ship is left as it is.
    pub fn hold(&mut self, ship: Entity) -> Result<(), Infeasible> {
        let day = self.clock.elapsed_days();
        let (_, mut account, plan) = self.ships.get_mut(ship).map_err(|_| Infeasible::NoDrive)?;
        let Some(mut plan) = plan else {
            return Ok(());
        };
        for effect in hold(&mut plan) {
            match effect {
                Effect::Release(kms) => account.release(kms),
                Effect::Truncate => {
                    // The tree's queries read rails, so the cut is queued
                    // rather than made through a second, conflicting query.
                    self.commands.queue(move |world: &mut World| {
                        if let Some(mut rail) = world.get_mut::<ShipRail>(ship)
                            && let Some(cut) = truncated(&rail.0, day)
                        {
                            rail.0 = cut;
                        }
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// PLAN-22: cancel a Scheduled plan; `false` if there was none to
    /// cancel.
    pub fn cancel(&mut self, ship: Entity) -> bool {
        let Ok((_, mut account, Some(mut plan))) = self.ships.get_mut(ship) else {
            return false;
        };
        match cancel(&mut plan) {
            Some(Effect::Release(kms)) => {
                account.release(kms);
                true
            }
            _ => false,
        }
    }
}

/// ARCH-14: an order given to a ship, as a message the simulation reads.
/// A Transfer with `Place::Hold` as its destination holds (PLAN-13).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub enum ShipOrder {
    Transfer {
        ship: Entity,
        order: Order,
    },
    /// PLAN-22: cancel the ship's Scheduled plan.
    Cancel {
        ship: Entity,
    },
}

/// What came of an order: a message for whoever gave it.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct OrderOutcome {
    pub ship: Entity,
    pub result: Result<(), Infeasible>,
}

/// ARCH-14: every order given this frame goes through the planner, and
/// its outcome is reported back.
pub fn take_orders(
    mut orders: MessageReader<ShipOrder>,
    mut planner: Planner,
    mut outcomes: MessageWriter<OrderOutcome>,
) {
    for order in orders.read() {
        let (ship, result) = match *order {
            ShipOrder::Transfer { ship, order } => (ship, planner.schedule(ship, &order)),
            ShipOrder::Cancel { ship } => (
                ship,
                planner.cancel(ship).then_some(()).ok_or(Infeasible::Busy),
            ),
        };
        outcomes.write(OrderOutcome { ship, result });
    }
}

/// The ship a plan drives.
type PlannedShips<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Plan,
        &'static mut DeltaV,
        Option<&'static InNamedOrbit>,
    ),
>;

/// Where a parked ship is: its named orbit, its phase there and the
/// orbit's radius.
pub type ParkedAt = (Entity, f64, f64);

/// PLAN-11: whether the target ship a leg was solved against is still
/// parked where it was: in the same orbit, and together (PHAS-02) with
/// the phase it had. `now` is where it is parked today, if it is.
pub fn target_unmoved(lock: &TargetLock, now: Option<ParkedAt>, together_km: f64) -> bool {
    now.is_some_and(|(orbit, phase0_rad, radius_km)| {
        orbit == lock.orbit
            && together(radius_km, gap_rad(lock.phase0_rad, phase0_rad), together_km)
    })
}

/// PLAN-10 / PLAN-11 / PLAN-21: step every plan to today and apply what
/// it asks for to its ship. On the depart day the leg must still find the
/// ship in its departure orbit and any target ship where it was.
pub fn run_plans(
    mut commands: Commands,
    clock: Res<GameClock>,
    tunables: Res<SimTunables>,
    bodies: Query<(Entity, &BodyClass)>,
    parked: Query<(&InNamedOrbit, &Orbit)>,
    mut ships: PlannedShips,
) {
    let day = clock.elapsed_days();
    let star = bodies
        .iter()
        .find(|(_, class)| **class == BodyClass::Star)
        .map(|(e, _)| e);
    let parked_at = |ship: Entity| -> Option<ParkedAt> {
        let (in_orbit, orbit) = parked.get(ship).ok()?;
        Some((in_orbit.0, orbit.phase0_rad, orbit.radius_km))
    };
    for (ship, mut plan, mut account, in_orbit) in &mut ships {
        if !plan.is_active() {
            continue;
        }
        let leg = plan.leg();
        let assumed = leg.departure_orbit();
        let in_departure_orbit = assumed.is_none() || assumed == in_orbit.map(|o| o.0);
        let target_there = leg
            .target_lock
            .is_none_or(|lock| target_unmoved(&lock, parked_at(lock.ship), tunables.together_km));
        let available = account.available_kms();
        let as_solved = in_departure_orbit && target_there;
        for effect in advance(&mut plan, day, as_solved, available) {
            apply(&mut commands, ship, star, &mut account, effect);
        }
    }
}

/// What each effect does to the ship's components.
fn apply(
    commands: &mut Commands,
    ship: Entity,
    star: Option<Entity>,
    account: &mut DeltaV,
    effect: Effect,
) {
    let mut entity = commands.entity(ship);
    match effect {
        Effect::Burn(kms) => account.burn(kms),
        Effect::Release(kms) => account.release(kms),
        Effect::Launch(trajectory) => {
            entity.insert(ShipRail(trajectory));
        }
        Effect::LeaveOrbit { primary } => {
            entity.remove::<(Orbit, InNamedOrbit)>().insert(InFlight);
            if Some(primary) == star {
                entity.remove::<ChildOf>();
            } else {
                entity.insert(ChildOf(primary));
            }
        }
        Effect::JoinOrbit { orbit, body, path } => {
            entity.remove::<(InFlight, ShipRail)>().insert((
                Orbit {
                    radius_km: path.radius_km,
                    period_days: path.period_days,
                    phase0_rad: path.phase0_rad,
                },
                InNamedOrbit(orbit),
            ));
            if Some(body) == star {
                entity.remove::<ChildOf>();
            } else {
                entity.insert(ChildOf(body));
            }
        }
        Effect::Truncate => {}
    }
}

/// RAIL-04 / RAIL-09: a ship in flight is where its rail says today, in
/// km relative to the segment's primary.
pub fn drive_rails(
    clock: Res<GameClock>,
    mut ships: Query<(&ShipRail, &mut OrbitPositionKm), With<InFlight>>,
) {
    let day = clock.elapsed_days();
    for (rail, mut position) in &mut ships {
        if let Some(state) = rail.0.state_at(day) {
            position.0 = state.position;
        }
    }
}

/// BAND-08: an escaping ship with no plan running is removed once it is
/// `lost_beyond_au` from the star.
pub fn remove_escaping_ships(
    mut commands: Commands,
    clock: Res<GameClock>,
    tunables: Res<SimTunables>,
    stars: Query<(Entity, &BodyClass)>,
    ships: Query<(Entity, &ShipRail, &OrbitPositionKm, Option<&Plan>), With<InFlight>>,
) {
    let star = stars
        .iter()
        .find(|(_, class)| **class == BodyClass::Star)
        .map(|(e, _)| e);
    let day = clock.elapsed_days();
    let lost_km = tunables.lost_beyond_au * KM_PER_AU;
    for (ship, rail, position, plan) in &ships {
        if plan.is_some_and(|plan| plan.is_active()) {
            continue;
        }
        let Some(segment) = rail.0.segment_at(day) else {
            continue;
        };
        let SegmentKind::Conic { primary, conic } = segment.kind else {
            continue;
        };
        if Some(primary) == star && is_escaping(&conic, lost_km) && position.0.length() > lost_km {
            commands.entity(ship).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::transfer::places::Band;
    use orbits::{Conic, Rail};

    const MU: f64 = 1e21;

    /// A one-leg plan parked in `orbit` of `body`, crossing from day 10
    /// to day 20 round `primary`, arriving on day 22, costing 5 km/s
    /// (3 at the start, 2 at the end).
    fn plan(world: &mut World, arrival: Arrival) -> (Plan, Entity, Entity) {
        let ship = world.spawn_empty().id();
        let orbit = world.spawn_empty().id();
        let body = world.spawn_empty().id();
        let primary = world.spawn_empty().id();
        let dest = world.spawn_empty().id();
        let path = CircularOrbit::new(MU, 1e5, 0.0);
        let conic = Conic::from_state(
            MU,
            CircularOrbit::new(MU, 1e8, 0.0).state_at(10.0),
            10.0,
            1e-6,
        )
        .unwrap();
        let rendezvous = arrival == Arrival::Rendezvous;
        let mut segments = vec![
            Segment {
                start_day: 5.0,
                end_day: Some(10.0),
                kind: SegmentKind::Parked {
                    orbit,
                    primary: body,
                    path,
                },
            },
            Segment {
                start_day: 10.0,
                end_day: rendezvous.then_some(20.0),
                kind: SegmentKind::Conic { primary, conic },
            },
        ];
        if rendezvous {
            segments.push(Segment {
                start_day: 20.0,
                end_day: None,
                kind: SegmentKind::Parked {
                    orbit: dest,
                    primary: body,
                    path,
                },
            });
        }
        let leg = Leg {
            destination: Place::Body {
                body,
                band: Band::High,
            },
            arrival,
            depart_day: 5.0,
            travel_days: 10.0,
            trajectory: ShipTrajectory::new(segments).unwrap(),
            origin: Some(Place::Body {
                body,
                band: Band::Low,
            }),
            primary,
            delta_v_kms: if rendezvous { 5.0 } else { 3.0 },
            burns: LegBurns {
                start_kms: 3.0,
                end_kms: if rendezvous { 2.0 } else { 0.0 },
            },
            crossing_start_day: 10.0,
            crossing_end_day: 20.0,
            arrival_day: 22.0,
            landing: if rendezvous {
                Landing::Orbit {
                    orbit: dest,
                    body,
                    phase0_rad: 0.4,
                }
            } else {
                Landing::Intercept
            },
            intercept_speed_kms: (!rendezvous).then_some(4.0),
            closing_speed_kms: None,
            arrival_speed_kms: Some(4.0),
            turn_rad: None,
            departure_wait_days: 0.0,
            arrival_wait_days: 0.0,
            closest_approach_rad: None,
            trim_days: None,
            target_lock: None,
        };
        (Plan::new(ship, vec![leg]).unwrap(), orbit, dest)
    }

    /// PLAN-01: a plan has exactly one leg until multi-leg plans exist.
    #[test]
    fn plans_with_more_than_one_leg_are_rejected() {
        let mut world = World::new();
        let (one, ..) = plan(&mut world, Arrival::Rendezvous);
        let ship = one.ship;
        let leg = one.leg().clone();
        assert_eq!(
            Plan::new(ship, vec![leg.clone(), leg]).unwrap_err(),
            Infeasible::MultiLeg
        );
        assert_eq!(Plan::new(ship, vec![]).unwrap_err(), Infeasible::MultiLeg);
    }

    /// PLAN-10 / PLAN-21: a Rendezvous leg launches on its depart day, burns
    /// and leaves orbit at the crossing start, burns and joins the
    /// destination at the crossing end, and is Done on arrival.
    #[test]
    fn a_rendezvous_leg_runs_through_its_transitions() {
        let mut world = World::new();
        let (mut plan, _, dest) = plan(&mut world, Arrival::Rendezvous);
        assert!(advance(&mut plan, 4.0, true, 0.0).is_empty());
        assert_eq!(plan.status, Status::Scheduled);

        let effects = advance(&mut plan, 5.0, true, 0.0);
        assert!(matches!(effects.as_slice(), [Effect::Launch(_)]));
        assert_eq!(plan.status, Status::InFlight);

        let effects = advance(&mut plan, 10.0, true, 0.0);
        assert_eq!(effects[0], Effect::Burn(3.0));
        assert!(matches!(effects[1], Effect::LeaveOrbit { .. }));
        assert!((plan.reserved_kms() - 2.0).abs() < 1e-12);

        let effects = advance(&mut plan, 20.5, true, 0.0);
        assert_eq!(effects[0], Effect::Burn(2.0));
        assert!(matches!(effects[1], Effect::JoinOrbit { orbit, .. } if orbit == dest));
        assert_eq!(plan.status, Status::InFlight, "still approaching");
        assert!(plan.reserved_kms().abs() < 1e-12);

        assert!(advance(&mut plan, 22.0, true, 0.0).is_empty());
        assert_eq!(plan.status, Status::Done);
        assert!(!plan.is_active());
    }

    /// PLAN-10: an Intercept leg is Done at the crossing's end, with no second
    /// burn and no orbit to join: the ship coasts.
    #[test]
    fn an_intercept_leg_is_done_at_the_crossing_end() {
        let mut world = World::new();
        let (mut plan, ..) = plan(&mut world, Arrival::Intercept { band: Band::High });
        advance(&mut plan, 5.0, true, 0.0);
        advance(&mut plan, 10.0, true, 0.0);
        let effects = advance(&mut plan, 20.0, true, 0.0);
        assert!(effects.is_empty());
        assert_eq!(plan.status, Status::Done);
    }

    /// PLAN-11: on the depart day the leg is re-validated: a ship that
    /// has left the assumed orbit, or can no longer pay, fails and the
    /// reservation is released.
    #[test]
    fn a_plan_fails_if_the_ship_moved_or_cannot_pay() {
        let mut world = World::new();
        let (mut plan, ..) = plan(&mut world, Arrival::Rendezvous);
        let effects = advance(&mut plan, 5.0, false, 10.0);
        assert_eq!(effects, vec![Effect::Release(5.0)]);
        assert_eq!(plan.status, Status::Failed);

        let (mut plan, ..) = self::plan(&mut world, Arrival::Rendezvous);
        // 1 available plus the 5 reserved is short of a leg that now
        // needs more than 6.
        plan.legs[0].delta_v_kms = 7.0;
        let effects = advance(&mut plan, 5.0, true, 1.0);
        assert_eq!(effects, vec![Effect::Release(5.0)]);
        assert_eq!(plan.status, Status::Failed);
    }

    /// PLAN-13: Hold on an InFlight plan truncates the rail at its
    /// current segment, releases the unspent reservation and leaves the
    /// plan Holding; on a Scheduled plan it does nothing.
    #[test]
    fn hold_cuts_the_rail_and_drops_later_burns() {
        let mut world = World::new();
        let (mut plan, ..) = plan(&mut world, Arrival::Rendezvous);
        assert!(hold(&mut plan).is_empty());
        assert_eq!(plan.status, Status::Scheduled);
        advance(&mut plan, 5.0, true, 0.0);
        advance(&mut plan, 10.0, true, 0.0);
        let effects = hold(&mut plan);
        assert_eq!(effects, vec![Effect::Truncate, Effect::Release(2.0)]);
        assert_eq!(plan.status, Status::Holding);
        assert!(advance(&mut plan, 25.0, true, 0.0).is_empty());

        let cut = truncated(&plan.leg().trajectory, 15.0).unwrap();
        assert_eq!(cut.segments().len(), 2);
        assert_eq!(cut.segments()[1].end_day, None);
        assert!(truncated(&plan.leg().trajectory, 1.0).is_none());
    }

    /// PLAN-22: cancelling a Scheduled plan releases its reservation;
    /// an InFlight plan can't be cancelled.
    #[test]
    fn cancelling_releases_a_scheduled_plan_only() {
        let mut world = World::new();
        let (mut plan, ..) = plan(&mut world, Arrival::Rendezvous);
        assert_eq!(cancel(&mut plan), Some(Effect::Release(5.0)));
        assert!(!plan.is_active());
        let (mut plan, ..) = self::plan(&mut world, Arrival::Rendezvous);
        advance(&mut plan, 5.0, true, 0.0);
        assert_eq!(cancel(&mut plan), None);
        assert_eq!(plan.status, Status::InFlight);
    }

    /// PLAN-11: a target ship is where the leg was solved against only
    /// while it is parked in the same orbit, together with the phase it
    /// had; moved along the ring, into another orbit or off into flight,
    /// it is not.
    #[test]
    fn a_target_ship_must_stay_where_it_was() {
        let mut world = World::new();
        let [ship, orbit, other] = [(); 3].map(|_| world.spawn_empty().id());
        let lock = TargetLock {
            ship,
            orbit,
            phase0_rad: 1.0,
        };
        let r = 60_000.0;
        assert!(target_unmoved(&lock, Some((orbit, 1.0 + 5.0 / r, r)), 10.0));
        assert!(!target_unmoved(&lock, Some((orbit, 1.1, r)), 10.0));
        assert!(!target_unmoved(&lock, Some((other, 1.0, r)), 10.0));
        assert!(!target_unmoved(&lock, None, 10.0));
    }
}

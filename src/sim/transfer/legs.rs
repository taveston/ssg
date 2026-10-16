//! Solving one leg (XFER-01 to XFER-10): from where a ship is and the
//! order it was given to the crossing it will fly, its timing, its
//! trajectory and its delta-v, or the reason it can't be flown. Pure
//! functions over the system snapshot (`tree.rs`) and plain values
//! (ARCH-20, XFER-11); the maths is the `orbits` crate's.
//!
//! A crossing goes round one primary, the lowest body both ends sit
//! under. The planet-sized legs at each end are not flown: they are a
//! cost in km/s and a few days (`places.rs`). An impulsive drive gets a
//! two-burn Lambert arc; a low-thrust drive gets a spiral whose length
//! the drive fixes.

use std::f64::consts::{PI, TAU};

use bevy::prelude::*;
use orbits::{
    CircularOrbit, Conic, Rail, Segment, Sense, Spiral, State, angle_of, circular_speed, lambert,
};

use super::places::{Band, Place, approach_days, departure_cost_kms};
use super::tree::{OrbitNode, ShipState, SystemTree};
use crate::sim::rail::{
    SegmentKind, ShipTrajectory, km_per_day_to_km_per_s, km_per_s_to_km_per_day,
};
use crate::sim::ship::Drive;
use crate::sim::tunables::SimTunables;

/// XFER-08: what a leg does when it reaches its destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arrival {
    /// Capture into the destination's orbit.
    Rendezvous,
    /// Fly past through `band` and coast on.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "ordered by the planner UI, not yet specced")
    )]
    Intercept { band: Band },
}

/// One leg as ordered (PLAN-01, PLAN-03): everything a trajectory is
/// rebuilt from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    pub destination: Place,
    pub arrival: Arrival,
    pub depart_day: f64,
    /// For an impulsive drive, the crossing's length; a low-thrust drive
    /// sets its own and ignores this.
    pub travel_days: f64,
}

/// Where a leg starts from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Departure {
    /// Parked in a named orbit at a phase.
    Parked { orbit: Entity, phase0_rad: f64 },
    /// Coasting on a conic round `primary` (BAND-07).
    Coasting { primary: Entity, conic: Conic },
}

/// Why a leg can't be flown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Infeasible {
    /// The destination names something the system doesn't have.
    UnknownPlace,
    /// The body has no High band to arrive at or pass through (BAND-01).
    NoHighBand,
    /// Hold is not a crossing (PLAN-13).
    HoldIsNotACrossing,
    /// No path joins the two ends in the time given (XFER-03).
    NoPath,
    /// An Intercept through Low needs the flyby turn, which is not specced
    /// (XFER-08).
    PassThroughLow,
    /// A low-thrust crossing needs circular orbits at both ends (XFER-06).
    LowThrustNeedsCircularEnds,
    /// A target ship's primary is not the crossing's (XFER-10).
    ShipPrimary,
    /// The target ship is mid-crossing, so it has no state to aim at.
    TargetInFlight,
    /// The ship itself is mid-crossing; re-tasking is not specced.
    ShipInFlight,
    /// The crossing would dip too close to its primary (XFER-07).
    Periapsis { periapsis_km: f64 },
    /// The leg needs more delta-v than the ship has available (XFER-04).
    DeltaV { needed_kms: f64, available_kms: f64 },
    /// The ship has a plan Scheduled or InFlight already (PLAN-02).
    Busy,
    /// The ship has no drive or no delta-v account (PLAN-20).
    NoDrive,
    /// A plan has more than one leg, which is not specced (PLAN-01).
    MultiLeg,
}

/// PLAN-21: what a leg burns at each end of its crossing, in km/s.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LegBurns {
    /// Departure cost and burn 1, or a spiral's whole delta-v.
    pub start_kms: f64,
    /// Burn 2 and the capture cost; zero for an Intercept.
    pub end_kms: f64,
}

/// What the ship does at the crossing's end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Landing {
    /// Joins a named orbit of `body` at `phase0_rad` (BAND-06).
    Orbit {
        orbit: Entity,
        body: Entity,
        phase0_rad: f64,
    },
    /// Shares a coasting target's conic (XFER-10).
    Coast,
    /// Coasts on past the destination (XFER-08).
    Intercept,
}

/// A leg solved: its timing, path and cost.
#[derive(Debug, Clone, PartialEq)]
pub struct SolvedLeg {
    /// Where the ship set out from, for display (PLAN-12); `None` from a
    /// coasting state.
    pub origin: Option<Place>,
    /// The body the crossing goes round (XFER-01).
    pub primary: Entity,
    pub crossing_start_day: f64,
    pub crossing_end_day: f64,
    pub arrival_day: f64,
    pub travel_days: f64,
    pub trajectory: ShipTrajectory,
    pub delta_v_kms: f64,
    pub burns: LegBurns,
    pub landing: Landing,
    /// XFER-09 / XFER-10: the closing speed at an Intercept, in km/s.
    pub intercept_speed_kms: Option<f64>,
}

/// What a destination comes to (BAND-02).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Target {
    /// A named orbit; `ship` if the place was a parked ship.
    Orbit {
        orbit: Entity,
        node: OrbitNode,
        ship: bool,
    },
    /// A coasting ship's conic round `primary`.
    Coasting { primary: Entity, conic: Conic },
}

/// BAND-02: the orbit or conic a place stands for.
fn resolve(tree: &SystemTree, place: Place) -> Result<Target, Infeasible> {
    let orbit = match place {
        Place::Body { body, band } => {
            if !tree.bodies.contains_key(&body) {
                return Err(Infeasible::UnknownPlace);
            }
            tree.band_orbit(body, band).ok_or(Infeasible::NoHighBand)?
        }
        // SCL-07: a station is a place in a band; until arrivals are timed
        // to meet it, a ship sent there arrives in that band.
        Place::Station { station } => {
            tree.stations
                .get(&station)
                .ok_or(Infeasible::UnknownPlace)?
                .orbit
        }
        Place::Gate { gate: place } | Place::Site { site: place } => {
            tree.docking_orbit(place).ok_or(Infeasible::UnknownPlace)?
        }
        Place::Ship { ship } => {
            return match tree.ships.get(&ship).ok_or(Infeasible::UnknownPlace)? {
                ShipState::Parked { orbit, .. } => {
                    let node = *tree.orbits.get(orbit).ok_or(Infeasible::UnknownPlace)?;
                    Ok(Target::Orbit {
                        orbit: *orbit,
                        node,
                        ship: true,
                    })
                }
                ShipState::Coasting { primary, conic } => Ok(Target::Coasting {
                    primary: *primary,
                    conic: *conic,
                }),
                ShipState::InFlight => Err(Infeasible::TargetInFlight),
            };
        }
        Place::Hold => return Err(Infeasible::HoldIsNotACrossing),
    };
    let node = *tree.orbits.get(&orbit).ok_or(Infeasible::UnknownPlace)?;
    Ok(Target::Orbit {
        orbit,
        node,
        ship: false,
    })
}

/// PLAN-12: the place a named orbit stands for.
fn place_of_orbit(orbit: Entity, node: &OrbitNode, tree: &SystemTree) -> Option<Place> {
    if let Some(band) = node.band {
        return Some(Place::Body {
            body: node.body,
            band,
        });
    }
    if !tree.orbits.contains_key(&orbit) {
        return None;
    }
    let owner = node.docking_for?;
    match tree.bodies.get(&owner)?.class {
        crate::sim::body::BodyClass::Gate => Some(Place::Gate { gate: owner }),
        crate::sim::body::BodyClass::SalvageSite => Some(Place::Site { site: owner }),
        _ => None,
    }
}

/// PLAN-12 / BAND-02: the place a ship parked in `orbit` at `phase0_rad`
/// sets out from: the station it is docked at, if it sits at a station's
/// phase, and otherwise the place the orbit stands for.
fn place_of_parked(
    orbit: Entity,
    phase0_rad: f64,
    node: &OrbitNode,
    tree: &SystemTree,
) -> Option<Place> {
    match tree.station_at(orbit, phase0_rad) {
        Some(station) => Some(Place::Station { station }),
        None => place_of_orbit(orbit, node, tree),
    }
}

/// One end of a crossing: the thing in the system tree it is at (a body
/// for a band, a gate or site for a docking orbit), the body
/// whose gravity charges for it, the orbit it sits in and how to read its
/// own state on a day.
#[derive(Clone, Copy)]
struct End {
    /// The node whose place in the tree the end takes (XFER-01).
    at: Entity,
    /// The body the end's orbit goes round, which charges for leaving and
    /// arriving (BAND-03, BAND-04).
    host: Entity,
    /// The radius of the orbit round `host` the end sits in, if any.
    radius_km: Option<f64>,
    own: Own,
}

#[derive(Clone, Copy)]
enum Own {
    Circular(CircularOrbit),
    Conic(Conic),
    /// A band of the primary, whose angle is free until the start is known.
    Band(OrbitNode),
}

impl End {
    /// XFER-01: the end's state round `primary` on `day`: its ancestor
    /// directly below the primary, or its own state if it sits at the
    /// primary itself. A band of the primary is aimed half a turn ahead of
    /// `start`.
    fn state(
        &self,
        tree: &SystemTree,
        primary: Entity,
        day: f64,
        start: Option<&State>,
    ) -> Option<State> {
        if let Some(child) = tree.child_below(primary, self.at) {
            return Some(tree.bodies.get(&child)?.orbit?.state_at(day));
        }
        if self.at != primary {
            return None;
        }
        Some(match self.own {
            Own::Circular(orbit) => orbit.state_at(day),
            Own::Conic(conic) => conic.state_at(day),
            Own::Band(node) => {
                let angle = start.map_or(0.0, |s| s.angle()) + PI;
                node.path(phase_for_angle(&node, angle, day)).state_at(day)
            }
        })
    }

    /// BAND-03 / BAND-04: what this end charges, in km/s and days; nothing
    /// at the primary itself or without an orbit of a massive body.
    fn charges(&self, tree: &SystemTree, primary: Entity) -> (f64, f64) {
        let (Some(radius_km), Some(node)) = (self.radius_km, tree.bodies.get(&self.host)) else {
            return (0.0, 0.0);
        };
        if self.host == primary {
            return (0.0, 0.0);
        }
        (
            departure_cost_kms(node.mu, radius_km),
            approach_days(node.mu, radius_km, node.hill_km),
        )
    }
}

/// The start phase that puts a circular orbit at `angle` on `day`.
fn phase_for_angle(node: &OrbitNode, angle: f64, day: f64) -> f64 {
    if node.period_days.is_finite() && node.period_days != 0.0 {
        angle - TAU * day / node.period_days
    } else {
        angle
    }
}

fn departure_end(
    tree: &SystemTree,
    departure: &Departure,
) -> Result<(End, Option<Place>), Infeasible> {
    match *departure {
        Departure::Parked { orbit, phase0_rad } => {
            let node = *tree.orbits.get(&orbit).ok_or(Infeasible::UnknownPlace)?;
            Ok((
                End {
                    at: node.docking_for.unwrap_or(node.body),
                    host: node.body,
                    radius_km: Some(node.radius_km),
                    own: Own::Circular(node.path(phase0_rad)),
                },
                place_of_parked(orbit, phase0_rad, &node, tree),
            ))
        }
        Departure::Coasting { primary, conic } => Ok((
            End {
                at: primary,
                host: primary,
                radius_km: None,
                own: Own::Conic(conic),
            },
            None,
        )),
    }
}

fn target_end(target: &Target) -> End {
    match *target {
        Target::Orbit { node, .. } => End {
            at: node.docking_for.unwrap_or(node.body),
            host: node.body,
            radius_km: Some(node.radius_km),
            own: Own::Band(node),
        },
        Target::Coasting { primary, conic } => End {
            at: primary,
            host: primary,
            radius_km: None,
            own: Own::Conic(conic),
        },
    }
}

/// The crossing's fixed facts, gathered before the solve.
struct Setup<'a> {
    tree: &'a SystemTree,
    tunables: &'a SimTunables,
    primary: Entity,
    mu: f64,
    from: End,
    to: End,
    target: Target,
    rendezvous: bool,
    /// Departure cost (km/s) and approach days.
    depart: (f64, f64),
    /// Capture cost (km/s) and approach days; zero for an Intercept.
    arrive: (f64, f64),
}

/// A crossing solved round the primary: the states at its two ends and
/// the velocities the path has there.
struct Crossing {
    start_day: f64,
    end_day: f64,
    start: State,
    end: State,
    path_start_velocity: bevy::math::DVec3,
    path_end_velocity: bevy::math::DVec3,
    segment: SegmentKind,
}

impl Setup<'_> {
    fn new<'a>(
        tree: &'a SystemTree,
        tunables: &'a SimTunables,
        departure: &Departure,
        order: &Order,
    ) -> Result<(Setup<'a>, Option<Place>), Infeasible> {
        let target = resolve(tree, order.destination)?;
        let (from, origin) = departure_end(tree, departure)?;
        let to = target_end(&target);
        let primary = tree
            .lowest_common_ancestor(from.at, to.at)
            .ok_or(Infeasible::UnknownPlace)?;
        if let Target::Coasting { primary: p, .. } = target
            && p != primary
        {
            return Err(Infeasible::ShipPrimary);
        }
        if let Arrival::Intercept { band: Band::Low } = order.arrival {
            return Err(Infeasible::PassThroughLow);
        }
        let rendezvous = order.arrival == Arrival::Rendezvous;
        let depart = from.charges(tree, primary);
        let arrive = if rendezvous {
            to.charges(tree, primary)
        } else {
            (0.0, 0.0)
        };
        Ok((
            Setup {
                tree,
                tunables,
                primary,
                mu: tree.mu(primary),
                from,
                to,
                target,
                rendezvous,
                depart,
                arrive,
            },
            origin,
        ))
    }

    fn ends(&self, start_day: f64, end_day: f64) -> Option<(State, State)> {
        let start = self.from.state(self.tree, self.primary, start_day, None)?;
        let end = self
            .to
            .state(self.tree, self.primary, end_day, Some(&start))?;
        Some((start, end))
    }

    /// XFER-03: the prograde Lambert arc between the two ends' positions.
    fn lambert_crossing(&self, start_day: f64, travel_days: f64) -> Option<Crossing> {
        let end_day = start_day + travel_days;
        let (start, end) = self.ends(start_day, end_day)?;
        let arc = lambert(
            self.mu,
            start.position,
            end.position,
            travel_days,
            Sense::Prograde,
        )?;
        let conic = Conic::from_state(
            self.mu,
            State::new(start.position, arc.departure_velocity),
            start_day,
            self.tunables.parabola_epsilon,
        )?;
        Some(Crossing {
            start_day,
            end_day,
            start,
            end,
            path_start_velocity: arc.departure_velocity,
            path_end_velocity: arc.arrival_velocity,
            segment: SegmentKind::Conic {
                primary: self.primary,
                conic,
            },
        })
    }

    /// XFER-03 to XFER-05: the two-burn crossing. The first solve, with no
    /// burn days, gives burn 1 and so the burn days; the crossing is then
    /// solved once more from the later start, and that solve's burns are
    /// the leg's.
    fn impulsive(&self, order: &Order, drive: &Drive) -> Result<(Crossing, LegBurns), Infeasible> {
        if order.travel_days <= 0.0 || order.travel_days.is_nan() {
            return Err(Infeasible::NoPath);
        }
        let earliest = order.depart_day + self.depart.1;
        let first = self
            .lambert_crossing(earliest, order.travel_days)
            .ok_or(Infeasible::NoPath)?;
        let burn_1 = (first.path_start_velocity - first.start.velocity).length();
        let burn_days = (km_per_s_to_km_per_day(self.depart.0) + burn_1) / drive.accel;
        let crossing = self
            .lambert_crossing(earliest + burn_days, order.travel_days)
            .ok_or(Infeasible::NoPath)?;
        let burn_1 = km_per_day_to_km_per_s(
            (crossing.path_start_velocity - crossing.start.velocity).length(),
        );
        let burn_2 =
            km_per_day_to_km_per_s((crossing.end.velocity - crossing.path_end_velocity).length());
        let burns = LegBurns {
            start_kms: self.depart.0 + burn_1,
            end_kms: if self.rendezvous {
                burn_2 + self.arrive.0
            } else {
                0.0
            },
        };
        Ok((crossing, burns))
    }

    /// XFER-06: the spiral crossing between the two ends' circular orbits,
    /// whose length the drive fixes: the whole delta-v (costs included)
    /// over the acceleration. The spiral itself spans the crossing, so it
    /// lands at the end's angle on the crossing's last day.
    fn low_thrust(&self, order: &Order, drive: &Drive) -> Result<(Crossing, LegBurns), Infeasible> {
        if matches!(self.from.own, Own::Conic(_)) || matches!(self.target, Target::Coasting { .. })
        {
            return Err(Infeasible::LowThrustNeedsCircularEnds);
        }
        let start_day = order.depart_day + self.depart.1;
        let (start, probe) = self.ends(start_day, start_day).ok_or(Infeasible::NoPath)?;
        let (r0, r1) = (start.radius(), probe.radius());
        let (v0, v1) = (circular_speed(self.mu, r0), circular_speed(self.mu, r1));
        let delta_v = km_per_s_to_km_per_day(self.depart.0 + self.arrive.0) + (v0 - v1).abs();
        let travel_days = delta_v / drive.accel;
        if !(travel_days > 0.0 && travel_days.is_finite()) {
            return Err(Infeasible::LowThrustNeedsCircularEnds);
        }
        let end_day = start_day + travel_days;
        let (_, end) = self.ends(start_day, end_day).ok_or(Infeasible::NoPath)?;
        let spiral = Spiral::new(
            self.mu,
            start_day,
            r0,
            start.angle(),
            r1,
            end.angle(),
            (v0 - v1).abs() / travel_days,
        )
        .ok_or(Infeasible::LowThrustNeedsCircularEnds)?;
        let burns = LegBurns {
            start_kms: km_per_day_to_km_per_s(delta_v),
            end_kms: 0.0,
        };
        Ok((
            Crossing {
                start_day,
                end_day,
                start,
                end,
                path_start_velocity: spiral.state_at(start_day).velocity,
                path_end_velocity: spiral.state_at(end_day).velocity,
                segment: SegmentKind::Spiral {
                    primary: self.primary,
                    spiral,
                },
            },
            burns,
        ))
    }

    /// XFER-07: a crossing whose path dips inside `min_periapsis_radii`
    /// mean radii of the primary, while on the crossing, is infeasible. A
    /// crossing that starts or ends in an orbit of the primary itself is
    /// exempt: a Low band lies inside that limit by construction, and the
    /// band is the place, not a near miss (F-148).
    fn check_periapsis(&self, crossing: &Crossing) -> Result<(), Infeasible> {
        let SegmentKind::Conic { conic, .. } = crossing.segment else {
            return Ok(());
        };
        if self.from.host == self.primary || self.to.host == self.primary {
            return Ok(());
        }
        let primary_radius = self
            .tree
            .bodies
            .get(&self.primary)
            .map_or(0.0, |b| b.radius_km);
        let periapsis_km = conic.periapsis_radius_km();
        if periapsis_km < self.tunables.min_periapsis_radii * primary_radius
            && conic.passes_periapsis_between(crossing.start_day, crossing.end_day)
        {
            return Err(Infeasible::Periapsis { periapsis_km });
        }
        Ok(())
    }

    /// XFER-09 / XFER-10: the closing speed at an Intercept: through a body's
    /// High band, the excess speed boosted by the body's gravity at that
    /// radius; at a ship, the plain relative speed.
    fn intercept_speed_kms(&self, crossing: &Crossing) -> Result<f64, Infeasible> {
        let v_inf = (crossing.path_end_velocity - crossing.end.velocity).length();
        let through_body = match self.target {
            Target::Orbit {
                ship: false, node, ..
            } => Some(node.body),
            _ => None,
        };
        let Some(body) = through_body else {
            return Ok(km_per_day_to_km_per_s(v_inf));
        };
        let high = self
            .tree
            .band_orbit(body, Band::High)
            .and_then(|orbit| self.tree.orbits.get(&orbit))
            .ok_or(Infeasible::NoHighBand)?;
        let mu = self.tree.mu(body);
        Ok(km_per_day_to_km_per_s(
            (v_inf * v_inf + 2.0 * mu / high.radius_km).sqrt(),
        ))
    }

    /// BAND-06 / XFER-08 / XFER-10: what happens at the crossing's end,
    /// and the final segment it leaves the ship on.
    fn landing(&self, crossing: &Crossing, arrival_day: f64) -> (Landing, Option<SegmentKind>) {
        if !self.rendezvous {
            return (Landing::Intercept, None);
        }
        match self.target {
            Target::Orbit { orbit, node, .. } => {
                let relative = crossing.path_end_velocity - crossing.end.velocity;
                let direction = if relative.length() > 0.0 {
                    angle_of(relative)
                } else {
                    crossing.end.angle()
                };
                let phase0_rad = phase_for_angle(&node, direction, arrival_day);
                (
                    Landing::Orbit {
                        orbit,
                        body: node.body,
                        phase0_rad,
                    },
                    Some(SegmentKind::Parked {
                        orbit,
                        primary: node.body,
                        path: node.path(phase0_rad),
                    }),
                )
            }
            Target::Coasting { primary, conic } => {
                (Landing::Coast, Some(SegmentKind::Conic { primary, conic }))
            }
        }
    }
}

/// The segment a ship follows before its crossing: parked where it is,
/// or coasting on as it was.
fn departure_segment(tree: &SystemTree, departure: &Departure) -> Option<SegmentKind> {
    Some(match *departure {
        Departure::Parked { orbit, phase0_rad } => {
            let node = tree.orbits.get(&orbit)?;
            SegmentKind::Parked {
                orbit,
                primary: node.body,
                path: node.path(phase0_rad),
            }
        }
        Departure::Coasting { primary, conic } => SegmentKind::Conic { primary, conic },
    })
}

/// XFER-02: the leg's rail: the departure segment until the crossing
/// starts, the crossing, then (for a Rendezvous) the destination segment from
/// the crossing's end; an Intercept extends the crossing without end. A
/// low-thrust Intercept coasts on from the spiral's end on the circle it
/// reached.
fn build_trajectory(
    setup: &Setup,
    departure: &Departure,
    order: &Order,
    crossing: &Crossing,
    last: Option<SegmentKind>,
) -> Result<ShipTrajectory, Infeasible> {
    let first = departure_segment(setup.tree, departure).ok_or(Infeasible::UnknownPlace)?;
    let mut segments = vec![Segment {
        start_day: order.depart_day.min(crossing.start_day),
        end_day: Some(crossing.start_day),
        kind: first,
    }];
    let last = match (last, crossing.segment) {
        (Some(last), _) => Some(last),
        (None, SegmentKind::Spiral { primary, spiral }) => {
            let conic = Conic::from_state(
                setup.mu,
                spiral.state_at(crossing.end_day),
                crossing.end_day,
                setup.tunables.parabola_epsilon,
            )
            .ok_or(Infeasible::NoPath)?;
            Some(SegmentKind::Conic { primary, conic })
        }
        (None, _) => None,
    };
    segments.push(Segment {
        start_day: crossing.start_day,
        end_day: last.map(|_| crossing.end_day),
        kind: crossing.segment,
    });
    if let Some(last) = last {
        segments.push(Segment {
            start_day: crossing.end_day,
            end_day: None,
            kind: last,
        });
    }
    ShipTrajectory::new(segments).map_err(|_| Infeasible::NoPath)
}

/// XFER-01 to XFER-10: the leg a ship with `drive` and `budget_kms` of
/// delta-v to spare flies for `order` from `departure`, or why it can't.
pub fn solve_leg(
    tree: &SystemTree,
    tunables: &SimTunables,
    departure: &Departure,
    order: &Order,
    drive: &Drive,
    budget_kms: f64,
) -> Result<SolvedLeg, Infeasible> {
    let (setup, origin) = Setup::new(tree, tunables, departure, order)?;
    let (crossing, burns) = if drive.impulsive {
        setup.impulsive(order, drive)?
    } else {
        setup.low_thrust(order, drive)?
    };
    setup.check_periapsis(&crossing)?;
    // XFER-04: Rendezvous pays both ends; Intercept only the departure.
    let delta_v_kms = burns.start_kms + burns.end_kms;
    if delta_v_kms > budget_kms {
        return Err(Infeasible::DeltaV {
            needed_kms: delta_v_kms,
            available_kms: budget_kms,
        });
    }
    let arrival_day = crossing.end_day + setup.arrive.1;
    let (landing, last) = setup.landing(&crossing, arrival_day);
    let intercept_speed_kms = if setup.rendezvous {
        None
    } else {
        Some(setup.intercept_speed_kms(&crossing)?)
    };
    let trajectory = build_trajectory(&setup, departure, order, &crossing, last)?;
    Ok(SolvedLeg {
        origin,
        primary: setup.primary,
        crossing_start_day: crossing.start_day,
        crossing_end_day: crossing.end_day,
        arrival_day,
        travel_days: crossing.end_day - crossing.start_day,
        trajectory,
        delta_v_kms,
        burns,
        landing,
        intercept_speed_kms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::body::BodyClass;
    use crate::sim::transfer::places::{band_radii, hill_radius_km};
    use crate::sim::transfer::tree::{BodyNode, OrbitNode, StationNode};
    use crate::sim::units::{
        EARTH_MASS_KG, GRAVITATIONAL_CONSTANT, KM_PER_AU, M_PER_KM, SOLAR_MASS_KG,
        STANDARD_GRAVITY_M_PER_S2, gravitational_parameter_km3_per_day2,
    };
    use orbits::{circular_period, hohmann, synodic_period};

    /// The reference layout of XFER-12: a star of 0.75 solar masses, a
    /// capital of radius 5,753 km and 0.82 g at 0.8 AU, an inner planet at
    /// 0.5 AU, with their bands; a station in the capital's Low band and a
    /// gate round the star for the docking cases.
    struct Layout {
        tree: SystemTree,
        tunables: SimTunables,
        star: Entity,
        inner: Entity,
        capital: Entity,
        station: Entity,
        gate: Entity,
        ntr: Drive,
        nep: Drive,
    }

    /// The layout station's phase in the capital's Low band.
    const STATION_PHASE: f64 = 0.5;

    fn capital_mass_kg() -> f64 {
        let radius_m = 5_753.0 * M_PER_KM;
        0.82 * STANDARD_GRAVITY_M_PER_S2 * radius_m * radius_m / GRAVITATIONAL_CONSTANT
    }

    fn layout() -> Layout {
        let mut world = World::new();
        let mut spawn = || world.spawn_empty().id();
        let [star, inner, capital, station, gate, gate_orbit] = [(); 6].map(|_| spawn());
        let tunables = SimTunables::default();
        let star_mass = 0.75 * SOLAR_MASS_KG;
        let star_mu = gravitational_parameter_km3_per_day2(star_mass);
        let mut tree = SystemTree::default();
        let body = |tree: &mut SystemTree,
                    entity,
                    parent,
                    class,
                    mass_kg,
                    radius_km,
                    orbit: Option<CircularOrbit>,
                    parent_mass| {
            let hill_km = orbit.and_then(|o| hill_radius_km(o.radius_km, mass_kg, parent_mass));
            tree.bodies.insert(
                entity,
                BodyNode {
                    parent,
                    class,
                    mass_kg,
                    mu: gravitational_parameter_km3_per_day2(mass_kg),
                    radius_km,
                    orbit,
                    hill_km,
                },
            );
            hill_km
        };
        body(
            &mut tree,
            star,
            None,
            BodyClass::Star,
            star_mass,
            500_000.0,
            None,
            0.0,
        );
        let inner_hill = body(
            &mut tree,
            inner,
            Some(star),
            BodyClass::Planet,
            0.3 * EARTH_MASS_KG,
            3_000.0,
            Some(CircularOrbit::new(star_mu, 0.5 * KM_PER_AU, 0.0)),
            star_mass,
        );
        let capital_hill = body(
            &mut tree,
            capital,
            Some(star),
            BodyClass::Planet,
            capital_mass_kg(),
            5_753.0,
            Some(CircularOrbit::new(star_mu, 0.8 * KM_PER_AU, 1.0)),
            star_mass,
        );
        body(
            &mut tree,
            gate,
            Some(star),
            BodyClass::Gate,
            0.0,
            5.0,
            Some(CircularOrbit::new(star_mu, 2.0 * KM_PER_AU, 2.0)),
            star_mass,
        );
        for (entity, radius_km, hill, mass) in [
            (star, 500_000.0, None, star_mass),
            (inner, 3_000.0, inner_hill, 0.3 * EARTH_MASS_KG),
            (capital, 5_753.0, capital_hill, capital_mass_kg()),
        ] {
            let radii = band_radii(radius_km, hill, &tunables);
            for band in [Band::Low, Band::High] {
                let Some(r) = radii.radius_km(band) else {
                    continue;
                };
                tree.orbits.insert(
                    spawn(),
                    OrbitNode {
                        body: entity,
                        radius_km: r,
                        period_days: circular_period(gravitational_parameter_km3_per_day2(mass), r),
                        band: Some(band),
                        docking_for: None,
                    },
                );
            }
        }
        // SCL-07: the station is a place in the capital's Low band, at its
        // own phase.
        let low = tree.band_orbit(capital, Band::Low).unwrap();
        let low_node = tree.orbits[&low];
        body(
            &mut tree,
            station,
            Some(capital),
            BodyClass::Station,
            0.0,
            1.0,
            Some(low_node.path(STATION_PHASE)),
            capital_mass_kg(),
        );
        tree.stations.insert(
            station,
            StationNode {
                orbit: low,
                phase0_rad: STATION_PHASE,
            },
        );
        tree.orbits.insert(
            gate_orbit,
            OrbitNode {
                body: star,
                radius_km: 2.0 * KM_PER_AU,
                period_days: circular_period(star_mu, 2.0 * KM_PER_AU),
                band: None,
                docking_for: Some(gate),
            },
        );
        Layout {
            tree,
            tunables,
            star,
            inner,
            capital,
            station,
            gate,
            ntr: Drive::new(true, 9.0, 0.2 * STANDARD_GRAVITY_M_PER_S2),
            nep: Drive::new(false, 35.0, 0.048e-3 * STANDARD_GRAVITY_M_PER_S2),
        }
    }

    impl Layout {
        fn band(&self, body: Entity, band: Band) -> Entity {
            self.tree.band_orbit(body, band).expect("the band exists")
        }

        fn parked(&self, body: Entity, band: Band) -> Departure {
            Departure::Parked {
                orbit: self.band(body, band),
                phase0_rad: 0.2,
            }
        }

        fn solve(
            &self,
            from: &Departure,
            order: &Order,
            drive: &Drive,
        ) -> Result<SolvedLeg, Infeasible> {
            solve_leg(&self.tree, &self.tunables, from, order, drive, 100.0)
        }

        /// The cheapest leg for `order` over one synodic period of
        /// departure days, a day at a time.
        fn cheapest(&self, from: &Departure, order: Order, drive: &Drive) -> (Order, SolvedLeg) {
            let inner = self.tree.bodies[&self.inner].orbit.unwrap();
            let capital = self.tree.bodies[&self.capital].orbit.unwrap();
            let synodic = synodic_period(inner.period_days, capital.period_days);
            let mut best: Option<(Order, SolvedLeg)> = None;
            let mut depart = 0.0;
            while depart < synodic {
                let order = Order {
                    depart_day: depart,
                    ..order
                };
                if let Ok(leg) = self.solve(from, &order, drive)
                    && best
                        .as_ref()
                        .is_none_or(|(_, b)| leg.delta_v_kms < b.delta_v_kms)
                {
                    best = Some((order, leg));
                }
                depart += 1.0;
            }
            best.expect("some departure day works")
        }

        fn charges(&self, body: Entity, band: Band) -> (f64, f64) {
            let node = self.tree.bodies[&body];
            let r = self.tree.orbits[&self.band(body, band)].radius_km;
            (
                departure_cost_kms(node.mu, r),
                approach_days(node.mu, r, node.hill_km),
            )
        }
    }

    fn rendezvous_with(body: Entity, band: Band, travel_days: f64) -> Order {
        Order {
            destination: Place::Body { body, band },
            arrival: Arrival::Rendezvous,
            depart_day: 0.0,
            travel_days,
        }
    }

    fn within(actual: f64, expected: f64, fraction: f64, what: &str) {
        assert!(
            ((actual - expected) / expected).abs() < fraction,
            "{what}: expected about {expected}, got {actual}"
        );
    }

    /// XFER-01 / XFER-03 / XFER-04 / XFER-12: the cheapest two-burn leg from
    /// the capital's High band to the inner planet's, in 111 days, goes
    /// round the star, with burns of about 7.5 km/s plus the two bands'
    /// departure and capture costs (about 0.9 km/s at the capital).
    #[test]
    fn a_planet_to_planet_leg_costs_the_hohmann_burns_plus_the_bands() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let (_, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        assert_eq!(leg.primary, l.star);
        let (depart_cost, _) = l.charges(l.capital, Band::High);
        let (capture_cost, _) = l.charges(l.inner, Band::High);
        within(depart_cost, 0.9, 0.03, "capital High departure cost");
        let burns = leg.delta_v_kms - depart_cost - capture_cost;
        within(burns, 7.5, 0.04, "burn 1 + burn 2");
        assert!((leg.burns.start_kms + leg.burns.end_kms - leg.delta_v_kms).abs() < 1e-9);
        assert!(leg.burns.start_kms > depart_cost && leg.burns.end_kms > capture_cost);
        assert_eq!(
            leg.origin,
            Some(Place::Body {
                body: l.capital,
                band: Band::High
            })
        );
    }

    /// XFER-02 / XFER-05: the crossing starts after the departure approach
    /// days and the burn days (departure cost plus burn 1 over the
    /// acceleration), lasts the ordered travel days, and arrival follows
    /// the arrival approach days. The rail is parked, conic, parked.
    #[test]
    fn leg_timing_adds_approach_and_burn_days_at_each_end() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let (order, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        let (_, depart_approach) = l.charges(l.capital, Band::High);
        let (_, arrive_approach) = l.charges(l.inner, Band::High);
        assert!(
            depart_approach > 0.5 && arrive_approach > 0.5,
            "the bands are days from the Hill sphere"
        );
        let burn_days = km_per_s_to_km_per_day(leg.burns.start_kms) / l.ntr.accel;
        assert!(
            burn_days > 0.01,
            "a 0.2 g burn of a few km/s takes tens of minutes"
        );
        let expected_start = order.depart_day + depart_approach + burn_days;
        assert!(
            (leg.crossing_start_day - expected_start).abs() < 1e-3,
            "{} vs {expected_start}",
            leg.crossing_start_day
        );
        assert!((leg.crossing_end_day - leg.crossing_start_day - 111.0).abs() < 1e-9);
        assert_eq!(leg.travel_days, 111.0);
        assert!((leg.arrival_day - leg.crossing_end_day - arrive_approach).abs() < 1e-9);

        let segments = leg.trajectory.segments();
        assert_eq!(segments.len(), 3);
        assert_eq!(
            segments[0].kind.parked_in(),
            Some(l.band(l.capital, Band::High))
        );
        assert_eq!(segments[0].start_day, order.depart_day);
        assert!(
            matches!(segments[1].kind, SegmentKind::Conic { primary, .. } if primary == l.star)
        );
        assert_eq!(
            segments[2].kind.parked_in(),
            Some(l.band(l.inner, Band::High))
        );
        assert_eq!(segments[2].end_day, None);
        // The conic joins the two planets' positions.
        let capital = l.tree.bodies[&l.capital].orbit.unwrap();
        let inner = l.tree.bodies[&l.inner].orbit.unwrap();
        let start = leg.trajectory.state_at(leg.crossing_start_day).unwrap();
        assert!((start.position - capital.position_at(leg.crossing_start_day)).length() < 1.0);
        let end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-6)
            .unwrap();
        assert!((end.position - inner.position_at(leg.crossing_end_day)).length() < 100.0);
    }

    /// XFER-01: a trip between two bands of one body has the body as
    /// primary, no departure or capture cost and no approach days: at the
    /// Hohmann travel time it costs exactly the Hohmann burns.
    #[test]
    fn a_band_to_band_leg_goes_round_the_body_at_no_extra_cost() {
        let l = layout();
        let capital = l.tree.bodies[&l.capital];
        let low = l.tree.orbits[&l.band(l.capital, Band::Low)].radius_km;
        let high = l.tree.orbits[&l.band(l.capital, Band::High)].radius_km;
        let expected = hohmann(capital.mu, low, high);
        let order = rendezvous_with(l.capital, Band::High, expected.days);
        let leg = l
            .solve(&l.parked(l.capital, Band::Low), &order, &l.ntr)
            .unwrap();
        assert_eq!(leg.primary, l.capital);
        let burns_kms = km_per_day_to_km_per_s(expected.first_burn + expected.second_burn);
        within(leg.delta_v_kms, burns_kms, 0.01, "band to band delta-v");
        let burn_days = km_per_s_to_km_per_day(leg.burns.start_kms) / l.ntr.accel;
        assert!(
            (leg.crossing_start_day - burn_days).abs() < 1e-6,
            "no approach days"
        );
        assert_eq!(leg.arrival_day, leg.crossing_end_day);
        assert!(
            matches!(leg.landing, Landing::Orbit { orbit, body, .. } if orbit == l.band(l.capital, Band::High) && body == l.capital)
        );
    }

    /// XFER-06: a low-thrust leg is a spiral between the two ends'
    /// circular orbits round the primary; its delta-v is the two bands'
    /// costs plus the difference of circular speeds, its length that over
    /// the acceleration, and it lands on the inner planet's angle on the
    /// crossing's last day. Its whole delta-v is charged at the start.
    #[test]
    fn a_low_thrust_leg_is_a_spiral_of_fixed_length() {
        let l = layout();
        let star_mu = l.tree.bodies[&l.star].mu;
        let order = rendezvous_with(l.inner, Band::High, 1.0);
        let leg = l
            .solve(&l.parked(l.capital, Band::High), &order, &l.nep)
            .unwrap();
        let (depart_cost, _) = l.charges(l.capital, Band::High);
        let (capture_cost, _) = l.charges(l.inner, Band::High);
        let speeds = km_per_day_to_km_per_s(
            (circular_speed(star_mu, 0.8 * KM_PER_AU) - circular_speed(star_mu, 0.5 * KM_PER_AU))
                .abs(),
        );
        within(
            leg.delta_v_kms,
            depart_cost + speeds + capture_cost,
            1e-6,
            "spiral delta-v",
        );
        let expected_days = km_per_s_to_km_per_day(leg.delta_v_kms) / l.nep.accel;
        within(leg.travel_days, expected_days, 1e-9, "spiral days");
        assert!(
            leg.travel_days > 100.0 && leg.travel_days < 400.0,
            "{}",
            leg.travel_days
        );
        assert_eq!(
            leg.burns,
            LegBurns {
                start_kms: leg.delta_v_kms,
                end_kms: 0.0
            }
        );
        assert!(matches!(
            leg.trajectory.segments()[1].kind,
            SegmentKind::Spiral { .. }
        ));
        let inner = l.tree.bodies[&l.inner].orbit.unwrap();
        let end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-6)
            .unwrap();
        assert!((end.position - inner.position_at(leg.crossing_end_day)).length() < 1_000.0);
        let start = leg.trajectory.state_at(leg.crossing_start_day).unwrap();
        within(start.radius(), 0.8 * KM_PER_AU, 1e-6, "spiral start radius");
    }

    /// XFER-06: a low-thrust crossing is only solved between circular
    /// ends: from a coasting state it is infeasible.
    #[test]
    fn a_low_thrust_leg_from_a_coast_is_infeasible() {
        let l = layout();
        let star_mu = l.tree.bodies[&l.star].mu;
        let path = CircularOrbit::new(star_mu, 0.6 * KM_PER_AU, 0.0);
        let conic = Conic::from_state(star_mu, path.state_at(0.0), 0.0, 1e-6).unwrap();
        let from = Departure::Coasting {
            primary: l.star,
            conic,
        };
        let order = rendezvous_with(l.inner, Band::High, 1.0);
        assert_eq!(
            l.solve(&from, &order, &l.nep),
            Err(Infeasible::LowThrustNeedsCircularEnds)
        );
        // The same coast is fine for an impulsive drive, with no departure
        // cost and no origin place.
        let (_, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 60.0), &l.ntr);
        assert_eq!(leg.origin, None);
        let (_, capture_cost) = (0.0, l.charges(l.inner, Band::High).0);
        assert!(leg.burns.end_kms > capture_cost);
    }

    /// XFER-07: a crossing whose path dips inside `min_periapsis_radii`
    /// of the primary, while on the crossing, is infeasible. An outward
    /// transfer starts at its periapsis, so with the limit raised past
    /// the inner planet's orbit it is refused; at the default limit it is
    /// fine, and so is an inward transfer that ends before periapsis.
    #[test]
    fn a_crossing_too_close_to_the_primary_is_infeasible() {
        let mut l = layout();
        let from = l.parked(l.inner, Band::High);
        let (order, _) = l.cheapest(&from, rendezvous_with(l.capital, Band::High, 111.0), &l.ntr);
        assert!(l.solve(&from, &order, &l.ntr).is_ok());
        l.tunables.min_periapsis_radii = 0.6 * KM_PER_AU / 500_000.0;
        let result = l.solve(&from, &order, &l.ntr);
        assert!(
            matches!(result, Err(Infeasible::Periapsis { periapsis_km }) if periapsis_km < 0.6 * KM_PER_AU),
            "{result:?}"
        );
        // Inward, the periapsis lies past the arrival point, so the limit
        // is never reached on the crossing.
        let from = l.parked(l.capital, Band::High);
        let (order, _) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        assert!(l.solve(&from, &order, &l.ntr).is_ok());
    }

    /// XFER-08 / XFER-09: an Intercept through the High band pays departure and
    /// burn 1 only, has no arrival approach, leaves the crossing conic
    /// open-ended, and reports the closing speed boosted by the body's
    /// gravity at the band's radius. An Intercept through Low is not specced.
    #[test]
    fn an_intercept_skips_capture_and_coasts_on() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let intercept = Order {
            arrival: Arrival::Intercept { band: Band::High },
            ..rendezvous_with(l.inner, Band::High, 111.0)
        };
        let (order, leg) = l.cheapest(&from, intercept, &l.ntr);
        let rendezvous = l
            .solve(
                &from,
                &Order {
                    arrival: Arrival::Rendezvous,
                    ..order
                },
                &l.ntr,
            )
            .unwrap();
        assert!((leg.delta_v_kms - rendezvous.burns.start_kms).abs() < 1e-9);
        assert_eq!(leg.burns.end_kms, 0.0);
        assert_eq!(leg.arrival_day, leg.crossing_end_day);
        assert_eq!(leg.landing, Landing::Intercept);
        let segments = leg.trajectory.segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[1].end_day, None);
        assert!(matches!(segments[1].kind, SegmentKind::Conic { .. }));

        let inner = l.tree.bodies[&l.inner];
        let arc_end = leg.trajectory.state_at(leg.crossing_end_day).unwrap();
        let v_inf = (arc_end.velocity
            - inner.orbit.unwrap().state_at(leg.crossing_end_day).velocity)
            .length();
        let high = l.tree.orbits[&l.band(l.inner, Band::High)].radius_km;
        let expected = km_per_day_to_km_per_s((v_inf * v_inf + 2.0 * inner.mu / high).sqrt());
        within(
            leg.intercept_speed_kms.unwrap(),
            expected,
            1e-5,
            "intercept speed",
        );
        assert!(leg.intercept_speed_kms.unwrap() > km_per_day_to_km_per_s(v_inf));

        let low = Order {
            arrival: Arrival::Intercept { band: Band::Low },
            ..order
        };
        assert_eq!(
            l.solve(&from, &low, &l.ntr),
            Err(Infeasible::PassThroughLow)
        );
    }

    /// XFER-10: a parked ship as a destination lands the leg in its named
    /// orbit; a coasting ship round another primary is infeasible, as is a
    /// ship mid-crossing.
    #[test]
    fn a_ship_can_be_a_destination() {
        let mut l = layout();
        let mut world = World::new();
        let [target, drifter, flier] = [(); 3].map(|_| world.spawn_empty().id());
        let inner_high = l.band(l.inner, Band::High);
        l.tree.ships.insert(
            target,
            ShipState::Parked {
                orbit: inner_high,
                phase0_rad: 1.0,
            },
        );
        let capital_mu = l.tree.bodies[&l.capital].mu;
        let path = CircularOrbit::new(capital_mu, 30_000.0, 0.0);
        let conic = Conic::from_state(capital_mu, path.state_at(0.0), 0.0, 1e-6).unwrap();
        l.tree.ships.insert(
            drifter,
            ShipState::Coasting {
                primary: l.capital,
                conic,
            },
        );
        l.tree.ships.insert(flier, ShipState::InFlight);

        let from = l.parked(l.capital, Band::High);
        let to_ship = |ship| Order {
            destination: Place::Ship { ship },
            ..rendezvous_with(l.inner, Band::High, 111.0)
        };
        let (_, leg) = l.cheapest(&from, to_ship(target), &l.ntr);
        assert!(matches!(leg.landing, Landing::Orbit { orbit, .. } if orbit == inner_high));

        let from_inner = l.parked(l.inner, Band::High);
        assert_eq!(
            l.solve(&from_inner, &to_ship(drifter), &l.ntr),
            Err(Infeasible::ShipPrimary)
        );
        assert_eq!(
            l.solve(&from, &to_ship(flier), &l.ntr),
            Err(Infeasible::TargetInFlight)
        );

        // From the capital's own band the drifter is reachable, and a Rendezvous
        // shares its conic.
        let (_, leg) = l.cheapest(
            &from,
            Order {
                travel_days: 0.2,
                ..to_ship(drifter)
            },
            &l.ntr,
        );
        assert_eq!(leg.primary, l.capital);
        assert_eq!(leg.landing, Landing::Coast);
        assert!(
            matches!(leg.trajectory.segments()[2].kind, SegmentKind::Conic { conic: c, .. } if c == conic)
        );
    }

    /// XFER-04: a leg that needs more delta-v than the ship has available
    /// is infeasible, and says by how much.
    #[test]
    fn a_leg_beyond_the_budget_is_infeasible() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let (order, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        let short = solve_leg(&l.tree, &l.tunables, &from, &order, &l.ntr, 1.0);
        assert_eq!(
            short,
            Err(Infeasible::DeltaV {
                needed_kms: leg.delta_v_kms,
                available_kms: 1.0
            })
        );
    }

    /// BAND-06: a ship that captures takes the phase of its arrival
    /// direction: the angle of its velocity relative to the body on the
    /// arrival day.
    #[test]
    fn a_captured_ship_sits_at_its_arrival_direction() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let (_, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        let Landing::Orbit { phase0_rad, .. } = leg.landing else {
            panic!()
        };
        let inner = l.tree.bodies[&l.inner].orbit.unwrap();
        let arc_end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-9)
            .unwrap();
        let direction = angle_of(arc_end.velocity - inner.state_at(leg.crossing_end_day).velocity);
        let node = l.tree.orbits[&l.band(l.inner, Band::High)];
        let parked = node.path(phase0_rad);
        let at_arrival = orbits::wrap_angle(parked.angle_at(leg.arrival_day));
        assert!((at_arrival - orbits::wrap_angle(direction)).abs() < 1e-9);
        let SegmentKind::Parked { path, .. } = leg.trajectory.segments()[2].kind else {
            panic!()
        };
        assert_eq!(path, parked);
    }

    /// PLAN-03: a leg solved twice from the same order and system is the
    /// same leg, trajectory included.
    #[test]
    fn solving_the_same_order_twice_gives_the_same_leg() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let (order, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        assert_eq!(l.solve(&from, &order, &l.ntr).unwrap(), leg);
    }

    /// BAND-02 / SCL-07 / PLAN-12: a station is the band it occupies, so
    /// a ship sent there arrives in that band; a ship docked at it sits at
    /// its phase and records it as its origin, while a ship elsewhere in
    /// the band sets out from the band. A gate or site is its docking
    /// orbit.
    #[test]
    fn a_station_is_a_place_in_its_band() {
        let l = layout();
        let capital_mu = l.tree.bodies[&l.capital].mu;
        let low = l.band(l.capital, Band::Low);
        let high_km = l.tree.orbits[&l.band(l.capital, Band::High)].radius_km;
        let low_km = l.tree.orbits[&low].radius_km;
        let order = Order {
            destination: Place::Station { station: l.station },
            ..rendezvous_with(
                l.capital,
                Band::High,
                hohmann(capital_mu, high_km, low_km).days,
            )
        };
        let leg = l
            .solve(&l.parked(l.capital, Band::High), &order, &l.ntr)
            .unwrap();
        assert_eq!(leg.primary, l.capital);
        assert!(matches!(leg.landing, Landing::Orbit { orbit, .. } if orbit == low));

        let gate = Order {
            destination: Place::Gate { gate: l.gate },
            ..rendezvous_with(l.capital, Band::High, 200.0)
        };
        let elsewhere_in_low = Departure::Parked {
            orbit: low,
            phase0_rad: STATION_PHASE + 1.0,
        };
        let (_, leg) = l.cheapest(&elsewhere_in_low, gate, &l.ntr);
        assert_eq!(
            leg.origin,
            Some(Place::Body {
                body: l.capital,
                band: Band::Low
            })
        );
        let from_station = Departure::Parked {
            orbit: low,
            phase0_rad: STATION_PHASE + TAU,
        };
        let (_, leg) = l.cheapest(&from_station, gate, &l.ntr);
        assert_eq!(leg.origin, Some(Place::Station { station: l.station }));
        assert_eq!(leg.primary, l.star);
        // BAND-03: a gate has no massive body, so arriving there costs
        // only burn 2.
        let arc_end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-9)
            .unwrap();
        let gate_v = l.tree.bodies[&l.gate]
            .orbit
            .unwrap()
            .state_at(leg.crossing_end_day)
            .velocity;
        let burn_2 = km_per_day_to_km_per_s((gate_v - arc_end.velocity).length());
        assert!(
            (leg.burns.end_kms - burn_2).abs() < 1e-3,
            "{} vs {burn_2}",
            leg.burns.end_kms
        );
        assert_eq!(leg.arrival_day, leg.crossing_end_day);

        let hold = Order {
            destination: Place::Hold,
            ..order
        };
        assert_eq!(
            l.solve(&from_station, &hold, &l.ntr),
            Err(Infeasible::HoldIsNotACrossing)
        );
    }
}

//! Solving one leg (XFER-01 to XFER-10, XFER-13, XFER-14, PHAS-03,
//! PHAS-10, PHAS-11, PHAS-15): from where a ship is and the order it was
//! given to the crossing it will fly, its timing, its trajectory and its delta-v, or
//! the reason it can't be flown. Pure functions over the system snapshot
//! (`tree.rs`) and plain values (ARCH-20, XFER-11); the maths is the
//! `orbits` crate's.
//!
//! A crossing goes round one primary, the lowest body both ends sit
//! under. The planet-sized legs at each end are not flown: they are a
//! cost in km/s and a few days (`places.rs`), and phase there only sets a
//! wait: for the exit point on leaving, for the target on arriving. An
//! impulsive drive gets a two-burn Lambert arc; a low-thrust drive gets a
//! spiral whose length the drive fixes. A destination in the ship's own
//! orbit is not a crossing at all (`phasing.rs`). An Intercept that skims
//! another body's Low band is swung round by it, and coasts on from
//! there on a new path round the primary.

use std::f64::consts::TAU;

use bevy::math::DVec3;
use bevy::prelude::*;
use orbits::{
    Conic, Rail, Segment, Sense, Spiral, State, capture_angle, circular_speed, closing_speed,
    exit_angle, lambert, pass_speed, turn_angle, turned_velocity,
};

use super::ends::{End, Own, Target, departure_end, phase_for_angle, resolve, target_end};
use super::order::{
    Arrival, Departure, Infeasible, Landing, LegBurns, Order, SolvedLeg, TargetLock,
};
use super::phase::{gap_rad, wait_days};
use super::phasing::solve_in_own_orbit;
use super::places::{Band, Place, band_radii};
use super::tree::{OrbitNode, SystemTree};
use crate::sim::rail::{
    SegmentKind, ShipTrajectory, km_per_day_to_km_per_s, km_per_s_to_km_per_day,
};
use crate::sim::ship::Drive;
use crate::sim::tunables::SimTunables;

/// How far under its limit a periapsis may come, as a fraction of the
/// limit, and still count as at it: room for the rounding of a path
/// solved to arrive at a Low band exactly (XFER-07).
const PERIAPSIS_ROUNDING: f64 = 1e-6;

/// Whether a periapsis is under `limit_km`, beyond rounding (XFER-07).
fn under_limit(periapsis_km: f64, limit_km: f64) -> bool {
    periapsis_km < limit_km * (1.0 - PERIAPSIS_ROUNDING)
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
    /// The band an Intercept passes through: the order's, or a station's
    /// or ship's own band (XFER-10).
    intercept_band: Option<Band>,
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
    path_start_velocity: DVec3,
    path_end_velocity: DVec3,
    segment: SegmentKind,
}

impl Crossing {
    /// The path's velocity at its end relative to the end state: the
    /// excess velocity at a body (BAND-06), or the closing velocity at a
    /// ship (XFER-10).
    fn arrival_relative_velocity(&self) -> DVec3 {
        self.path_end_velocity - self.end.velocity
    }
}

/// A crossing with its burns and the wait before it.
struct Flight {
    crossing: Crossing,
    burns: LegBurns,
    /// PHAS-03: days waiting for the exit point.
    departure_wait: f64,
}

/// XFER-13: an Intercept through another body's Low band. The body
/// swings the ship round, so from the crossing end day it coasts on a new
/// conic round the primary, not on the crossing's own.
struct LowPass {
    /// How far the pass turned the ship's velocity relative to the body,
    /// in radians, prograde.
    turn_rad: f64,
    /// The conic the ship coasts on after the pass, round the primary.
    coast: Conic,
}

/// How a Rendezvous ends: in an orbit or on a conic, and when.
struct Arrived {
    landing: Landing,
    last: Option<SegmentKind>,
    wait_days: f64,
    arrival_day: f64,
}

impl Setup<'_> {
    fn new<'a>(
        tree: &'a SystemTree,
        tunables: &'a SimTunables,
        departure: &Departure,
        order: &Order,
        target: Target,
    ) -> Result<(Setup<'a>, Option<Place>), Infeasible> {
        let (from, origin) = departure_end(tree, departure, tunables.together_km)?;
        let to = target_end(&target);
        let primary = tree
            .lowest_common_ancestor(from.at, to.at)
            .ok_or(Infeasible::UnknownPlace)?;
        if let Target::Coasting { primary: p, .. } = target
            && p != primary
        {
            return Err(Infeasible::ShipPrimary);
        }
        // XFER-10: an Intercept at a station or a parked ship takes the
        // target's band.
        let intercept_band = match (order.arrival, target) {
            (Arrival::Rendezvous, _) => None,
            (Arrival::Intercept { band }, Target::Point { node, .. }) => {
                Some(node.band.unwrap_or(band))
            }
            (Arrival::Intercept { band }, _) => Some(band),
        };
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
                intercept_band,
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

    /// The target's named orbit, if the target sits in an orbit of a body
    /// other than the primary: met at the edge of that body's frame, where
    /// phase only sets a wait.
    fn beyond_frame(&self) -> Option<OrbitNode> {
        let (_, node) = self.target.orbit()?;
        (node.body != self.primary).then_some(node)
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

    /// PHAS-03: an impulsive drive leaving an orbit of a body other than
    /// the primary waits until the ship reaches its exit point: the point
    /// of the orbit from which one burn sends it off along the crossing.
    /// `first` is the crossing solved with no wait and no burn days; the
    /// wait is the time from the ship's angle on the depart day round to
    /// that point, under one period. Leaving from the primary's own
    /// frame, or from a coast, there is nothing to wait for.
    fn departure_wait(&self, departure: &Departure, depart_day: f64, first: &Crossing) -> f64 {
        let (Departure::Parked { .. }, Own::Circular(path), Some(radius_km)) =
            (departure, self.from.own, self.from.radius_km)
        else {
            return 0.0;
        };
        let mu = self.tree.mu(self.from.host);
        if self.from.host == self.primary || mu <= 0.0 {
            return 0.0;
        }
        let v_inf = first.path_start_velocity - first.start.velocity;
        let exit = exit_angle(mu, radius_km, v_inf);
        wait_days(path.angle_at(depart_day), exit, path.period_days)
    }

    /// XFER-03 to XFER-05, PHAS-03: the two-burn crossing taking
    /// `travel_days`. A first solve, with no wait and no burn days, gives
    /// the exit point and burn 1, and so the wait and the burn days; the
    /// crossing is then solved once more from the later start, and that
    /// solve's burns are the leg's.
    fn impulsive(
        &self,
        order: &Order,
        drive: &Drive,
        departure: &Departure,
        travel_days: f64,
    ) -> Result<Flight, Infeasible> {
        if travel_days <= 0.0 || travel_days.is_nan() {
            return Err(Infeasible::NoPath);
        }
        let earliest = order.depart_day + self.depart.1;
        let first = self
            .lambert_crossing(earliest, travel_days)
            .ok_or(Infeasible::NoPath)?;
        let departure_wait = self.departure_wait(departure, order.depart_day, &first);
        let burn_1 = (first.path_start_velocity - first.start.velocity).length();
        let burn_days = (km_per_s_to_km_per_day(self.depart.0) + burn_1) / drive.accel;
        let crossing = self
            .lambert_crossing(earliest + departure_wait + burn_days, travel_days)
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
        Ok(Flight {
            crossing,
            burns,
            departure_wait,
        })
    }

    /// XFER-06: the spiral crossing between the two ends' circular orbits,
    /// whose length the drive fixes: the whole delta-v (costs included)
    /// over the acceleration, plus any `trim_days` (PHAS-15), which only
    /// make the spiral gentler. The spiral spans the crossing, so it lands
    /// at the end's angle on the crossing's last day.
    fn low_thrust(
        &self,
        order: &Order,
        drive: &Drive,
        trim_days: f64,
    ) -> Result<Flight, Infeasible> {
        if matches!(self.from.own, Own::Conic(_)) || matches!(self.target, Target::Coasting { .. })
        {
            return Err(Infeasible::LowThrustNeedsCircularEnds);
        }
        let start_day = order.depart_day + self.depart.1;
        let (start, probe) = self.ends(start_day, start_day).ok_or(Infeasible::NoPath)?;
        let (r0, r1) = (start.radius(), probe.radius());
        let (v0, v1) = (circular_speed(self.mu, r0), circular_speed(self.mu, r1));
        let delta_v = km_per_s_to_km_per_day(self.depart.0 + self.arrive.0) + (v0 - v1).abs();
        let travel_days = delta_v / drive.accel + trim_days;
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
        let crossing = Crossing {
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
        };
        Ok(Flight {
            crossing,
            burns,
            departure_wait: 0.0,
        })
    }

    /// The crossing for the order, lengthened by `trim_days`.
    fn fly(
        &self,
        order: &Order,
        drive: &Drive,
        departure: &Departure,
        trim_days: f64,
    ) -> Result<Flight, Infeasible> {
        if drive.impulsive {
            self.impulsive(order, drive, departure, order.travel_days + trim_days)
        } else {
            self.low_thrust(order, drive, trim_days)
        }
    }

    /// The station or parked ship an Intercept is trimmed to meet: one in
    /// an orbit of a body other than the primary (PHAS-15). Inside the
    /// primary's frame the crossing already ends where the target is
    /// (XFER-10).
    fn trim_target(&self) -> Option<(OrbitNode, f64)> {
        match self.target {
            Target::Point {
                node, phase0_rad, ..
            } if !self.rendezvous && node.body != self.primary => Some((node, phase0_rad)),
            _ => None,
        }
    }

    /// PHAS-15: an Intercept at a station or ship in another body's band
    /// meets it. The travel time is lengthened by a trim, under one period
    /// of the band, until the target is at the Intercept's closest-approach
    /// point on the crossing end day. A longer crossing arrives with a
    /// different velocity and so passes closest somewhere else, so the
    /// leg is solved again with each trim until the target is within
    /// `together_km` of that point along the band, or it gives up.
    fn trimmed(
        &self,
        order: &Order,
        drive: &Drive,
        departure: &Departure,
        (node, phase0_rad): (OrbitNode, f64),
    ) -> Result<(Flight, f64), Infeasible> {
        let target = node.path(phase0_rad);
        let mut trim = 0.0;
        for _ in 0..self.tunables.trim_max_iterations {
            let flight = self.fly(order, drive, departure, trim)?;
            let closest = self
                .closest_approach(&flight.crossing)
                .ok_or(Infeasible::CantLineUp)?;
            let still_to_go = gap_rad(target.angle_at(flight.crossing.end_day), closest);
            let miss_km = node.radius_km * still_to_go.min(TAU - still_to_go);
            if miss_km < self.tunables.together_km {
                return Ok((flight, trim));
            }
            trim = (trim + still_to_go / TAU * node.period_days).rem_euclid(node.period_days);
        }
        Err(Infeasible::CantLineUp)
    }

    /// XFER-07: the closest a conic of this leg may come to the primary,
    /// in km. Out in the open it is `min_periapsis_radii` mean radii. A
    /// conic that starts or ends round the primary itself (in a band, at a
    /// station, on a coast round it) has to get that close, so its limit
    /// is the primary's Low band instead: it may touch the band but not
    /// dip under it.
    fn periapsis_limit_km(&self, round_the_primary: bool) -> f64 {
        let Some(primary) = self.tree.bodies.get(&self.primary) else {
            return 0.0;
        };
        if round_the_primary {
            band_radii(primary.radius_km, primary.hill_km, self.tunables).low_km
        } else {
            self.tunables.min_periapsis_radii * primary.radius_km
        }
    }

    /// XFER-07: a crossing is infeasible if its conic reaches a periapsis
    /// under the limit while on the crossing. A spiral never dips below
    /// the lower of its two ends.
    fn check_periapsis(&self, crossing: &Crossing) -> Result<(), Infeasible> {
        let SegmentKind::Conic { conic, .. } = crossing.segment else {
            return Ok(());
        };
        let round_the_primary = self.from.host == self.primary || self.to.host == self.primary;
        let periapsis_km = conic.periapsis_radius_km();
        if under_limit(periapsis_km, self.periapsis_limit_km(round_the_primary))
            && conic.passes_periapsis_between(crossing.start_day, crossing.end_day)
        {
            return Err(Infeasible::Periapsis { periapsis_km });
        }
        Ok(())
    }

    /// XFER-07: an Intercept is infeasible if the conic the ship coasts on
    /// after it (the rail's last segment: the crossing run on, or the
    /// turned conic after a pass through Low) still has a periapsis under
    /// the limit ahead of it. The coast starts where the crossing ended,
    /// so it takes the Low band limit when that end is round the primary.
    fn check_coast(
        &self,
        crossing: &Crossing,
        trajectory: &ShipTrajectory,
    ) -> Result<(), Infeasible> {
        if self.rendezvous {
            return Ok(());
        }
        let Some(SegmentKind::Conic { conic, .. }) = trajectory.segments().last().map(|s| s.kind)
        else {
            return Ok(());
        };
        let periapsis_km = conic.periapsis_radius_km();
        let limit_km = self.periapsis_limit_km(self.to.host == self.primary);
        if under_limit(periapsis_km, limit_km)
            && conic.passes_periapsis_between(crossing.end_day, f64::INFINITY)
        {
            return Err(Infeasible::CoastPeriapsis { periapsis_km });
        }
        Ok(())
    }

    /// The ring an Intercept passes through: a station's or ship's own
    /// band, or the order's band of the destination body.
    fn intercept_ring(&self) -> Option<OrbitNode> {
        match self.target {
            Target::Point { node, .. } => Some(node),
            Target::Ring { node, .. } => {
                let orbit = self.tree.band_orbit(node.body, self.intercept_band?)?;
                self.tree.orbits.get(&orbit).copied()
            }
            Target::Coasting { .. } => None,
        }
    }

    /// BAND-06: where a ship arriving at `node`'s body from outside its
    /// frame comes closest to it: the closest point of the prograde
    /// hyperbola it arrives on, at the orbit's radius.
    fn capture_point(&self, node: &OrbitNode, crossing: &Crossing) -> f64 {
        capture_angle(
            self.tree.mu(node.body),
            node.radius_km,
            crossing.arrival_relative_velocity(),
        )
    }

    /// PHAS-11: an Intercept's closest-approach phase, as an angle round
    /// the body on the crossing end day: the capture point's formula with
    /// the intercept band's radius. Inside the primary's frame the
    /// crossing simply ends at its point. A coasting target has none.
    fn closest_approach(&self, crossing: &Crossing) -> Option<f64> {
        let (_, node) = self.target.orbit()?;
        if node.body == self.primary {
            return Some(crossing.end.angle());
        }
        Some(self.capture_point(&self.intercept_ring()?, crossing))
    }

    /// The band of another body an Intercept passes through: the
    /// destination body's band, or a station's or ship's own band beyond
    /// the primary's frame. `None` when the crossing ends at the target
    /// itself.
    fn ring_passed(&self) -> Result<Option<OrbitNode>, Infeasible> {
        Ok(match self.target {
            Target::Ring { .. } => Some(self.intercept_ring().ok_or(Infeasible::NoHighBand)?),
            Target::Point { node, .. } if node.body != self.primary => Some(node),
            _ => None,
        })
    }

    /// XFER-09 / XFER-10: the speed at an Intercept: through a body's
    /// band, the excess speed boosted by the body's gravity at the band's
    /// radius; at a ship or station whose state is the end state, the
    /// plain relative speed.
    fn intercept_speed_kms(&self, crossing: &Crossing) -> Result<f64, Infeasible> {
        let v_inf = crossing.arrival_relative_velocity().length();
        Ok(km_per_day_to_km_per_s(match self.ring_passed()? {
            Some(ring) => pass_speed(self.tree.mu(ring.body), ring.radius_km, v_inf),
            None => v_inf,
        }))
    }

    /// XFER-14 / XFER-10: the speed at which an Intercept at a station or
    /// a ship goes by it. A target in another body's band moves the way
    /// the pass goes round, so the pass closes at its own speed less the
    /// band's circular speed; a target whose state is the end state
    /// closes at the plain relative speed. An Intercept at a body has no
    /// target to close on.
    fn closing_speed_kms(&self, crossing: &Crossing) -> Option<f64> {
        let v_inf = crossing.arrival_relative_velocity().length();
        let closing = match self.target {
            Target::Ring { .. } => return None,
            Target::Point { node, .. } if node.body != self.primary => {
                closing_speed(self.tree.mu(node.body), node.radius_km, v_inf)
            }
            Target::Point { .. } | Target::Coasting { .. } => v_inf,
        };
        Some(km_per_day_to_km_per_s(closing))
    }

    /// XFER-13: an Intercept through the Low band of a body other than
    /// the primary is swung round by it. Its velocity relative to the body
    /// keeps its size and turns prograde by 2 arcsin(1 / e); the ship
    /// leaves from the body's position with the body's velocity plus the
    /// turned one. Costs no delta-v and no time. A pass through High, or
    /// inside the primary's own frame, takes no turn.
    fn low_pass(&self, crossing: &Crossing) -> Result<Option<LowPass>, Infeasible> {
        if self.rendezvous {
            return Ok(None);
        }
        let Some(ring) = self.ring_passed()? else {
            return Ok(None);
        };
        if ring.band != Some(Band::Low) || ring.body == self.primary {
            return Ok(None);
        }
        let mu = self.tree.mu(ring.body);
        let v_inf = crossing.arrival_relative_velocity();
        let velocity = crossing.end.velocity + turned_velocity(mu, ring.radius_km, v_inf);
        let coast = Conic::from_state(
            self.mu,
            State::new(crossing.end.position, velocity),
            crossing.end_day,
            self.tunables.parabola_epsilon,
        )
        .ok_or(Infeasible::NoPath)?;
        Ok(Some(LowPass {
            turn_rad: turn_angle(mu, ring.radius_km, v_inf.length()),
            coast,
        }))
    }

    /// BAND-06 / PHAS-10 / XFER-08 / XFER-10: what happens at the
    /// crossing's end: the orbit and phase the ship joins, or the conic it
    /// shares, and the day it gets there.
    fn arrive(&self, crossing: &Crossing) -> Arrived {
        let reached = crossing.end_day + self.arrive.1;
        let (landing, last, wait_days) = match self.target {
            _ if !self.rendezvous => (Landing::Intercept, None, 0.0),
            Target::Coasting { primary, conic } => (
                Landing::Coast,
                Some(SegmentKind::Conic { primary, conic }),
                0.0,
            ),
            Target::Ring { orbit, node } => {
                let angle = match self.beyond_frame() {
                    Some(_) => self.capture_point(&node, crossing),
                    None => crossing.end.angle(),
                };
                let phase0_rad = phase_for_angle(&node, angle, reached);
                let (landing, last) = parked_landing(orbit, node, phase0_rad);
                (landing, last, 0.0)
            }
            Target::Point {
                orbit,
                node,
                phase0_rad,
                ..
            } => {
                let wait = match self.beyond_frame() {
                    Some(_) => wait_days(
                        node.path(phase0_rad).angle_at(reached),
                        self.capture_point(&node, crossing),
                        node.period_days,
                    ),
                    None => 0.0,
                };
                let (landing, last) = parked_landing(orbit, node, phase0_rad);
                (landing, last, wait)
            }
        };
        Arrived {
            landing,
            last,
            wait_days,
            arrival_day: if self.rendezvous {
                reached + wait_days
            } else {
                crossing.end_day
            },
        }
    }
}

/// A ship joining `orbit` at `phase0_rad`: the landing and the segment
/// it parks on.
fn parked_landing(
    orbit: Entity,
    node: OrbitNode,
    phase0_rad: f64,
) -> (Landing, Option<SegmentKind>) {
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
/// starts (waits included), the crossing, then `last` from the crossing's
/// end: the destination segment of a Rendezvous, or the coast after an
/// Intercept through Low (XFER-13). Any other Intercept extends the
/// crossing without end, and a low-thrust one coasts on from the
/// spiral's end on the circle it reached.
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

/// PLAN-11: the parked ship a leg's destination was solved against.
fn target_lock(target: &Target) -> Option<TargetLock> {
    match *target {
        Target::Point { lock, .. } => lock,
        _ => None,
    }
}

/// XFER-01 to XFER-14, PHAS-04 to PHAS-15: the leg a ship with `drive`
/// and `budget_kms` of delta-v to spare flies for `order` from
/// `departure`, or why it can't. A destination in the ship's own orbit is
/// already reached or met by a phasing leg; anything else is a crossing.
pub fn solve_leg(
    tree: &SystemTree,
    tunables: &SimTunables,
    departure: &Departure,
    order: &Order,
    drive: &Drive,
    budget_kms: f64,
) -> Result<SolvedLeg, Infeasible> {
    let target = resolve(tree, order.destination)?;
    if let Departure::Parked { orbit, phase0_rad } = *departure
        && target.orbit().is_some_and(|(o, _)| o == orbit)
    {
        return solve_in_own_orbit(
            tree,
            tunables,
            (orbit, phase0_rad),
            &target,
            order,
            drive,
            budget_kms,
        );
    }
    let (setup, origin) = Setup::new(tree, tunables, departure, order, target)?;
    let (flight, trim_days) = match setup.trim_target() {
        Some(point) => {
            let (flight, trim) = setup.trimmed(order, drive, departure, point)?;
            (flight, Some(trim))
        }
        None => (setup.fly(order, drive, departure, 0.0)?, None),
    };
    let crossing = &flight.crossing;
    setup.check_periapsis(crossing)?;
    let arrived = setup.arrive(crossing);
    let (intercept_speed_kms, closing_speed_kms, closest_approach_rad) = if setup.rendezvous {
        (None, None, None)
    } else {
        (
            Some(setup.intercept_speed_kms(crossing)?),
            setup.closing_speed_kms(crossing),
            setup.closest_approach(crossing),
        )
    };
    // PLNR-26: what the arrival row reads.
    let arrival_speed_kms = if setup.rendezvous {
        Some(km_per_day_to_km_per_s(
            crossing.arrival_relative_velocity().length(),
        ))
    } else {
        closing_speed_kms.or(intercept_speed_kms)
    };
    let low_pass = setup.low_pass(crossing)?;
    let last = match &low_pass {
        Some(pass) => Some(SegmentKind::Conic {
            primary: setup.primary,
            conic: pass.coast,
        }),
        None => arrived.last,
    };
    let trajectory = build_trajectory(&setup, departure, order, crossing, last)?;
    setup.check_coast(crossing, &trajectory)?;
    // XFER-04: Rendezvous pays both ends; Intercept only the departure.
    let delta_v_kms = flight.burns.start_kms + flight.burns.end_kms;
    if delta_v_kms > budget_kms {
        return Err(Infeasible::DeltaV {
            needed_kms: delta_v_kms,
            available_kms: budget_kms,
        });
    }
    Ok(SolvedLeg {
        origin,
        primary: setup.primary,
        crossing_start_day: crossing.start_day,
        crossing_end_day: crossing.end_day,
        arrival_day: arrived.arrival_day,
        travel_days: crossing.end_day - crossing.start_day,
        trajectory,
        delta_v_kms,
        burns: flight.burns,
        landing: arrived.landing,
        intercept_speed_kms,
        closing_speed_kms,
        arrival_speed_kms,
        turn_rad: low_pass.map(|pass| pass.turn_rad),
        departure_wait_days: flight.departure_wait,
        arrival_wait_days: arrived.wait_days,
        closest_approach_rad,
        trim_days,
        target_lock: target_lock(&setup.target),
        phasing_laps: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::transfer::phase::separation_km;
    use crate::sim::transfer::test_layout::{
        FAR_STATION_PHASE, Layout, STATION_PHASE, layout, rendezvous_with, within,
    };
    use crate::sim::transfer::tree::ShipState;
    use crate::sim::units::KM_PER_AU;
    use orbits::{CircularOrbit, angle_of, hohmann};

    /// The angle between two directions, in [0, π].
    fn angle_between(a: f64, b: f64) -> f64 {
        let gap = gap_rad(a, b);
        gap.min(TAU - gap)
    }

    /// The excess velocity a leg arrives with at `body`: the rail's
    /// velocity just before the crossing ends, less the body's.
    fn arrival_v_inf(l: &Layout, leg: &SolvedLeg, body: Entity) -> DVec3 {
        let arc_end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-9)
            .unwrap();
        let body_orbit = l.tree.bodies[&body].orbit.unwrap();
        arc_end.velocity - body_orbit.state_at(leg.crossing_end_day).velocity
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

    /// XFER-02 / XFER-05 / PHAS-03: the crossing starts after the
    /// departure wait, the departure approach days and the burn days
    /// (departure cost plus burn 1 over the acceleration), lasts the
    /// ordered travel days, and arrival follows the arrival approach days.
    /// The rail is parked (through the wait), conic, parked.
    #[test]
    fn leg_timing_adds_wait_approach_and_burn_days() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let (order, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        let (_, depart_approach) = l.charges(l.capital, Band::High);
        let (_, arrive_approach) = l.charges(l.inner, Band::High);
        assert!(
            depart_approach > 0.5 && arrive_approach > 0.5,
            "the bands are days from the Hill sphere"
        );
        let period = l.node(l.capital, Band::High).period_days;
        assert!((0.0..period).contains(&leg.departure_wait_days));
        let burn_days = km_per_s_to_km_per_day(leg.burns.start_kms) / l.ntr.accel;
        assert!(
            burn_days > 0.01,
            "a 0.2 g burn of a few km/s takes tens of minutes"
        );
        let expected_start =
            order.depart_day + leg.departure_wait_days + depart_approach + burn_days;
        assert!(
            (leg.crossing_start_day - expected_start).abs() < 1e-3,
            "{} vs {expected_start}",
            leg.crossing_start_day
        );
        assert!((leg.crossing_end_day - leg.crossing_start_day - 111.0).abs() < 1e-9);
        assert_eq!(leg.travel_days, 111.0);
        assert_eq!(leg.arrival_wait_days, 0.0, "a band has nothing to wait for");
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

    /// BAND-05 / PHAS-03: two ships in one band at different phases, given
    /// the same order, pay the same to leave it but leave at different
    /// times: each waits, under one period, until it is at the exit point,
    /// the point from which one burn sends it along the crossing.
    #[test]
    fn ships_in_one_band_leave_from_the_exit_point_at_their_own_times() {
        let l = layout();
        let order = Order {
            depart_day: 40.0,
            ..rendezvous_with(l.inner, Band::High, 111.0)
        };
        let high = l.node(l.capital, Band::High);
        let capital = l.tree.bodies[&l.capital];
        let legs: Vec<SolvedLeg> = [0.2, 2.6]
            .map(|phase| {
                l.solve(&l.parked_at(l.capital, Band::High, phase), &order, &l.ntr)
                    .unwrap()
            })
            .into();
        let (depart_cost, _) = l.charges(l.capital, Band::High);
        assert_ne!(legs[0].departure_wait_days, legs[1].departure_wait_days);
        for (leg, phase) in legs.iter().zip([0.2, 2.6]) {
            assert!((0.0..high.period_days).contains(&leg.departure_wait_days));
            assert!(leg.burns.start_kms > depart_cost);
            // The ship's angle once it has waited is the exit point of the
            // crossing it flies, to within the drift of a few days' later
            // start.
            let leave_day = order.depart_day + leg.departure_wait_days;
            let ship_angle = high.path(phase).angle_at(leave_day);
            let arc_start = leg.trajectory.state_at(leg.crossing_start_day).unwrap();
            let capital_v = capital
                .orbit
                .unwrap()
                .state_at(leg.crossing_start_day)
                .velocity;
            let exit = exit_angle(capital.mu, high.radius_km, arc_start.velocity - capital_v);
            assert!(
                angle_between(ship_angle, exit) < 5f64.to_radians(),
                "{} vs {}",
                ship_angle.to_degrees(),
                exit.to_degrees()
            );
        }
        // BAND-05: on any day the two ships are 2r sin(gap / 2) apart.
        let gap = gap_rad(0.2, 2.6);
        let apart = (high.path(0.2).position_at(7.0) - high.path(2.6).position_at(7.0)).length();
        assert!((apart - separation_km(high.radius_km, gap)).abs() < 1e-6);
    }

    /// XFER-01: a trip between two bands of one body has the body as
    /// primary, no departure or capture cost, no approach days and no
    /// wait: at the Hohmann travel time it costs exactly the Hohmann burns.
    /// The ship lands where the arc ends.
    #[test]
    fn a_band_to_band_leg_goes_round_the_body_at_no_extra_cost() {
        let l = layout();
        let capital = l.tree.bodies[&l.capital];
        let low = l.node(l.capital, Band::Low).radius_km;
        let high = l.node(l.capital, Band::High);
        let expected = hohmann(capital.mu, low, high.radius_km);
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
            "no approach days and no wait"
        );
        assert_eq!(leg.departure_wait_days, 0.0);
        assert_eq!(leg.arrival_day, leg.crossing_end_day);
        let Landing::Orbit {
            orbit,
            body,
            phase0_rad,
        } = leg.landing
        else {
            panic!("a Rendezvous lands in an orbit")
        };
        assert_eq!((orbit, body), (l.band(l.capital, Band::High), l.capital));
        let arc_end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-9)
            .unwrap();
        let landed = high.path(phase0_rad).position_at(leg.crossing_end_day);
        assert!((landed - arc_end.position).length() < 10.0);
    }

    /// XFER-06: a low-thrust leg is a spiral between the two ends'
    /// circular orbits round the primary; its delta-v is the two bands'
    /// costs plus the difference of circular speeds, its length that over
    /// the acceleration, and it lands on the inner planet's angle on the
    /// crossing's last day. Its whole delta-v is charged at the start,
    /// and it doesn't wait for an exit point.
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
        assert_eq!(leg.departure_wait_days, 0.0);
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
        // cost, no wait and no origin place.
        let (_, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 60.0), &l.ntr);
        assert_eq!(leg.origin, None);
        assert_eq!(leg.departure_wait_days, 0.0);
        let capture_cost = l.charges(l.inner, Band::High).0;
        assert!(leg.burns.end_kms > capture_cost);
    }

    /// XFER-07: a crossing whose path dips inside `min_periapsis_radii`
    /// of the primary, while on the crossing, is infeasible. An outward
    /// crossing that passes its periapsis (at or inside the inner planet's
    /// orbit) after it starts is refused with the limit raised past that
    /// orbit; at the default limit it is fine, and so is an inward
    /// transfer that ends before periapsis.
    #[test]
    fn a_crossing_too_close_to_the_primary_is_infeasible() {
        let mut l = layout();
        let from = l.parked(l.inner, Band::High);
        let dips = |leg: &SolvedLeg| match leg.trajectory.segments()[1].kind {
            SegmentKind::Conic { conic, .. } => {
                conic.passes_periapsis_between(leg.crossing_start_day, leg.crossing_end_day)
            }
            _ => false,
        };
        let order = (0..600)
            .map(|day| Order {
                depart_day: f64::from(day),
                ..rendezvous_with(l.capital, Band::High, 111.0)
            })
            .find(|order| l.solve(&from, order, &l.ntr).is_ok_and(|leg| dips(&leg)))
            .expect("some outward crossing passes its periapsis");
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

    /// A phase in the capital's High band from which an arc of the
    /// Hohmann time to the Low band station sweeps about 90°, on the near
    /// side of the planet.
    const NEAR_SIDE: f64 = 4.7;

    /// The Hohmann travel time between the capital's two bands.
    fn capital_hohmann_days(l: &Layout) -> f64 {
        let low = l.node(l.capital, Band::Low).radius_km;
        let high = l.node(l.capital, Band::High).radius_km;
        hohmann(l.tree.bodies[&l.capital].mu, high, low).days
    }

    /// XFER-07: a crossing that ends in a Low band of its primary may not
    /// dip under that band. From the High band, an arc to the Low band
    /// station on the far side of the planet passes through it and is
    /// refused (PLNR-92's "Path passes too close"); the same order from
    /// the near side is flown.
    #[test]
    fn an_arc_to_a_low_band_station_on_the_far_side_of_the_planet_is_infeasible() {
        let l = layout();
        let low_km = l.node(l.capital, Band::Low).radius_km;
        let order = Order {
            destination: Place::Station { station: l.station },
            ..rendezvous_with(l.capital, Band::High, capital_hohmann_days(&l))
        };
        let far = l.solve(&l.parked_at(l.capital, Band::High, 0.0), &order, &l.ntr);
        assert!(
            matches!(far, Err(Infeasible::Periapsis { periapsis_km }) if periapsis_km < low_km),
            "{far:?}"
        );
        let near = l.solve(
            &l.parked_at(l.capital, Band::High, NEAR_SIDE),
            &order,
            &l.ntr,
        );
        assert!(near.is_ok(), "{near:?}");
    }

    /// XFER-07: a crossing may start or end exactly at its primary's Low
    /// band. The Hohmann arcs between the capital's bands touch the Low
    /// band at their periapsis, both ways, and an Intercept through the
    /// Low band at that time coasts round on it; all are flown, rounding
    /// and all. A slower arc leaving the Low band climbs straight away,
    /// and is flown too.
    #[test]
    fn arcs_that_touch_the_low_band_of_their_primary_are_flown() {
        let l = layout();
        let days = capital_hohmann_days(&l);
        let low = l.parked(l.capital, Band::Low);
        let high = l.parked(l.capital, Band::High);
        let down = rendezvous_with(l.capital, Band::Low, days);
        let through = Order {
            arrival: Arrival::Intercept { band: Band::Low },
            ..down
        };
        let up = rendezvous_with(l.capital, Band::High, days);
        let slow_up = rendezvous_with(l.capital, Band::High, 1.2 * days);
        for (from, order) in [
            (&high, &down),
            (&high, &through),
            (&low, &up),
            (&low, &slow_up),
        ] {
            let leg = l.solve(from, order, &l.ntr);
            assert!(leg.is_ok(), "{order:?}: {leg:?}");
        }
    }

    /// XFER-07: an Intercept in a planet's frame coasts on along its
    /// crossing. Arriving at the Low band from above, slower than the
    /// Hohmann time, the ship is still heading down, so its coast would
    /// dip under the band: the Intercept is refused for its coast, though
    /// a Rendezvous on the same arc is flown.
    #[test]
    fn an_intercept_whose_coast_dips_under_the_low_band_is_infeasible() {
        let l = layout();
        let low_km = l.node(l.capital, Band::Low).radius_km;
        let rendezvous = rendezvous_with(l.capital, Band::Low, 1.2 * capital_hohmann_days(&l));
        let intercept = Order {
            arrival: Arrival::Intercept { band: Band::Low },
            ..rendezvous
        };
        let from = l.parked(l.capital, Band::High);
        assert!(l.solve(&from, &rendezvous, &l.ntr).is_ok());
        let result = l.solve(&from, &intercept, &l.ntr);
        assert!(
            matches!(result, Err(Infeasible::CoastPeriapsis { periapsis_km }) if periapsis_km < low_km),
            "{result:?}"
        );
    }

    /// XFER-07 / XFER-13: the conic a ship is turned onto by a pass
    /// through another body's Low band is a coast too. With the limit
    /// raised just past that coast's periapsis round the star, the same
    /// Intercept is refused for its coast.
    #[test]
    fn the_coast_after_a_low_pass_is_kept_off_the_primary() {
        let mut l = layout();
        let from = l.parked(l.capital, Band::High);
        let intercept = Order {
            arrival: Arrival::Intercept { band: Band::Low },
            ..rendezvous_with(l.inner, Band::Low, 111.0)
        };
        let (order, leg) = l.cheapest(&from, intercept, &l.ntr);
        let Some(SegmentKind::Conic { conic, .. }) =
            leg.trajectory.segments().last().map(|s| s.kind)
        else {
            panic!("the ship coasts on after the pass");
        };
        let coast_km = conic.periapsis_radius_km();
        let star_km = l.tree.bodies[&l.star].radius_km;
        l.tunables.min_periapsis_radii = 1.001 * coast_km / star_km;
        let result = l.solve(&from, &order, &l.ntr);
        assert!(
            matches!(result, Err(Infeasible::CoastPeriapsis { periapsis_km }) if periapsis_km == coast_km),
            "{result:?}"
        );
    }

    /// XFER-08 / XFER-09 / PHAS-11: an Intercept through the High band
    /// pays departure and burn 1 only, has no arrival approach, leaves the
    /// crossing conic open-ended, reports the closing speed boosted by the
    /// body's gravity at the band's radius, and records its closest
    /// approach: the capture point's angle at the band's radius. It takes
    /// no turn and has no target to close on.
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
        assert_eq!(leg.trim_days, None, "a band is not trimmed");
        let segments = leg.trajectory.segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[1].end_day, None);
        assert!(matches!(segments[1].kind, SegmentKind::Conic { .. }));

        let inner = l.tree.bodies[&l.inner];
        let v_inf = arrival_v_inf(&l, &leg, l.inner);
        let high = l.node(l.inner, Band::High).radius_km;
        let expected =
            km_per_day_to_km_per_s((v_inf.length_squared() + 2.0 * inner.mu / high).sqrt());
        within(
            leg.intercept_speed_kms.unwrap(),
            expected,
            1e-5,
            "intercept speed",
        );
        assert!(leg.intercept_speed_kms.unwrap() > km_per_day_to_km_per_s(v_inf.length()));
        let closest = capture_angle(inner.mu, high, v_inf);
        assert!(angle_between(leg.closest_approach_rad.unwrap(), closest) < 1e-6);
        assert_eq!(rendezvous.closest_approach_rad, None);
        assert_eq!(leg.turn_rad, None, "a pass through High takes no turn");
        assert_eq!(
            leg.closing_speed_kms, None,
            "a body has no target to close on"
        );
    }

    /// XFER-08 / XFER-13 / XFER-09: an Intercept through another body's
    /// Low band costs what one through High does, but the crossing ends on
    /// the crossing end day: from then the ship coasts on a new conic
    /// round the star, from the body's position, with the body's velocity
    /// plus its excess velocity turned prograde by 2 arcsin(1 / e). The
    /// excess speed is unchanged.
    #[test]
    fn an_intercept_through_low_is_turned_by_the_body() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let intercept = Order {
            arrival: Arrival::Intercept { band: Band::Low },
            ..rendezvous_with(l.inner, Band::Low, 111.0)
        };
        let (order, leg) = l.cheapest(&from, intercept, &l.ntr);
        let high = Order {
            arrival: Arrival::Intercept { band: Band::High },
            destination: Place::Body {
                body: l.inner,
                band: Band::High,
            },
            ..order
        };
        let through_high = l.solve(&from, &high, &l.ntr).unwrap();
        assert!((leg.delta_v_kms - through_high.delta_v_kms).abs() < 1e-9);
        assert_eq!(leg.crossing_end_day, through_high.crossing_end_day);
        assert_eq!(leg.arrival_day, leg.crossing_end_day);
        assert_eq!(leg.landing, Landing::Intercept);

        let segments = leg.trajectory.segments();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[1].end_day, Some(leg.crossing_end_day));
        assert_eq!(segments[2].start_day, leg.crossing_end_day);
        assert_eq!(segments[2].end_day, None);
        let SegmentKind::Conic { primary, .. } = segments[2].kind else {
            panic!("the ship coasts on after the pass");
        };
        assert_eq!(primary, l.star);

        let inner = l.tree.bodies[&l.inner];
        let body = inner.orbit.unwrap().state_at(leg.crossing_end_day);
        let v_in = arrival_v_inf(&l, &leg, l.inner);
        let after = leg.trajectory.state_at(leg.crossing_end_day).unwrap();
        assert!((after.position - body.position).length() < 1.0);
        let v_out = after.velocity - body.velocity;
        within(v_out.length(), v_in.length(), 1e-6, "excess speed");
        let low = l.node(l.inner, Band::Low).radius_km;
        let e = 1.0 + low * v_in.length_squared() / inner.mu;
        let turn = 2.0 * (1.0 / e).asin();
        within(leg.turn_rad.unwrap(), turn, 1e-9, "turn");
        let turned = gap_rad(angle_of(v_in), angle_of(v_out));
        assert!((turned - turn).abs() < 1e-6, "turned {turned} prograde");
        assert!(turn > 10f64.to_radians(), "a Low pass is a real turn");

        // XFER-09: the speed is taken at the Low band's radius.
        let expected =
            km_per_day_to_km_per_s((v_in.length_squared() + 2.0 * inner.mu / low).sqrt());
        within(
            leg.intercept_speed_kms.unwrap(),
            expected,
            1e-6,
            "intercept speed",
        );
        let closest = capture_angle(inner.mu, low, v_in);
        assert!(angle_between(leg.closest_approach_rad.unwrap(), closest) < 1e-6);
    }

    /// XFER-13: an Intercept through a Low band of the crossing's own
    /// primary takes no turn: the crossing's conic already goes round that
    /// body, and runs on without end.
    #[test]
    fn an_intercept_through_the_primarys_own_low_band_is_not_turned() {
        let l = layout();
        let capital_mu = l.tree.bodies[&l.capital].mu;
        let low = l.node(l.capital, Band::Low).radius_km;
        let high = l.node(l.capital, Band::High).radius_km;
        let order = Order {
            arrival: Arrival::Intercept { band: Band::Low },
            ..rendezvous_with(l.capital, Band::Low, hohmann(capital_mu, high, low).days)
        };
        let leg = l
            .solve(&l.parked(l.capital, Band::High), &order, &l.ntr)
            .unwrap();
        assert_eq!(leg.primary, l.capital);
        assert_eq!(leg.turn_rad, None);
        let segments = leg.trajectory.segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[1].end_day, None);
    }

    /// XFER-14 / XFER-13 / PHAS-15: an Intercept at a station in another
    /// body's Low band goes through that band: it is trimmed to meet the
    /// station, turned by the body, and passes the station at the pass's
    /// speed less the band's circular speed. One at a station in a High
    /// band closes the same way and takes no turn.
    #[test]
    fn an_intercept_at_a_station_closes_at_the_pass_speed_less_the_bands() {
        let l = layout();
        let raid = |station| Order {
            destination: Place::Station { station },
            arrival: Arrival::Intercept { band: Band::High },
            ..rendezvous_with(l.capital, Band::Low, 111.0)
        };
        let (_, leg) = l.cheapest(&l.parked(l.inner, Band::High), raid(l.station), &l.ntr);
        let capital = l.tree.bodies[&l.capital];
        let low = l.node(l.capital, Band::Low);
        assert!(leg.trim_days.is_some_and(|trim| trim < low.period_days));
        assert!(leg.turn_rad.is_some());
        assert_eq!(leg.trajectory.segments().len(), 3);
        let v_inf = arrival_v_inf(&l, &leg, l.capital);
        let pass = (v_inf.length_squared() + 2.0 * capital.mu / low.radius_km).sqrt();
        let closing = km_per_day_to_km_per_s(pass - (capital.mu / low.radius_km).sqrt());
        within(
            leg.closing_speed_kms.unwrap(),
            closing,
            1e-6,
            "closing speed",
        );
        within(
            leg.intercept_speed_kms.unwrap(),
            km_per_day_to_km_per_s(pass),
            1e-6,
            "speed relative to the capital",
        );
        let station = low.path(STATION_PHASE).angle_at(leg.crossing_end_day);
        let closest = leg.closest_approach_rad.unwrap();
        assert!(low.radius_km * angle_between(station, closest) < l.tunables.together_km);

        let from = l.parked(l.capital, Band::High);
        let (_, leg) = l.cheapest(&from, raid(l.far_station), &l.ntr);
        let inner = l.tree.bodies[&l.inner];
        let high = l.node(l.inner, Band::High).radius_km;
        let v_inf = arrival_v_inf(&l, &leg, l.inner).length();
        let closing = (v_inf * v_inf + 2.0 * inner.mu / high).sqrt() - (inner.mu / high).sqrt();
        within(
            leg.closing_speed_kms.unwrap(),
            km_per_day_to_km_per_s(closing),
            1e-6,
            "closing speed in High",
        );
        assert_eq!(leg.turn_rad, None);
    }

    /// XFER-10 / PHAS-10 / PLAN-11: a parked ship in another body's band
    /// is met where it is: the leg waits on arrival until the ship reaches
    /// the capture point, lands at its phase, and remembers where the
    /// target was. A coasting ship round another primary is infeasible, as
    /// is a ship mid-crossing; one round the crossing's primary is met on
    /// its conic, which a Rendezvous then shares.
    #[test]
    fn a_ship_can_be_a_destination() {
        let mut l = layout();
        let inner_high = l.band(l.inner, Band::High);
        let target = l.add_ship(ShipState::Parked {
            orbit: inner_high,
            phase0_rad: 1.0,
        });
        let capital_mu = l.tree.bodies[&l.capital].mu;
        let path = CircularOrbit::new(capital_mu, 30_000.0, 0.0);
        let conic = Conic::from_state(capital_mu, path.state_at(0.0), 0.0, 1e-6).unwrap();
        let drifter = l.add_ship(ShipState::Coasting {
            primary: l.capital,
            conic,
        });
        let flier = l.add_ship(ShipState::InFlight);

        let from = l.parked(l.capital, Band::High);
        let to_ship = |ship| Order {
            destination: Place::Ship { ship },
            ..rendezvous_with(l.inner, Band::High, 111.0)
        };
        let (_, leg) = l.cheapest(&from, to_ship(target), &l.ntr);
        assert!(
            matches!(leg.landing, Landing::Orbit { orbit, phase0_rad, .. } if orbit == inner_high && phase0_rad == 1.0)
        );
        assert_eq!(
            leg.target_lock,
            Some(TargetLock {
                ship: target,
                orbit: inner_high,
                phase0_rad: 1.0
            })
        );

        let from_inner = l.parked(l.inner, Band::High);
        assert_eq!(
            l.solve(&from_inner, &to_ship(drifter), &l.ntr),
            Err(Infeasible::ShipPrimary)
        );
        assert_eq!(
            l.solve(&from, &to_ship(flier), &l.ntr),
            Err(Infeasible::TargetInFlight)
        );

        // From the capital's own band the drifter is reachable, and a
        // Rendezvous shares its conic.
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
        assert_eq!(leg.target_lock, None);
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

    /// BAND-06: a ship that captures into a band from outside the body's
    /// frame takes the phase of its capture point: the closest point of
    /// the prograde hyperbola it arrives on, arccos(1 / e) short of the
    /// direction of its excess velocity, on the arrival day.
    #[test]
    fn a_captured_ship_sits_at_its_capture_point() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let (_, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        let Landing::Orbit { phase0_rad, .. } = leg.landing else {
            panic!()
        };
        let v_inf = arrival_v_inf(&l, &leg, l.inner);
        let node = l.node(l.inner, Band::High);
        let mu = l.tree.bodies[&l.inner].mu;
        let e = 1.0 + node.radius_km * v_inf.length_squared() / mu;
        let expected = angle_of(v_inf) - (1.0 / e).acos();
        let parked = node.path(phase0_rad);
        let at_arrival = parked.angle_at(leg.arrival_day);
        assert!(angle_between(at_arrival, expected) < 1e-9);
        assert!(
            angle_between(at_arrival, angle_of(v_inf)) > 0.1,
            "not the arrival direction"
        );
        let SegmentKind::Parked { path, .. } = leg.trajectory.segments()[2].kind else {
            panic!()
        };
        assert_eq!(path, parked);
    }

    /// BAND-06 / PHAS-10 / XFER-02: a Rendezvous at a station in another
    /// body's band waits, after the approach days, until the station
    /// reaches the capture point: the wait is under one period, and on the
    /// arrival day the ship is together with the station there.
    #[test]
    fn a_rendezvous_at_a_station_waits_for_it_at_the_capture_point() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let order = Order {
            destination: Place::Station {
                station: l.far_station,
            },
            ..rendezvous_with(l.inner, Band::High, 111.0)
        };
        let (_, leg) = l.cheapest(&from, order, &l.ntr);
        let node = l.node(l.inner, Band::High);
        let (_, approach) = l.charges(l.inner, Band::High);
        assert!((0.0..node.period_days).contains(&leg.arrival_wait_days));
        assert!(
            (leg.arrival_day - (leg.crossing_end_day + approach + leg.arrival_wait_days)).abs()
                < 1e-9
        );
        let station = node.path(FAR_STATION_PHASE);
        let Landing::Orbit { phase0_rad, .. } = leg.landing else {
            panic!()
        };
        assert_eq!(phase0_rad, FAR_STATION_PHASE);
        let ship_at = leg.trajectory.state_at(leg.arrival_day).unwrap().position;
        assert!((ship_at - station.position_at(leg.arrival_day)).length() < 1e-6);
        let capture = capture_angle(
            l.tree.bodies[&l.inner].mu,
            node.radius_km,
            arrival_v_inf(&l, &leg, l.inner),
        );
        assert!(angle_between(station.angle_at(leg.arrival_day), capture) < 1e-6);
    }

    /// XFER-10 / BAND-02: a station in another orbit of the crossing's
    /// primary is met where it is: the crossing ends at the station's
    /// position on the crossing end day, and the ship lands at its phase,
    /// docked. A ship docked at it sets out from the station, and one
    /// elsewhere in the band from the band.
    #[test]
    fn a_station_round_the_primary_is_met_where_it_is() {
        let l = layout();
        let capital_mu = l.tree.bodies[&l.capital].mu;
        let low = l.node(l.capital, Band::Low);
        let high_km = l.node(l.capital, Band::High).radius_km;
        let order = Order {
            destination: Place::Station { station: l.station },
            ..rendezvous_with(
                l.capital,
                Band::High,
                hohmann(capital_mu, high_km, low.radius_km).days,
            )
        };
        // Set out from a quarter turn behind where the station will be, so
        // the arc stays on the near side of the planet (XFER-07).
        let leg = l
            .solve(
                &l.parked_at(l.capital, Band::High, NEAR_SIDE),
                &order,
                &l.ntr,
            )
            .unwrap();
        assert_eq!(leg.primary, l.capital);
        assert_eq!(leg.arrival_wait_days, 0.0);
        let Landing::Orbit {
            orbit, phase0_rad, ..
        } = leg.landing
        else {
            panic!()
        };
        assert_eq!(
            (orbit, phase0_rad),
            (l.band(l.capital, Band::Low), STATION_PHASE)
        );
        let station = low.path(STATION_PHASE);
        let arc_end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-9)
            .unwrap();
        assert!((arc_end.position - station.position_at(leg.crossing_end_day)).length() < 10.0);

        let gate = Order {
            destination: Place::Gate { gate: l.gate },
            ..rendezvous_with(l.capital, Band::High, 200.0)
        };
        let elsewhere = l.parked_at(l.capital, Band::Low, STATION_PHASE + 1.0);
        let (_, leg) = l.cheapest(&elsewhere, gate, &l.ntr);
        assert_eq!(
            leg.origin,
            Some(Place::Body {
                body: l.capital,
                band: Band::Low
            })
        );
        // 5 km along the band from the station is still docked (PHAS-02).
        let docked = l.parked_at(l.capital, Band::Low, STATION_PHASE + 5.0 / low.radius_km);
        let (_, leg) = l.cheapest(&docked, gate, &l.ntr);
        assert_eq!(leg.origin, Some(Place::Station { station: l.station }));
        assert_eq!(leg.primary, l.star);
    }

    /// BAND-02 / BAND-03: a gate has no massive body, so arriving there
    /// costs only burn 2, and the ship lands at the gate.
    #[test]
    fn a_gate_is_its_docking_orbit() {
        let l = layout();
        let gate = Order {
            destination: Place::Gate { gate: l.gate },
            ..rendezvous_with(l.capital, Band::High, 200.0)
        };
        let (_, leg) = l.cheapest(&l.parked(l.capital, Band::Low), gate, &l.ntr);
        let arc_end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-9)
            .unwrap();
        let gate_state = l.tree.bodies[&l.gate]
            .orbit
            .unwrap()
            .state_at(leg.crossing_end_day);
        let burn_2 = km_per_day_to_km_per_s((gate_state.velocity - arc_end.velocity).length());
        assert!(
            (leg.burns.end_kms - burn_2).abs() < 1e-3,
            "{} vs {burn_2}",
            leg.burns.end_kms
        );
        assert_eq!(leg.arrival_day, leg.crossing_end_day);
        let parked = leg.trajectory.state_at(leg.arrival_day).unwrap().position;
        assert!((parked - gate_state.position).length() < 10.0);

        let hold = Order {
            destination: Place::Hold,
            ..gate
        };
        assert_eq!(
            l.solve(&l.parked(l.capital, Band::Low), &hold, &l.ntr),
            Err(Infeasible::HoldIsNotACrossing)
        );
    }

    /// PHAS-15 / XFER-10: an Intercept at a station in another body's band
    /// goes through that band and is trimmed to meet it: the travel time
    /// grows by less than one period of the band, and on the crossing end
    /// day the station is within `together_km` of the closest-approach
    /// point. Allowed only one solve, it can't line up.
    #[test]
    fn an_intercept_at_a_station_is_trimmed_to_meet_it() {
        let mut l = layout();
        let from = l.parked(l.capital, Band::High);
        let order = Order {
            destination: Place::Station {
                station: l.far_station,
            },
            arrival: Arrival::Intercept { band: Band::High },
            ..rendezvous_with(l.inner, Band::High, 111.0)
        };
        let (order, leg) = l.cheapest(&from, order, &l.ntr);
        let node = l.node(l.inner, Band::High);
        let trim = leg.trim_days.expect("trimmed");
        assert!((0.0..node.period_days).contains(&trim));
        assert!((leg.travel_days - (order.travel_days + trim)).abs() < 1e-9);
        let closest = leg.closest_approach_rad.unwrap();
        let station = node.path(FAR_STATION_PHASE).angle_at(leg.crossing_end_day);
        assert!(node.radius_km * angle_between(station, closest) < l.tunables.together_km);
        let expected = capture_angle(
            l.tree.bodies[&l.inner].mu,
            node.radius_km,
            arrival_v_inf(&l, &leg, l.inner),
        );
        assert!(angle_between(closest, expected) < 1e-6);

        l.tunables.trim_max_iterations = 1;
        assert_eq!(l.solve(&from, &order, &l.ntr), Err(Infeasible::CantLineUp));
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

    /// PLNR-26: a Rendezvous arrives at its speed relative to the body
    /// before capture (burn 2 matches it); an Intercept's arrival speed is
    /// its speed past the body (XFER-09).
    #[test]
    fn the_arrival_speed_is_the_speed_relative_to_the_destination() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let (order, leg) = l.cheapest(&from, rendezvous_with(l.inner, Band::High, 111.0), &l.ntr);
        let v_inf = km_per_day_to_km_per_s(arrival_v_inf(&l, &leg, l.inner).length());
        within(leg.arrival_speed_kms.unwrap(), v_inf, 1e-6, "arrival speed");
        let capture_cost = l.charges(l.inner, Band::High).0;
        within(
            leg.burns.end_kms - capture_cost,
            v_inf,
            1e-6,
            "burn 2 matches it",
        );
        let intercept = l
            .solve(
                &from,
                &Order {
                    arrival: Arrival::Intercept { band: Band::High },
                    ..order
                },
                &l.ntr,
            )
            .unwrap();
        assert_eq!(intercept.arrival_speed_kms, intercept.intercept_speed_kms);
    }
}

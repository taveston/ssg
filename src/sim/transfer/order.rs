//! What goes into the leg solver and what comes out (PLAN-01, PLAN-03,
//! XFER-08): the order a ship is given, where it starts from, the leg
//! solved for it, and every reason a leg can't be flown. Plain data; the
//! rules that fill it in are in `legs.rs` and `phasing.rs`.

use bevy::prelude::*;
use orbits::Conic;

use super::places::{Band, Place};
use crate::sim::rail::ShipTrajectory;

/// XFER-08: what a leg does when it reaches its destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arrival {
    /// Capture into the destination's orbit.
    Rendezvous,
    /// Fly past through `band` and coast on.
    Intercept { band: Band },
}

impl Arrival {
    /// Whether the leg flies past rather than capturing.
    pub fn is_intercept(self) -> bool {
        matches!(self, Arrival::Intercept { .. })
    }
}

/// One leg as ordered (PLAN-01, PLAN-03): everything a trajectory is
/// rebuilt from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    pub destination: Place,
    pub arrival: Arrival,
    pub depart_day: f64,
    /// For an impulsive drive, the crossing's length, or the least it
    /// takes when the solver lengthens it (PHAS-08, PHAS-15); a low-thrust
    /// drive sets its own and ignores this.
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
    /// The coast after an Intercept would dip too close to its primary
    /// (XFER-07).
    CoastPeriapsis { periapsis_km: f64 },
    /// The leg needs more delta-v than the ship has available (XFER-04).
    DeltaV { needed_kms: f64, available_kms: f64 },
    /// The ship has a plan Scheduled or InFlight already (PLAN-02).
    Busy,
    /// The ship has no drive or no delta-v account (PLAN-20).
    NoDrive,
    /// A plan has more than one leg, which is not specced (PLAN-01).
    MultiLeg,
    /// The destination is where the ship already is: its own orbit, or a
    /// station or ship it is together with (PHAS-04).
    AlreadyThere,
    /// No phasing orbit within the lap limit stays between the body's Low
    /// band and its Hill sphere (PHAS-08).
    NoPhasingOrbit,
    /// A phasing leg needs an impulsive drive (PHAS-05).
    PhasingNeedsImpulsive,
    /// A target in the ship's own orbit can only be met by a Rendezvous
    /// (PHAS-05).
    PhasingIsRendezvous,
    /// An Intercept trimmed to meet its target didn't settle within
    /// `trim_max_iterations` solves (PHAS-15).
    CantLineUp,
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

/// PLAN-11: the parked target ship a leg was solved against: the orbit it
/// was in and its phase there. If it has moved by the depart day, the
/// plan fails.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetLock {
    pub ship: Entity,
    pub orbit: Entity,
    pub phase0_rad: f64,
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
    /// XFER-09 / XFER-10: an Intercept's speed relative to the body it
    /// passes, at the band; or relative to the target, when the crossing
    /// ends at the target itself. In km/s.
    pub intercept_speed_kms: Option<f64>,
    /// XFER-14 / XFER-10: the speed at which an Intercept at a station or
    /// a ship goes by it, in km/s; `None` for an Intercept at a body or a
    /// Rendezvous.
    pub closing_speed_kms: Option<f64>,
    /// PLNR-26: the speed the planner's arrival row shows, in km/s. For a
    /// Rendezvous, the path's speed relative to its end state (the
    /// destination body, or the target where the crossing ends at it)
    /// before capture; for an Intercept, its closing speed at a target,
    /// else its speed past the body. `None` for a phasing leg.
    pub arrival_speed_kms: Option<f64>,
    /// XFER-13: how far an Intercept through another body's Low band
    /// turned the ship, in radians; `None` for any other leg.
    pub turn_rad: Option<f64>,
    /// PHAS-03: days spent waiting in the departure orbit for the exit
    /// point; the ship leaves its orbit on the depart day plus this.
    pub departure_wait_days: f64,
    /// PHAS-10: days spent waiting in the destination orbit for the target
    /// to reach the capture point.
    pub arrival_wait_days: f64,
    /// PHAS-11: an Intercept's closest-approach phase: the angle round the
    /// body, on the crossing end day, at which it passes closest.
    pub closest_approach_rad: Option<f64>,
    /// PHAS-15: the days the solver added to the requested travel time so
    /// an Intercept meets its target; `None` for a leg that isn't trimmed.
    pub trim_days: Option<f64>,
    /// PLAN-11: the parked target ship the leg was solved against.
    pub target_lock: Option<TargetLock>,
    /// PHAS-06: the whole laps of its phasing orbit a phasing leg flies,
    /// which the planner's Travel time readout shows (PLNR-40); `None`
    /// for a crossing.
    pub phasing_laps: Option<u32>,
}

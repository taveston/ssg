//! How a crossing ends (XFER-07 to XFER-10, XFER-13, XFER-14, BAND-06,
//! PHAS-10, PHAS-11): the checks on its periapsis and on the coast after
//! an Intercept, an Intercept's speeds and closest approach, the turn of a
//! pass through another body's Low band, and the orbit or conic a
//! Rendezvous ends on. The crossing itself is solved in `legs.rs`.

use bevy::prelude::*;
use orbits::{Conic, State, capture_angle, closing_speed, pass_speed, turn_angle, turned_velocity};

use super::ends::{Target, phase_for_angle};
use super::legs::{Crossing, Setup};
use super::order::{Infeasible, Landing};
use super::phase::wait_days;
use super::places::{Band, band_radii};
use super::tree::OrbitNode;
use crate::sim::rail::{SegmentKind, ShipTrajectory, km_per_day_to_km_per_s};

/// How far under its limit a periapsis may come, as a fraction of the
/// limit, and still count as at it: room for the rounding of a path
/// solved to arrive at a Low band exactly (XFER-07).
const PERIAPSIS_ROUNDING: f64 = 1e-6;

/// Whether a periapsis is under `limit_km`, beyond rounding (XFER-07).
fn under_limit(periapsis_km: f64, limit_km: f64) -> bool {
    periapsis_km < limit_km * (1.0 - PERIAPSIS_ROUNDING)
}

/// XFER-13: an Intercept through another body's Low band. The body
/// swings the ship round, so from the crossing end day it coasts on a new
/// conic round the primary, not on the crossing's own.
pub(super) struct LowPass {
    /// How far the pass turned the ship's velocity relative to the body,
    /// in radians, prograde.
    pub(super) turn_rad: f64,
    /// The conic the ship coasts on after the pass, round the primary.
    pub(super) coast: Conic,
}

/// How a Rendezvous ends: in an orbit or on a conic, and when.
pub(super) struct Arrived {
    pub(super) landing: Landing,
    pub(super) last: Option<SegmentKind>,
    pub(super) wait_days: f64,
    pub(super) arrival_day: f64,
}

/// What a leg's arrival reads, in km/s: an Intercept's speed at the
/// target, its closing speed and its closest approach (rad); and the
/// speed the arrival row shows (PLNR-26).
pub(super) struct ArrivalSpeeds {
    pub(super) intercept_kms: Option<f64>,
    pub(super) closing_kms: Option<f64>,
    pub(super) arrival_kms: Option<f64>,
    pub(super) closest_approach_rad: Option<f64>,
}

impl Setup<'_> {
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
    pub(super) fn check_periapsis(&self, crossing: &Crossing) -> Result<(), Infeasible> {
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
    pub(super) fn check_coast(
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
    pub(super) fn closest_approach(&self, crossing: &Crossing) -> Option<f64> {
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
    pub(super) fn low_pass(&self, crossing: &Crossing) -> Result<Option<LowPass>, Infeasible> {
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
    pub(super) fn arrive(&self, crossing: &Crossing) -> Arrived {
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

    /// The crossing's arrival speeds. A Rendezvous has no intercept,
    /// closing speed or closest approach, and its arrival row reads the
    /// speed relative to the target; an Intercept's reads the closing
    /// speed, else the intercept speed (PLNR-26).
    pub(super) fn arrival_speeds(&self, crossing: &Crossing) -> Result<ArrivalSpeeds, Infeasible> {
        if self.rendezvous {
            return Ok(ArrivalSpeeds {
                intercept_kms: None,
                closing_kms: None,
                arrival_kms: Some(km_per_day_to_km_per_s(
                    crossing.arrival_relative_velocity().length(),
                )),
                closest_approach_rad: None,
            });
        }
        let intercept_kms = Some(self.intercept_speed_kms(crossing)?);
        let closing_kms = self.closing_speed_kms(crossing);
        Ok(ArrivalSpeeds {
            intercept_kms,
            closing_kms,
            arrival_kms: closing_kms.or(intercept_kms),
            closest_approach_rad: self.closest_approach(crossing),
        })
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

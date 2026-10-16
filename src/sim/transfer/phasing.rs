//! A destination in the ship's own named orbit (PHAS-04 to PHAS-09):
//! already reached, or met by a phasing leg. Pure functions over the
//! system snapshot (`tree.rs`, ARCH-20); the phasing orbit itself is the
//! `orbits` crate's (PHAS-12).
//!
//! Everything in one orbit turns together, so a gap never closes on its
//! own and there are no windows: the ship can leave any day. It burns
//! onto a slightly smaller or larger ellipse that touches the orbit where
//! it is, flies whole laps of it, and burns back at the same point just
//! as the target comes round. More laps cost less and take longer; the
//! order's travel time says how long the player is willing to wait.

use bevy::prelude::*;
use orbits::{Conic, PhasingOrbit, PhasingSide, Rail, Segment, State, meeting_days, phasing_orbit};

use super::ends::{Target, place_of_parked};
use super::order::{Arrival, Infeasible, Landing, LegBurns, Order, SolvedLeg, TargetLock};
use super::phase::{gap_rad, together};
use super::places::Band;
use super::tree::{OrbitNode, SystemTree};
use crate::sim::rail::{SegmentKind, ShipTrajectory, km_per_day_to_km_per_s};
use crate::sim::ship::Drive;
use crate::sim::tunables::SimTunables;
use crate::sim::units::KM_PER_AU;

/// How close a phasing orbit may come to its body, and how far out it
/// may reach, in km (PHAS-08).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhasingLimits {
    /// The body's Low band radius: no lower.
    pub lowest_km: f64,
    /// The body's Hill radius, or `lost_beyond_au` round the star: no
    /// higher.
    pub highest_km: f64,
}

impl PhasingLimits {
    fn allow(&self, orbit: &PhasingOrbit) -> bool {
        orbit.closest_km >= self.lowest_km && orbit.farthest_km <= self.highest_km
    }
}

/// PHAS-08: the phasing orbit for a target `gap_turns` of a turn ahead on
/// a circular orbit of `radius_km` (period `period_days`) round a body of
/// gravitational parameter `mu`, for a request of `travel_days`. The first
/// meeting k at or after the request is tried first, then each later one
/// up to `max_laps`; of each k's inner and outer orbits the one inside
/// `limits` with the smaller delta-v is taken, the inner on a tie.
/// Returns the meeting k and its orbit, or `None` if nothing fits.
pub fn choose_phasing_orbit(
    mu: f64,
    radius_km: f64,
    period_days: f64,
    gap_turns: f64,
    travel_days: f64,
    limits: PhasingLimits,
    max_laps: u32,
) -> Option<(u32, PhasingOrbit)> {
    // The smallest k with T(k − g) ≥ travel_days. The small allowance
    // keeps a request equal to a meeting time on that meeting, so an
    // order rebuilt from its solved time gives the same orbit (PLAN-03).
    let first_k = (travel_days / period_days + gap_turns - 1e-9)
        .ceil()
        .max(1.0);
    if !(first_k.is_finite() && first_k <= f64::from(max_laps)) {
        return None;
    }
    (first_k as u32..=max_laps).find_map(|k| {
        let mut best: Option<PhasingOrbit> = None;
        for side in [PhasingSide::Inner, PhasingSide::Outer] {
            let Some(orbit) = phasing_orbit(mu, radius_km, gap_turns, k, side) else {
                continue;
            };
            if limits.allow(&orbit) && best.is_none_or(|b| orbit.delta_v() < b.delta_v()) {
                best = Some(orbit);
            }
        }
        best.map(|orbit| (k, orbit))
    })
}

/// PHAS-08: how low and how high a phasing orbit round `body` may go.
fn limits(tree: &SystemTree, tunables: &SimTunables, body: Entity) -> PhasingLimits {
    let node = tree.bodies.get(&body);
    let lowest_km = tree
        .band_orbit(body, Band::Low)
        .and_then(|orbit| tree.orbits.get(&orbit))
        .map_or_else(|| node.map_or(0.0, |n| n.radius_km), |low| low.radius_km);
    let highest_km = node
        .and_then(|n| n.hill_km)
        .unwrap_or(tunables.lost_beyond_au * KM_PER_AU);
    PhasingLimits {
        lowest_km,
        highest_km,
    }
}

/// PHAS-04 / PHAS-05: a destination in the orbit the ship is parked in,
/// at `from` (the orbit and the ship's phase). The orbit itself, or a
/// station or ship together with the ship, is already reached; a station
/// or ship elsewhere in the orbit is met by a phasing leg.
pub(super) fn solve_in_own_orbit(
    tree: &SystemTree,
    tunables: &SimTunables,
    from: (Entity, f64),
    target: &Target,
    order: &Order,
    drive: &Drive,
    budget_kms: f64,
) -> Result<SolvedLeg, Infeasible> {
    let Target::Point {
        node,
        phase0_rad,
        lock,
        ..
    } = *target
    else {
        return Err(Infeasible::AlreadyThere);
    };
    let gap = gap_rad(from.1, phase0_rad);
    if together(node.radius_km, gap, tunables.together_km) {
        return Err(Infeasible::AlreadyThere);
    }
    if order.arrival != Arrival::Rendezvous {
        return Err(Infeasible::PhasingIsRendezvous);
    }
    if !drive.impulsive {
        return Err(Infeasible::PhasingNeedsImpulsive);
    }
    let point = (node, phase0_rad, lock);
    phasing_leg(tree, tunables, from, point, order, drive, budget_kms)
}

/// PHAS-05 to PHAS-09: the phasing leg. Burn 1 is made at the ship's
/// point of the orbit once the burn days are past; the ship flies its
/// laps on a conic round the body and is back at that point, with the
/// target, when the meeting time is up; burn 2 puts it back in the orbit
/// at the target's phase. No departure or capture cost, no approach days.
fn phasing_leg(
    tree: &SystemTree,
    tunables: &SimTunables,
    (orbit, ship_phase): (Entity, f64),
    (node, target_phase, lock): (OrbitNode, f64, Option<TargetLock>),
    order: &Order,
    drive: &Drive,
    budget_kms: f64,
) -> Result<SolvedLeg, Infeasible> {
    let mu = tree.mu(node.body);
    let gap_turns = gap_rad(ship_phase, target_phase) / std::f64::consts::TAU;
    let (k, chosen) = choose_phasing_orbit(
        mu,
        node.radius_km,
        node.period_days,
        gap_turns,
        order.travel_days,
        limits(tree, tunables, node.body),
        tunables.max_phasing_laps,
    )
    .ok_or(Infeasible::NoPhasingOrbit)?;
    let burn_kms = km_per_day_to_km_per_s(chosen.burn);
    let delta_v_kms = 2.0 * burn_kms;
    if delta_v_kms > budget_kms {
        return Err(Infeasible::DeltaV {
            needed_kms: delta_v_kms,
            available_kms: budget_kms,
        });
    }
    let start_day = order.depart_day + chosen.burn / drive.accel;
    let travel_days = meeting_days(node.period_days, gap_turns, k);
    let end_day = start_day + travel_days;
    let ship = node.path(ship_phase);
    let at_burn = ship.state_at(start_day);
    let after_burn = State::new(
        at_burn.position,
        at_burn.velocity.normalize() * chosen.burn_point_speed,
    );
    let conic = Conic::from_state(mu, after_burn, start_day, tunables.parabola_epsilon)
        .ok_or(Infeasible::NoPhasingOrbit)?;
    let parked = |phase0_rad| SegmentKind::Parked {
        orbit,
        primary: node.body,
        path: node.path(phase0_rad),
    };
    let trajectory = ShipTrajectory::new(vec![
        Segment {
            start_day: order.depart_day.min(start_day),
            end_day: Some(start_day),
            kind: parked(ship_phase),
        },
        Segment {
            start_day,
            end_day: Some(end_day),
            kind: SegmentKind::Conic {
                primary: node.body,
                conic,
            },
        },
        Segment {
            start_day: end_day,
            end_day: None,
            kind: parked(target_phase),
        },
    ])
    .map_err(|_| Infeasible::NoPath)?;
    Ok(SolvedLeg {
        origin: place_of_parked(orbit, ship_phase, tree, tunables.together_km),
        primary: node.body,
        crossing_start_day: start_day,
        crossing_end_day: end_day,
        arrival_day: end_day,
        travel_days,
        trajectory,
        delta_v_kms,
        burns: LegBurns {
            start_kms: burn_kms,
            end_kms: burn_kms,
        },
        landing: Landing::Orbit {
            orbit,
            body: node.body,
            phase0_rad: target_phase,
        },
        intercept_speed_kms: None,
        departure_wait_days: 0.0,
        arrival_wait_days: 0.0,
        closest_approach_rad: None,
        trim_days: None,
        target_lock: lock,
    })
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;
    use crate::sim::rail::km_per_s_to_km_per_day;
    use crate::sim::transfer::legs::solve_leg;
    use crate::sim::transfer::places::Place;
    use crate::sim::transfer::test_layout::{Layout, STATION_PHASE, layout, within};
    use crate::sim::transfer::tree::ShipState;

    /// A ship parked in the capital's `band` at `phase0_rad`, as a target.
    fn target_in(l: &mut Layout, band: Band, phase0_rad: f64) -> Entity {
        let orbit = l.band(l.capital, band);
        l.add_ship(ShipState::Parked { orbit, phase0_rad })
    }

    fn meet(ship: Entity, travel_days: f64) -> Order {
        Order {
            destination: Place::Ship { ship },
            arrival: Arrival::Rendezvous,
            depart_day: 3.0,
            travel_days,
        }
    }

    /// PHAS-05 / PHAS-06 / PHAS-07 / PHAS-08 / PHAS-09: in the capital's
    /// High band, with the target half a turn ahead and three days asked
    /// for, the leg flies to the third meeting (4.86 days) for 0.29 km/s,
    /// one burn at each end, with no costs, approach days or waits. The
    /// rail is parked, then a conic round the planet, then parked at the
    /// target's phase, where the target is.
    #[test]
    fn a_phasing_leg_meets_a_ship_in_the_same_band() {
        let mut l = layout();
        let target = target_in(&mut l, Band::High, 0.2 + PI);
        let from = l.parked(l.capital, Band::High);
        let leg = l.solve(&from, &meet(target, 3.0), &l.ntr).unwrap();
        let high = l.node(l.capital, Band::High);
        assert_eq!(leg.primary, l.capital);
        within(leg.travel_days, 4.86, 0.03, "third meeting");
        within(leg.travel_days / high.period_days, 2.5, 1e-9, "k − g");
        within(leg.delta_v_kms, 0.29, 0.03, "delta-v");
        assert_eq!(leg.burns.start_kms, leg.burns.end_kms);
        assert!((leg.burns.start_kms * 2.0 - leg.delta_v_kms).abs() < 1e-12);
        let burn_days = km_per_s_to_km_per_day(leg.burns.start_kms) / l.ntr.accel;
        assert!((leg.crossing_start_day - (3.0 + burn_days)).abs() < 1e-9);
        assert_eq!(leg.arrival_day, leg.crossing_end_day);
        assert_eq!(leg.departure_wait_days, 0.0);
        assert_eq!(leg.intercept_speed_kms, None);
        assert_eq!(
            leg.landing,
            Landing::Orbit {
                orbit: l.band(l.capital, Band::High),
                body: l.capital,
                phase0_rad: 0.2 + PI
            }
        );
        let segments = leg.trajectory.segments();
        assert_eq!(segments.len(), 3);
        assert!(
            matches!(segments[1].kind, SegmentKind::Conic { primary, .. } if primary == l.capital)
        );
        let target_path = high.path(0.2 + PI);
        let end = leg
            .trajectory
            .state_at(leg.crossing_end_day - 1e-9)
            .unwrap();
        assert!((end.position - target_path.position_at(leg.crossing_end_day)).length() < 1.0);
        let mid = leg
            .trajectory
            .state_at((leg.crossing_start_day + leg.crossing_end_day) / 2.0)
            .unwrap();
        assert!(
            (mid.radius() - high.radius_km).abs() > 100.0,
            "off the ring between the burns"
        );
        assert_eq!(leg.target_lock.unwrap().ship, target);
    }

    /// PHAS-08: in the Low band an inner orbit dips below the band, so with
    /// the target half a turn ahead the first meeting is skipped and the
    /// leg takes the second on the outer orbit: 2.5 hours for 1.46 km/s.
    #[test]
    fn a_phasing_leg_in_low_stays_above_the_band() {
        let mut l = layout();
        let target = target_in(&mut l, Band::Low, 0.2 + PI);
        let from = l.parked(l.capital, Band::Low);
        let leg = l.solve(&from, &meet(target, 0.0), &l.ntr).unwrap();
        within(leg.travel_days * 24.0, 2.5, 0.03, "second meeting, hours");
        within(leg.delta_v_kms, 1.46, 0.03, "delta-v");
        let low = l.node(l.capital, Band::Low).radius_km;
        let SegmentKind::Conic { conic, .. } = leg.trajectory.segments()[1].kind else {
            panic!()
        };
        assert!(conic.periapsis_radius_km() >= low - 1.0);
    }

    /// PHAS-08: of a meeting's two orbits the cheaper one inside the
    /// limits is taken; an orbit that would leave the Hill sphere is not,
    /// and with no orbit inside the limits up to `max_laps` there is none.
    #[test]
    fn the_cheaper_orbit_inside_the_limits_is_chosen() {
        let mu = 1e11;
        let r = 50_000.0;
        let period = orbits::circular_period(mu, r);
        let roomy = PhasingLimits {
            lowest_km: 0.0,
            highest_km: f64::INFINITY,
        };
        let (k, orbit) = choose_phasing_orbit(mu, r, period, 0.9, period, roomy, 50).unwrap();
        assert_eq!(k, 2);
        assert_eq!(
            orbit.side,
            PhasingSide::Outer,
            "a near gap behind: drop back"
        );
        let tight = PhasingLimits {
            lowest_km: 0.0,
            highest_km: r * 1.001,
        };
        let (_, orbit) = choose_phasing_orbit(mu, r, period, 0.9, period, tight, 50).unwrap();
        assert_eq!(orbit.side, PhasingSide::Inner);
        let none = PhasingLimits {
            lowest_km: r * 0.999,
            highest_km: r * 1.001,
        };
        assert!(choose_phasing_orbit(mu, r, period, 0.5, 0.0, none, 50).is_none());
        assert!(choose_phasing_orbit(mu, r, period, 0.5, 60.0 * period, roomy, 50).is_none());
    }

    /// PHAS-08: past `max_phasing_laps` meetings the leg is infeasible.
    #[test]
    fn a_phasing_leg_too_long_is_infeasible() {
        let mut l = layout();
        let target = target_in(&mut l, Band::High, 2.0);
        let from = l.parked(l.capital, Band::High);
        let period = l.node(l.capital, Band::High).period_days;
        let order = meet(target, 60.0 * period);
        assert_eq!(
            l.solve(&from, &order, &l.ntr),
            Err(Infeasible::NoPhasingOrbit)
        );
    }

    /// PHAS-05 / PHAS-07: a phasing leg is a Rendezvous by an impulsive
    /// drive within the ship's budget; an Intercept, a low-thrust drive or
    /// too little delta-v makes it infeasible.
    #[test]
    fn a_phasing_leg_needs_a_rendezvous_an_impulsive_drive_and_the_delta_v() {
        let mut l = layout();
        let target = target_in(&mut l, Band::High, 2.0);
        let from = l.parked(l.capital, Band::High);
        let intercept = Order {
            arrival: Arrival::Intercept { band: Band::High },
            ..meet(target, 1.0)
        };
        assert_eq!(
            l.solve(&from, &intercept, &l.ntr),
            Err(Infeasible::PhasingIsRendezvous)
        );
        assert_eq!(
            l.solve(&from, &meet(target, 1.0), &l.nep),
            Err(Infeasible::PhasingNeedsImpulsive)
        );
        let leg = l.solve(&from, &meet(target, 1.0), &l.ntr).unwrap();
        let short = solve_leg(
            &l.tree,
            &l.tunables,
            &from,
            &meet(target, 1.0),
            &l.ntr,
            0.01,
        );
        assert_eq!(
            short,
            Err(Infeasible::DeltaV {
                needed_kms: leg.delta_v_kms,
                available_kms: 0.01
            })
        );
    }

    /// PHAS-04: the ship's own band, or a station or ship in it that the
    /// ship is together with, is already reached.
    #[test]
    fn the_ships_own_orbit_is_already_there() {
        let mut l = layout();
        let low = l.node(l.capital, Band::Low);
        let alongside = target_in(&mut l, Band::Low, STATION_PHASE + 3.0 / low.radius_km);
        let docked = l.parked_at(l.capital, Band::Low, STATION_PHASE);
        let own_band = Order {
            destination: Place::Body {
                body: l.capital,
                band: Band::Low,
            },
            ..meet(alongside, 1.0)
        };
        let station = Order {
            destination: Place::Station { station: l.station },
            ..own_band
        };
        for order in [own_band, station, meet(alongside, 1.0)] {
            assert_eq!(
                l.solve(&docked, &order, &l.ntr),
                Err(Infeasible::AlreadyThere)
            );
        }
    }

    /// PHAS-12 / PLAN-03: rebuilding a phasing leg from its order with the
    /// solved travel time gives the same leg.
    #[test]
    fn a_phasing_leg_rebuilds_the_same() {
        let mut l = layout();
        let target = target_in(&mut l, Band::High, 4.0);
        let from = l.parked(l.capital, Band::High);
        let leg = l.solve(&from, &meet(target, 2.0), &l.ntr).unwrap();
        let again = l
            .solve(&from, &meet(target, leg.travel_days), &l.ntr)
            .unwrap();
        assert_eq!(again, leg);
    }
}

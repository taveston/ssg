//! What becomes of a ship after an Intercept (PLNR-88): it coasts on, and
//! the planner warns of the first thing that happens on the way: it
//! crosses a planet's orbit, or it leaves the system. A pure function of
//! the leg and the system snapshot (ARCH-20); the map draws the coast up
//! to that event (PLNR-67).

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "drawn by the planner's map marks, built next (PLNR-67)"
    )
)]

use bevy::prelude::*;
use orbits::{Conic, Rail};

use super::ends::resolve;
use super::order::{Landing, SolvedLeg};
use super::places::Place;
use super::tree::SystemTree;
use crate::sim::body::BodyClass;
use crate::sim::rail::SegmentKind;

/// PLNR-88: the first event of an Intercept's coast, with the days to it
/// counted from today.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoastEvent {
    /// Its distance from the star reaches the orbit radius of `body`.
    Crosses { body: Entity, days: f64 },
    /// Its distance from the star first passes the loss distance
    /// (BAND-08).
    Leaves { days: f64 },
}

/// A planet's orbit round the star: the planet and its radius in km.
pub type PlanetOrbit = (Entity, f64);

/// PLNR-88: the first event on a coast round the star that starts on
/// `start_day`: the first day its distance from the star equals one of
/// `planets`' orbit radii, or first exceeds `lost_km`, whichever comes
/// first. The coast starts at the intercepted planet's orbit, so that
/// planet's own orbit counts only when the ship comes back to it: the
/// crossing on the other side of the loop, not the one it is leaving.
/// Returns the event and its day, or `None` if the coast never reaches
/// any of them.
pub fn first_coast_event(
    coast: &Conic,
    start_day: f64,
    planets: &[PlanetOrbit],
    intercepted: Option<Entity>,
    lost_km: f64,
) -> Option<(CoastEvent, f64)> {
    // Past periapsis (a true anomaly from 0 to π) the ship is heading out.
    let heading_out = coast.true_anomaly_at(start_day) >= 0.0;
    let mut first: Option<(CoastEvent, f64)> = None;
    let mut consider = |event: CoastEvent, day: Option<f64>| {
        if let Some(day) = day
            && first.is_none_or(|(_, earliest)| day < earliest)
        {
            first = Some((event, day));
        }
    };
    for &(body, radius_km) in planets {
        let Some(nu) = coast.true_anomaly_at_radius(radius_km) else {
            continue;
        };
        let angles: &[f64] = if Some(body) == intercepted {
            // The crossing it is leaving is at +nu going out, −nu coming
            // in; only the other one is a return.
            if heading_out { &[-nu] } else { &[nu] }
        } else {
            &[nu, -nu]
        };
        for &angle in angles {
            let day = coast.next_day_at_true_anomaly(angle, start_day);
            consider(CoastEvent::Crosses { body, days: 0.0 }, day);
        }
    }
    let lost_day = if coast.state_at(start_day).radius() > lost_km {
        Some(start_day)
    } else {
        coast
            .true_anomaly_at_radius(lost_km)
            .and_then(|nu| coast.next_day_at_true_anomaly(nu, start_day))
    };
    consider(CoastEvent::Leaves { days: 0.0 }, lost_day);
    first
}

/// PLNR-88: the coast event of a leg, counted from `today`. Defined only
/// for an Intercept whose coast goes round the star; any other leg, or a
/// coast that meets nothing, has none. The planets are the bodies that
/// orbit the star; the intercepted one is the destination's planet.
pub fn coast_event(
    tree: &SystemTree,
    lost_beyond_km: f64,
    leg: &SolvedLeg,
    destination: Place,
    today: f64,
) -> Option<CoastEvent> {
    if leg.landing != Landing::Intercept {
        return None;
    }
    let star = leg.primary;
    if tree.bodies.get(&star)?.parent.is_some() {
        return None;
    }
    let SegmentKind::Conic { primary, conic } = leg.trajectory.segments().last()?.kind else {
        return None;
    };
    if primary != star {
        return None;
    }
    let planets: Vec<PlanetOrbit> = tree
        .bodies
        .iter()
        .filter(|(_, node)| node.class == BodyClass::Planet && node.parent == Some(star))
        .filter_map(|(&body, node)| Some((body, node.orbit?.radius_km)))
        .collect();
    let intercepted = resolve(tree, destination)
        .ok()
        .and_then(|target| target.orbit())
        .and_then(|(_, node)| tree.child_below(star, node.docking_for.unwrap_or(node.body)));
    let (event, day) = first_coast_event(
        &conic,
        leg.crossing_end_day,
        &planets,
        intercepted,
        lost_beyond_km,
    )?;
    let days = day - today;
    Some(match event {
        CoastEvent::Crosses { body, .. } => CoastEvent::Crosses { body, days },
        CoastEvent::Leaves { .. } => CoastEvent::Leaves { days },
    })
}

#[cfg(test)]
mod tests {
    use bevy::math::DVec3;
    use orbits::State;

    use super::*;
    use crate::sim::transfer::order::Arrival;
    use crate::sim::transfer::places::Band;
    use crate::sim::transfer::test_layout::layout;
    use crate::sim::units::KM_PER_AU;

    const MU: f64 = 1e21;

    /// A coast leaving 1 AU on day 0 at `speed` times the circular speed,
    /// turned `tilt` radians from along the circle towards straight out
    /// (negative: in).
    fn coast(speed: f64, tilt: f64) -> Conic {
        let r = KM_PER_AU;
        let v = orbits::circular_speed(MU, r) * speed;
        let velocity = DVec3::new(v * tilt.sin(), 0.0, v * tilt.cos());
        let state = State::new(DVec3::new(r, 0.0, 0.0), velocity);
        Conic::from_state(MU, state, 0.0, 1e-6).unwrap()
    }

    fn entities() -> [Entity; 3] {
        let mut world = World::new();
        [(); 3].map(|_| world.spawn_empty().id())
    }

    /// PLNR-88: a coast climbing out from 1 AU first crosses the nearest
    /// orbit outside it, and a hyperbola with nothing in its way leaves
    /// the system at the loss distance.
    #[test]
    fn a_coast_crosses_the_next_orbit_out_first() {
        let [near, far, _] = entities();
        let planets = [(far, 3.0 * KM_PER_AU), (near, 1.5 * KM_PER_AU)];
        let climbing = coast(1.2, 0.0);
        let (event, day) =
            first_coast_event(&climbing, 0.0, &planets, None, 6.0 * KM_PER_AU).unwrap();
        assert_eq!(
            event,
            CoastEvent::Crosses {
                body: near,
                days: 0.0
            }
        );
        let r = climbing.state_at(day).radius();
        assert!((r - 1.5 * KM_PER_AU).abs() < 1.0, "{r}");

        let escaping = coast(1.6, 0.0);
        let (event, day) = first_coast_event(&escaping, 0.0, &[], None, 6.0 * KM_PER_AU).unwrap();
        assert_eq!(event, CoastEvent::Leaves { days: 0.0 });
        assert!((escaping.state_at(day).radius() - 6.0 * KM_PER_AU).abs() < 10.0);
    }

    /// PLNR-88: the intercepted planet's own orbit counts only on a
    /// return: a loop climbing out from 1 AU crosses 1 AU again on its
    /// way back in, within one loop; with no other orbit in reach, that
    /// return is the event. A coast reaching nothing has no event.
    #[test]
    fn the_intercepted_orbit_counts_only_on_a_return() {
        let [home, inner, _] = entities();
        let looping = coast(1.06, 0.1);
        let period = looping.period_days().unwrap();
        let planets = [(home, KM_PER_AU)];
        let (event, day) =
            first_coast_event(&looping, 0.0, &planets, Some(home), 6.0 * KM_PER_AU).unwrap();
        assert_eq!(
            event,
            CoastEvent::Crosses {
                body: home,
                days: 0.0
            }
        );
        assert!(day > 0.1 * period && day < period, "{day} vs {period}");
        let back = looping.state_at(day);
        assert!((back.radius() - KM_PER_AU).abs() < 1.0);
        assert!(back.position.dot(back.velocity) < 0.0, "coming back in");

        // Falling inward from 1 AU: an orbit at 0.9 AU is met on the way
        // down, before the return.
        let falling = coast(0.95, -0.1);
        let planets = [(home, KM_PER_AU), (inner, 0.9 * KM_PER_AU)];
        let (event, _) =
            first_coast_event(&falling, 0.0, &planets, Some(home), 6.0 * KM_PER_AU).unwrap();
        assert_eq!(
            event,
            CoastEvent::Crosses {
                body: inner,
                days: 0.0
            }
        );

        let tiny = [(inner, 0.1 * KM_PER_AU)];
        assert_eq!(
            first_coast_event(&falling, 0.0, &tiny, None, 6.0 * KM_PER_AU),
            None
        );
    }

    /// PLNR-88: a leg's coast event is counted from today; a Rendezvous
    /// has none.
    #[test]
    fn a_legs_coast_event_counts_from_today() {
        let l = layout();
        let from = l.parked(l.capital, Band::High);
        let order = crate::sim::transfer::order::Order {
            destination: Place::Body {
                body: l.inner,
                band: Band::High,
            },
            arrival: Arrival::Intercept { band: Band::High },
            depart_day: 0.0,
            travel_days: 100.0,
        };
        let leg = l.solve(&from, &order, &l.ntr).unwrap();
        let lost = l.tunables.lost_beyond_au * KM_PER_AU;
        let event = coast_event(&l.tree, lost, &leg, order.destination, 5.0).unwrap();
        let days = match event {
            CoastEvent::Crosses { days, .. } | CoastEvent::Leaves { days } => days,
        };
        assert!(days > leg.crossing_end_day - 5.0, "{days}");

        let rendezvous = crate::sim::transfer::order::Order {
            arrival: Arrival::Rendezvous,
            ..order
        };
        let leg = l.solve(&from, &rendezvous, &l.ntr).unwrap();
        assert_eq!(
            coast_event(&l.tree, lost, &leg, order.destination, 5.0),
            None
        );
    }
}

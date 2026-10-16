//! Headless tests of transfers (`docs/spec/transfers.md`) and the
//! transfer planner's rules (`docs/spec/planner.md`, section 7): ships
//! flying between bands, intercepts, escapes, failed and cancelled plans,
//! ships and stations in their bands, and the planner's search for its
//! test ships.

use super::*;
use crate::sim::body::Mass;
use crate::sim::ship::ShipRail;
use crate::sim::transfer::order::Landing;

/// A headless simulation of Corvane with no view at all.
fn sim_app() -> App {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin));
    run(&mut app, 1);
    app
}

/// Orison's order from Halden IV's High band down to its Low band, as a
/// Hohmann transfer leaving on `depart_day`.
fn high_to_low(app: &mut App, halden: Entity, depart_day: f64) -> Order {
    let high = band_orbit(app, halden, Band::High);
    let low = band_orbit(app, halden, Band::Low);
    let mass = app.world().get::<Mass>(halden).unwrap().0;
    let mu = crate::sim::units::gravitational_parameter_km3_per_day2(mass);
    let radius = |app: &App, orbit| app.world().get::<NamedOrbit>(orbit).unwrap().radius_km;
    let travel_days = orbits::hohmann(mu, radius(app, high), radius(app, low)).days;
    Order {
        destination: Place::Body {
            body: halden,
            band: Band::Low,
        },
        arrival: Arrival::Rendezvous,
        depart_day,
        travel_days,
    }
}

/// TEST-20 / PLAN-10 / PLAN-21 / RAIL-09 / BAND-05 / BAND-06 / XFER-02:
/// Orison, parked in Halden IV's High band, is ordered down to Low. The
/// plan is Scheduled with its delta-v reserved; on the depart day it is
/// InFlight; at the crossing start the ship burns, leaves its named orbit
/// and flies round Halden IV on its rail; at the crossing end it burns
/// again and joins Low where its arc ends; on arrival the plan is
/// Done and the whole delta-v is spent.
#[test]
fn a_warship_flies_from_high_to_low_and_parks() {
    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let high = band_orbit(&mut app, halden, Band::High);
    let low = band_orbit(&mut app, halden, Band::Low);
    let low_km = app.world().get::<NamedOrbit>(low).unwrap().radius_km;
    let high_km = app.world().get::<NamedOrbit>(high).unwrap().radius_km;
    assert_eq!(app.world().get::<InNamedOrbit>(orison).unwrap().0, high);
    assert_eq!(account(&app, orison), DeltaV::full(9.9));

    let order = high_to_low(&mut app, halden, 1.0);
    schedule(&mut app, orison, order).unwrap();
    let leg = plan(&app, orison).leg().clone();
    assert_eq!(plan(&app, orison).status, Status::Scheduled);
    assert!(
        leg.delta_v_kms > 3.0 && leg.delta_v_kms < 6.0,
        "{}",
        leg.delta_v_kms
    );
    assert!((account(&app, orison).reserved_kms - leg.delta_v_kms).abs() < 1e-9);
    assert_eq!(leg.primary, halden);

    run_until(&mut app, 1.0, 0.95);
    assert_eq!(plan(&app, orison).status, Status::Scheduled);
    assert!(app.world().get::<InFlight>(orison).is_none());

    run_until(
        &mut app,
        1.0,
        (leg.crossing_start_day + leg.crossing_end_day) / 2.0,
    );
    assert_eq!(plan(&app, orison).status, Status::InFlight);
    assert!(app.world().get::<InFlight>(orison).is_some());
    assert!(app.world().get::<InNamedOrbit>(orison).is_none());
    assert!(app.world().get::<Orbit>(orison).is_none());
    assert_eq!(app.world().get::<ChildOf>(orison).unwrap().parent(), halden);
    let mid = app
        .world()
        .get::<OrbitPositionKm>(orison)
        .unwrap()
        .0
        .length();
    assert!(
        mid < high_km && mid > low_km,
        "{mid} km, between {low_km} and {high_km}"
    );
    let rail = app.world().get::<ShipRail>(orison).unwrap();
    let expected = rail.0.state_at(day(&app)).unwrap().position;
    assert!((app.world().get::<OrbitPositionKm>(orison).unwrap().0 - expected).length() < 1e-6);
    let mid_account = account(&app, orison);
    assert!((mid_account.spent_kms - leg.burns.start_kms).abs() < 1e-9);
    assert!((mid_account.reserved_kms - leg.burns.end_kms).abs() < 1e-9);

    run_until(&mut app, 1.0, leg.arrival_day + 0.1);
    assert_eq!(plan(&app, orison).status, Status::Done);
    assert!(app.world().get::<InFlight>(orison).is_none());
    assert!(app.world().get::<ShipRail>(orison).is_none());
    assert_eq!(app.world().get::<InNamedOrbit>(orison).unwrap().0, low);
    let orbit = app.world().get::<Orbit>(orison).unwrap();
    assert_eq!(orbit.radius_km, low_km);
    let Landing::Orbit { phase0_rad, .. } = leg.landing else {
        panic!("a Rendezvous lands in an orbit");
    };
    assert_eq!(orbit.phase0_rad, phase0_rad);
    let parked = app
        .world()
        .get::<OrbitPositionKm>(orison)
        .unwrap()
        .0
        .length();
    assert!((parked - low_km).abs() < 1e-6);
    let done = account(&app, orison);
    assert!((done.spent_kms - leg.delta_v_kms).abs() < 1e-9);
    assert!(done.reserved_kms.abs() < 1e-9);
    assert!((done.available_kms() - (9.9 - leg.delta_v_kms)).abs() < 1e-9);
}

/// TEST-20 / XFER-06 / PLAN-21: RFA Tolland's ion drive spirals from High
/// down to Low over the days its acceleration dictates, with the whole
/// delta-v charged at the crossing start.
#[test]
fn a_tanker_spirals_between_bands() {
    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let tolland = body(&mut app, "RFA Tolland");
    let low = band_orbit(&mut app, halden, Band::Low);
    let order = Order {
        travel_days: 1.0,
        ..high_to_low(&mut app, halden, 0.5)
    };
    schedule(&mut app, tolland, order).unwrap();
    let leg = plan(&app, tolland).leg().clone();
    assert!(
        leg.travel_days > 50.0 && leg.travel_days < 400.0,
        "{}",
        leg.travel_days
    );
    assert!(matches!(
        leg.trajectory.segments()[1].kind,
        SegmentKind::Spiral { .. }
    ));
    assert_eq!(leg.burns.end_kms, 0.0);

    run_until(&mut app, 100.0, leg.crossing_start_day + 5.0);
    assert_eq!(plan(&app, tolland).status, Status::InFlight);
    let account = account(&app, tolland);
    assert!(
        (account.spent_kms - leg.delta_v_kms).abs() < 1e-9,
        "the spiral is charged at once"
    );
    assert!(account.reserved_kms.abs() < 1e-9);

    run_until(&mut app, 100.0, leg.arrival_day + 2.0);
    assert_eq!(plan(&app, tolland).status, Status::Done);
    assert_eq!(app.world().get::<InNamedOrbit>(tolland).unwrap().0, low);
}

/// XFER-08 / BAND-07 / RAIL-08 / PLAN-12: Orison passes Iver. At the
/// crossing's end the plan is Done and the ship coasts round the star on
/// the same conic, unchanged by passing through Iver's Hill sphere; its
/// plan still says where it came from and what it did.
#[test]
fn an_intercept_leaves_the_ship_coasting_unchanged_by_the_planet() {
    use crate::sim::transfer::tree::{ShipState, TreeQuery};
    use bevy::ecs::system::RunSystemOnce;

    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let iver = body(&mut app, "Iver");
    let orison = body(&mut app, "Orison");
    let order = cheapest_intercept_of_iver(&mut app, orison, iver);
    schedule(&mut app, orison, order).unwrap();
    let leg = plan(&app, orison).leg().clone();
    assert_eq!(leg.arrival, Arrival::Intercept { band: Band::High });
    assert!(leg.intercept_speed_kms.is_some());
    assert_eq!(leg.trajectory.segments().len(), 2);

    run_until(&mut app, 20.0, leg.crossing_start_day + 1.0);
    assert!(
        app.world().get::<ChildOf>(orison).is_none(),
        "round the star"
    );
    let SegmentKind::Conic { conic: before, .. } =
        app.world().get::<ShipRail>(orison).unwrap().0.segments()[1].kind
    else {
        panic!("a conic crossing");
    };

    run_until(&mut app, 20.0, leg.crossing_end_day + 30.0);
    assert_eq!(plan(&app, orison).status, Status::Done);
    assert!(app.world().get::<InFlight>(orison).is_some());
    assert!(app.world().get::<InNamedOrbit>(orison).is_none());
    let rail = app.world().get::<ShipRail>(orison).unwrap().0.clone();
    let SegmentKind::Conic {
        conic: after,
        primary,
    } = rail.segment_at(day(&app)).unwrap().kind
    else {
        panic!("still coasting on the crossing conic");
    };
    assert_eq!(before, after, "passing Iver changed nothing (RAIL-08)");
    assert_eq!(primary, body(&mut app, "Corvane"));
    let position = app.world().get::<OrbitPositionKm>(orison).unwrap().0;
    assert!((position - rail.state_at(day(&app)).unwrap().position).length() < 1e-6);
    let state = app
        .world_mut()
        .run_system_once(move |tree: TreeQuery, clock: Res<GameClock>| {
            tree.snapshot(clock.elapsed_days()).ships[&orison]
        })
        .unwrap();
    assert!(matches!(state, ShipState::Coasting { conic, .. } if conic == after));
    let plan = plan(&app, orison);
    assert_eq!(
        plan.leg().origin,
        Some(Place::Body {
            body: halden,
            band: Band::High
        })
    );
    assert_eq!(
        plan.leg().destination,
        Place::Body {
            body: iver,
            band: Band::High
        }
    );
    let spent = account(&app, orison).spent_kms;
    assert!((spent - leg.delta_v_kms).abs() < 1e-9);
}

/// XFER-13 / PLAN-10 / BAND-07: Orison passes Iver through its Low band.
/// At the crossing's end the plan is Done and the ship coasts round the
/// star on a new conic, swung round by Iver: it leaves Iver at the speed
/// it came in with, its direction turned prograde by the leg's turn.
#[test]
fn an_intercept_through_low_leaves_the_ship_coasting_on_the_turned_path() {
    use crate::sim::transfer::phase::gap_rad;
    use orbits::{CircularOrbit, Rail, angle_of};

    let mut app = sim_app();
    let iver = body(&mut app, "Iver");
    let orison = body(&mut app, "Orison");
    let order = Order {
        destination: Place::Body {
            body: iver,
            band: Band::Low,
        },
        arrival: Arrival::Intercept { band: Band::Low },
        ..cheapest_intercept_of_iver(&mut app, orison, iver)
    };
    schedule(&mut app, orison, order).unwrap();
    let leg = plan(&app, orison).leg().clone();
    let turn = leg.turn_rad.expect("a pass through Low turns");
    let SegmentKind::Conic {
        conic: crossing, ..
    } = leg.trajectory.segments()[1].kind
    else {
        panic!("a conic crossing");
    };

    run_until(&mut app, 20.0, leg.crossing_end_day + 30.0);
    assert_eq!(plan(&app, orison).status, Status::Done);
    assert!(app.world().get::<InNamedOrbit>(orison).is_none());
    let rail = app.world().get::<ShipRail>(orison).unwrap().0.clone();
    let segment = rail.segment_at(day(&app)).unwrap();
    assert_eq!(segment.end_day, None);
    let SegmentKind::Conic {
        conic: after,
        primary,
    } = segment.kind
    else {
        panic!("coasting after the pass");
    };
    assert_eq!(primary, body(&mut app, "Corvane"));
    assert_ne!(after, crossing, "the pass changed the path");

    let end = leg.crossing_end_day;
    let iver_v = CircularOrbit::from(app.world().get::<Orbit>(iver).unwrap())
        .state_at(end)
        .velocity;
    let v_in = crossing.state_at(end).velocity - iver_v;
    let v_out = after.state_at(end).velocity - iver_v;
    assert!((v_out.length() - v_in.length()).abs() < 1e-6 * v_in.length());
    assert!((gap_rad(angle_of(v_in), angle_of(v_out)) - turn).abs() < 1e-6);
}

/// BAND-08: a coasting ship on a hyperbola round the star is removed
/// once it is `lost_beyond_au` out; a bound coaster is not.
#[test]
fn an_escaping_ship_is_removed_beyond_the_loss_distance() {
    use orbits::{Conic, Segment, State, Trajectory};

    let mut app = sim_app();
    let star = body(&mut app, "Corvane");
    let star_mass = app.world().get::<Mass>(star).unwrap().0;
    let mu = crate::sim::units::gravitational_parameter_km3_per_day2(star_mass);
    let start = bevy::math::DVec3::new(5.9 * KM_PER_AU, 0.0, 0.0);
    let spawn_coaster = |app: &mut App, outward_kms: f64, along_kms: f64| {
        let velocity = bevy::math::DVec3::new(
            orbits::km_per_s_to_km_per_day(outward_kms),
            0.0,
            orbits::km_per_s_to_km_per_day(along_kms),
        );
        let conic = Conic::from_state(mu, State::new(start, velocity), 0.0, 1e-6).unwrap();
        let rail = Trajectory::new(vec![Segment {
            start_day: 0.0,
            end_day: None,
            kind: SegmentKind::Conic {
                primary: star,
                conic,
            },
        }])
        .unwrap();
        app.world_mut()
            .spawn((
                BodyClass::Ship,
                DisplayName("coaster".into()),
                Affiliation::Neutral,
                InFlight,
                ShipRail(rail),
                OrbitPositionKm(start),
            ))
            .id()
    };
    let escaping = spawn_coaster(&mut app, 20.0, 5.0);
    let falling = spawn_coaster(&mut app, 0.0, 5.0);
    run_until(&mut app, 50.0, 200.0);
    assert!(app.world().get_entity(escaping).is_err(), "gone past 6 AU");
    assert!(
        app.world().get_entity(falling).is_ok(),
        "a bound coaster stays"
    );
    let fell = app
        .world()
        .get::<OrbitPositionKm>(falling)
        .unwrap()
        .0
        .length();
    assert!(fell < 5.9 * KM_PER_AU, "it falls inward on its rail");
}

/// PLAN-11: on the depart day the leg is re-validated: a ship no longer
/// in the orbit the plan assumed fails its plan, and the reservation is
/// released.
#[test]
fn a_plan_fails_if_the_ship_is_no_longer_where_it_was() {
    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let order = high_to_low(&mut app, halden, 2.0);
    schedule(&mut app, orison, order).unwrap();
    assert!(account(&app, orison).reserved_kms > 0.0);
    let low = band_orbit(&mut app, halden, Band::Low);
    app.world_mut().entity_mut(orison).insert(InNamedOrbit(low));
    run_until(&mut app, 1.0, 2.5);
    assert_eq!(plan(&app, orison).status, Status::Failed);
    assert_eq!(account(&app, orison), DeltaV::full(9.9));
    assert!(app.world().get::<ShipRail>(orison).is_none());
}

/// PLAN-02 / PLAN-22 / ARCH-14: a ship takes one plan at a time; a
/// Scheduled plan can be cancelled, which gives its delta-v back, and a
/// new one given; orders also arrive as messages.
#[test]
fn a_scheduled_plan_can_be_cancelled_but_not_doubled() {
    use crate::sim::transfer::plan::ShipOrder;
    use bevy::ecs::system::RunSystemOnce;

    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let order = high_to_low(&mut app, halden, 5.0);
    schedule(&mut app, orison, order).unwrap();
    assert_eq!(schedule(&mut app, orison, order), Err(Infeasible::Busy));
    let cancelled = app
        .world_mut()
        .run_system_once(move |mut planner: Planner| planner.cancel(orison))
        .unwrap();
    assert!(cancelled);
    assert_eq!(account(&app, orison), DeltaV::full(9.9));
    assert!(!plan(&app, orison).is_active());

    app.world_mut().write_message(ShipOrder::Transfer {
        ship: orison,
        order,
    });
    run(&mut app, 1);
    assert_eq!(plan(&app, orison).status, Status::Scheduled);
    app.world_mut()
        .write_message(ShipOrder::Cancel { ship: orison });
    run(&mut app, 1);
    assert_eq!(plan(&app, orison).status, Status::Failed);
    assert_eq!(account(&app, orison), DeltaV::full(9.9));
}

/// PLAN-13: Hold mid-flight keeps the ship on its crossing conic without
/// end, drops the capture burn and gives its delta-v back; the ship never
/// parks.
#[test]
fn hold_drops_the_later_burns() {
    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let order = high_to_low(&mut app, halden, 1.0);
    schedule(&mut app, orison, order).unwrap();
    let leg = plan(&app, orison).leg().clone();
    run_until(
        &mut app,
        1.0,
        (leg.crossing_start_day + leg.crossing_end_day) / 2.0,
    );
    let hold = Order {
        destination: Place::Hold,
        ..order
    };
    schedule(&mut app, orison, hold).unwrap();
    assert_eq!(plan(&app, orison).status, Status::Holding);
    let rail = app.world().get::<ShipRail>(orison).unwrap().0.clone();
    assert_eq!(rail.segments().len(), 2);
    assert_eq!(rail.segments()[1].end_day, None);
    let account = account(&app, orison);
    assert!((account.spent_kms - leg.burns.start_kms).abs() < 1e-9);
    assert!(account.reserved_kms.abs() < 1e-9);

    run_until(&mut app, 1.0, leg.arrival_day + 1.0);
    assert!(app.world().get::<InFlight>(orison).is_some());
    assert!(app.world().get::<InNamedOrbit>(orison).is_none());
    assert_eq!(plan(&app, orison).status, Status::Holding);
}

/// A ship's named orbit and phase there, read from its components.
fn parked_phase(app: &App, ship: Entity) -> (Entity, f64) {
    let orbit = app.world().get::<InNamedOrbit>(ship).unwrap().0;
    (orbit, app.world().get::<Orbit>(ship).unwrap().phase0_rad)
}

fn position_km(app: &App, entity: Entity) -> bevy::math::DVec3 {
    app.world().get::<OrbitPositionKm>(entity).unwrap().0
}

/// BAND-05 / TEST-10: Orison and Ardent share Halden IV's High band but
/// not a place in it: on every day they are 2r sin(gap / 2) apart.
#[test]
fn ships_in_one_band_are_apart_by_their_phase_gap() {
    use crate::sim::transfer::phase::{gap_rad, separation_km};

    let mut app = sim_app();
    let orison = body(&mut app, "Orison");
    let ardent = body(&mut app, "Ardent");
    let (high, orison_phase) = parked_phase(&app, orison);
    let (ardent_orbit, ardent_phase) = parked_phase(&app, ardent);
    assert_eq!(high, ardent_orbit);
    let radius = app.world().get::<NamedOrbit>(high).unwrap().radius_km;
    let expected = separation_km(radius, gap_rad(orison_phase, ardent_phase));
    assert!(expected > 1_000.0, "{expected} km");
    for until in [0.3, 2.7] {
        run_until(&mut app, 1.0, until);
        let apart = (position_km(&app, orison) - position_km(&app, ardent)).length();
        assert!((apart - expected).abs() < 1e-3, "{apart} vs {expected}");
    }
}

/// An order for Orison to meet Ardent in their shared band.
fn meet_ardent(app: &mut App, depart_day: f64) -> Order {
    let ardent = body(app, "Ardent");
    Order {
        destination: Place::Ship { ship: ardent },
        arrival: Arrival::Rendezvous,
        depart_day,
        travel_days: 1.0,
    }
}

/// PHAS-05 / PHAS-09 / PHAS-02: Orison closes the gap to Ardent in their
/// shared band with a phasing leg. It leaves its orbit at the first burn,
/// flies round Halden IV, and on arrival is back in High, together with
/// Ardent, with both burns spent.
#[test]
fn a_ship_closes_the_gap_to_another_in_its_band() {
    let mut app = sim_app();
    let orison = body(&mut app, "Orison");
    let ardent = body(&mut app, "Ardent");
    let (high, _) = parked_phase(&app, orison);
    let order = meet_ardent(&mut app, 0.5);
    schedule(&mut app, orison, order).unwrap();
    let leg = plan(&app, orison).leg().clone();
    assert_eq!(leg.primary, body(&mut app, "Halden IV"));
    assert!(leg.travel_days >= 1.0);

    run_until(
        &mut app,
        1.0,
        (leg.crossing_start_day + leg.crossing_end_day) / 2.0,
    );
    assert!(app.world().get::<InFlight>(orison).is_some());

    run_until(&mut app, 1.0, leg.arrival_day + 0.2);
    assert_eq!(plan(&app, orison).status, Status::Done);
    assert_eq!(parked_phase(&app, orison).0, high);
    let apart = (position_km(&app, orison) - position_km(&app, ardent)).length();
    let together_km = app
        .world()
        .resource::<crate::sim::tunables::SimTunables>()
        .together_km;
    assert!(apart < together_km, "{apart} km apart");
    assert!((account(&app, orison).spent_kms - leg.delta_v_kms).abs() < 1e-9);
}

/// PHAS-04: an order to the ship's own band is already done: it is
/// accepted, makes no plan and costs nothing.
#[test]
fn an_order_to_where_the_ship_is_changes_nothing() {
    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let order = Order {
        destination: Place::Body {
            body: halden,
            band: Band::High,
        },
        ..meet_ardent(&mut app, 1.0)
    };
    assert_eq!(
        preview(&mut app, orison, order),
        Err(Infeasible::AlreadyThere)
    );
    assert_eq!(schedule(&mut app, orison, order), Ok(()));
    run(&mut app, 1);
    assert!(app.world().get::<Plan>(orison).is_none());
    assert_eq!(account(&app, orison), DeltaV::full(9.9));
}

/// PLAN-11: a leg that meets a ship remembers where the ship was. If the
/// target has moved round the band by the depart day, the plan fails and
/// its delta-v is given back.
#[test]
fn a_plan_fails_if_its_target_ship_has_moved() {
    let mut app = sim_app();
    let orison = body(&mut app, "Orison");
    let ardent = body(&mut app, "Ardent");
    let order = meet_ardent(&mut app, 2.0);
    schedule(&mut app, orison, order).unwrap();
    assert!(plan(&app, orison).leg().target_lock.is_some());
    app.world_mut().get_mut::<Orbit>(ardent).unwrap().phase0_rad += 0.5;
    run_until(&mut app, 1.0, 2.5);
    assert_eq!(plan(&app, orison).status, Status::Failed);
    assert_eq!(account(&app, orison), DeltaV::full(9.9));
}

/// PHAS-15 / XFER-10: Orison's Intercept at Iver Yard, in Iver's High
/// band, is trimmed by less than one period of the band so that the yard
/// is at the Intercept's closest-approach point on the crossing end day.
#[test]
fn an_intercept_at_iver_yard_is_trimmed_to_meet_it() {
    use crate::sim::transfer::phase::gap_rad;

    let mut app = sim_app();
    let orison = body(&mut app, "Orison");
    let iver = body(&mut app, "Iver");
    let yard = body(&mut app, "Iver Yard");
    let order = Order {
        destination: Place::Station { station: yard },
        ..cheapest_intercept_of_iver(&mut app, orison, iver)
    };
    let leg = preview(&mut app, orison, order).unwrap();
    let (band, yard_phase) = parked_phase(&app, yard);
    let named = app.world().get::<NamedOrbit>(band).unwrap().clone();
    let trim = leg.trim_days.expect("an Intercept at a station is trimmed");
    assert!((0.0..named.period_days).contains(&trim), "{trim} d");
    let yard_angle = orbits::CircularOrbit {
        radius_km: named.radius_km,
        period_days: named.period_days,
        phase0_rad: yard_phase,
    }
    .angle_at(leg.crossing_end_day);
    let miss = gap_rad(yard_angle, leg.closest_approach_rad.unwrap());
    let miss_km = named.radius_km * miss.min(std::f64::consts::TAU - miss);
    assert!(miss_km < 10.0, "{miss_km} km");
}

/// BAND-02 / SCL-07 / TEST-02 / XFER-10: Arden Highport is a place in
/// Halden IV's Low band (period about 0.07 d) at its own phase, and offers
/// no orbit of its own. An order to the station flies to that band and
/// meets the station where it is, landing at its phase.
#[test]
fn a_station_is_a_place_in_its_band_at_its_phase() {
    use crate::sim::transfer::tree::{StationNode, TreeQuery};
    use bevy::ecs::system::RunSystemOnce;

    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let highport = body(&mut app, "Arden Highport");
    let low = band_orbit(&mut app, halden, Band::Low);
    let period = app.world().get::<NamedOrbit>(low).unwrap().period_days;
    assert!((period - 0.067).abs() < 0.002, "{period} d");
    let mut named = app.world_mut().query::<&NamedOrbit>();
    assert!(
        !named.iter(app.world()).any(|o| o.name == "Arden Highport"),
        "a station has no docking orbit of its own"
    );

    let station = app
        .world_mut()
        .run_system_once(move |tree: TreeQuery| tree.snapshot(0.0).stations[&highport])
        .unwrap();
    assert_eq!(
        station,
        StationNode {
            orbit: low,
            phase0_rad: 2.4
        }
    );

    let to_band = high_to_low(&mut app, halden, 1.0);
    let to_station = Order {
        destination: Place::Station { station: highport },
        ..to_band
    };
    let leg = preview(&mut app, orison, to_station).unwrap();
    assert!(
        matches!(leg.landing, Landing::Orbit { orbit, phase0_rad, .. } if orbit == low && phase0_rad == 2.4)
    );
    let low_km = app.world().get::<NamedOrbit>(low).unwrap().radius_km;
    let station_then = orbits::CircularOrbit {
        radius_km: low_km,
        period_days: period,
        phase0_rad: 2.4,
    }
    .position_at(leg.crossing_end_day);
    let arc_end = leg
        .trajectory
        .state_at(leg.crossing_end_day - 1e-9)
        .unwrap();
    assert!((arc_end.position - station_then).length() < 10.0);
}

/// TEST-30 / PLNR-80 to PLNR-86: the planner's tests plan with Orison and
/// RFA Tolland from Halden IV's High band. Orison's drive is the hydrogen
/// NTR, impulsive, so an Intercept of Iver has a Depart range set by the
/// two planets' periods and a Travel range round the least-fuel time,
/// and all three presets; Fastest is within its 9.9 km/s. Tolland's NEP
/// ion/Hall drive is low-thrust: 180 days of Depart range, one travel
/// time, and no Next window.
#[test]
fn the_planners_test_ships_search_from_haldens_high_band() {
    use crate::sim::transfer::ranges::Slider;
    use crate::sim::transfer::search::Request;

    let mut app = sim_app();
    let iver = body(&mut app, "Iver");
    let orison = body(&mut app, "Orison");
    let tolland = body(&mut app, "RFA Tolland");
    let request = Request {
        destination: Place::Body {
            body: iver,
            band: Band::High,
        },
        arrival: Arrival::Rendezvous,
    };

    let intercept = Request {
        arrival: Arrival::Intercept { band: Band::High },
        ..request
    };
    let (context, found) = search(&mut app, orison, intercept);
    assert_eq!(context.drive.name, "hydrogen NTR");
    assert_eq!(context.available_kms, 9.9);
    let found = found.unwrap();
    assert!(found.sliders.windows);
    assert!(matches!(found.sliders.travel, Slider::Range { .. }));
    let fastest = found.presets.fastest.expect("Iver is within reach");
    assert!(context.solve(intercept, fastest).unwrap().delta_v_kms <= 9.9);
    assert!(found.presets.cheapest.is_some());
    assert!(found.presets.next_window.is_some());

    let (context, found) = search(&mut app, tolland, request);
    assert_eq!(context.drive.name, "NEP ion/Hall");
    let found = found.unwrap();
    assert!(!found.sliders.windows);
    let Slider::Days {
        soonest: start,
        end,
        ..
    } = found.sliders.depart
    else {
        panic!("a Depart range");
    };
    assert!((end - start - 180.0).abs() < 1e-9);
    assert!(matches!(found.sliders.travel, Slider::Fixed(_)));
    assert_eq!(found.presets.next_window, None);
}

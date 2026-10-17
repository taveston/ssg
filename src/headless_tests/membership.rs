//! Headless tests of orbit membership as ships come and go (RAIL-09,
//! SCL-07): a ship that arrives in a named orbit is one of its members,
//! and the orbit it left loses it, on the map, on the rings and in the
//! outline. Played out in Sol, with Marlin sent from Earth's Low band,
//! where it is docked at Perigee Station, to the Moon's Low band, which
//! nothing is in.

use std::collections::HashMap;

use bevy::ecs::system::RunSystemOnce;
use bevy::picking::{InteractionPlugin, PickingPlugin, PickingSystems};

use super::camera::{follow_cursor_with_pointer, move_cursor};
use super::*;
use crate::sim::transfer::plan::ShipOrder;
use crate::sim::transfer::search::Request;
use crate::view::inset::outline::{Expanded, OutlinePlugin, OutlineRoot, OutlineRow};
use crate::view::planner::PlannerPlugin;
use crate::view::planner::input::order_to_schedule;
use crate::view::rings::picking::{MemberTailTarget, RingPaths, RingPickingPlugin, RingTarget};
use crate::view::shared_orbit::{OrbitMembers, OrbitRings};
use crate::view::targets::OrbitFor;

/// Swaps Corvane for Sol before the system is spawned.
fn load_sol(mut waiting: ResMut<SystemDataResource>) {
    waiting.0 = Some(crate::content::sol::sol_system());
}

/// Sol with markers, multi-markers, labels, the planner, the outline and
/// the rings' hit targets under Bevy's own picking, paused.
fn sol_app() -> App {
    let mut app = symbols_app_with(|app| {
        app.add_systems(Startup, load_sol.in_set(Boot::World))
            .add_plugins((
                PlannerPlugin,
                OutlinePlugin,
                SharedOrbitPlugin,
                RingPickingPlugin,
                PickingPlugin,
                InteractionPlugin,
            ))
            .add_systems(
                PreUpdate,
                follow_cursor_with_pointer.before(PickingSystems::ProcessInput),
            )
            .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>();
    });
    let window = the_window(&mut app);
    app.world_mut()
        .entity_mut(window)
        .insert(bevy::window::PrimaryWindow);
    app.world_mut()
        .spawn(bevy::picking::pointer::PointerId::Mouse);
    app.world_mut().spawn((OutlineRoot, Node::default()));
    run(&mut app, 2);
    app
}

/// Moves the clock on by `days` in one step, as a skip would.
fn skip_days(app: &mut App, days: f64) {
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    let rate = clock.rate_days_per_sec();
    clock.advance(days / rate);
    clock.toggle_pause();
    run(app, 3);
}

/// Sends Marlin through the planner on a Rendezvous in the Moon's Low
/// band, as soon as it can leave, then moves the clock past its arrival.
/// Marlin stays selected if `keep_selected`; otherwise nothing is.
fn fly_marlin_to_the_moon(app: &mut App, marlin: Entity, moon: Entity, keep_selected: bool) {
    app.world_mut().resource_mut::<Selection>().select(marlin);
    app.world_mut().resource_mut::<PlannerState>().open(marlin);
    run(app, 1);
    app.world_mut()
        .resource_mut::<PlannerState>()
        .session
        .as_mut()
        .unwrap()
        .pick(Request {
            destination: Place::Body {
                body: moon,
                band: Band::Low,
            },
            arrival: Arrival::Rendezvous,
        });
    run(app, 8);
    let (ship, order, _) =
        order_to_schedule(app.world().resource::<PlannerState>()).expect("an order to schedule");
    app.world_mut()
        .write_message(ShipOrder::Transfer { ship, order });
    app.world_mut().resource_mut::<PlannerState>().end();
    if !keep_selected {
        app.world_mut().resource_mut::<Selection>().clear();
    }
    run(app, 3);
    let arrival = plan(app, marlin).leg().arrival_day;
    skip_days(app, arrival - day(app) + 0.5);
    assert_eq!(plan(app, marlin).status, Status::Done);
}

/// The members of each ring the view draws, by the ring's owner.
fn ring_members(app: &mut App) -> HashMap<Entity, Vec<Entity>> {
    app.world_mut()
        .run_system_once(|rings: OrbitRings| -> HashMap<Entity, Vec<Entity>> {
            rings
                .owners()
                .into_iter()
                .filter_map(|owner| rings.ring(owner))
                .map(|ring| (ring.owner, ring.members.iter().map(|m| m.body).collect()))
                .collect()
        })
        .unwrap()
}

/// The outline's rows, as the body or orbit each stands for, in order.
fn outline_rows(app: &mut App) -> Vec<Entity> {
    let mut roots = app
        .world_mut()
        .query_filtered::<&Children, With<OutlineRoot>>();
    let rows: Vec<Entity> = roots.single(app.world()).unwrap().iter().collect();
    rows.into_iter()
        .filter_map(|row| app.world().get::<OutlineRow>(row).map(|r| r.0))
        .collect()
}

/// The ring targets, as (target, what the ring stands for, the orbit it
/// targets as a set, the body it targets).
fn ring_targets(app: &mut App) -> Vec<(Entity, Entity, Option<Entity>, Option<Entity>)> {
    let mut targets = app
        .world_mut()
        .query::<(Entity, &RingTarget, Option<&OrbitFor>, Option<&MarkerFor>)>();
    targets
        .iter(app.world())
        .map(|(e, t, o, m)| (e, t.0, o.map(|o| o.0), m.map(|m| m.0)))
        .collect()
}

/// RAIL-09 / SCL-07 / INS-05 / GRP-01: Marlin, sent from Earth's Low band
/// to the Moon's, arrives as a member of the Moon's Low orbit and of no
/// other. Its symbol is drawn on the Moon's orbit, round the Moon; seen
/// from Earth it folds into the Moon, not into Earth; and the outline,
/// left as it was, lists it under the Moon's Low orbit row and no longer
/// under Earth's.
#[test]
fn a_ship_sent_to_the_moon_is_drawn_and_listed_there() {
    let mut app = sol_app();
    let earth = body(&mut app, "Earth");
    let moon = body(&mut app, "Moon");
    let sun = body(&mut app, "Sol");
    let marlin = body(&mut app, "Marlin");
    let others = ["Perigee Station", "Vestige"].map(|n| body(&mut app, n));
    let earth_low = band_orbit(&mut app, earth, Band::Low);
    let moon_low = band_orbit(&mut app, moon, Band::Low);
    {
        let mut expanded = app.world_mut().resource_mut::<Expanded>();
        expanded.entries.extend([sun, earth, moon]);
        expanded.orbits.extend([earth_low, moon_low]);
    }
    run(&mut app, 2);
    let rows = outline_rows(&mut app);
    let at = |rows: &[Entity], e: Entity| rows.iter().position(|&r| r == e);
    assert!(at(&rows, earth_low) < at(&rows, marlin) && at(&rows, marlin) < at(&rows, moon));
    assert_eq!(
        at(&rows, moon_low),
        None,
        "nothing is in the Moon's Low band"
    );
    assert!(ring_members(&mut app)[&earth_low].contains(&marlin));

    fly_marlin_to_the_moon(&mut app, marlin, moon, false);
    let members = app.world().resource::<OrbitMembers>().0.clone();
    assert_eq!(members[&moon_low], [marlin]);
    assert_eq!(members[&earth_low], others);
    let rings = ring_members(&mut app);
    assert_eq!(rings[&moon_low], [marlin], "on the Moon's ring");
    assert_eq!(rings[&earth_low], others, "off Earth's ring");

    // In Earth's frame, where render space is fine enough to tell, seeing
    // Earth's satellites.
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(earth);
    run(&mut app, 60 + move_frames());

    // Its symbol sits on the Moon's Low orbit, round the Moon.
    let world = &app.world().resource::<WorldPositions>().0;
    let low_km = app.world().get::<NamedOrbit>(moon_low).unwrap().radius_km;
    let from_moon = world[&marlin].distance(world[&moon]);
    assert!(
        (from_moon - km_to_render(low_km)).abs() < km_to_render(low_km) * 0.01,
        "{from_moon} from the Moon, its Low orbit {}",
        km_to_render(low_km)
    );

    // GRP-01: seen from Earth, it folds into the Moon, and Earth's fold
    // marker doesn't count it.
    let fold = multi_marker(&app, MultiKey::Fold(moon)).expect("the Moon's fold marker");
    assert_eq!(fold.things, [marlin]);
    let earth_fold = multi_marker(&app, MultiKey::Fold(earth));
    assert!(earth_fold.is_none_or(|f| !f.things.contains(&marlin)));

    // INS-05: the outline moved it without being asked.
    let rows = outline_rows(&mut app);
    assert!(at(&rows, moon) < at(&rows, moon_low) && at(&rows, moon_low) < at(&rows, marlin));
    assert!(at(&rows, earth_low) < at(&rows, others[1]) && at(&rows, others[1]) < at(&rows, moon));
    assert_eq!(
        rows.iter().filter(|&&r| r == marlin).count(),
        1,
        "listed once"
    );
}

/// MRK-09 / HOV-02 / HOV-04 / RAIL-09: the Moon's Low orbit has no
/// members, so no ring and no hit target. Marlin arriving there gives it
/// a ring standing for Marlin, which draws Marlin's tail; hovering the
/// tail hovers Marlin and clicking it selects Marlin. Earth's Low orbit,
/// still shared by Perigee Station and Vestige, has no tail target for
/// Marlin any more.
#[test]
fn a_ship_arriving_in_an_empty_orbit_gets_a_ring_that_reaches_it() {
    let mut app = sol_app();
    let earth = body(&mut app, "Earth");
    let moon = body(&mut app, "Moon");
    let marlin = body(&mut app, "Marlin");
    let earth_low = band_orbit(&mut app, earth, Band::Low);
    let moon_low = band_orbit(&mut app, moon, Band::Low);
    let tail_on_earth = |app: &mut App| {
        let mut tails = app.world_mut().query::<&MemberTailTarget>();
        tails
            .iter(app.world())
            .any(|t| t.orbit == earth_low && t.body == marlin)
    };
    assert!(tail_on_earth(&mut app));
    let targets = ring_targets(&mut app);
    assert!(
        targets.iter().all(|t| t.1 != moon_low),
        "no ring, no target"
    );
    assert!(
        targets
            .iter()
            .any(|t| t.1 == earth_low && t.2 == Some(earth_low)),
        "Earth's Low orbit is shared"
    );

    fly_marlin_to_the_moon(&mut app, marlin, moon, false);
    let targets = ring_targets(&mut app);
    let on_moon: Vec<_> = targets.iter().filter(|t| t.1 == moon_low).collect();
    assert_eq!(on_moon.len(), 1);
    assert_eq!((on_moon[0].2, on_moon[0].3), (None, Some(marlin)));
    let target = on_moon[0].0;
    assert!(!tail_on_earth(&mut app), "the orbit it left loses it");

    // Close enough that the Moon's Low orbit unfolds and Marlin's tail is
    // drawn, and traced for hit-testing.
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(moon);
    run(&mut app, 60 + move_frames());
    zoom_in_until(&mut app, 30, |app| {
        app.world().resource::<RingPaths>().0.contains_key(&target)
    });
    let ring = app
        .world_mut()
        .run_system_once(move |rings: OrbitRings| rings.ring(moon_low).unwrap())
        .unwrap();
    let tails: Vec<Entity> = ring.tails().map(|m| m.body).collect();
    assert_eq!(tails, [marlin], "the ring draws Marlin's tail");

    // The far end of the tail, away from Marlin's own marker.
    let tail_end = app.world().resource::<RingPaths>().0[&target][0].0;
    move_cursor(&mut app, tail_end);
    run(&mut app, 3);
    assert!(
        hovered(&app).contains(&target),
        "the ring is under the pointer"
    );
    assert!(
        app.world()
            .resource::<crate::view::hovered::HoveredBodies>()
            .contains(marlin)
    );
    click_node(&mut app, target, 1);
    run(&mut app, 1);
    assert_eq!(app.world().resource::<Selection>().selected, Some(marlin));
}

/// INS-05 / RAIL-09: Marlin stays selected while it flies to the Moon.
/// The outline had opened its path at Earth; once it arrives, its new
/// path through the Moon and its Low orbit is opened too, so its row is
/// still listed.
#[test]
fn a_selected_ship_that_arrives_elsewhere_keeps_its_row_in_view() {
    let mut app = sol_app();
    let moon = body(&mut app, "Moon");
    let marlin = body(&mut app, "Marlin");
    let moon_low = band_orbit(&mut app, moon, Band::Low);
    fly_marlin_to_the_moon(&mut app, marlin, moon, true);
    assert_eq!(app.world().resource::<Selection>().selected, Some(marlin));
    let expanded = app.world().resource::<Expanded>();
    assert!(expanded.entries.contains(&moon) && expanded.orbits.contains(&moon_low));
    let rows = outline_rows(&mut app);
    let at = |e: Entity| rows.iter().position(|&r| r == e);
    assert!(at(moon) < at(moon_low) && at(moon_low) < at(marlin));
}

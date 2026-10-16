//! Headless tests of transfers on the map (`docs/spec/planner.md`
//! section 6): the planner's paths, ghosts and labels, orders on their
//! ships, ships in flight and the countdown in a ship's label, on Corvane
//! with a camera that projects.

use super::*;
use crate::sim::transfer::plan::ShipOrder;
use crate::sim::transfer::search::Request;
use crate::view::labels::LabelState;
use crate::view::layers::{BODIES, GHOSTS, PATH_DOTS};
use crate::view::markers::SecondLineFor;
use crate::view::paths::PathsPlugin;
use crate::view::paths::gather::{MapPaths, PathDraw};
use crate::view::paths::ghost::GhostLook;
use crate::view::paths::look::Emphasis;
use crate::view::paths::marks::{PathDot, PathGhost, PathLabel};
use crate::view::paths::strokes::DrawnPaths;
use crate::view::planner::PlannerPlugin;
use crate::view::planner::input::order_to_schedule;
use crate::view::planner_state::PickTarget;
use crate::view::world::ViewNow;

/// Corvane in Halden IV's frame, with markers, labels, the planner and
/// the paths, paused.
fn paths_app() -> App {
    let mut app = star_paths_app();
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());
    app
}

/// As `paths_app`, seeing the whole system from the star's frame.
fn star_paths_app() -> App {
    symbols_app_with(|app| {
        app.add_plugins((PlannerPlugin, PathsPlugin))
            .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>();
    })
}

/// Opens the planner for Orison and picks Halden IVa, as a click would.
fn plan_orison_to_iva(app: &mut App) -> (Entity, Entity) {
    plan_orison_to(app, "Halden IVa")
}

/// Opens the planner for Orison and picks `destination`.
fn plan_orison_to(app: &mut App, destination: &str) -> (Entity, Entity) {
    let orison = body(app, "Orison");
    let place = body(app, destination);
    app.world_mut().resource_mut::<Selection>().select(orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    run(app, 1);
    app.world_mut()
        .resource_mut::<PlannerState>()
        .pick(PickTarget::Body(place));
    run(app, 6);
    (orison, place)
}

/// Opens the planner for Orison and asks for an Intercept of Iver through
/// its High band; returns Orison.
fn plan_orison_to_intercept_iver(app: &mut App) -> Entity {
    let orison = body(app, "Orison");
    let iver = body(app, "Iver");
    app.world_mut().resource_mut::<Selection>().select(orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    run(app, 1);
    app.world_mut()
        .resource_mut::<PlannerState>()
        .session
        .as_mut()
        .unwrap()
        .pick(Request {
            destination: Place::Body {
                body: iver,
                band: Band::High,
            },
            arrival: Arrival::Intercept { band: Band::High },
        });
    run(app, 8);
    orison
}

/// The crossing's position on `day`, relative to its primary, in km.
fn crossing_at(leg: &SolvedLeg, day: f64) -> bevy::math::DVec3 {
    use orbits::Rail;
    leg.trajectory
        .segments()
        .iter()
        .find(|s| matches!(s.kind, SegmentKind::Conic { .. }))
        .expect("a crossing")
        .kind
        .state_at(day)
        .position
}

/// Dash pieces joined where one ends as the next starts on screen: a dash
/// can span two pieces of the line.
fn dashes_on_screen(
    app: &mut App,
    pieces: &[crate::view::paths::geometry::Piece],
) -> Vec<(Vec2, Vec2)> {
    let mut dashes: Vec<(Vec2, Vec2)> = Vec::new();
    for piece in pieces {
        let (Some(a), Some(b)) = (project(app, piece.from), project(app, piece.to)) else {
            continue;
        };
        match dashes.last_mut() {
            Some(last) if last.1.distance(a) < 0.01 => last.1 = b,
            _ => dashes.push((a, b)),
        }
    }
    dashes
}

/// Schedules the planner's order as Schedule does, a few depart steps on
/// so it stays Scheduled for a while.
fn schedule(app: &mut App, steps: i32) {
    if steps != 0 {
        crate::view::planner::input::step(
            &mut app.world_mut().resource_mut::<PlannerState>(),
            crate::view::planner_state::SliderKind::Depart,
            steps,
        );
        run(app, 4);
    }
    let (ship, order, _) =
        order_to_schedule(app.world().resource::<PlannerState>()).expect("an order to schedule");
    app.world_mut()
        .write_message(ShipOrder::Transfer { ship, order });
    app.world_mut().resource_mut::<PlannerState>().end();
    run(app, 3);
}

fn paths(app: &App) -> Vec<PathDraw> {
    app.world().resource::<MapPaths>().0.clone()
}

/// Moves the clock on by `days`, paused before and after.
fn skip_days(app: &mut App, days: f64) {
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    let rate = clock.rate_days_per_sec();
    clock.advance(days / rate);
    clock.toggle_pause();
    run(app, 3);
}

/// Where a world point lands on screen this frame.
fn project(app: &mut App, world: Vec3) -> Option<Vec2> {
    let mut state = bevy::ecs::system::SystemState::<ViewNow>::new(app.world_mut());
    state.get(app.world()).ok()?.project(world)
}

/// The shown nodes carrying `T`, with their node and layer.
fn shown_nodes<T: Component>(app: &mut App) -> Vec<(Entity, Node, i32)> {
    let mut query = app
        .world_mut()
        .query_filtered::<(Entity, &Node, &GlobalZIndex), With<T>>();
    query
        .iter(app.world())
        .filter(|(_, node, _)| node.display != Display::None)
        .map(|(e, node, z)| (e, node.clone(), z.0))
        .collect()
}

fn node_centre(node: &Node) -> Vec2 {
    let (Val::Px(l), Val::Px(t), Val::Px(w), Val::Px(h)) =
        (node.left, node.top, node.width, node.height)
    else {
        panic!("node not placed in pixels");
    };
    Vec2::new(l + w / 2.0, t + h / 2.0)
}

/// PLNR-60 / PLNR-62 / PLNR-63: while tuning, the leg of the current
/// setting is drawn in full: it starts where Orison will be on the day
/// its crossing starts and ends where Halden IVa will be on the day it
/// ends, each point placed round Halden IV where it is now, not where
/// either is today. A ghost of Orison marks the start and a ghost of
/// Halden IVa the end.
#[test]
fn a_planned_path_runs_from_the_ship_to_where_its_destination_will_be() {
    let mut app = paths_app();
    let (orison, iva) = plan_orison_to_iva(&mut app);
    let leg = app
        .world()
        .resource::<PlannerState>()
        .session
        .as_ref()
        .unwrap()
        .solved_leg()
        .cloned()
        .expect("a solved leg");
    let drawn = paths(&app);
    assert_eq!(drawn.len(), 1, "the planned path only");
    let path = &drawn[0];
    assert_eq!(path.ship, orison);
    assert_eq!(path.emphasis, Emphasis::Full);
    assert_eq!(path.departure_ghost.as_ref().unwrap().object, orison);
    assert_eq!(path.arrival_ghost.as_ref().unwrap().object, iva);

    let halden = body(&mut app, "Halden IV");
    let halden_now = app.world().resource::<WorldPositions>().0[&halden];
    let at = |day: f64| halden_now + km_to_render_vec3(crossing_at(&leg, day));
    let laid = app.world().resource::<DrawnPaths>().0[0].clone();
    let start = laid.start.unwrap();
    let end = laid.end.unwrap();
    assert!(start.distance(at(leg.crossing_start_day)) < 1e-4);
    assert!(end.distance(at(leg.crossing_end_day)) < 1e-4);
    // The end is where Halden IVa will be, not where it is now.
    let iva_orbit = *app.world().get::<Orbit>(iva).unwrap();
    let iva_then =
        halden_now + km_to_render_vec3(orbit_position_km(&iva_orbit, leg.crossing_end_day));
    let iva_now = app.world().resource::<WorldPositions>().0[&iva];
    let tolerance = km_to_render(5_000.0);
    assert!(end.distance(iva_then) < tolerance, "ends at the moon then");
    assert!(
        end.distance(iva_now) > 10.0 * tolerance,
        "not where it is today"
    );
}

/// PLNR-61 / PLNR-63 / PLNR-69: the part to come is dashed on screen,
/// `path_dash_px` on and `path_gap_px` off; a dot `departure_dot_px`
/// across marks the start, under every marker; the ghosts are dashed
/// circles of their markers' sizes in the neutral colour at `ghost_alpha`,
/// in the bodies' layer beneath every real marker; none takes a click.
#[test]
fn a_planned_path_is_dashed_with_a_start_dot_and_ghosts() {
    let mut app = paths_app();
    plan_orison_to_iva(&mut app);
    let tunables = app.world().resource::<Tunables>().clone();
    let laid = app.world().resource::<DrawnPaths>().0[0].clone();
    assert!(laid.flown.is_empty(), "nothing flown yet");
    assert!(laid.ahead.len() > 3);
    let dashes = dashes_on_screen(&mut app, &laid.ahead);
    assert!(dashes.len() > 3);
    for pair in dashes.windows(2).take(dashes.len() - 2) {
        let ((start, end), (next, _)) = (pair[0], pair[1]);
        // Measured along the curve, so a dash's chord is a hair shorter.
        let len = start.distance(end);
        assert!((len - tunables.path_dash_px).abs() < 0.1, "dash {len}");
        let gap = end.distance(next);
        assert!((gap - tunables.path_gap_px).abs() < 0.1, "gap {gap}");
    }

    let dots = shown_nodes::<PathDot>(&mut app);
    assert_eq!(dots.len(), 1);
    let (dot, node, layer) = &dots[0];
    assert_eq!(*layer, PATH_DOTS);
    const { assert!(PATH_DOTS < BODIES) };
    assert_eq!(node.width, px(tunables.departure_dot_px));
    let start = project(&mut app, laid.start.unwrap()).unwrap();
    assert!(node_centre(node).distance(start) < 0.01);
    assert_eq!(app.world().get::<Pickable>(*dot), Some(&Pickable::IGNORE));

    let ghosts = shown_nodes::<PathGhost>(&mut app);
    assert_eq!(ghosts.len(), 2);
    let mut sizes: Vec<f32> = Vec::new();
    for (ghost, _, layer) in &ghosts {
        assert_eq!(*layer, GHOSTS);
        const { assert!(GHOSTS < BODIES) };
        let look = *app.world().get::<GhostLook>(*ghost).unwrap();
        assert!((look.color.alpha() - tunables.ghost_alpha).abs() < 1e-6);
        assert_eq!(look.width_px, tunables.ghost_width_px);
        assert_eq!(look.dash_px, tunables.ghost_dash_px);
        assert_eq!(app.world().get::<Pickable>(*ghost), Some(&Pickable::IGNORE));
        sizes.push(look.size_px);
    }
    sizes.sort_by(f32::total_cmp);
    let mut want = [tunables.moon_marker_px, tunables.ship_symbol_px];
    want.sort_by(f32::total_cmp);
    assert_eq!(sizes, want);
}

/// PLNR-65: the arrival ghost's label reads "arrive *date*" for a
/// Rendezvous and is placed with the other labels, beside the ghost.
#[test]
fn the_arrival_ghost_is_labelled_with_its_date() {
    let mut app = paths_app();
    plan_orison_to_iva(&mut app);
    let path = paths(&app).remove(0);
    let label = path.arrival_ghost.unwrap().label.unwrap();
    assert!(label.starts_with("arrive "), "{label}");
    run(&mut app, 3);
    let mut labels = app
        .world_mut()
        .query::<(&PathLabel, &LabelState, &Children)>();
    let placed: Vec<(PathLabel, LabelState, Entity)> = labels
        .iter(app.world())
        .filter(|(p, ..)| p.anchor.is_some())
        .map(|(p, s, c)| (*p, *s, c[0]))
        .collect();
    assert_eq!(placed.len(), 1);
    let (_, state, text) = placed[0];
    assert!(state.shown.is_some(), "a free label is placed");
    assert_eq!(app.world().get::<Text>(text).unwrap().0, label);
}

/// PLNR-12 / PLNR-62: while choosing, hovering a place draws its Fastest
/// path with both ghosts; nothing else is drawn.
#[test]
fn hovering_a_place_while_choosing_draws_its_fastest_path() {
    let mut app = paths_app();
    let orison = body(&mut app, "Orison");
    let ivb = body(&mut app, "Halden IVb");
    app.world_mut().resource_mut::<Selection>().select(orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    run(&mut app, 2);
    assert!(paths(&app).is_empty(), "nothing hovered, nothing drawn");
    let marker = marker_of(&mut app, ivb);
    app.world_mut().entity_mut(marker).insert(Hovered);
    run(&mut app, 6);
    let drawn = paths(&app);
    assert_eq!(drawn.len(), 1);
    assert_eq!(drawn[0].arrival_ghost.as_ref().unwrap().object, ivb);
    assert!(drawn[0].departure_ghost.is_some());
}

/// PLNR-62 / PLNR-66 / SYM-13: a Scheduled order draws nothing until its
/// ship is selected, then its path and arrival ghost, with no departure
/// ghost. Its ship's label ends with "departs in *N*", counting down.
#[test]
fn a_scheduled_order_shows_when_its_ship_is_selected() {
    let mut app = paths_app();
    let (orison, iva) = plan_orison_to_iva(&mut app);
    schedule(&mut app, 30);
    let plan = app.world().get::<Plan>(orison).unwrap().clone();
    assert_eq!(plan.status, Status::Scheduled);
    let drawn = paths(&app);
    assert_eq!(drawn.len(), 1, "selected: drawn");
    assert_eq!(drawn[0].emphasis, Emphasis::Full);
    assert!(drawn[0].departure_ghost.is_none());
    assert_eq!(drawn[0].arrival_ghost.as_ref().unwrap().object, iva);

    let line = second_line(&mut app, orison);
    assert!(line.starts_with("departs in "), "{line}");
    skip_days(&mut app, 0.1);
    let later = second_line(&mut app, orison);
    assert!(later.starts_with("departs in ") && later != line, "{later}");

    app.world_mut().resource_mut::<Selection>().clear();
    run(&mut app, 2);
    assert!(paths(&app).is_empty(), "not selected: only the label");
}

/// The text of a ship's label's second line.
fn second_line(app: &mut App, ship: Entity) -> String {
    let mut lines = app.world_mut().query::<(&SecondLineFor, &Text)>();
    lines
        .iter(app.world())
        .find(|(line, _)| line.0 == ship)
        .map(|(_, text)| text.0.clone())
        .expect("a second line")
}

/// PLNR-62 / PLNR-66 / PLNR-68 / GRP-08 / MOT-08: once Orison has left
/// for Iver, its path always shows: faint when it is neither hovered nor
/// selected, solid where flown and dashed ahead. Orison is drawn as its
/// symbol at its rail position, never folded and never an exposure ring,
/// even at a rate that turns the ships parked round Halden IV into rings,
/// and its label counts down to its arrival.
#[test]
fn a_ship_in_flight_shows_its_path_and_is_drawn_as_a_dot() {
    let mut app = star_paths_app();
    let orison = plan_orison_to_intercept_iver(&mut app);
    schedule(&mut app, 0);
    let leave = app.world().get::<Plan>(orison).unwrap().leg().leave_day();
    let crossing = app
        .world()
        .get::<Plan>(orison)
        .unwrap()
        .leg()
        .crossing_end_day;
    let today = app.world().resource::<GameClock>().elapsed_days();
    skip_days(&mut app, (leave - today) + (crossing - leave) * 0.4);
    assert!(app.world().get::<InFlight>(orison).is_some());

    app.world_mut().resource_mut::<Selection>().clear();
    run(&mut app, 3);
    let drawn = paths(&app);
    assert_eq!(drawn.len(), 1);
    assert_eq!(drawn[0].emphasis, Emphasis::Traffic);
    assert!(drawn[0].arrival_ghost.is_none());
    let laid = app.world().resource::<DrawnPaths>().0[0].clone();
    assert!(!laid.flown.is_empty() && !laid.ahead.is_empty());

    let line = second_line(&mut app, orison);
    assert!(line.starts_with("intercepts Iver in "), "{line}");

    // GRP-08 / MOT-08: never folded, never a ring; a parked neighbour is.
    app.world_mut().resource_mut::<GameClock>().run_at(64.0);
    run(&mut app, eased_frames());
    assert!(app.world().get::<SatelliteFoldState>(orison).is_none());
    assert!(app.world().get::<MotionModeState>(orison).is_none());
    let ardent = body(&mut app, "Ardent");
    assert!(
        app.world()
            .get::<MotionModeState>(ardent)
            .unwrap()
            .ring_mode
    );
    // PLNR-68: drawn at its rail position, in full.
    let marker = marker_of(&mut app, orison);
    let node = app.world().get::<Node>(marker).unwrap().clone();
    assert_ne!(node.display, Display::None);
    let screen = app
        .world()
        .resource::<ScreenPositions>()
        .get(orison)
        .unwrap();
    assert!(node_centre(&node).distance(screen) < 0.5);
}

/// PLNR-62: a ship in flight's path is full while it is hovered, and
/// gains its arrival ghost while it is selected.
#[test]
fn a_ship_in_flight_is_drawn_in_full_while_hovered_or_selected() {
    let mut app = paths_app();
    let (orison, iva) = plan_orison_to_iva(&mut app);
    schedule(&mut app, 0);
    skip_days(&mut app, 0.1);
    let drawn = paths(&app);
    assert_eq!(drawn[0].emphasis, Emphasis::Full, "selected");
    assert_eq!(drawn[0].arrival_ghost.as_ref().unwrap().object, iva);
    app.world_mut().resource_mut::<Selection>().clear();
    let marker = marker_of(&mut app, orison);
    app.world_mut().entity_mut(marker).insert(Hovered);
    run(&mut app, 3);
    let drawn = paths(&app);
    assert_eq!(drawn[0].emphasis, Emphasis::Full, "hovered");
    assert!(drawn[0].arrival_ghost.is_none());
}

/// PLNR-67 / PLNR-65: an Intercept of Iver from Halden IV coasts on round
/// the star; its coast is drawn dotted up to its event, `consequence_dot_px`
/// on and `consequence_gap_px` off, labelled "then coasting · ...", and
/// the arrival label gives the intercept's date and speed.
#[test]
fn an_intercept_draws_its_coast_up_to_its_event() {
    let mut app = star_paths_app();
    plan_orison_to_intercept_iver(&mut app);
    let drawn = paths(&app);
    assert_eq!(drawn.len(), 1);
    let label = drawn[0]
        .arrival_ghost
        .as_ref()
        .unwrap()
        .label
        .clone()
        .unwrap();
    assert!(
        label.starts_with("intercept ") && label.contains("km/s"),
        "{label}"
    );
    let coast = drawn[0].coast.clone().expect("a coast with an event");
    assert!(
        coast.label.starts_with("then coasting · "),
        "{}",
        coast.label
    );
    let laid = app.world().resource::<DrawnPaths>().0[0].clone();
    assert!(!laid.coast.is_empty());
    let tunables = app.world().resource::<Tunables>().clone();
    let dots = dashes_on_screen(&mut app, &laid.coast);
    assert!(dots.len() > 3);
    for pair in dots.windows(2).take(dots.len() - 2) {
        let ((start, end), (next, _)) = (pair[0], pair[1]);
        let len = start.distance(end);
        assert!((len - tunables.consequence_dot_px).abs() < 0.1, "dot {len}");
        let gap = end.distance(next);
        assert!((gap - tunables.consequence_gap_px).abs() < 0.1, "gap {gap}");
    }
}

/// The leader holder of `body`'s symbol: whether it shows, its clockwise
/// angle from screen up, and its bar's top and height in pixels.
fn leader_of(app: &mut App, body: Entity) -> (bool, f32, f32, f32) {
    use crate::view::leaders::{Leader, LeaderBar};
    let mut holders = app
        .world_mut()
        .query::<(&Leader, &Node, &UiTransform, &Children)>();
    let (shown, angle, bar) = holders
        .iter(app.world())
        .find(|(leader, ..)| leader.body == body)
        .map(|(_, node, transform, children)| {
            (
                node.display != Display::None,
                transform.rotation.as_radians(),
                children[0],
            )
        })
        .expect("a leader");
    assert!(app.world().get::<LeaderBar>(bar).is_some());
    let node = app.world().get::<Node>(bar).unwrap();
    let (Val::Px(top), Val::Px(height)) = (node.top, node.height) else {
        return (shown, angle, 0.0, 0.0);
    };
    (shown, angle, top, height)
}

/// SYM-11: Orison in flight to Iver has a leader that starts at the edge
/// of its circle frame, points the way its symbol moves across the screen
/// and is as long as its speed relative to the star says; a parked ship
/// has none, and nor has a stale track.
#[test]
fn a_ship_in_flight_has_a_velocity_leader_the_way_it_moves() {
    use crate::sim::rail::km_per_day_to_km_per_s;
    use crate::sim::track::Track;
    use crate::sim::velocity::FlightVelocities;
    use crate::view::leaders::{LeadersPlugin, leader_length_px};
    use crate::view::screen::screen_angle;

    let mut app = symbols_app_with(|app| {
        app.add_plugins((PlannerPlugin, PathsPlugin, LeadersPlugin))
            .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>();
    });
    let orison = plan_orison_to_intercept_iver(&mut app);
    // Parked before it leaves: no leader.
    assert!(!leader_of(&mut app, orison).0);
    schedule(&mut app, 0);
    let leave = app.world().get::<Plan>(orison).unwrap().leg().leave_day();
    let crossing = app
        .world()
        .get::<Plan>(orison)
        .unwrap()
        .leg()
        .crossing_end_day;
    let today = app.world().resource::<GameClock>().elapsed_days();
    skip_days(&mut app, (leave - today) + (crossing - leave) * 0.4);
    assert!(app.world().get::<InFlight>(orison).is_some());
    app.world_mut().resource_mut::<Selection>().clear();
    run(&mut app, 3);

    let (shown, angle, top, height) = leader_of(&mut app, orison);
    assert!(shown);
    // Its length is its speed relative to the star's.
    let mut state = bevy::ecs::system::SystemState::<FlightVelocities>::new(app.world_mut());
    let velocity = state.get(app.world()).unwrap().of(orison).unwrap();
    let speed = km_per_day_to_km_per_s(velocity.length()) as f32;
    let tunables = app.world().resource::<Tunables>().clone();
    assert!((height - leader_length_px(speed, &tunables).unwrap()).abs() < 1e-3);
    // It hangs from the frame's edge: an own ship's circle reaches the
    // edge of its square, so the bar ends at the holder's top.
    assert!((top + height).abs() < 1e-3, "{top} + {height}");

    // It points the way the ship moves on screen over a short time (the
    // unsnapped projection: a whole-pixel symbol moves in steps).
    let world = |app: &App| app.world().resource::<WorldPositions>().0[&orison];
    let start = world(&app);
    skip_days(&mut app, 2.0);
    let end = world(&app);
    let (before, after) = (
        project(&mut app, start).unwrap(),
        project(&mut app, end).unwrap(),
    );
    assert!(before.distance(after) > 0.5, "the ship moved on screen");
    let moved = screen_angle(before, after);
    let off = Rot2::radians(moved - angle).as_radians().abs();
    assert!(off < 0.05, "leader at {angle}, motion at {moved}");

    // A parked ship has none.
    let ardent = body(&mut app, "Ardent");
    assert!(!leader_of(&mut app, ardent).0);

    // Nor has a stale track.
    app.world_mut().get_mut::<ShipRecord>(orison).unwrap().track = Track::Stale {
        last_observed_day: 0.0,
    };
    run(&mut app, 2);
    assert!(!leader_of(&mut app, orison).0);
}

/// LBL-05: every label on the map carries the ground-colour halo,
/// `label_halo_px` wide: body and symbol labels with their second lines,
/// exposure ring labels with their period lines, and the planner's path
/// labels.
#[test]
fn every_map_label_carries_the_ground_halo() {
    use crate::view::colors;
    use crate::view::halo::{HaloPlugin, TextHalo};
    use crate::view::markers::MarkerLabel;
    use crate::view::ring_marks::{RingLabelBox, RingMarksPlugin};

    let mut app = symbols_app_with(|app| {
        app.add_plugins((
            PlannerPlugin,
            PathsPlugin,
            SharedOrbitPlugin,
            RingMarksPlugin,
            HaloPlugin,
        ))
        .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>();
    });
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());
    plan_orison_to_iva(&mut app);
    run(&mut app, 3);

    let want = TextHalo {
        width_px: Tunables::default().label_halo_px,
        color: colors::GROUND,
    };
    let mut roots = app.world_mut().query_filtered::<(
        Entity,
        Has<MarkerLabel>,
        Has<RingLabelBox>,
    ), Or<(With<MarkerLabel>, With<RingLabelBox>, With<PathLabel>)>>();
    let roots: Vec<(Entity, bool, bool)> = roots.iter(app.world()).collect();
    let (mut marker, mut ring, mut path) = (0, 0, 0);
    for (root, is_marker, is_ring) in roots {
        for text in descendants(&mut app, root) {
            if app.world().get::<Text>(text).is_none() {
                continue;
            }
            assert_eq!(
                app.world().get::<TextHalo>(text),
                Some(&want),
                "a label's text without the halo"
            );
            match (is_marker, is_ring) {
                (true, _) => marker += 1,
                (_, true) => ring += 1,
                _ => path += 1,
            }
        }
    }
    assert!(marker > 0 && ring > 0 && path > 0, "{marker} {ring} {path}");
}

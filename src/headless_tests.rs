//! Headless integration tests (ARCH-13, ARCH-31): real plugins in an `App`
//! with `MinimalPlugins`, no window and no GPU.
//!
//! The view's grouping needs a camera that can project, which normally
//! takes the renderer; `test_camera` fills in the camera's computed values
//! by hand, as Bevy's own camera tests do, so the real grouping systems run
//! unchanged. Covers SCL-04, GRP-01, GRP-02, GRP-04, MOT-01 to MOT-04 and
//! ZOOM-15 against Corvane.

use std::time::Duration;

use bevy::camera::RenderTargetInfo;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::WindowResolution;

use crate::content::ContentPlugin;
use crate::sim::SimPlugin;
use crate::sim::body::DisplayName;
use crate::sim::clock::GameClock;
use crate::sim::orbit::{Orbit, OrbitPositionKm, orbit_position_km};
use crate::sim::units::KM_PER_AU;
use crate::view::camera::stops::{StopsPlugin, ZoomStops, distance_for_radius};
use crate::view::edge::{EdgePlugin, GateEdgeTargets};
use crate::view::fold::SatelliteFoldState;
use crate::view::fonts::Fonts;
use crate::view::grouping::GroupingPlugin;
use crate::view::motion::MotionPlugin;
use crate::view::motion_mode::MotionModeState;
use crate::view::projection::ProjectionPlugin;
use crate::view::scale::km_to_render;
use crate::view::screen::{ScreenPlugin, ScreenPositions};
use crate::view::selection::Selection;
use crate::view::tunables::{Tunables, TunablesPlugin};
use crate::{Boot, Stage};

/// One frame at 60 fps.
const FRAME: Duration = Duration::from_micros(16_667);
/// Enough frames for transforms to propagate and a 200 ms fold or
/// cross-fade to finish.
const SETTLE_FRAMES: usize = 30;
const VIEWPORT: UVec2 = UVec2::new(1600, 900);
/// The camera's default steep tilt (see `camera/state.rs`).
const PITCH: f32 = 1.15;

/// A headless app with the same set ordering as `main`, stepping time by a
/// fixed frame so results don't depend on how fast the test runs.
fn headless_app(frame: Duration) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(TransformPlugin)
        .insert_resource(TimeUpdateStrategy::ManualDuration(frame))
        .configure_sets(Startup, (Boot::World, Boot::Derive, Boot::Overlay).chain())
        .configure_sets(
            Update,
            (
                Stage::Simulation,
                Stage::Camera,
                Stage::Projection,
                Stage::Grouping,
                Stage::Motion,
                Stage::Screen,
                Stage::Layout,
                Stage::Overlay,
            )
                .chain(),
        );
    app
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn body(app: &mut App, name: &str) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &DisplayName)>();
    query
        .iter(app.world())
        .find(|(_, display)| display.0 == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("{name} missing"))
}

/// A camera that can project without a renderer: its target size and
/// projection matrix filled in as the render world would.
fn test_camera(target: Vec3, distance: f32) -> (Camera, Projection, Transform) {
    let mut projection = Projection::Perspective(PerspectiveProjection::default());
    projection.update(VIEWPORT.x as f32, VIEWPORT.y as f32);
    let mut camera = Camera::default();
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: VIEWPORT,
        scale_factor: 1.0,
    });
    camera.computed.clip_from_view = projection.get_clip_from_view();
    let offset = Vec3::new(0.0, distance * PITCH.sin(), distance * PITCH.cos());
    let transform = Transform::from_translation(target + offset).looking_at(target, Vec3::Y);
    (camera, projection, transform)
}

/// Font handles for plugins that spawn text; nothing is rendered.
fn placeholder_fonts() -> Fonts {
    Fonts {
        label: Handle::default(),
        label_semibold: Handle::default(),
        mono: Handle::default(),
    }
}

fn fov() -> f32 {
    PerspectiveProjection::default().fov
}

/// ARCH-13: the simulation runs headless with `MinimalPlugins` and the sim
/// plugin alone - with no content it simply starts empty.
#[test]
fn arch_13_simulation_runs_headless() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SimPlugin));
    run(&mut app, 5);
    assert!(app.world().get_resource::<GameClock>().is_some());

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ContentPlugin, SimPlugin));
    run(&mut app, 5);
    let mut orbits = app.world_mut().query::<&Orbit>();
    assert!(orbits.iter(app.world()).count() > 20, "Corvane spawned");
}

/// ARCH-11 / ARCH-31 / SCL-04: after a known time, every body is where its
/// orbit says, in km, read from the sim component with no `Transform`.
#[test]
fn arch_31_orbital_positions_after_a_known_time() {
    // Tenth-second frames: Bevy's virtual time caps a frame at 250 ms.
    let mut app = headless_app(Duration::from_millis(100));
    app.add_plugins((ContentPlugin, SimPlugin));
    run(&mut app, 101);

    let elapsed = app.world().resource::<GameClock>().elapsed_days();
    // One day per second at the default rate; the very first update has no
    // previous frame to measure from.
    assert!(
        (9.9..=10.1).contains(&elapsed),
        "{elapsed} days after 101 tenth-second frames"
    );

    let mut bodies = app.world_mut().query::<(&Orbit, &OrbitPositionKm)>();
    for (orbit, position) in bodies.iter(app.world()) {
        let expected = orbit_position_km(orbit, elapsed);
        assert!(
            (position.0 - expected).length() < 1e-3,
            "a body is {} km off its orbit",
            (position.0 - expected).length()
        );
    }

    // And in absolute terms: Halden IV is on its 2 AU orbit, and Arden
    // Highport (0.1 d) has gone round about 100 times but is still 9,394 km
    // from its planet.
    let halden = body(&mut app, "Halden IV");
    let halden_km = app.world().get::<OrbitPositionKm>(halden).unwrap().0;
    assert!((halden_km.length() / KM_PER_AU - 2.0).abs() < 1e-9);
    let highport = body(&mut app, "Arden Highport");
    let highport_km = app.world().get::<OrbitPositionKm>(highport).unwrap().0;
    assert!((highport_km.length() - 9_394.0).abs() < 1e-6);
}

/// ARCH-31 / GRP-01 / GRP-02 / GRP-04: fold state follows the scale. At the
/// system frame every satellite is folded; at Halden IV's satellite frame
/// its outer moons unfold while the close-in station stays folded; and back
/// out, everything folds again.
#[test]
fn arch_31_fold_state_after_a_scale_change() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin)).add_plugins((
        TunablesPlugin,
        ProjectionPlugin,
        StopsPlugin,
        GroupingPlugin,
    ));
    run(&mut app, 2);
    // Hold everything still so the camera stays on its target.
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, 2);

    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let system_radius = app.world().resource::<ZoomStops>().system_radius;
    let system_distance = distance_for_radius(system_radius, fov(), PITCH, margin);
    let halden = body(&mut app, "Halden IV");
    let satellite_radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let satellite_distance = distance_for_radius(satellite_radius, fov(), PITCH, margin);
    let halden_pos = app
        .world()
        .get::<GlobalTransform>(halden)
        .unwrap()
        .translation();

    let camera = app
        .world_mut()
        .spawn(test_camera(Vec3::ZERO, system_distance))
        .id();
    run(&mut app, SETTLE_FRAMES);
    let folded = |app: &mut App, name: &str| {
        let entity = body(app, name);
        app.world()
            .get::<SatelliteFoldState>(entity)
            .unwrap()
            .folded
    };
    let mut satellites = app.world_mut().query::<&SatelliteFoldState>();
    assert!(
        satellites.iter(app.world()).all(|fold| fold.folded),
        "GRP-04: everything folded at the system frame"
    );

    // Zoom in to Halden IV's satellite frame.
    let (_, _, close) = test_camera(halden_pos, satellite_distance);
    *app.world_mut().get_mut::<Transform>(camera).unwrap() = close;
    run(&mut app, SETTLE_FRAMES);
    assert!(
        !folded(&mut app, "Halden IVd"),
        "the outermost moon unfolds"
    );
    assert!(!folded(&mut app, "Halden IVc"));
    assert!(
        folded(&mut app, "Arden Highport"),
        "GRP-02: inner ones later"
    );
    assert!(folded(&mut app, "Iver Yard"), "other planets stay folded");

    // And back out.
    let (_, _, far) = test_camera(Vec3::ZERO, system_distance);
    *app.world_mut().get_mut::<Transform>(camera).unwrap() = far;
    run(&mut app, SETTLE_FRAMES);
    let mut satellites = app.world_mut().query::<&SatelliteFoldState>();
    assert!(
        satellites
            .iter(app.world())
            .all(|fold| fold.folded && fold.effective_hidden)
    );
}

/// ARCH-31 / MOT-01 to MOT-04: the motion mode follows the time rate.
#[test]
fn arch_31_motion_mode_after_a_rate_change() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, MotionPlugin))
        // Shared view state the ring labels read; `ViewPlugin` owns it.
        .init_resource::<Selection>()
        .insert_resource(placeholder_fonts());
    let ring = |app: &mut App, name: &str| {
        let entity = body(app, name);
        app.world()
            .get::<MotionModeState>(entity)
            .unwrap()
            .ring_mode
    };

    run(&mut app, SETTLE_FRAMES);
    // TEST-02 at 1 d/s: IVa (1.5 s) is a ring, IVc (31 s) a dot.
    assert!(ring(&mut app, "Halden IVa"));
    assert!(!ring(&mut app, "Halden IVc"));

    // At 5 d/s IVc's apparent period is 6.2 s: a ring.
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);
    run(&mut app, SETTLE_FRAMES);
    assert!(ring(&mut app, "Halden IVc"));
    let ivc = body(&mut app, "Halden IVc");
    let state = app.world().get::<MotionModeState>(ivc).unwrap();
    assert_eq!(
        state.crossfade_t, 1.0,
        "MOT-03: the cross-fade has finished"
    );

    // MOT-04: paused, everything is a dot.
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, SETTLE_FRAMES);
    let mut modes = app.world_mut().query::<&MotionModeState>();
    assert!(modes.iter(app.world()).all(|mode| !mode.ring_mode));
}

/// ARCH-31 / ZOOM-15: the stops come from Corvane's real orbits.
#[test]
fn arch_31_zoom_stops_for_corvane() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin, StopsPlugin));
    run(&mut app, 1);

    let close = |a: f32, b: f32| (a - b).abs() < b * 1e-5;
    let stops = app.world().resource::<ZoomStops>();
    // System frame: Morrow at 4.3 AU. Region frame: Halden IV at 2 AU.
    assert!(close(stops.system_radius, km_to_render(4.3 * KM_PER_AU)));
    assert!(close(
        stops.inner_system_radius,
        km_to_render(2.0 * KM_PER_AU)
    ));
    let satellite_frames = stops.per_body_satellite_radius.clone();

    for (planet, outermost_km) in [
        ("Halden IV", 726_432.0),
        ("Iver", 45_000.0),
        ("Morrow", 290_000.0),
    ] {
        let entity = body(&mut app, planet);
        let radius = satellite_frames[&entity];
        assert!(
            close(radius, km_to_render(outermost_km)),
            "{planet}'s satellite frame is its outermost orbit"
        );
    }
    let pell = body(&mut app, "Pell");
    assert!(
        !satellite_frames.contains_key(&pell),
        "Pell has no satellites"
    );
}

/// EDGE-01 (F4-08): zoomed in and tilted, every gate still has an arrow on
/// the screen edge - including gates behind the camera, which have no
/// projected position at all.
#[test]
fn edge_01_every_gate_has_an_arrow_when_zoomed_in() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, ProjectionPlugin, StopsPlugin))
        .add_plugins((ScreenPlugin, EdgePlugin))
        .insert_resource(placeholder_fonts());
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(VIEWPORT.x, VIEWPORT.y),
        ..default()
    });
    run(&mut app, 2);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, 2);

    let halden = body(&mut app, "Halden IV");
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let distance = distance_for_radius(radius, fov(), PITCH, margin);
    let halden_pos = app
        .world()
        .get::<GlobalTransform>(halden)
        .unwrap()
        .translation();
    app.world_mut().spawn(test_camera(halden_pos, distance));
    run(&mut app, 4);

    let gates: Vec<Entity> = ["Gate Ostra", "Gate Brenn", "Gate Soll"]
        .into_iter()
        .map(|name| body(&mut app, name))
        .collect();
    let screen = app.world().resource::<ScreenPositions>();
    let pinned = app.world().resource::<GateEdgeTargets>();
    for gate in gates {
        let on_screen = screen.get(gate).is_some_and(|pos| screen.in_viewport(pos));
        assert!(
            on_screen || pinned.is_pinned(gate),
            "a gate has neither a marker nor an edge arrow"
        );
    }
}

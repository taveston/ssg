//! Headless integration tests (ARCH-13, ARCH-31): real plugins in an `App`
//! with `MinimalPlugins`, no window and no GPU.
//!
//! The view's grouping needs a camera that can project, which normally
//! takes the renderer; `test_camera` fills in the camera's computed values
//! by hand, as Bevy's own camera tests do, so the real grouping systems run
//! unchanged. Input is simulated as messages (keys, mouse buttons, wheel,
//! motion), so camera tests drive the real handlers. Each test's doc line
//! names the IDs it covers.

use std::time::Duration;

use bevy::camera::RenderTargetInfo;
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::WindowResolution;

use crate::content::ContentPlugin;
use crate::sim::SimPlugin;
use crate::sim::body::{BodyClass, DisplayName, Knowledge, Ownership, PhysicalRadius};
use crate::sim::clock::GameClock;
use crate::sim::orbit::{Orbit, OrbitPositionKm, orbit_position_km};
use crate::sim::system::{
    BeltBlueprint, BodyBlueprint, GateBlueprint, SystemData, SystemDataResource,
};
use crate::sim::units::{EARTH_MASS_KG, KM_PER_AU, SOLAR_MASS_KG};
use crate::view::bindings::BindingsPlugin;
use crate::view::camera::CameraPlugin;
use crate::view::camera::requests::FocusRequest;
use crate::view::camera::state::{ActiveMove, CameraRig, OrbitCamera};
use crate::view::camera::stops::{StopsPlugin, ZoomStops, distance_for_radius};
use crate::view::edge::{EdgePlugin, GateEdgeTargets};
use crate::view::focus::Focus;
use crate::view::fold::SatelliteFoldState;
use crate::view::fonts::Fonts;
use crate::view::gesture::PressStart;
use crate::view::grouping::GroupingPlugin;
use crate::view::marker_images::MarkerImages;
use crate::view::markers::{MarkersPlugin, ring_outer_half};
use crate::view::motion::MotionPlugin;
use crate::view::motion_mode::MotionModeState;
use crate::view::pills::PillsPlugin;
use crate::view::projection::ProjectionPlugin;
use crate::view::scale::km_to_render;
use crate::view::screen::{ScreenPlugin, ScreenPositions};
use crate::view::selection::Selection;
use crate::view::targets::{GroupPillFor, MarkerFor};
use crate::view::topbar::TOP_BAR_HEIGHT_PX;
use crate::view::true_size::projected_radius_px;
use crate::view::tunables::{Tunables, TunablesPlugin};
use crate::view::world::WorldPositions;
use crate::{Boot, Stage};

/// One frame at 60 fps.
const FRAME: Duration = Duration::from_micros(16_667);
/// Enough frames for transforms to propagate and a 200 ms fold or
/// cross-fade to finish.
const SETTLE_FRAMES: usize = 30;
const VIEWPORT: UVec2 = UVec2::new(1600, 900);
/// A steep tilt from straight down, so projections are checked off the
/// vertical.
const TILT: f32 = 0.42;

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
                Stage::Projection,
                Stage::Camera,
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
fn test_camera(target: Vec3, distance: f32) -> (Camera3d, Camera, Projection, Transform) {
    let mut projection = Projection::Perspective(PerspectiveProjection::default());
    projection.update(VIEWPORT.x as f32, VIEWPORT.y as f32);
    let mut camera = Camera::default();
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: VIEWPORT,
        scale_factor: 1.0,
    });
    camera.computed.clip_from_view = projection.get_clip_from_view();
    let elevation = std::f32::consts::FRAC_PI_2 - TILT;
    let offset = Vec3::new(0.0, distance * elevation.sin(), distance * elevation.cos());
    let transform = Transform::from_translation(target + offset).looking_at(target, Vec3::Y);
    (Camera3d::default(), camera, projection, transform)
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
fn simulation_runs_headless() {
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
fn orbital_positions_after_a_known_time() {
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
fn fold_state_after_a_scale_change() {
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
    let system_distance = distance_for_radius(system_radius, fov(), TILT, margin);
    let halden = body(&mut app, "Halden IV");
    let satellite_radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let satellite_distance = distance_for_radius(satellite_radius, fov(), TILT, margin);
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
    let (_, _, _, close) = test_camera(halden_pos, satellite_distance);
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
    let (_, _, _, far) = test_camera(Vec3::ZERO, system_distance);
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
fn motion_mode_after_a_rate_change() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, ProjectionPlugin, MotionPlugin))
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
fn zoom_stops_come_from_corvanes_orbits() {
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
        ("Morrow", 6_619_046.0),
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

/// EDGE-01: zoomed in and tilted, every gate still has an arrow on
/// the screen edge - including gates behind the camera, which have no
/// projected position at all.
#[test]
fn every_gate_has_an_arrow_when_zoomed_in() {
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
    let distance = distance_for_radius(radius, fov(), TILT, margin);
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

/// Give the real camera (spawned by `CameraPlugin`) the computed values the
/// renderer would, so it can project without a GPU.
fn compute_camera_headless(app: &mut App) {
    let mut cameras = app
        .world_mut()
        .query_filtered::<(&mut Camera, &mut Projection), With<OrbitCamera>>();
    for (mut camera, mut projection) in cameras.iter_mut(app.world_mut()) {
        projection.update(VIEWPORT.x as f32, VIEWPORT.y as f32);
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: VIEWPORT,
            scale_factor: 1.0,
        });
        camera.computed.clip_from_view = projection.get_clip_from_view();
    }
}

/// CAM-05: a focused planet keeps its place on screen at any time
/// rate. At 5 d/s, Halden IV moves about 65 px a frame at its satellite
/// frame; following it a frame late made it shake. With zero offset it
/// must sit at the screen centre on every frame.
#[test]
fn focused_body_keeps_its_place_on_screen() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin, InputPlugin))
        .add_plugins((
            TunablesPlugin,
            BindingsPlugin,
            ProjectionPlugin,
            CameraPlugin,
        ))
        .add_plugins(ScreenPlugin)
        .init_resource::<Selection>()
        .init_resource::<Focus>()
        .init_resource::<PressStart>();
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(VIEWPORT.x, VIEWPORT.y),
        ..default()
    });
    run(&mut app, 1);
    compute_camera_headless(&mut app);

    let halden = body(&mut app, "Halden IV");
    let radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    app.world_mut().resource_mut::<CameraRig>().distance =
        distance_for_radius(radius, fov(), 0.0, margin);
    *app.world_mut().resource_mut::<Focus>() = Focus {
        body: Some(halden),
        offset: Vec3::ZERO,
    };
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);

    let centre = VIEWPORT.as_vec2() / 2.0;
    for frame in 0..SETTLE_FRAMES {
        run(&mut app, 1);
        let on_screen = app
            .world()
            .resource::<ScreenPositions>()
            .get(halden)
            .unwrap();
        assert!(
            on_screen.distance(centre) < 0.5,
            "frame {frame}: Halden IV at {on_screen}, {} px off centre",
            on_screen.distance(centre)
        );
    }
}

/// One wheel notch (positive zooms in), then enough frames for the 200 ms
/// step to finish.
fn scroll(app: &mut App, notches: f32) {
    let window = {
        let mut windows = app.world_mut().query_filtered::<Entity, With<Window>>();
        windows.single(app.world()).unwrap()
    };
    app.world_mut().write_message(MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: notches,
        window,
        phase: TouchPhase::Moved,
    });
    run(app, 20);
}

/// The real camera in a headless app, paused, with a window to project into.
fn camera_app() -> App {
    camera_app_with(|_| {})
}

/// `camera_app` with more plugins added before startup runs.
fn camera_app_with(extra: impl FnOnce(&mut App)) -> App {
    let mut app = headless_app(FRAME);
    extra(&mut app);
    app.add_plugins((ContentPlugin, SimPlugin, InputPlugin))
        .add_plugins((
            TunablesPlugin,
            BindingsPlugin,
            ProjectionPlugin,
            CameraPlugin,
        ))
        .add_plugins(ScreenPlugin)
        .init_resource::<Selection>()
        .init_resource::<Focus>()
        .init_resource::<PressStart>();
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(VIEWPORT.x, VIEWPORT.y),
        ..default()
    });
    run(&mut app, 1);
    compute_camera_headless(&mut app);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, 2);
    app
}

/// CAM-21 / ZOOM-14 / CAM-08 with the real zoom: with nothing focused over
/// Halden IV, zooming in focuses it once the step goes past the region
/// frame, with no jump; zoom stops at its satellite frame; zooming back out
/// to the system frame ends the focus.
#[test]
fn zooming_in_focuses_and_zooming_out_ends_focus() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    let halden_pos = app.world().resource::<WorldPositions>().0[&halden];
    let system_distance = app.world().resource::<CameraRig>().distance;
    // Put the (unfocused) view over Halden IV.
    app.world_mut().resource_mut::<CameraRig>().pivot = halden_pos;
    run(&mut app, 2);

    let mut focused_at = None;
    for notch in 0..30 {
        let before = *app.world().resource::<CameraRig>();
        scroll(&mut app, 1.0);
        let focus = *app.world().resource::<Focus>();
        if focus.body.is_some() && focused_at.is_none() {
            assert_eq!(focus.body, Some(halden), "CAM-21 focuses the nearest body");
            let after = *app.world().resource::<CameraRig>();
            assert!(
                (after.pivot - before.pivot).length() < 1e-3,
                "CAM-21: no jump when focus starts"
            );
            focused_at = Some(notch);
        }
    }
    assert!(focused_at.is_some(), "zooming in focused something");

    // ZOOM-14: the inner limit is Halden IV's satellite frame.
    let radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let limit = distance_for_radius(radius, fov(), 0.0, margin);
    let distance = app.world().resource::<CameraRig>().distance;
    assert!(
        (distance - limit).abs() < limit * 1e-3,
        "stopped at {distance}, limit {limit}"
    );

    // CAM-08: zooming back out to the system frame ends focus.
    for _ in 0..40 {
        scroll(&mut app, -1.0);
    }
    let rig = *app.world().resource::<CameraRig>();
    assert!((rig.distance - system_distance).abs() < system_distance * 1e-3);
    assert_eq!(
        app.world().resource::<Focus>().body,
        None,
        "CAM-08: focus ended"
    );
}

/// ZOOM-13 / CAM-08: zooming while focused keeps the focused body where it
/// is on screen, and keeps the focus.
#[test]
fn focused_zoom_keeps_the_body_in_place() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    *app.world_mut().resource_mut::<Focus>() = Focus {
        body: Some(halden),
        offset: Vec3::ZERO,
    };
    run(&mut app, 3);
    for _ in 0..6 {
        scroll(&mut app, 1.0);
        let screen = app
            .world()
            .resource::<ScreenPositions>()
            .get(halden)
            .unwrap();
        let centre = VIEWPORT.as_vec2() / 2.0;
        assert!(screen.distance(centre) < 0.5, "Halden IV at {screen}");
        assert_eq!(app.world().resource::<Focus>().body, Some(halden));
    }
}

fn the_window(app: &mut App) -> Entity {
    let mut windows = app.world_mut().query_filtered::<Entity, With<Window>>();
    windows.single(app.world()).unwrap()
}

fn move_cursor(app: &mut App, to: Vec2) {
    let window = the_window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(to));
}

fn mouse(app: &mut App, button: MouseButton, state: ButtonState) {
    let window = the_window(app);
    app.world_mut().write_message(MouseButtonInput {
        button,
        state,
        window,
    });
}

/// CAM-10: drag the star from where it is to `to`, a few frames at a time.
fn drag_star(app: &mut App, star: Entity, to: Vec2) {
    drag_star_with(app, MouseButton::Left, star, to);
}

/// Drag the star to `to` with `button` held.
fn drag_star_with(app: &mut App, button: MouseButton, star: Entity, to: Vec2) {
    let from = app.world().resource::<ScreenPositions>().get(star).unwrap();
    move_cursor(app, from);
    run(app, 1);
    mouse(app, button, ButtonState::Pressed);
    run(app, 1);
    for step in 1..=10 {
        move_cursor(app, from.lerp(to, step as f32 / 10.0));
        run(app, 1);
    }
    mouse(app, button, ButtonState::Released);
    run(app, 2);
}

/// CAM-10: the plane point grabbed at the start of a left drag stays under
/// the cursor - also when tilted - and a press that barely moves is a
/// click, which doesn't pan.
#[test]
fn left_drag_keeps_the_grabbed_point_under_the_cursor() {
    let mut app = camera_app();
    let star = body(&mut app, "Corvane");

    for tilt in [0.0, 0.8] {
        app.world_mut().resource_mut::<CameraRig>().tilt = tilt;
        run(&mut app, 2);
        let to = Vec2::new(1100.0, 300.0);
        drag_star(&mut app, star, to);
        let at = app.world().resource::<ScreenPositions>().get(star).unwrap();
        assert!(
            at.distance(to) < 1.0,
            "tilt {tilt}: star at {at}, cursor at {to}"
        );
    }

    // A 2 px jitter is a click: the view doesn't move.
    let before = app.world().resource::<CameraRig>().pivot;
    let start = Vec2::new(500.0, 500.0);
    move_cursor(&mut app, start);
    run(&mut app, 1);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    run(&mut app, 1);
    move_cursor(&mut app, start + Vec2::new(2.0, 0.0));
    run(&mut app, 1);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    run(&mut app, 2);
    assert_eq!(app.world().resource::<CameraRig>().pivot, before);
}

/// CAM-07: panning while focused moves the offset and keeps the focus.
#[test]
fn panning_while_focused_keeps_focus() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    *app.world_mut().resource_mut::<Focus>() = Focus {
        body: Some(halden),
        offset: Vec3::ZERO,
    };
    run(&mut app, 3);
    let star = body(&mut app, "Corvane");
    drag_star(&mut app, star, Vec2::new(700.0, 400.0));
    let focus = *app.world().resource::<Focus>();
    assert_eq!(focus.body, Some(halden));
    assert!(
        focus.offset.length() > 1.0,
        "the offset moved: {}",
        focus.offset
    );
}

/// ZOOM-16 / CAM-03 / CAM-04 with the real camera: a focus request frames
/// the body's satellites and focuses it; N levels and turns north up,
/// keeping pivot and scale; Home returns to the system frame, level and
/// north up, and ends the focus.
#[test]
fn focus_north_and_home_move_the_real_camera() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    let system_distance = app.world().resource::<CameraRig>().distance;

    app.world_mut().resource_mut::<FocusRequest>().0 = Some(halden);
    run(&mut app, 40);
    let radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let framed = distance_for_radius(radius, fov(), 0.0, margin);
    let rig = *app.world().resource::<CameraRig>();
    assert!(
        (rig.distance - framed).abs() < framed * 1e-3,
        "ZOOM-16 frames the satellites"
    );
    assert_eq!(app.world().resource::<Focus>().body, Some(halden));

    // Tilt and turn, then N.
    {
        let mut rig = app.world_mut().resource_mut::<CameraRig>();
        rig.tilt = 0.7;
        rig.heading = 2.0 * std::f32::consts::TAU + 1.0;
    }
    run(&mut app, 2);
    let before = *app.world().resource::<CameraRig>();
    press_key(&mut app, KeyCode::KeyN);
    run(&mut app, 40);
    let after = *app.world().resource::<CameraRig>();
    assert!(after.tilt.abs() < 1e-5, "CAM-03 levels");
    assert!(
        (after.heading - 2.0 * std::f32::consts::TAU).abs() < 1e-4,
        "north up, the short way"
    );
    assert!(
        (after.distance - before.distance).abs() < 1e-5,
        "scale kept"
    );
    assert_eq!(
        app.world().resource::<Focus>().body,
        Some(halden),
        "pivot (focus) kept"
    );

    // Home.
    app.world_mut().resource_mut::<CameraRig>().tilt = 0.5;
    run(&mut app, 2);
    press_key(&mut app, KeyCode::Home);
    run(&mut app, 40);
    let home = *app.world().resource::<CameraRig>();
    assert!((home.distance - system_distance).abs() < system_distance * 1e-3);
    assert!(home.tilt.abs() < 1e-5 && home.pivot.length() < 1e-3);
    assert_eq!(
        app.world().resource::<Focus>().body,
        None,
        "CAM-04: Home ends focus"
    );
}

fn press_key(app: &mut App, key: KeyCode) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        key_event(app, key, state);
        run(app, 1);
    }
}

fn key_event(app: &mut App, key: KeyCode, state: ButtonState) {
    let window = the_window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code: key,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: None,
        repeat: false,
        window,
    });
}

/// Hold `key` down for `frames` frames, then let it go.
fn hold_key(app: &mut App, key: KeyCode, frames: usize) {
    key_event(app, key, ButtonState::Pressed);
    run(app, frames);
    key_event(app, key, ButtonState::Released);
    run(app, 1);
}

/// MOT-08: the motion rule covers every orbiting object - planets, moons,
/// stations and parked ships - and not the gates. At 5 d/s Pell (an 86-day
/// orbit, 17 s apparent) becomes an exposure ring like any satellite.
#[test]
fn the_rule_applies_to_every_orbiting_object() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, ProjectionPlugin, MotionPlugin))
        .init_resource::<Selection>()
        .insert_resource(placeholder_fonts());
    run(&mut app, 2);
    for name in [
        "Pell",
        "Iver Yard",
        "Orison",
        "K-3",
        "Halden IVc",
        "Lantern",
    ] {
        let entity = body(&mut app, name);
        assert!(
            app.world().get::<MotionModeState>(entity).is_some(),
            "{name}"
        );
    }
    let gate = body(&mut app, "Gate Ostra");
    assert!(
        app.world().get::<MotionModeState>(gate).is_none(),
        "gates keep edge arrows"
    );

    app.world_mut().resource_mut::<GameClock>().run_at(5.0);
    run(&mut app, SETTLE_FRAMES);
    let pell = body(&mut app, "Pell");
    assert!(app.world().get::<MotionModeState>(pell).unwrap().ring_mode);
}

/// EDGE-01: turning the heading a full circle while focused
/// at Halden IV's satellite frame and tilted, every gate always has its
/// marker on screen or an arrow on the edge, and no arrow hides under the
/// top bar.
#[test]
fn every_gate_has_an_arrow_as_the_heading_turns() {
    let mut app = camera_app();
    app.add_plugins(EdgePlugin)
        .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FocusRequest>().0 = Some(halden);
    run(&mut app, 40);
    app.world_mut().resource_mut::<CameraRig>().tilt = 0.6;
    let gates: Vec<Entity> = ["Gate Ostra", "Gate Brenn", "Gate Soll"]
        .into_iter()
        .map(|name| body(&mut app, name))
        .collect();
    for step in 0..24 {
        app.world_mut().resource_mut::<CameraRig>().heading = step as f32 * 0.2618;
        run(&mut app, 2);
        let screen = app.world().resource::<ScreenPositions>();
        let pinned = app.world().resource::<GateEdgeTargets>();
        for &gate in &gates {
            let on_screen = screen.get(gate).is_some_and(|pos| screen.in_viewport(pos));
            assert!(
                on_screen || pinned.is_pinned(gate),
                "heading step {step}: a gate is lost"
            );
            if let Some(&arrow) = pinned.0.get(&gate) {
                assert!(
                    arrow.y > TOP_BAR_HEIGHT_PX,
                    "an arrow under the top bar: {arrow}"
                );
            }
        }
    }
}

fn blueprint(name: &'static str, class: BodyClass, radius_km: f64) -> BodyBlueprint {
    BodyBlueprint {
        name,
        class,
        ownership: Ownership::Neutral,
        description: "",
        key_facts: vec![],
        knowledge: Knowledge::Unsurveyed,
        orbit_radius_km: radius_km,
        radius_km: 1_000.0,
        phase0_rad: 0.0,
        mass_kg: 0.0,
        children: vec![],
    }
}

/// A small system that isn't Corvane, built the way a generator would: a
/// planet with a moon, a ship in transit round the star, and a gate.
fn generated_system() -> SystemData {
    let mut planet = blueprint("Tessaly", BodyClass::Planet, 1.5 * KM_PER_AU);
    planet.mass_kg = EARTH_MASS_KG;
    planet.children = vec![blueprint("Tessaly a", BodyClass::Moon, 200_000.0)];
    SystemData {
        star_name: "Vey",
        star_class: "G5V",
        star_mass_kg: SOLAR_MASS_KG,
        star_radius_km: 696_000.0,
        star_knowledge: Knowledge::Unsurveyed,
        planets: vec![
            planet,
            blueprint("Courier", BodyClass::Ship, 0.9 * KM_PER_AU),
        ],
        belt: BeltBlueprint {
            inner_radius_km: 0.5 * KM_PER_AU,
            outer_radius_km: 0.6 * KM_PER_AU,
            salvage_sites: vec![],
        },
        gates: vec![GateBlueprint {
            name: "Gate Vey",
            destination: "Ostra",
            radius_km: 5.0,
            distance_km: 4.0 * KM_PER_AU,
            angle_deg: 30.0,
        }],
    }
}

/// SCL-03 / GRP-08: a system from other records runs through the same
/// simulation and view code - bodies placed, zoom stops from its own
/// orbits, its moon folded at the system frame - and its ship in transit,
/// which orbits the star, never gets fold state.
#[test]
fn another_system_runs_through_the_same_code() {
    let mut app = headless_app(FRAME);
    app.insert_resource(SystemDataResource(Some(generated_system())))
        .add_plugins(SimPlugin)
        .add_plugins((
            TunablesPlugin,
            ProjectionPlugin,
            StopsPlugin,
            GroupingPlugin,
        ));
    run(&mut app, 2);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let system_radius = app.world().resource::<ZoomStops>().system_radius;
    assert!((system_radius - km_to_render(1.5 * KM_PER_AU)).abs() < system_radius * 1e-5);
    let distance = distance_for_radius(system_radius, fov(), TILT, margin);
    app.world_mut().spawn(test_camera(Vec3::ZERO, distance));
    run(&mut app, SETTLE_FRAMES);

    let tessaly = body(&mut app, "Tessaly");
    assert!(
        app.world()
            .resource::<WorldPositions>()
            .0
            .contains_key(&tessaly)
    );
    let moon = body(&mut app, "Tessaly a");
    assert!(app.world().get::<SatelliteFoldState>(moon).unwrap().folded);
    let courier = body(&mut app, "Courier");
    assert!(
        app.world().get::<SatelliteFoldState>(courier).is_none(),
        "GRP-08: a ship in transit never folds"
    );
    body(&mut app, "Gate Vey");
}

/// MRK-06: zoomed all the way out, where the inner planets crowd
/// together, every planet still has its own marker position on screen and
/// none is ever folded.
#[test]
fn planets_are_never_merged() {
    let mut app = camera_app_with(|app| {
        app.add_plugins(GroupingPlugin);
    });
    for _ in 0..10 {
        scroll(&mut app, -1.0);
    }
    run(&mut app, SETTLE_FRAMES);
    let moon = body(&mut app, "Halden IVc");
    assert!(app.world().get::<SatelliteFoldState>(moon).unwrap().folded);
    let names = ["Pell", "Iver", "Halden IV", "Morrow"];
    let planets: Vec<Entity> = names.iter().map(|name| body(&mut app, name)).collect();
    let screen = app.world().resource::<ScreenPositions>();
    let positions: Vec<Vec2> = planets
        .iter()
        .map(|&planet| screen.get(planet).unwrap())
        .collect();
    for (i, &planet) in planets.iter().enumerate() {
        assert!(app.world().get::<SatelliteFoldState>(planet).is_none());
        assert!(screen.in_viewport(positions[i]), "{} off screen", names[i]);
        for j in 0..i {
            assert_ne!(positions[i], positions[j]);
        }
    }
}

/// GRP-05: at the system frame Halden IV shows a "+N" pill counting all
/// its folded satellites, beside its marker and clear of it and its ring.
#[test]
fn a_parent_with_folded_satellites_shows_a_count_pill() {
    let mut app = camera_app_with(|app| {
        app.add_plugins((GroupingPlugin, PillsPlugin))
            .insert_resource(placeholder_fonts());
    });
    run(&mut app, SETTLE_FRAMES);

    let halden = body(&mut app, "Halden IV");
    let satellites = app.world().get::<Children>(halden).unwrap().len();
    let marker = app
        .world()
        .resource::<ScreenPositions>()
        .get(halden)
        .unwrap();
    let mut pills = app.world_mut().query::<(&GroupPillFor, &Node, &Children)>();
    let (_, node, children) = pills
        .iter(app.world())
        .find(|(pill_for, ..)| pill_for.0 == halden)
        .unwrap();
    assert_eq!(node.display, Display::Flex);
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("pill not placed in pixels");
    };
    let clear = ring_outer_half(BodyClass::Planet);
    assert!(left - marker.x >= clear, "the pill overlaps the marker");
    assert!(
        top > marker.y,
        "the pill sits beside the marker, below its centre"
    );
    let count = children
        .iter()
        .find_map(|child| app.world().get::<Text>(child))
        .unwrap();
    assert_eq!(count.0, format!("+{satellites}"));
}

/// CAM-11: middle drag pans exactly as left drag: the grabbed point stays
/// under the cursor.
#[test]
fn middle_drag_pans_like_left_drag() {
    let mut app = camera_app();
    let star = body(&mut app, "Corvane");
    let to = Vec2::new(1000.0, 600.0);
    drag_star_with(&mut app, MouseButton::Middle, star, to);
    let at = app.world().resource::<ScreenPositions>().get(star).unwrap();
    assert!(at.distance(to) < 1.0, "star at {at}, cursor at {to}");
}

/// Right-drag the cursor `dx` pixels sideways from the screen centre.
fn right_drag(app: &mut App, dx: f32) {
    let start = VIEWPORT.as_vec2() / 2.0;
    move_cursor(app, start);
    run(app, 1);
    mouse(app, MouseButton::Right, ButtonState::Pressed);
    run(app, 1);
    for step in 1..=10 {
        move_cursor(app, start + Vec2::new(dx * step as f32 / 10.0, 0.0));
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(dx / 10.0, 0.0),
        });
        run(app, 1);
    }
    mouse(app, MouseButton::Right, ButtonState::Released);
    run(app, 2);
}

/// CAM-15: Q turns as dragging left does and E as dragging right, at
/// `key_rotate_deg_per_s`; Page Up tilts towards the horizon and Page
/// Down back towards straight down, clamped at `max_pitch_deg`.
#[test]
fn q_e_and_page_keys_turn_and_tilt_the_camera() {
    let mut app = camera_app();
    let tunables = app.world().resource::<Tunables>().clone();
    let rate = tunables.key_rotate_deg_per_s.to_radians();
    let heading = |app: &App| app.world().resource::<CameraRig>().heading;
    let tilt = |app: &App| app.world().resource::<CameraRig>().tilt;

    let before = heading(&app);
    hold_key(&mut app, KeyCode::KeyQ, 30);
    let turned = heading(&app) - before;
    let expected = rate * 30.0 * FRAME.as_secs_f32();
    assert!(
        (turned - expected).abs() < expected * 0.1,
        "Q turned {turned}, expected {expected}"
    );

    let before = heading(&app);
    hold_key(&mut app, KeyCode::KeyE, 30);
    assert!(heading(&app) < before, "E turns the other way");

    let mut dragged = camera_app();
    let before = heading(&dragged);
    right_drag(&mut dragged, -100.0);
    assert!(
        heading(&dragged) > before,
        "dragging left turns the way Q does"
    );

    hold_key(&mut app, KeyCode::PageUp, 30);
    assert!(tilt(&app) > 0.0, "Page Up leans towards the horizon");
    hold_key(&mut app, KeyCode::PageUp, 300);
    let max = tunables.max_pitch_deg.to_radians();
    assert!((tilt(&app) - max).abs() < 1e-4, "tilt clamped at {max}");
    hold_key(&mut app, KeyCode::PageDown, 300);
    assert!(tilt(&app).abs() < 1e-4, "Page Down back to straight down");
}

/// CAM-16: = and numpad + zoom in one step, and - and numpad - out one
/// step, landing exactly where a wheel notch does.
#[test]
fn plus_and_minus_keys_zoom_like_a_wheel_notch() {
    let distance = |app: &App| app.world().resource::<CameraRig>().distance;
    let mut wheel = camera_app();
    scroll(&mut wheel, 1.0);
    let wheel_in = distance(&wheel);
    scroll(&mut wheel, 1.0);
    scroll(&mut wheel, -1.0);
    let wheel_out = distance(&wheel);

    for (zoom_in, zoom_out) in [
        (KeyCode::Equal, KeyCode::Minus),
        (KeyCode::NumpadAdd, KeyCode::NumpadSubtract),
    ] {
        let mut keys = camera_app();
        press_key(&mut keys, zoom_in);
        run(&mut keys, 20);
        assert!((distance(&keys) - wheel_in).abs() < wheel_in * 1e-4);
        press_key(&mut keys, zoom_in);
        run(&mut keys, 20);
        press_key(&mut keys, zoom_out);
        run(&mut keys, 20);
        assert!((distance(&keys) - wheel_out).abs() < wheel_out * 1e-4);
    }
}

/// Frames in one `focus_jump_ms` move, plus a few to settle.
fn move_frames() -> usize {
    let ms = Tunables::default().focus_jump_ms;
    (ms / 1000.0 / FRAME.as_secs_f32()).ceil() as usize + 5
}

/// ZOOM-10: Home goes back to the system frame in an animated move, not a
/// cut: partway through, the camera is between where it was and the frame.
#[test]
fn home_is_an_animated_move_to_the_system_frame() {
    let mut app = camera_app();
    let system_distance = app.world().resource::<CameraRig>().distance;
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FocusRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    let close = app.world().resource::<CameraRig>().distance;

    press_key(&mut app, KeyCode::Home);
    run(&mut app, move_frames() / 4);
    let partway = app.world().resource::<CameraRig>().distance;
    assert!(
        close < partway && partway < system_distance * 0.999,
        "partway at {partway}, from {close} to {system_distance}"
    );
    run(&mut app, move_frames());
    let home = app.world().resource::<CameraRig>().distance;
    assert!((home - system_distance).abs() < system_distance * 1e-3);
}

/// Press a bookmark slot with Ctrl held, which saves it.
fn save_bookmark(app: &mut App, slot: KeyCode) {
    key_event(app, KeyCode::ControlLeft, ButtonState::Pressed);
    press_key(app, slot);
    key_event(app, KeyCode::ControlLeft, ButtonState::Released);
    run(app, 1);
}

/// ZOOM-11: Ctrl+1 saves the view and 1 recalls it with an animated move;
/// a bookmark saved while focused brings the focus back.
#[test]
fn bookmarks_save_and_recall_the_view() {
    let mut app = camera_app();
    let saved = CameraRig {
        pivot: Vec3::new(3.0, 0.0, -2.0),
        heading: 0.8,
        tilt: 0.4,
        distance: app.world().resource::<CameraRig>().distance * 0.5,
    };
    *app.world_mut().resource_mut::<CameraRig>() = saved;
    run(&mut app, 2);
    save_bookmark(&mut app, KeyCode::Digit1);

    // Focus Halden IV and save that in slot 2.
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FocusRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    save_bookmark(&mut app, KeyCode::Digit2);
    let focused_distance = app.world().resource::<CameraRig>().distance;

    press_key(&mut app, KeyCode::Digit1);
    run(&mut app, 3);
    let early = *app.world().resource::<CameraRig>();
    assert!(
        (early.distance - saved.distance).abs() > saved.distance * 1e-3,
        "recalling is a move, not a cut"
    );
    run(&mut app, move_frames());
    let rig = *app.world().resource::<CameraRig>();
    assert!((rig.pivot - saved.pivot).length() < 1e-3);
    assert!((rig.heading - saved.heading).abs() < 1e-4);
    assert!((rig.tilt - saved.tilt).abs() < 1e-4);
    assert!((rig.distance - saved.distance).abs() < saved.distance * 1e-4);
    assert_eq!(app.world().resource::<Focus>().body, None);

    press_key(&mut app, KeyCode::Digit2);
    run(&mut app, move_frames());
    assert_eq!(app.world().resource::<Focus>().body, Some(halden));
    let distance = app.world().resource::<CameraRig>().distance;
    assert!((distance - focused_distance).abs() < focused_distance * 1e-4);
}

/// CAM-20: pan or rotate input during an animated move ends the move and
/// carries on from where the camera is, with no jump.
#[test]
fn input_during_a_move_takes_over_without_a_jump() {
    for interrupt in [KeyCode::KeyQ, KeyCode::KeyD] {
        let mut app = camera_app();
        let halden = body(&mut app, "Halden IV");
        app.world_mut().resource_mut::<FocusRequest>().0 = Some(halden);
        run(&mut app, move_frames() / 3);
        assert!(app.world().resource::<ActiveMove>().0.is_some());
        let before = *app.world().resource::<CameraRig>();

        key_event(&mut app, interrupt, ButtonState::Pressed);
        run(&mut app, 1);
        let after = *app.world().resource::<CameraRig>();
        assert!(
            app.world().resource::<ActiveMove>().0.is_none(),
            "{interrupt:?} ends the move"
        );
        // One frame of input moves the camera a little, never back to the
        // start or on to the target.
        assert!((after.distance - before.distance).abs() < before.distance * 0.01);
        assert!((after.pivot - before.pivot).length() < before.distance * 0.01);
        assert!((after.heading - before.heading).abs() < 0.05);
        key_event(&mut app, interrupt, ButtonState::Released);
        run(&mut app, 20);
        let later = app.world().resource::<CameraRig>().distance;
        assert!(
            (later - after.distance).abs() < after.distance * 1e-4,
            "the move stays ended"
        );
    }
}

/// The marker node drawn for a body, as its width in pixels.
fn marker_width_px(app: &mut App, body: Entity) -> f32 {
    let mut markers = app.world_mut().query::<(&MarkerFor, &Node)>();
    let (_, node) = markers
        .iter(app.world())
        .find(|(marker, _)| marker.0 == body)
        .expect("a marker for the body");
    match node.width {
        Val::Px(width) => width,
        other => panic!("marker width {other:?}"),
    }
}

/// MRK-07: at the system frame Halden IV is its class's fixed marker; close
/// in, where its true diameter is larger than that, the marker is a disc of
/// the true on-screen size.
#[test]
fn a_body_grows_into_a_true_size_disc_when_close() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .insert_resource(MarkerImages {
                triangle: Handle::default(),
                diamond: Handle::default(),
            })
            .add_plugins((EdgePlugin, MarkersPlugin));
    });
    let halden = body(&mut app, "Halden IV");
    run(&mut app, 2);
    let far = marker_width_px(&mut app, halden);
    assert_eq!(far, 12.0, "a planet's fixed marker at the system frame");

    let distance = 0.01;
    *app.world_mut().resource_mut::<Focus>() = Focus {
        body: Some(halden),
        offset: Vec3::ZERO,
    };
    app.world_mut().resource_mut::<CameraRig>().distance = distance;
    run(&mut app, 3);
    let radius_km = app.world().get::<PhysicalRadius>(halden).unwrap().0;
    let expected =
        2.0 * projected_radius_px(km_to_render(radius_km), distance, fov(), VIEWPORT.y as f32);
    let drawn = marker_width_px(&mut app, halden);
    assert!(expected > 12.0, "the test view is close enough: {expected}");
    assert!(
        (drawn - expected).abs() < expected * 0.01,
        "drawn {drawn} px, true size {expected} px"
    );
}

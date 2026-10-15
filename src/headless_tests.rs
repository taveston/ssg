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
use crate::sim::body::{Affiliation, BodyClass, DisplayName, Knowledge, Mass, PhysicalRadius};
use crate::sim::clock::GameClock;
use crate::sim::named_orbit::{InNamedOrbit, NamedOrbit};
use crate::sim::orbit::{Orbit, OrbitPositionKm, orbit_position_km};
use crate::sim::rail::SegmentKind;
use crate::sim::ship::{DeltaV, InFlight, ShipRail};
use crate::sim::system::{
    BeltBlueprint, BodyBlueprint, GateBlueprint, OrbitPlacement, SystemData, SystemDataResource,
};
use crate::sim::transfer::legs::{Arrival, Infeasible, Landing, Order, SolvedLeg};
use crate::sim::transfer::places::{Band, Place};
use crate::sim::transfer::plan::{Plan, Planner, Status};
use crate::sim::units::{EARTH_MASS_KG, KM_PER_AU, SOLAR_MASS_KG};
use crate::view::bindings::BindingsPlugin;
use crate::view::breadcrumb::{BreadcrumbPlugin, BreadcrumbRoot};
use crate::view::camera::CameraPlugin;
use crate::view::camera::keep::inside_margin;
use crate::view::camera::requests::{FrameRequest, RecentreRequest};
use crate::view::camera::state::{ActiveMove, CameraRig, OrbitCamera};
use crate::view::camera::stops::{StopsPlugin, ZoomStops, distance_for_radius};
use crate::view::edge::{EdgePlugin, GateEdgeTargets};
use crate::view::fold::SatelliteFoldState;
use crate::view::fonts::Fonts;
use crate::view::frame::{Frame, FramePlugin};
use crate::view::gesture::PressStart;
use crate::view::grouping::GroupingPlugin;
use crate::view::hovered::HoveredPlugin;
use crate::view::markers::{
    BodyDetail, BodyRim, MarkerHalfSize, MarkersPlugin, body_fill, ring_outer_half,
};
use crate::view::motion::MotionPlugin;
use crate::view::motion_mode::MotionModeState;
use crate::view::pills::PillsPlugin;
use crate::view::plot_marks::{NorthArrow, NorthDial, PlotMarksPlugin, ScaleBarLine};
use crate::view::projection::ProjectionPlugin;
use crate::view::scale::{km_to_render, km_to_render_vec3};
use crate::view::screen::{ScreenPlugin, ScreenPositions};
use crate::view::selection::Selection;
use crate::view::shared_orbit::SharedOrbitPlugin;
use crate::view::symbol_draw::Tint;
use crate::view::targets::{GroupPillFor, MarkerFor};
use crate::view::topbar::TOP_BAR_HEIGHT_PX;
use crate::view::true_size::projected_radius_px;
use crate::view::tunables::{Tunables, TunablesPlugin};
use crate::view::world::{RenderOrigin, WorldPositions};
use crate::{Boot, Stage};

/// One frame at 60 fps.
const FRAME: Duration = Duration::from_micros(16_667);
/// Enough frames for transforms to propagate and a 200 ms fold to finish.
const SETTLE_FRAMES: usize = 30;

/// Frames for a tail to finish easing to a new length (MOT-13), and then
/// settle.
fn eased_frames() -> usize {
    let ease_secs = Tunables::default().tail_ease_ms / 1000.0;
    (ease_secs / FRAME.as_secs_f32()).ceil() as usize + SETTLE_FRAMES
}
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
        text: Handle::default(),
        text_semibold: Handle::default(),
        data: Handle::default(),
    }
}

/// The camera distance that fits a circle of `radius` in the test window.
fn fit_distance(radius: f32, fov_y: f32, tilt: f32, margin: f32) -> f32 {
    distance_for_radius(radius, fov_y, tilt, margin, VIEWPORT.as_vec2())
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
    // Highport (0.1 d) has gone round about 100 times but is still about
    // 9,394 km from its planet; Orison sits on its named orbit "High"
    // (SCL-07).
    let halden = body(&mut app, "Halden IV");
    let halden_km = app.world().get::<OrbitPositionKm>(halden).unwrap().0;
    assert!((halden_km.length() / KM_PER_AU - 2.0).abs() < 1e-9);
    let highport = body(&mut app, "Arden Highport");
    let highport_km = app.world().get::<OrbitPositionKm>(highport).unwrap().0;
    assert!((highport_km.length() - 9_394.0).abs() < 1.0);
    let orison = body(&mut app, "Orison");
    let orison_km = app.world().get::<OrbitPositionKm>(orison).unwrap().0;
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    let high = app.world().get::<NamedOrbit>(high).unwrap();
    assert_eq!((high.parent, high.name.as_str()), (halden, "High"));
    assert!((orison_km.length() - high.radius_km).abs() < 1e-6);
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
    let system_distance = fit_distance(system_radius, fov(), TILT, margin);
    let halden = body(&mut app, "Halden IV");
    let satellite_radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let satellite_distance = fit_distance(satellite_radius, fov(), TILT, margin);
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
        "GRP-04: everything folded at the system fit"
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

/// ARCH-31 / MOT-01, MOT-02, MOT-04, MOT-12, MOT-13: the motion mode
/// follows the time rate, easing into each change.
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

    // MOT-12: paused, everything is a full-brightness dot with its tail,
    // capped at `dot_fade_from_degrees`.
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    // MOT-13: the tail eases down rather than snapping, so a frame after
    // pausing IVc's dot is still fading back in.
    run(&mut app, 1);
    let ivc = body(&mut app, "Halden IVc");
    let easing = *app.world().get::<MotionModeState>(ivc).unwrap();
    assert!(easing.tail_degrees > 300.0 && easing.dot_alpha() < 1.0);
    let settled = eased_frames();
    run(&mut app, settled);
    let mut modes = app.world_mut().query::<&MotionModeState>();
    assert!(
        modes
            .iter(app.world())
            .all(|mode| !mode.ring_mode && mode.dot_alpha() == 1.0 && mode.tail_degrees <= 300.0)
    );
    // Unpausing restores the running state.
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, settled);
    assert!(ring(&mut app, "Halden IVc"));
}

/// TEST-08 / MOT-02: at 1 d/s six of Morrow's moons (periods under 8 d)
/// are exposure rings and five (8 d or more) are dots.
#[test]
fn morrows_moons_straddle_both_motion_modes() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, ProjectionPlugin, MotionPlugin))
        .init_resource::<Selection>()
        .insert_resource(placeholder_fonts());
    run(&mut app, 2);
    let morrow = body(&mut app, "Morrow");
    let mut moons = app
        .world_mut()
        .query::<(&ChildOf, &BodyClass, &MotionModeState)>();
    let (rings, dots): (Vec<bool>, Vec<bool>) = moons
        .iter(app.world())
        .filter(|(child_of, class, _)| child_of.parent() == morrow && **class == BodyClass::Moon)
        .map(|(_, _, mode)| mode.ring_mode)
        .partition(|ring| *ring);
    assert_eq!((rings.len(), dots.len()), (6, 5));
}

/// ARCH-31 / ZOOM-15: the stops come from Corvane's real orbits.
#[test]
fn zoom_stops_come_from_corvanes_orbits() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin, StopsPlugin));
    run(&mut app, 1);

    let close = |a: f32, b: f32| (a - b).abs() < b * 1e-5;
    let stops = app.world().resource::<ZoomStops>();
    // System frame: Morrow at 4.3 AU.
    assert!(close(stops.system_radius, km_to_render(4.3 * KM_PER_AU)));
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
        .init_resource::<Selection>()
        .add_plugins(crate::view::sidebar::SidebarPlugin)
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
    let distance = fit_distance(radius, fov(), TILT, margin);
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

    // EDGE-01 / EDGE-02: every pinned gate's tab is shown, labelled with
    // its destination alone.
    let pinned: Vec<Entity> = pinned.0.keys().copied().collect();
    assert!(!pinned.is_empty());
    let mut tabs = app
        .world_mut()
        .query::<(Entity, &crate::view::edge::EdgeTabFor, &Node)>();
    let tabs: Vec<(Entity, Entity, Display)> = tabs
        .iter(app.world())
        .map(|(e, t, n)| (e, t.0, n.display))
        .collect();
    for gate in pinned {
        let &(tab, _, display) = tabs.iter().find(|t| t.1 == gate).unwrap();
        assert_eq!(display, Display::Flex);
        let destination = app
            .world()
            .get::<crate::sim::body::GateDestination>(gate)
            .unwrap()
            .0
            .clone();
        assert!(!destination.starts_with("Gate"));
        assert!(
            descendants(&mut app, tab).iter().any(|&e| app
                .world()
                .get::<Text>(e)
                .is_some_and(|t| t.0 == destination)),
            "the tab is labelled {destination}"
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
        // INS-07: the camera draws in its viewport, the window less the
        // sidebar, when it has one.
        let size = camera
            .viewport
            .as_ref()
            .map_or(VIEWPORT, |viewport| viewport.physical_size);
        projection.update(size.x as f32, size.y as f32);
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: VIEWPORT,
            scale_factor: 1.0,
        });
        camera.computed.clip_from_view = projection.get_clip_from_view();
    }
}

/// The frame the camera is in.
fn frame(app: &App) -> Frame {
    *app.world().resource::<Frame>()
}

/// The frame body.
fn frame_body(app: &App) -> Entity {
    frame(app).body
}

/// Whether the camera is in the star's frame.
fn in_star_frame(app: &App) -> bool {
    frame(app).in_star_frame()
}

/// Put the camera in `body`'s frame with `offset`, as if a jump had ended.
fn set_frame(app: &mut App, body: Entity, offset: Vec3) {
    let current = frame(app);
    *app.world_mut().resource_mut::<Frame>() = current.on(body, offset);
}

/// CAM-05: a planet that is the frame body keeps its place on screen at
/// any time rate. At 5 d/s, Halden IV moves about 65 px a frame at its
/// satellite fit; following it a frame late made it shake. With zero
/// offset it must sit at the screen centre on every frame.
#[test]
fn the_frame_body_keeps_its_place_on_screen() {
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
        .add_plugins((FramePlugin, crate::view::sidebar::SidebarPlugin))
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
    app.world_mut().resource_mut::<CameraRig>().distance = fit_distance(radius, fov(), 0.0, margin);
    set_frame(&mut app, halden, Vec3::ZERO);
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

/// SCL-06: render space is centred on the frame body. Framed 4.3 AU out,
/// Morrow sits exactly at the render origin and on the screen centre on
/// every frame at a close zoom, the star sits at minus its position, and a
/// moon's render position is its own offset from Morrow at full precision.
#[test]
fn render_space_is_centred_on_a_distant_frame_body() {
    let mut app = camera_app();
    let morrow = body(&mut app, "Morrow");
    let moon = body(&mut app, "Morrow I");
    let star = frame(&app).star;
    app.world_mut().resource_mut::<CameraRig>().distance = 0.01;
    set_frame(&mut app, morrow, Vec3::ZERO);
    app.world_mut().resource_mut::<GameClock>().run_at(1.0);

    let centre = VIEWPORT.as_vec2() / 2.0;
    for frame in 0..SETTLE_FRAMES {
        run(&mut app, 1);
        let positions = app.world().resource::<WorldPositions>();
        assert_eq!(positions.0[&morrow], Vec3::ZERO, "frame {frame}");
        let local = app.world().get::<OrbitPositionKm>(moon).unwrap().0;
        let expected = km_to_render_vec3(local);
        let error = (positions.0[&moon] - expected).length() / expected.length();
        assert!(error < 1e-6, "frame {frame}: moon off by {error}");
        let morrow_km = app.world().resource::<RenderOrigin>().km;
        let star_at = positions.0[&star];
        assert!((star_at - km_to_render_vec3(-morrow_km)).length() < 1e-3);
        let on_screen = app
            .world()
            .resource::<ScreenPositions>()
            .get(morrow)
            .unwrap();
        assert!(
            on_screen.distance(centre) < 0.01,
            "frame {frame}: Morrow {} px off centre",
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
        .add_plugins((FramePlugin, crate::view::sidebar::SidebarPlugin))
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

/// CAM-06 / ZOOM-14 / ZOOM-13 with the real zoom: the view starts in the
/// star's frame; zooming in on the cursor keeps the point under the cursor
/// in place, never changes the frame, and stops at the minimum view
/// radius.
#[test]
fn zooming_in_the_star_frame_keeps_the_frame_and_the_cursor_point() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    let halden_pos = app.world().resource::<WorldPositions>().0[&halden];
    assert!(in_star_frame(&app), "CAM-05: the star's frame by default");
    app.world_mut().resource_mut::<Frame>().offset = halden_pos;
    run(&mut app, 2);
    // ZOOM-13: the cursor's point stays under the cursor.
    let cursor = Vec2::new(1000.0, 600.0);
    move_cursor(&mut app, cursor);
    run(&mut app, 1);
    let before = plane_point_under(&mut app, cursor);
    scroll(&mut app, 1.0);
    let after = plane_point_under(&mut app, cursor);
    assert!(
        (after - before).length() < before.length() * 1e-3,
        "ZOOM-13: {before} drifted to {after}"
    );
    for _ in 0..30 {
        scroll(&mut app, 1.0);
    }
    assert!(in_star_frame(&app), "CAM-06");
    let tilt = app.world().resource::<CameraRig>().tilt;
    let radius = km_to_render(0.05 * KM_PER_AU);
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let limit = fit_distance(radius, fov(), tilt, margin);
    let distance = app.world().resource::<CameraRig>().distance;
    assert!(
        (distance - limit).abs() < limit * 1e-3,
        "ZOOM-14 stopped at {distance}, limit {limit}"
    );
}

/// ZOOM-14 / ZOOM-15 / CAM-08 / CAM-23 with the real zoom: in a planet's
/// frame, zooming in settles on the satellite fit, then goes on to the
/// body's inner limit (its diameter a quarter of the map height) and no
/// further; zooming out to the system fit returns the frame to the star and
/// eases the view to the standard tilt.
#[test]
fn zoom_in_a_frame_has_a_satellite_stop_an_inner_limit_and_ends_at_the_system_fit() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    let system_distance = app.world().resource::<CameraRig>().distance;
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    let tilt = app.world().resource::<CameraRig>().tilt;
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let satellites = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let satellite_distance = fit_distance(satellites, fov(), tilt, margin);

    // Out a notch, then back in: the step lands exactly on the stop.
    scroll(&mut app, -1.0);
    scroll(&mut app, 1.0);
    let distance = app.world().resource::<CameraRig>().distance;
    assert!(
        (distance - satellite_distance).abs() < satellite_distance * 1e-3,
        "{distance} vs {satellite_distance}"
    );

    for _ in 0..30 {
        scroll(&mut app, 1.0);
    }
    let radius_km = app.world().get::<PhysicalRadius>(halden).unwrap().0;
    let diameter_px = 2.0
        * projected_radius_px(
            km_to_render(radius_km),
            app.world().resource::<CameraRig>().distance,
            fov(),
            VIEWPORT.y as f32,
        );
    let map_height = VIEWPORT.y as f32 - TOP_BAR_HEIGHT_PX;
    assert!(
        (diameter_px - 0.25 * map_height).abs() < 1.0,
        "inner limit: {diameter_px} px of {map_height}"
    );
    assert_eq!(frame_body(&app), halden);

    // Tilt away from the standard, then zoom all the way out.
    app.world_mut().resource_mut::<CameraRig>().tilt = 0.7;
    run(&mut app, 2);
    for _ in 0..40 {
        scroll(&mut app, -1.0);
    }
    let rig = *app.world().resource::<CameraRig>();
    let standard = app
        .world()
        .resource::<Tunables>()
        .standard_tilt_deg
        .to_radians();
    assert!((rig.distance - system_distance).abs() < system_distance * 1e-3);
    assert!((rig.tilt - standard).abs() < 1e-4, "CAM-23 standard tilt");
    assert!(rig.pivot.length() < 1e-3, "CAM-23 pivot on the star");
    assert!(in_star_frame(&app), "CAM-08");
}

/// ZOOM-13 / CAM-08: zooming in a body's frame keeps the frame body where
/// it is on screen, and keeps the frame.
#[test]
fn zoom_in_a_frame_keeps_the_body_in_place() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
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
        assert_eq!(frame_body(&app), halden);
    }
}

fn the_window(app: &mut App) -> Entity {
    let mut windows = app.world_mut().query_filtered::<Entity, With<Window>>();
    windows.single(app.world()).unwrap()
}

/// The point on the orbital plane under a screen position, through the
/// camera as placed this frame.
fn plane_point_under(app: &mut App, screen: Vec2) -> Vec3 {
    let mut cameras = app
        .world_mut()
        .query_filtered::<(&Camera, &Transform), With<OrbitCamera>>();
    let (camera, transform) = cameras.single(app.world()).unwrap();
    let ray = camera
        .viewport_to_world(&GlobalTransform::from(*transform), screen)
        .unwrap();
    crate::view::camera::rules::plane_point(ray).unwrap()
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

/// CAM-07: panning moves the offset and keeps the frame.
#[test]
fn panning_keeps_the_frame() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 3);
    let star = body(&mut app, "Corvane");
    drag_star(&mut app, star, Vec2::new(700.0, 400.0));
    let framed = frame(&app);
    assert_eq!(framed.body, halden);
    assert!(
        framed.offset.length() > 1.0,
        "the offset moved: {}",
        framed.offset
    );
}

/// ZOOM-16 / CAM-03 / CAM-04 with the real camera: a frame request moves
/// to the body's satellite fit in its frame; N levels and turns north up,
/// keeping pivot and scale; Home returns to the system fit, level and
/// north up, and returns the frame to the star.
#[test]
fn frame_north_and_home_move_the_real_camera() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    let system_distance = app.world().resource::<CameraRig>().distance;

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    let radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let standard_tilt = app
        .world()
        .resource::<Tunables>()
        .standard_tilt_deg
        .to_radians();
    let framed = fit_distance(radius, fov(), standard_tilt, margin);
    let rig = *app.world().resource::<CameraRig>();
    assert!(
        (rig.distance - framed).abs() < framed * 1e-3,
        "ZOOM-16 frames the satellites"
    );
    assert_eq!(frame_body(&app), halden);

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
    assert!(
        (after.tilt - standard_tilt).abs() < 1e-5,
        "CAM-03 standard tilt"
    );
    assert!(
        (after.heading - 2.0 * std::f32::consts::TAU).abs() < 1e-4,
        "north up, the short way"
    );
    assert!(
        (after.distance - before.distance).abs() < 1e-5,
        "scale kept"
    );
    assert_eq!(frame_body(&app), halden, "frame kept");

    // Home.
    app.world_mut().resource_mut::<CameraRig>().tilt = 0.5;
    run(&mut app, 2);
    press_key(&mut app, KeyCode::Home);
    run(&mut app, 40);
    let home = *app.world().resource::<CameraRig>();
    assert!((home.distance - system_distance).abs() < system_distance * 1e-3);
    assert!((home.tilt - standard_tilt).abs() < 1e-5 && home.pivot.length() < 1e-3);
    assert!(in_star_frame(&app), "ZOOM-10: Home sets the star's frame");
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

    app.world_mut().resource_mut::<GameClock>().run_at(16.0);
    run(&mut app, SETTLE_FRAMES);
    let pell = body(&mut app, "Pell");
    assert!(app.world().get::<MotionModeState>(pell).unwrap().ring_mode);
}

/// EDGE-01: turning the heading a full circle in Halden IV's frame
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
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
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
        affiliation: Affiliation::Neutral,
        record: None,
        propulsion: None,
        description: "",
        key_facts: vec![],
        knowledge: Knowledge::Unsurveyed,
        orbit: OrbitPlacement::Own { radius_km },
        radius_km: 1_000.0,
        phase0_rad: 0.0,
        mass_kg: 0.0,
        orbits: vec![],
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
            entry_tolerance_kms: 0.5,
        }],
    }
}

/// SCL-03 / GRP-08: a system from other records runs through the same
/// simulation and view code - bodies placed, zoom stops from its own
/// orbits, its moon folded at the system fit - and its ship in transit,
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
    let distance = fit_distance(system_radius, fov(), TILT, margin);
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

/// GRP-05: at the system fit Halden IV shows a "+N" pill counting all
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
    let clear = ring_outer_half(BodyClass::Planet, &Tunables::default());
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

/// ZOOM-10: Home goes back to the system fit in an animated move, not a
/// cut: partway through, the camera is between where it was and the frame.
#[test]
fn home_is_an_animated_move_to_the_system_frame() {
    let mut app = camera_app();
    let system_distance = app.world().resource::<CameraRig>().distance;
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
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
/// a bookmark saved in a body's frame brings the frame back, and one saved
/// in the star's frame returns the frame to the star (CAM-06, CAM-08).
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
    // In the star's frame the offset is the pivot (CAM-05).
    app.world_mut().resource_mut::<Frame>().offset = saved.pivot;
    run(&mut app, 2);
    save_bookmark(&mut app, KeyCode::Digit1);

    // Move into Halden IV's frame and save that in slot 2.
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    save_bookmark(&mut app, KeyCode::Digit2);
    let framed_distance = app.world().resource::<CameraRig>().distance;

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
    assert!(in_star_frame(&app));

    press_key(&mut app, KeyCode::Digit2);
    run(&mut app, move_frames());
    assert_eq!(frame_body(&app), halden);
    let distance = app.world().resource::<CameraRig>().distance;
    assert!((distance - framed_distance).abs() < framed_distance * 1e-4);
}

/// CAM-20: pan or rotate input during an animated move ends the move and
/// carries on from where the camera is, with no jump.
#[test]
fn input_during_a_move_takes_over_without_a_jump() {
    for interrupt in [KeyCode::KeyQ, KeyCode::KeyD] {
        let mut app = camera_app();
        let halden = body(&mut app, "Halden IV");
        app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
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
/// A body marker's fill colour and whether its class detail shows.
fn marker_fill_and_detail(app: &mut App, body: Entity) -> (Color, bool) {
    let mut markers = app
        .world_mut()
        .query_filtered::<(Entity, &MarkerFor, &Tint), With<MarkerHalfSize>>();
    let (marker, _, tint) = markers
        .iter(app.world())
        .find(|(_, marker, _)| marker.0 == body)
        .expect("a marker for the body");
    let fill = tint.color;
    let mut details = app
        .world_mut()
        .query_filtered::<(&Node, &ChildOf), With<BodyDetail>>();
    let shown = details
        .iter(app.world())
        .filter(|(_, child_of)| child_of.parent() == marker)
        .all(|(node, _)| node.display != Display::None);
    (fill, shown)
}

fn marker_width_px(app: &mut App, body: Entity) -> f32 {
    let mut markers = app
        .world_mut()
        .query_filtered::<(&MarkerFor, &Node), With<MarkerHalfSize>>();
    let (_, node) = markers
        .iter(app.world())
        .find(|(marker, _)| marker.0 == body)
        .expect("a marker for the body");
    match node.width {
        Val::Px(width) => width,
        other => panic!("marker width {other:?}"),
    }
}

/// MRK-07: at the system fit Halden IV is its class's fixed marker; close
/// in, where its true diameter is larger than that, the marker is a disc of
/// the true on-screen size.
#[test]
fn a_body_grows_into_a_true_size_disc_when_close() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            EdgePlugin,
            MarkersPlugin,
            HoveredPlugin,
        ));
    });
    let halden = body(&mut app, "Halden IV");
    run(&mut app, 2);
    let far = marker_width_px(&mut app, halden);
    let planet_px = Tunables::default().planet_marker_px;
    assert_eq!(far, planet_px, "a planet's fixed marker at the system fit");
    // MRK-13: as a marker it is a ground-filled outline with its dot.
    let tunables = Tunables::default();
    assert_eq!(
        marker_fill_and_detail(&mut app, halden),
        (body_fill(false, &tunables), true)
    );

    let distance = 0.01;
    set_frame(&mut app, halden, Vec3::ZERO);
    app.world_mut().resource_mut::<CameraRig>().distance = distance;
    run(&mut app, 3);
    let radius_km = app.world().get::<PhysicalRadius>(halden).unwrap().0;
    let expected =
        2.0 * projected_radius_px(km_to_render(radius_km), distance, fov(), VIEWPORT.y as f32);
    let drawn = marker_width_px(&mut app, halden);
    assert!(
        expected > planet_px,
        "the test view is close enough: {expected}"
    );
    assert!(
        (drawn - expected).abs() < expected * 0.01,
        "drawn {drawn} px, true size {expected} px"
    );
    // MRK-11: the true-size disc is filled, without the marker's dot.
    assert_eq!(
        marker_fill_and_detail(&mut app, halden),
        (body_fill(true, &tunables), false)
    );
}

/// MRK-12: natural bodies are layered by depth, nearer on top, and the
/// order follows the view as it turns; symbols sit above every body and
/// labels above every marker.
#[test]
fn nearer_bodies_draw_over_farther_and_symbols_over_bodies() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            EdgePlugin,
            MarkersPlugin,
            HoveredPlugin,
        ));
    });
    let names = ["Pell", "Iver", "Halden IV", "Morrow"];
    let bodies: Vec<Entity> = names.iter().map(|n| body(&mut app, n)).collect();
    let layers_agree_with_depth = |app: &mut App| {
        run(app, 2);
        let camera = {
            let mut q = app
                .world_mut()
                .query_filtered::<&Transform, With<OrbitCamera>>();
            *q.single(app.world()).unwrap()
        };
        let mut markers = app
            .world_mut()
            .query_filtered::<(&MarkerFor, &GlobalZIndex), With<crate::view::markers::MarkerShape>>(
            );
        let layer_of: std::collections::HashMap<Entity, i32> =
            markers.iter(app.world()).map(|(m, z)| (m.0, z.0)).collect();
        let positions = app.world().resource::<WorldPositions>();
        let depth = |body: Entity| (positions.0[&body] - camera.translation).dot(*camera.forward());
        let mut by_depth = bodies.clone();
        by_depth.sort_by(|a, b| depth(*b).total_cmp(&depth(*a)));
        let layers: Vec<i32> = by_depth.iter().map(|b| layer_of[b]).collect();
        assert!(layers.windows(2).all(|w| w[0] < w[1]), "{layers:?}");
        assert!(layers.iter().all(|&l| l < crate::view::layers::SYMBOLS));
    };
    layers_agree_with_depth(&mut app);
    app.world_mut().resource_mut::<CameraRig>().heading += std::f32::consts::PI;
    layers_agree_with_depth(&mut app);

    let ship = body(&mut app, "Iver Yard");
    let mut z = app.world_mut().query::<(
        &MarkerFor,
        &GlobalZIndex,
        Has<crate::view::markers::MarkerLabel>,
    )>();
    for (marker_for, layer, is_label) in z.iter(app.world()) {
        if marker_for.0 == ship && !is_label {
            assert_eq!(layer.0, crate::view::layers::SYMBOLS);
        }
        if is_label {
            assert_eq!(layer.0, crate::view::layers::LABELS);
        }
    }
}

/// CAM-07: panning a long way in a body's frame stops with the body at
/// least `focus_keep_margin_px` inside the map area, and keeps the frame.
#[test]
fn panning_in_a_frame_keeps_the_body_in_the_map_area() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 3);
    let star = body(&mut app, "Corvane");
    drag_star(&mut app, star, Vec2::new(1590.0, 890.0));
    run(&mut app, 3);
    let screen = app
        .world()
        .resource::<ScreenPositions>()
        .get(halden)
        .unwrap();
    let margin = app.world().resource::<Tunables>().focus_keep_margin_px;
    assert!(
        inside_margin(screen, VIEWPORT.as_vec2(), margin - 1.0),
        "Halden IV at {screen}"
    );
    assert_eq!(frame_body(&app), halden);
}

/// A camera app with the motion rule running at `rate` days a second.
fn motion_camera_app(rate: f64) -> App {
    let mut app = camera_app_with(|app| {
        app.add_plugins(MotionPlugin)
            .insert_resource(placeholder_fonts());
    });
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(rate);
    run(&mut app, SETTLE_FRAMES);
    app
}

/// TEST-09: Morrow IX's own satellites at 1 d/s: Tern Relay is an exposure
/// ring and the trader a dot. Both fold away at the system fit; framing
/// Morrow IX, three levels below the star, moves to its satellite fit,
/// where the trader unfolds.
#[test]
fn a_moon_with_satellites_folds_and_frames_three_levels_deep() {
    let mut app = camera_app_with(|app| {
        app.add_plugins((MotionPlugin, GroupingPlugin))
            .insert_resource(placeholder_fonts());
    });
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(1.0);
    run(&mut app, SETTLE_FRAMES);
    let ix = body(&mut app, "Morrow IX");
    let relay = body(&mut app, "Tern Relay");
    let trader = body(&mut app, "Kessel trader");
    let ring_mode = |app: &App, e: Entity| app.world().get::<MotionModeState>(e).unwrap().ring_mode;
    let folded = |app: &App, e: Entity| app.world().get::<SatelliteFoldState>(e).unwrap().folded;
    assert!(ring_mode(&app, relay));
    assert!(!ring_mode(&app, trader));
    assert!(folded(&app, relay) && folded(&app, trader));

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(ix);
    run(&mut app, 60);
    assert_eq!(frame_body(&app), ix);
    let morrow = body(&mut app, "Morrow");
    assert_eq!(app.world().get::<ChildOf>(ix).unwrap().parent(), morrow);
    assert!(
        app.world()
            .resource::<ZoomStops>()
            .satellite_radius(ix)
            .is_some()
    );
    assert!(
        !folded(&app, trader),
        "the trader unfolds at IX's satellite fit"
    );
}

/// CAM-24 / ZOOM-16: an object in exposure-ring mode can't be a frame; a
/// request for it moves the frame to its parent instead, and the view to
/// the parent's satellite fit.
#[test]
fn a_ring_mode_object_cannot_be_a_frame() {
    let mut app = motion_camera_app(1.0);
    let iva = body(&mut app, "Halden IVa");
    let halden = body(&mut app, "Halden IV");
    assert!(app.world().get::<MotionModeState>(iva).unwrap().ring_mode);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iva);
    run(&mut app, 1);
    let target = app.world().resource::<ActiveMove>().0.unwrap().target;
    run(&mut app, 40);
    assert_eq!(frame_body(&app), halden);
    let fit = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let tilt = app.world().resource::<CameraRig>().tilt;
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let expected = distance_for_radius(fit, fov(), tilt, margin, VIEWPORT.as_vec2());
    assert!((target.distance - expected).abs() < expected * 1e-3);
}

/// CAM-24: when the frame body enters ring mode, the frame moves to its
/// parent and the view moves to the parent's satellite fit.
#[test]
fn the_frame_moves_to_the_parent_when_the_frame_body_becomes_a_ring() {
    let mut app = motion_camera_app(0.1);
    let ivc = body(&mut app, "Halden IVc");
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(ivc);
    run(&mut app, 40);
    assert_eq!(frame_body(&app), ivc);

    // IVc's apparent period drops below tail_secs.
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);
    run(&mut app, eased_frames());
    let new_frame = frame(&app);
    assert_eq!(new_frame.body, halden, "the frame moved to the parent");
    // The view moved to bring Halden IV back in, as a frame jump does.
    let rig = *app.world().resource::<CameraRig>();
    let stops = app.world().resource::<ZoomStops>();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let framed = fit_distance(
        stops.satellite_radius(halden).unwrap(),
        fov(),
        rig.tilt,
        margin,
    );
    assert!((rig.distance - framed).abs() < framed * 1e-3, "framed");
    assert!(new_frame.offset.length() < 1e-3, "centred on the parent");

    // Slowing down again does not move the frame back.
    app.world_mut().resource_mut::<GameClock>().run_at(0.1);
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(frame_body(&app), halden);
}

/// CAM-02: the view starts in the standard orientation: north up, tilted by
/// `standard_tilt_deg`, over the star at the system fit.
#[test]
fn the_view_starts_in_the_standard_orientation() {
    let app = camera_app();
    let rig = *app.world().resource::<CameraRig>();
    let standard = app
        .world()
        .resource::<Tunables>()
        .standard_tilt_deg
        .to_radians();
    assert_eq!((rig.heading, rig.pivot), (0.0, Vec3::ZERO));
    assert!((rig.tilt - standard).abs() < 1e-6);
}

/// ZOOM-13 / CAM-05: zooming in the frame of a fast-moving body keeps it
/// at the same place on screen on every frame of the transition, not just
/// at its end.
#[test]
fn a_zoom_transition_follows_a_fast_frame_body() {
    let mut app = motion_camera_app(5.0);
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    let window = the_window(&mut app);
    for notches in [1.0, -1.0] {
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: notches,
            window,
            phase: TouchPhase::Moved,
        });
        for frame in 0..20 {
            run(&mut app, 1);
            let on_screen = app
                .world()
                .resource::<ScreenPositions>()
                .get(halden)
                .unwrap();
            assert!(
                on_screen.distance(VIEWPORT.as_vec2() / 2.0) < 0.5,
                "frame {frame}: Halden IV at {on_screen}"
            );
        }
    }
}

/// ZOOM-02 / ZOOM-13: big wheel inputs that retarget a zoom in flight, on a
/// fast frame body, never move it off its place on screen.
#[test]
fn rapid_large_wheel_inputs_keep_a_fast_frame_body_in_place() {
    let mut app = motion_camera_app(5.0);
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    let window = the_window(&mut app);
    let notches = [3.0, -2.0, 4.0, -5.0, 2.0, 1.0, -3.0];
    for (index, y) in notches.into_iter().enumerate() {
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y,
            window,
            phase: TouchPhase::Moved,
        });
        for frame in 0..4 {
            run(&mut app, 1);
            let on_screen = app
                .world()
                .resource::<ScreenPositions>()
                .get(halden)
                .unwrap();
            assert!(
                on_screen.distance(VIEWPORT.as_vec2() / 2.0) < 0.5,
                "input {index}, frame {frame}: Halden IV at {on_screen}"
            );
        }
    }
}

/// CAM-24 / CAM-08: a planet's frame with no ancestor left: when the planet
/// turns into an exposure ring, the frame returns to the star at the system
/// fit.
#[test]
fn a_ringed_planet_hands_the_frame_back_to_the_star() {
    let mut app = motion_camera_app(0.1);
    let system_distance = app.world().resource::<CameraRig>().distance;
    let pell = body(&mut app, "Pell");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(pell);
    run(&mut app, move_frames());
    assert_eq!(frame_body(&app), pell);

    app.world_mut().resource_mut::<GameClock>().run_at(16.0);
    run(&mut app, SETTLE_FRAMES + move_frames());
    assert!(in_star_frame(&app));
    let rig = *app.world().resource::<CameraRig>();
    let tilt_ratio = system_distance / rig.distance;
    assert!(
        rig.pivot.length() < 1e-3,
        "centred on the star: {}",
        rig.pivot
    );
    // The system fit at the current tilt (the start-up one is at the
    // standard tilt, which this view still has).
    assert!((tilt_ratio - 1.0).abs() < 1e-3, "system fit");
}

/// ZOOM-16: a frame request for the star, from inside a planet's frame,
/// sets the star's frame and moves to the system fit, keeping heading and
/// tilt.
#[test]
fn a_jump_to_the_star_goes_to_the_system_fit() {
    let mut app = camera_app();
    let system_distance = app.world().resource::<CameraRig>().distance;
    let halden = body(&mut app, "Halden IV");
    let star = body(&mut app, "Corvane");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    app.world_mut().resource_mut::<CameraRig>().heading = 0.9;
    run(&mut app, 2);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(star);
    run(&mut app, move_frames());
    assert!(in_star_frame(&app));
    let rig = *app.world().resource::<CameraRig>();
    assert!((rig.distance - system_distance).abs() < system_distance * 1e-3);
    assert!(rig.pivot.length() < 1e-3);
    assert!((rig.heading - 0.9).abs() < 1e-5, "heading kept");
}

/// CAM-25: the frame body's own breadcrumb entry re-centres it with zero
/// offset at the same scale.
#[test]
fn re_centring_keeps_the_scale_and_clears_the_offset() {
    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, move_frames());
    drag_star(&mut app, halden, Vec2::new(700.0, 400.0));
    assert!(frame(&app).offset.length() > 1e-5);
    let distance = app.world().resource::<CameraRig>().distance;

    app.world_mut().resource_mut::<RecentreRequest>().0 = true;
    run(&mut app, move_frames());
    assert_eq!(frame_body(&app), halden);
    assert!(frame(&app).offset.length() < 1e-5);
    let after = app.world().resource::<CameraRig>().distance;
    assert!((after - distance).abs() < distance * 1e-4, "scale kept");
}

/// The breadcrumb's entry texts, in order, separators left out.
fn breadcrumb_entries(app: &mut App) -> Vec<(Entity, String)> {
    let mut roots = app
        .world_mut()
        .query_filtered::<&Children, With<BreadcrumbRoot>>();
    let children: Vec<Entity> = roots
        .single(app.world())
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .filter_map(|child| {
            let text = app.world().get::<Text>(child)?.0.clone();
            (text.trim() != "/").then_some((child, text))
        })
        .collect()
}

/// CAM-25 / CLK-02: in Arden Highport's frame the breadcrumb reads
/// "Cluster / Corvane / Halden IV / Arden Highport", in the label font
/// (MRK-02); clicking the Halden IV entry makes Halden IV the frame.
#[test]
fn the_breadcrumb_shows_the_frame_path_and_reframes_on_click() {
    let fonts = placeholder_fonts();
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .add_plugins(BreadcrumbPlugin);
    });
    app.world_mut().spawn((BreadcrumbRoot, Node::default()));
    run(&mut app, 2);
    let names: Vec<String> = breadcrumb_entries(&mut app)
        .into_iter()
        .map(|(_, t)| t)
        .collect();
    assert_eq!(names, ["Cluster", "Corvane"]);

    let highport = body(&mut app, "Arden Highport");
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, highport, Vec3::ZERO);
    run(&mut app, 2);
    let entries = breadcrumb_entries(&mut app);
    let names: Vec<&str> = entries.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(names, ["Cluster", "Corvane", "Halden IV", "Arden Highport"]);
    let font = app.world().get::<TextFont>(entries[2].0).unwrap();
    assert_eq!(font.font, fonts.label(1.0).font, "MRK-02");

    let entry = entries[2].0;
    let camera = app.world_mut().spawn_empty().id();
    app.world_mut().trigger(PointerClick {
        entity: entry,
        pointer: bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: VIEWPORT.x,
                    height: VIEWPORT.y,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: Duration::from_millis(80),
        count: 1,
    });
    run(&mut app, move_frames());
    assert_eq!(frame_body(&app), halden);
}

/// A press and release of the left button at `at`, without moving.
fn click_at(app: &mut App, at: Vec2) {
    move_cursor(app, at);
    run(app, 1);
    mouse(app, MouseButton::Left, ButtonState::Pressed);
    run(app, 1);
    mouse(app, MouseButton::Left, ButtonState::Released);
    run(app, 2);
}

/// INS-03: a click on empty map space clears the selection; the release
/// at the end of a pan doesn't.
#[test]
fn an_empty_click_clears_the_selection_and_a_pan_does_not() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .add_plugins(crate::view::hover::HoverPlugin);
    });
    let halden = body(&mut app, "Halden IV");
    let star = body(&mut app, "Corvane");

    app.world_mut().resource_mut::<Selection>().select(halden);
    drag_star(&mut app, star, Vec2::new(900.0, 500.0));
    assert_eq!(app.world().resource::<Selection>().selected, Some(halden));

    click_at(&mut app, Vec2::new(300.0, 700.0));
    assert_eq!(app.world().resource::<Selection>().selected, None);
}

/// HOV-02: every drawn ring is traced for hit-testing, in dot mode too;
/// a folded satellite's isn't, and nor is the frame body's.
#[test]
fn dot_mode_rings_are_hit_targets() {
    let mut app = camera_app_with(|app| {
        // The backend's output, without the rest of picking.
        app.add_message::<bevy::picking::backend::PointerHits>()
            .insert_resource(placeholder_fonts())
            .add_plugins((
                GroupingPlugin,
                MotionPlugin,
                HoveredPlugin,
                SharedOrbitPlugin,
                crate::view::ring_picking::RingPickingPlugin,
            ));
    });
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let ivc = body(&mut app, "Halden IVc");
    assert!(
        !app.world()
            .get::<MotionModeState>(halden)
            .unwrap()
            .ring_mode
    );
    let traced = |app: &mut App, body: Entity| {
        let mut targets = app.world_mut().query::<(Entity, &MarkerFor)>();
        let target = targets
            .iter(app.world())
            .filter(|(_, m)| m.0 == body)
            .map(|(e, _)| e)
            .collect::<Vec<_>>();
        let paths = app
            .world()
            .resource::<crate::view::ring_picking::RingPaths>();
        target.iter().any(|t| paths.0.contains_key(t))
    };
    assert!(traced(&mut app, halden), "a dot-mode planet's ring");
    assert!(!traced(&mut app, ivc), "IVc is folded at the system fit");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 1);
    assert!(!traced(&mut app, halden), "the frame body's ring");
}

/// HOV-02 / HOV-05: framed on a ship in a shared orbit, the shared ring is
/// no target, nor is the frame body's own tail, but the other members'
/// tails still target those members.
#[test]
fn a_frame_body_on_a_shared_ring_leaves_the_others_tails_as_targets() {
    use crate::view::ring_picking::{MemberTailTarget, RingPaths, RingTarget};
    let mut app = camera_app_with(|app| {
        app.add_message::<bevy::picking::backend::PointerHits>()
            .insert_resource(placeholder_fonts())
            .add_plugins((
                GroupingPlugin,
                MotionPlugin,
                HoveredPlugin,
                SharedOrbitPlugin,
                crate::view::ring_picking::RingPickingPlugin,
            ));
    });
    let orison = body(&mut app, "Orison");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    set_frame(&mut app, orison, Vec3::ZERO);
    // Close enough that High's ships are unfolded round Halden IV.
    app.world_mut().resource_mut::<CameraRig>().distance = 1.0;
    run(&mut app, SETTLE_FRAMES);
    assert!(
        !app.world()
            .get::<MotionModeState>(orison)
            .unwrap()
            .ring_mode,
        "High is in dot mode at 1 d/s"
    );
    let mut rings = app.world_mut().query::<(Entity, &RingTarget)>();
    let ring = rings
        .iter(app.world())
        .find(|(_, r)| r.0 == high)
        .unwrap()
        .0;
    let mut tails = app.world_mut().query::<(Entity, &MemberTailTarget)>();
    let tails: Vec<(Entity, Entity)> = tails
        .iter(app.world())
        .filter(|(_, t)| t.orbit == high)
        .map(|(e, t)| (e, t.body))
        .collect();
    let paths = app.world().resource::<RingPaths>();
    assert!(!paths.0.contains_key(&ring), "the shared ring");
    let traced: Vec<Entity> = tails
        .iter()
        .filter(|(e, _)| paths.0.contains_key(e))
        .map(|&(_, b)| b)
        .collect();
    assert!(!traced.contains(&orison), "the frame body's own tail");
    assert!(traced.len() >= 2, "the other members' tails");
}

/// SYM-04 / MRK-03: a ship's symbol fits `ship_symbol_px` at every zoom; it
/// never grows to its true size as natural bodies do.
#[test]
fn a_symbol_keeps_its_size_at_every_zoom() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            EdgePlugin,
            MarkersPlugin,
            HoveredPlugin,
        ));
    });
    let symbol_px = app.world().resource::<Tunables>().ship_symbol_px;
    let orison = body(&mut app, "Orison");
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    let mut shown = 0;
    for distance in [2.0, 1.0, 0.5, 0.25] {
        app.world_mut().resource_mut::<CameraRig>().distance = distance;
        run(&mut app, 20);
        let mut markers = app
            .world_mut()
            .query_filtered::<(&MarkerFor, &Node), With<MarkerHalfSize>>();
        let (_, node) = markers
            .iter(app.world())
            .find(|(m, _)| m.0 == orison)
            .unwrap();
        if node.display == Display::Flex {
            shown += 1;
            assert_eq!(node.width, Val::Px(symbol_px), "at {distance}");
        }
    }
    assert!(shown >= 2, "the symbol was on screen at {shown} zooms");
}

/// MRK-16 / MRK-17: the scale bar is a round length within its maximum;
/// the dial's arrow follows the heading; clicking the dial turns north up
/// as N does (CAM-03).
#[test]
fn the_scale_bar_fits_and_the_north_dial_follows_and_resets_the_heading() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .add_plugins(PlotMarksPlugin);
    });
    run(&mut app, 2);
    let max = Tunables::default().scale_bar_max_px;
    let mut lines = app
        .world_mut()
        .query_filtered::<&Node, With<ScaleBarLine>>();
    let Val::Px(width) = lines.single(app.world()).unwrap().width else {
        panic!("the scale bar has a pixel width");
    };
    // A round length (1, 2 or 5) is at least 0.4 of the next one up.
    assert!(width <= max && width >= 0.4 * max - 1.0, "{width} px");

    let arrow_angle = |app: &mut App| {
        let mut arrows = app
            .world_mut()
            .query_filtered::<&UiTransform, With<NorthArrow>>();
        arrows.single(app.world()).unwrap().rotation.as_radians()
    };
    assert!(arrow_angle(&mut app).abs() < 1e-3, "north up at the start");
    {
        let mut rig = app.world_mut().resource_mut::<CameraRig>();
        rig.heading = 0.5;
        rig.tilt = 0.0;
    }
    run(&mut app, 2);
    assert!(
        (arrow_angle(&mut app).abs() - 0.5).abs() < 1e-2,
        "seen straight down, north turns with the heading"
    );

    let mut dials = app.world_mut().query_filtered::<Entity, With<NorthDial>>();
    let dial = dials.single(app.world()).unwrap();
    let camera = app.world_mut().spawn_empty().id();
    app.world_mut().trigger(PointerClick {
        entity: dial,
        pointer: bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: VIEWPORT.x,
                    height: VIEWPORT.y,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: Duration::from_millis(80),
        count: 1,
    });
    run(&mut app, move_frames());
    let heading = app.world().resource::<CameraRig>().heading;
    assert!(heading.abs() < 1e-3, "heading {heading}");
    assert!(arrow_angle(&mut app).abs() < 1e-3);
}

/// SCL-07 / TEST-10 / HOV-04 / HOV-02: every ship parked at Halden IV
/// sits in its named orbit "High" with no radius of its own, so they share
/// one ring: the orbit is a ring owner and the ships are not, and the ring
/// has one hit target standing for the orbit, traced at Halden IV's
/// satellite fit. "Low" is empty and gets no ring. Arden Highport, with an
/// orbit of its own, is its own ring.
#[test]
fn ships_parked_in_one_named_orbit_share_one_ring() {
    use crate::view::ring_picking::{RingPaths, RingPickingPlugin, RingTarget};
    use crate::view::shared_orbit::OrbitRings;
    use crate::view::targets::OrbitFor;
    use bevy::ecs::system::RunSystemOnce;

    let mut app = camera_app_with(|app| {
        app.add_message::<bevy::picking::backend::PointerHits>()
            .insert_resource(placeholder_fonts())
            .add_plugins((
                GroupingPlugin,
                MotionPlugin,
                HoveredPlugin,
                SharedOrbitPlugin,
                RingPickingPlugin,
            ));
    });
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let ardent = body(&mut app, "Ardent");
    let highport = body(&mut app, "Arden Highport");

    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    assert_eq!(app.world().get::<InNamedOrbit>(ardent).unwrap().0, high);
    let named = app.world().get::<NamedOrbit>(high).unwrap().clone();
    assert_eq!((named.parent, named.name.as_str()), (halden, "High"));
    for ship in [orison, ardent] {
        let orbit = app.world().get::<Orbit>(ship).unwrap();
        assert_eq!(orbit.radius_km, named.radius_km);
        assert_eq!(orbit.period_days, named.period_days);
    }

    let owners = app
        .world_mut()
        .run_system_once(|rings: OrbitRings| rings.owners())
        .unwrap();
    assert!(owners.contains(&high) && owners.contains(&halden));
    assert!(!owners.contains(&orison) && !owners.contains(&ardent));
    let ring = app
        .world_mut()
        .run_system_once(move |rings: OrbitRings| rings.ring(high).unwrap())
        .unwrap();
    assert!(ring.shared);
    let members: Vec<Entity> = ring.members.iter().map(|m| m.body).collect();
    assert_eq!(&members[..2], [orison, ardent]);
    assert!(members.len() >= 13 && members.contains(&body(&mut app, "Sennet")));
    let low = {
        let mut named = app.world_mut().query::<(Entity, &NamedOrbit)>();
        named
            .iter(app.world())
            .find(|(_, o)| o.parent == halden && o.name == "Low")
            .map(|(e, _)| e)
            .expect("Halden IV offers Low")
    };
    assert!(!owners.contains(&low), "an empty named orbit has no ring");

    let mut targets = app
        .world_mut()
        .query::<(Entity, &RingTarget, Option<&OrbitFor>, Option<&MarkerFor>)>();
    let targets: Vec<(Entity, Entity, Option<Entity>, Option<Entity>)> = targets
        .iter(app.world())
        .map(|(e, t, o, m)| (e, t.0, o.map(|o| o.0), m.map(|m| m.0)))
        .collect();
    let shared: Vec<_> = targets.iter().filter(|t| t.1 == high).collect();
    assert_eq!(shared.len(), 1, "one target for the shared orbit");
    assert_eq!((shared[0].2, shared[0].3), (Some(high), None));
    assert!(
        !targets
            .iter()
            .any(|t| t.3 == Some(orison) || t.3 == Some(ardent)),
        "a member of a shared orbit has no ring target of its own"
    );
    assert!(
        targets
            .iter()
            .any(|t| t.3 == Some(highport) && t.2.is_none()),
        "a lone member's orbit is that member's ring"
    );

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + SETTLE_FRAMES);
    assert_eq!(frame_body(&app), halden);
    let paths = app.world().resource::<RingPaths>();
    assert!(
        paths.0.contains_key(&shared[0].0),
        "the shared ring's tails are traced for hit-testing"
    );
}

/// Every entity under `root` in the UI hierarchy.
fn descendants(app: &mut App, root: Entity) -> Vec<Entity> {
    let children = app.world_mut().query::<&Children>();
    let mut found = vec![];
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if let Ok(kids) = children.get_manual(app.world(), e) {
            for kid in kids.iter() {
                found.push(kid);
                stack.push(kid);
            }
        }
    }
    found
}

/// MOT-02 / MOT-14 / SYM-16: every ring has a mark and label: a "+N" pill
/// targeting the orbit for a shared orbit, a symbol for a lone ship or
/// station, a body marker for a lone moon. At 0.2 d/s in Halden IV's
/// frame IVa's ring (1.5 d, 7.5 s apparent) is an exposure ring, so its
/// mark shows, centred on a whole pixel; the parked ships' High band
/// (about 1.9 d, 9.5 s) is in dot mode, so its pill is hidden, and the
/// highport, folded at that fit, shows no mark either.
#[test]
fn an_exposure_ring_carries_an_identity_mark() {
    use crate::view::pills::PillCountText;
    use crate::view::ring_marks::{IdentityPill, RingMarkFor, RingMarksPlugin};
    use crate::view::symbol_shape::SymbolPart;
    use crate::view::targets::OrbitFor;

    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            GroupingPlugin,
            MotionPlugin,
            HoveredPlugin,
            SharedOrbitPlugin,
            RingMarksPlugin,
        ));
    });
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let highport = body(&mut app, "Arden Highport");
    let iva = body(&mut app, "Halden IVa");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;

    let mark_of = |app: &mut App, owner: Entity| {
        let mut marks =
            app.world_mut()
                .query::<(Entity, &RingMarkFor, Option<&OrbitFor>, Option<&MarkerFor>)>();
        marks
            .iter(app.world())
            .find(|(_, m, ..)| m.owner == owner)
            .map(|(e, m, o, k)| (e, m.half, o.map(|o| o.0), k.map(|k| k.0)))
            .unwrap_or_else(|| panic!("no mark for {owner}"))
    };

    let (high_mark, _, orbit_for, marker_for) = mark_of(&mut app, high);
    assert_eq!((orbit_for, marker_for), (Some(high), None));
    let parts = descendants(&mut app, high_mark);
    let pill = parts
        .iter()
        .find(|&&e| app.world().get::<IdentityPill>(e).is_some())
        .expect("a shared orbit's mark is a pill");
    assert_eq!(app.world().get::<IdentityPill>(*pill).unwrap().0, high);
    let count = parts
        .iter()
        .find(|&&e| app.world().get::<PillCountText>(e).is_some())
        .unwrap();
    let in_high = app
        .world()
        .resource::<crate::view::shared_orbit::OrbitMembers>()
        .0[&high]
        .len();
    assert!(in_high >= 13);
    assert_eq!(
        app.world().get::<Text>(*count).unwrap().0,
        format!("+{in_high}")
    );
    assert!(
        parts
            .iter()
            .any(|&e| app.world().get::<Text>(e).is_some_and(|t| t.0 == "High")),
        "the pill is labelled with the orbit's name"
    );

    let (highport_mark, _, orbit_for, marker_for) = mark_of(&mut app, highport);
    assert_eq!((orbit_for, marker_for), (None, Some(highport)));
    assert!(
        descendants(&mut app, highport_mark)
            .iter()
            .any(|&e| app.world().get::<SymbolPart>(e).is_some()),
        "a lone station's mark is its symbol"
    );
    assert!(
        descendants(&mut app, highport_mark).iter().any(|&e| app
            .world()
            .get::<Text>(e)
            .is_some_and(|t| t.0 == "Arden Highport")),
        "a lone ring is labelled with its body's name (MOT-02)"
    );
    let (iva_mark, iva_half, _, marker_for) = mark_of(&mut app, iva);
    assert_eq!(marker_for, Some(iva));
    assert!(
        descendants(&mut app, iva_mark)
            .iter()
            .any(|&e| app.world().get::<BodyRim>(e).is_some()),
        "a lone moon's mark is its body marker"
    );

    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(0.2);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());
    assert_eq!(frame_body(&app), halden);
    let shown = |app: &App, mark: Entity| app.world().get::<Node>(mark).unwrap().display;
    assert_eq!(
        shown(&app, high_mark),
        Display::None,
        "1.9 d at 0.2 d/s is dot mode"
    );
    assert_eq!(
        shown(&app, highport_mark),
        Display::None,
        "folded at this fit"
    );
    assert_eq!(shown(&app, iva_mark), Display::Flex);
    let node = app.world().get::<Node>(iva_mark).unwrap();
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("the mark is placed in pixels");
    };
    let centre = Vec2::new(left + iva_half, top + iva_half);
    assert_eq!(centre, centre.round(), "centred on a whole pixel");
}

/// The outline's rows, as the body or orbit each stands for, in order.
fn outline_rows(app: &mut App) -> Vec<Entity> {
    use crate::view::outline::OutlineRow;
    let mut rows = app.world_mut().query::<(Entity, &OutlineRow)>();
    let mut listed: Vec<(Entity, Entity)> = rows.iter(app.world()).map(|(e, r)| (e, r.0)).collect();
    listed.sort_by_key(|(e, _)| e.index_u32());
    listed.into_iter().map(|(_, body)| body).collect()
}

/// A primary click with `count` presses on a UI node.
fn click_node(app: &mut App, node: Entity, count: u8) {
    let camera = app.world_mut().spawn_empty().id();
    app.world_mut().trigger(PointerClick {
        entity: node,
        pointer: bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: VIEWPORT.x,
                    height: VIEWPORT.y,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: Duration::from_millis(80),
        count,
    });
}

/// INS-05 / HOV-05 / ZOOM-16: the outline is the system from the star
/// down. Selecting Halden IV opens the path to it and its satellites,
/// inside out, with the parked ships under one "Halden IV · High · N" row
/// that a ring click opens; the selected row is highlighted. A click on an
/// entry selects it at once, a double-click asks for a frame jump, and
/// the toggle folds an entry's satellites away while the rest stays.
#[test]
fn the_outline_shows_the_system_tree_and_opens_the_selections_path() {
    use crate::view::camera::requests::PlanetaryViewRequest;
    use crate::view::outline::{Expanded, OutlinePlugin, OutlineRoot, OutlineRow};

    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin, TunablesPlugin, OutlinePlugin))
        .init_resource::<Selection>()
        .init_resource::<Frame>()
        .init_resource::<FrameRequest>()
        .init_resource::<PlanetaryViewRequest>()
        .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    let star = body(&mut app, "Corvane");
    let halden = body(&mut app, "Halden IV");
    let iver = body(&mut app, "Iver");
    let morrow = body(&mut app, "Morrow");
    let iva = body(&mut app, "Halden IVa");
    let ivd = body(&mut app, "Halden IVd");
    let highport = body(&mut app, "Arden Highport");
    let orison = body(&mut app, "Orison");
    let morrow_ix = body(&mut app, "Morrow IX");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    app.world_mut().spawn((OutlineRoot, Node::default()));

    run(&mut app, 2);
    assert_eq!(outline_rows(&mut app), [star], "collapsed to the star");

    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 2);
    let rows = outline_rows(&mut app);
    assert_eq!(rows[0], star);
    let at = |rows: &[Entity], e: Entity| rows.iter().position(|&r| r == e);
    assert!(
        at(&rows, iver) < at(&rows, halden) && at(&rows, halden) < at(&rows, morrow),
        "inside out"
    );
    assert!(
        at(&rows, highport) < at(&rows, iva),
        "the highport is the innermost"
    );
    assert!(
        at(&rows, iva) < at(&rows, high),
        "IVa orbits inside the High band"
    );
    assert!(at(&rows, high) < at(&rows, ivd));
    assert!(!rows.contains(&orison), "the shared orbit is collapsed");
    assert!(!rows.contains(&morrow_ix), "Morrow is not on the path");
    let row_of = |app: &mut App, body: Entity| {
        let mut rows = app.world_mut().query::<(Entity, &OutlineRow)>();
        rows.iter(app.world())
            .find(|(_, r)| r.0 == body)
            .map(|(e, _)| e)
            .unwrap()
    };
    let halden_row = row_of(&mut app, halden);
    assert_ne!(
        app.world().get::<BackgroundColor>(halden_row).unwrap().0,
        Color::NONE
    );
    let iver_row = row_of(&mut app, iver);
    assert_eq!(
        app.world().get::<BackgroundColor>(iver_row).unwrap().0,
        Color::NONE
    );
    let high_row = row_of(&mut app, high);
    let title = descendants(&mut app, high_row)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .find(|t| t.starts_with("Halden IV"))
        .unwrap();
    assert!(title.starts_with("Halden IV · High · 1"), "{title}");

    app.world_mut()
        .resource_mut::<Selection>()
        .select_through_orbit(halden, high);
    run(&mut app, 2);
    assert!(outline_rows(&mut app).contains(&orison));

    // A single click on IVa's label selects it at once.
    let iva_row = row_of(&mut app, iva);
    let label = app.world().get::<Children>(iva_row).unwrap()[1];
    click_node(&mut app, label, 1);
    run(&mut app, 1);
    assert_eq!(app.world().resource::<Selection>().selected, Some(iva));
    assert!(app.world().resource::<FrameRequest>().0.is_none());
    let iva_row = row_of(&mut app, iva);
    let label = app.world().get::<Children>(iva_row).unwrap()[1];
    click_node(&mut app, label, 2);
    run(&mut app, 1);
    assert_eq!(app.world().resource::<FrameRequest>().0, Some(iva));

    // Halden IV's toggle folds its satellites away; the star stays open.
    let halden_row = row_of(&mut app, halden);
    let toggle = app.world().get::<Children>(halden_row).unwrap()[0];
    click_node(&mut app, toggle, 1);
    run(&mut app, 2);
    assert!(!app.world().resource::<Expanded>().entries.contains(&halden));
    let rows = outline_rows(&mut app);
    assert!(rows.contains(&halden) && !rows.contains(&iva));
}

/// INS-05 / INS-07: the sidebar keeps its outline across selections -
/// only the details are rebuilt - and a ring click on a shared orbit
/// opens its row in that same outline.
#[test]
fn the_sidebar_keeps_its_outline_across_selections() {
    use crate::view::fidelity::FidelityImages;
    use crate::view::inspect::{InsetPanel, InspectPlugin};
    use crate::view::outline::{OutlinePlugin, OutlineRoot};
    use crate::view::sidebar::SidebarPlugin;

    let mut app = headless_app(FRAME);
    app.add_plugins((
        ContentPlugin,
        SimPlugin,
        TunablesPlugin,
        BindingsPlugin,
        SidebarPlugin,
        OutlinePlugin,
        InspectPlugin,
    ))
    .init_resource::<Selection>()
    .init_resource::<Frame>()
    .init_resource::<FrameRequest>()
    .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<bevy::input::mouse::AccumulatedMouseScroll>()
    .insert_resource(FidelityImages {
        surveyed: Handle::default(),
        unsurveyed: Handle::default(),
    })
    .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    let halden = body(&mut app, "Halden IV");
    let iver = body(&mut app, "Iver");
    let orison = body(&mut app, "Orison");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    let root = {
        let mut roots = app
            .world_mut()
            .query_filtered::<Entity, With<OutlineRoot>>();
        roots.single(app.world()).unwrap()
    };
    let panel = {
        let mut panels = app.world_mut().query_filtered::<Entity, With<InsetPanel>>();
        panels.single(app.world()).unwrap()
    };
    assert!(
        descendants(&mut app, panel).contains(&root),
        "the outline is in the sidebar"
    );

    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 3);
    assert!(outline_rows(&mut app).contains(&halden));
    assert!(!outline_rows(&mut app).contains(&orison));
    let halden_texts: Vec<String> = descendants(&mut app, panel)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect();
    assert!(halden_texts.contains(&"Halden IV".to_string()));

    app.world_mut().resource_mut::<Selection>().select(iver);
    run(&mut app, 3);
    let mut roots = app
        .world_mut()
        .query_filtered::<Entity, With<OutlineRoot>>();
    assert_eq!(roots.single(app.world()).unwrap(), root, "the same outline");
    assert!(
        outline_rows(&mut app).contains(&halden),
        "Halden IV stays open as left"
    );

    app.world_mut()
        .resource_mut::<Selection>()
        .select_through_orbit(halden, high);
    run(&mut app, 3);
    assert!(outline_rows(&mut app).contains(&orison));
}

/// INS-09: the key facts are label-over-value pairs, the label as small
/// spaced capitals in the secondary colour and the value in the data face
/// in the neutral colour; the "In orbit" section below them has a heading
/// in spaced capitals over a hairline.
#[test]
fn key_facts_are_label_over_value_pairs_under_headed_sections() {
    use crate::view::colors;
    use crate::view::fidelity::FidelityImages;
    use crate::view::inspect::{
        FactLabel, FactValue, Hairline, InsetPanel, InspectPlugin, SectionHeading,
    };
    use crate::view::outline::OutlinePlugin;
    use crate::view::sidebar::SidebarPlugin;

    let mut app = headless_app(FRAME);
    app.add_plugins((
        ContentPlugin,
        SimPlugin,
        TunablesPlugin,
        BindingsPlugin,
        SidebarPlugin,
        OutlinePlugin,
        InspectPlugin,
    ))
    .init_resource::<Selection>()
    .init_resource::<Frame>()
    .init_resource::<FrameRequest>()
    .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<bevy::input::mouse::AccumulatedMouseScroll>()
    .insert_resource(FidelityImages {
        surveyed: Handle::default(),
        unsurveyed: Handle::default(),
    })
    .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    let fonts = placeholder_fonts();
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 3);

    let panel = {
        let mut panels = app.world_mut().query_filtered::<Entity, With<InsetPanel>>();
        panels.single(app.world()).unwrap()
    };
    let parts = descendants(&mut app, panel);
    let labels: Vec<(Entity, String)> = parts
        .iter()
        .filter_map(|&e| app.world().get::<FactLabel>(e).map(|l| (e, l.0.clone())))
        .collect();
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert!(
        names.contains(&"Population") && names.contains(&"Mean radius"),
        "{names:?}"
    );
    let (label, _) = labels.iter().find(|(_, l)| l == "Mean radius").unwrap();
    let letters: Vec<String> = descendants(&mut app, *label)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect();
    assert_eq!(
        letters.concat(),
        "MEANRADIUS",
        "one letter per text, upper case"
    );
    let first = descendants(&mut app, *label)
        .into_iter()
        .find(|&e| app.world().get::<Text>(e).is_some())
        .unwrap();
    assert_eq!(
        app.world().get::<TextColor>(first).unwrap().0,
        colors::SECONDARY
    );
    assert_eq!(
        app.world().get::<TextFont>(first).unwrap().font,
        fonts.label(1.0).font
    );
    // The value follows its label in the same fact, in the data face.
    let fact = app.world().get::<ChildOf>(*label).unwrap().parent();
    let value = app
        .world()
        .get::<Children>(fact)
        .unwrap()
        .iter()
        .find(|&e| app.world().get::<FactValue>(e).is_some())
        .unwrap();
    assert_eq!(app.world().get::<FactValue>(value).unwrap().0, "6,700 km");
    assert_eq!(
        app.world().get::<TextColor>(value).unwrap().0,
        colors::NEUTRAL
    );
    assert_eq!(
        app.world().get::<TextFont>(value).unwrap().font,
        fonts.data(1.0).font
    );
    let row = app.world().get::<ChildOf>(fact).unwrap().parent();
    assert_eq!(
        app.world().get::<Node>(row).unwrap().flex_wrap,
        FlexWrap::Wrap
    );

    let heading = parts
        .iter()
        .find(|&&e| {
            app.world()
                .get::<SectionHeading>(e)
                .is_some_and(|h| h.0 == "In orbit")
        })
        .expect("the outline's heading");
    let hairline = parts
        .iter()
        .find(|&&e| app.world().get::<Hairline>(e).is_some())
        .expect("a hairline above the section");
    assert!(
        hairline.index_u32() < heading.index_u32(),
        "the hairline comes first"
    );
    assert_eq!(app.world().get::<Node>(*hairline).unwrap().height, px(1.0));
}

/// INS-07 / CAM-05 / ZOOM-15: opening the sidebar narrows the camera's
/// viewport, the map area and the fits to the window less its width, and
/// the frame body stays centred in the visible map at the same scale
/// throughout the slide; closing it gives the whole window back.
#[test]
fn the_sidebar_narrows_the_map_and_keeps_the_frame_body_centred() {
    use crate::view::camera::fits::Fits;
    use crate::view::sidebar::SidebarState;
    use bevy::ecs::system::RunSystemOnce;

    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 2);
    let distance = app.world().resource::<CameraRig>().distance;
    fn screen(app: &App) -> &ScreenPositions {
        app.world().resource::<ScreenPositions>()
    }
    assert_eq!(screen(&app).viewport, VIEWPORT.as_vec2());
    let centred = |app: &App| {
        let s = screen(app);
        let x = s.get(halden).unwrap().x;
        assert!(
            (x - s.viewport.x / 2.0).abs() <= 1.0,
            "{x} in {}",
            s.viewport.x
        );
    };
    centred(&app);

    app.world_mut().resource_mut::<Selection>().select(halden);
    let mut seen_mid_slide = false;
    for _ in 0..30 {
        app.update();
        compute_camera_headless(&mut app);
        app.update();
        compute_camera_headless(&mut app);
        let slide = app.world().resource::<SidebarState>().slide;
        if slide > 0.0 && slide < 1.0 {
            seen_mid_slide = true;
            let width = screen(&app).viewport.x;
            assert!(width < VIEWPORT.x as f32 && width > VIEWPORT.x as f32 - 320.0);
            centred(&app);
        }
    }
    assert!(seen_mid_slide, "the slide takes several frames");
    assert_eq!(app.world().resource::<SidebarState>().slide, 1.0);
    assert_eq!(
        screen(&app).viewport,
        Vec2::new(VIEWPORT.x as f32 - 320.0, VIEWPORT.y as f32)
    );
    centred(&app);
    assert_eq!(
        app.world().resource::<CameraRig>().distance,
        distance,
        "same scale"
    );
    let fits_viewport = app
        .world_mut()
        .run_system_once(|fits: Fits| fits.viewport())
        .unwrap();
    assert_eq!(fits_viewport.x, VIEWPORT.x as f32 - 320.0);
    let mut cameras = app
        .world_mut()
        .query_filtered::<&Camera, With<OrbitCamera>>();
    let viewport = cameras
        .single(app.world())
        .unwrap()
        .viewport
        .as_ref()
        .unwrap();
    assert_eq!(
        viewport.physical_size,
        UVec2::new(VIEWPORT.x - 320, VIEWPORT.y)
    );

    app.world_mut().resource_mut::<Selection>().clear();
    for _ in 0..30 {
        app.update();
        compute_camera_headless(&mut app);
    }
    assert_eq!(screen(&app).viewport, VIEWPORT.as_vec2());
    assert!(cameras.single(app.world()).unwrap().viewport.is_none());
    centred(&app);
}

/// LBL-01 / LBL-03 / LBL-04: in Halden IV's frame with its satellites
/// unfolded, every shown label sits clear of every other label and of
/// every marker but its own, the selection's label is never hidden, and
/// labels fade in over `label_fade_ms` rather than appearing at once.
#[test]
fn labels_keep_clear_of_each_other_and_of_markers() {
    use crate::view::labels::{LabelState, LabelsPlugin, label_rect, overlaps};
    use crate::view::markers::{MarkerLabel, MarkersPlugin};
    use crate::view::pills::PillsPlugin;

    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            EdgePlugin,
            GroupingPlugin,
            MotionPlugin,
            MarkersPlugin,
            PillsPlugin,
            HoveredPlugin,
            LabelsPlugin,
        ));
    });
    let halden = body(&mut app, "Halden IV");
    let ivc = body(&mut app, "Halden IVc");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());

    // No layout runs headless: give every marker the size its node asks
    // for, and every label the size a short name has.
    const LABEL_SIZE: Vec2 = Vec2::new(80.0, 14.0);
    fn size_nodes(app: &mut App) {
        let mut markers = app
            .world_mut()
            .query_filtered::<(&Node, &mut ComputedNode), With<MarkerHalfSize>>();
        for (node, mut computed) in markers.iter_mut(app.world_mut()) {
            if let (Val::Px(w), Val::Px(h)) = (node.width, node.height) {
                computed.size = Vec2::new(w, h);
            }
        }
        let mut labels = app
            .world_mut()
            .query_filtered::<&mut ComputedNode, With<MarkerLabel>>();
        for mut computed in labels.iter_mut(app.world_mut()) {
            computed.size = LABEL_SIZE;
        }
    }
    size_nodes(&mut app);
    app.world_mut().resource_mut::<Selection>().select(ivc);
    run(&mut app, 2);
    size_nodes(&mut app);
    run(&mut app, 1);

    let mut labels = app
        .world_mut()
        .query_filtered::<(&LabelState, &ChildOf, &Node, &MarkerFor), With<MarkerLabel>>();
    let fading = labels
        .iter(app.world())
        .any(|(state, ..)| state.shown.is_some() && state.alpha > 0.0 && state.alpha < 1.0);
    assert!(fading, "labels fade in over label_fade_ms");
    run(&mut app, 30);

    let marker_rect = |app: &App, marker: Entity| -> Option<Rect> {
        let node = app.world().get::<Node>(marker)?;
        if node.display == Display::None {
            return None;
        }
        let (Val::Px(l), Val::Px(t), Val::Px(w), Val::Px(h)) =
            (node.left, node.top, node.width, node.height)
        else {
            return None;
        };
        Some(Rect::from_corners(Vec2::new(l, t), Vec2::new(l + w, t + h)))
    };
    let gap = app.world().resource::<Tunables>().label_gap_px;
    let mut shown: Vec<(Entity, Entity, Rect)> = Vec::new();
    let listed: Vec<(LabelState, Entity, Entity)> = labels
        .iter(app.world())
        .map(|(state, child_of, _, marker_for)| (*state, child_of.parent(), marker_for.0))
        .collect();
    for (state, marker, body) in listed {
        let (Some(placement), Some(rect)) = (state.shown, marker_rect(&app, marker)) else {
            continue;
        };
        if state.alpha <= 0.0 {
            continue;
        }
        shown.push((
            marker,
            body,
            label_rect(rect, LABEL_SIZE, placement.slot, gap),
        ));
    }
    assert!(shown.len() >= 3, "{} labels shown", shown.len());
    assert!(
        shown.iter().any(|(_, body, _)| *body == ivc),
        "the selection's label is never hidden"
    );
    let mut markers = app
        .world_mut()
        .query_filtered::<Entity, With<MarkerHalfSize>>();
    let marker_rects: Vec<(Entity, Rect)> = markers
        .iter(app.world())
        .filter_map(|m| Some((m, marker_rect(&app, m)?)))
        .collect();
    for (i, (own, _, rect)) in shown.iter().enumerate() {
        for (j, (_, _, other)) in shown.iter().enumerate() {
            assert!(i == j || !overlaps(*rect, *other), "two labels overprint");
        }
        for (marker, marker_box) in &marker_rects {
            assert!(
                marker == own || !overlaps(*rect, *marker_box),
                "a label overprints a marker"
            );
        }
    }
}

// --- Transfers (docs/spec/transfers.md) ---

/// A headless simulation of Corvane with no view at all.
fn sim_app() -> App {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin));
    run(&mut app, 1);
    app
}

/// The named orbit standing for `body`'s band.
fn band_orbit(app: &mut App, body: Entity, band: Band) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &NamedOrbit, &Band)>();
    query
        .iter(app.world())
        .find(|(_, orbit, b)| orbit.parent == body && **b == band)
        .map(|(e, ..)| e)
        .unwrap_or_else(|| panic!("{band:?} band missing"))
}

fn day(app: &App) -> f64 {
    app.world().resource::<GameClock>().elapsed_days()
}

/// Run the simulation at `rate` days per second until the clock passes
/// `until` days.
fn run_until(app: &mut App, rate: f64, until: f64) {
    app.world_mut().resource_mut::<GameClock>().run_at(rate);
    let mut guard = 0;
    while day(app) < until {
        app.update();
        guard += 1;
        assert!(guard < 200_000, "the clock never reached day {until}");
    }
}

fn schedule(app: &mut App, ship: Entity, order: Order) -> Result<(), Infeasible> {
    use bevy::ecs::system::RunSystemOnce;
    app.world_mut()
        .run_system_once(move |mut planner: Planner| planner.schedule(ship, &order))
        .unwrap()
}

fn preview(app: &mut App, ship: Entity, order: Order) -> Result<SolvedLeg, Infeasible> {
    use bevy::ecs::system::RunSystemOnce;
    app.world_mut()
        .run_system_once(move |planner: Planner| planner.preview(ship, &order))
        .unwrap()
}

fn plan(app: &App, ship: Entity) -> &Plan {
    app.world().get::<Plan>(ship).expect("a plan")
}

fn account(app: &App, ship: Entity) -> DeltaV {
    *app.world().get::<DeltaV>(ship).unwrap()
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
        arrival: Arrival::Stop,
        depart_day,
        travel_days,
    }
}

/// TEST-20 / PLAN-10 / PLAN-21 / RAIL-09 / BAND-05 / BAND-06 / XFER-02:
/// Orison, parked in Halden IV's High band, is ordered down to Low. The
/// plan is Scheduled with its delta-v reserved; on the depart day it is
/// InFlight; at the crossing start the ship burns, leaves its named orbit
/// and flies round Halden IV on its rail; at the crossing end it burns
/// again and joins Low at its arrival direction; on arrival the plan is
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
        panic!("a Stop lands in an orbit");
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

/// The cheapest Pass of Iver's High band for Orison, searching departure
/// days over one synodic period.
fn cheapest_pass_of_iver(app: &mut App, orison: Entity, iver: Entity) -> Order {
    let halden = body(app, "Halden IV");
    let halden_orbit = *app.world().get::<Orbit>(halden).unwrap();
    let iver_orbit = *app.world().get::<Orbit>(iver).unwrap();
    let synodic = orbits::synodic_period(halden_orbit.period_days, iver_orbit.period_days);
    let a = (halden_orbit.radius_km + iver_orbit.radius_km) / 2.0;
    let travel_days = halden_orbit.period_days * (a / halden_orbit.radius_km).powf(1.5) / 2.0;
    let mut best: Option<(Order, f64)> = None;
    let mut depart = 1.0;
    while depart < synodic {
        let order = Order {
            destination: Place::Body {
                body: iver,
                band: Band::High,
            },
            arrival: Arrival::Pass { band: Band::High },
            depart_day: depart,
            travel_days,
        };
        if let Ok(leg) = preview(app, orison, order)
            && best.as_ref().is_none_or(|(_, kms)| leg.delta_v_kms < *kms)
        {
            best = Some((order, leg.delta_v_kms));
        }
        depart += 3.0;
    }
    best.expect("some departure fits the budget").0
}

/// XFER-08 / BAND-07 / RAIL-08 / PLAN-12: Orison passes Iver. At the
/// crossing's end the plan is Done and the ship coasts round the star on
/// the same conic, unchanged by passing through Iver's Hill sphere; its
/// plan still says where it came from and what it did.
#[test]
fn a_pass_leaves_the_ship_coasting_unchanged_by_the_planet() {
    use crate::sim::transfer::tree::{ShipState, TreeQuery};
    use bevy::ecs::system::RunSystemOnce;

    let mut app = sim_app();
    let halden = body(&mut app, "Halden IV");
    let iver = body(&mut app, "Iver");
    let orison = body(&mut app, "Orison");
    let order = cheapest_pass_of_iver(&mut app, orison, iver);
    schedule(&mut app, orison, order).unwrap();
    let leg = plan(&app, orison).leg().clone();
    assert_eq!(leg.arrival, Arrival::Pass { band: Band::High });
    assert!(leg.pass_speed_kms.is_some());
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

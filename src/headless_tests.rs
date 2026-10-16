//! Headless integration tests (ARCH-13, ARCH-31): real plugins in an `App`
//! with `MinimalPlugins`, no window and no GPU.
//!
//! The view's grouping needs a camera that can project, which normally
//! takes the renderer; `test_camera` fills in the camera's computed values
//! by hand, as Bevy's own camera tests do, so the real grouping systems run
//! unchanged. Input is simulated as messages (keys, mouse buttons, wheel,
//! motion), so camera tests drive the real handlers. Each test's doc line
//! names the IDs it covers.
//!
//! The tests are grouped by area in `headless_tests/`; the app builders,
//! input helpers and constants they share are here.

use std::time::Duration;

use bevy::camera::RenderTargetInfo;
use bevy::input::InputPlugin;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::WindowResolution;

use crate::content::ContentPlugin;
use crate::sim::SimPlugin;
use crate::sim::body::{Affiliation, BodyClass, DisplayName, Knowledge, PhysicalRadius};
use crate::sim::clock::GameClock;
use crate::sim::named_orbit::{InNamedOrbit, NamedOrbit};
use crate::sim::orbit::{Orbit, OrbitPositionKm, orbit_position_km};
use crate::sim::rail::SegmentKind;
use crate::sim::ship::{DeltaV, InFlight};
use crate::sim::system::{BodyBlueprint, OrbitPlacement, SystemDataResource};
use crate::sim::track::ShipRecord;
use crate::sim::transfer::order::{Arrival, Infeasible, Order, SolvedLeg};
use crate::sim::transfer::places::{Band, Place};
use crate::sim::transfer::plan::{Plan, Planner, Status};
use crate::sim::units::KM_PER_AU;
use crate::view::bindings::BindingsPlugin;
use crate::view::camera::CameraPlugin;
use crate::view::camera::requests::FrameRequest;
use crate::view::camera::state::{ActiveMove, CameraRig, OrbitCamera};
use crate::view::camera::stops::{StopsPlugin, ZoomStops, distance_for_radius};
use crate::view::edge::EdgePlugin;
use crate::view::fold::SatelliteFoldState;
use crate::view::fonts::Fonts;
use crate::view::frame::{Frame, FramePlugin};
use crate::view::gesture::PressStart;
use crate::view::grouping::GroupingPlugin;
use crate::view::hovered::HoveredPlugin;
use crate::view::markers::pills::PillsPlugin;
use crate::view::markers::{MarkerHalfSize, MarkersPlugin};
use crate::view::motion::MotionPlugin;
use crate::view::motion_mode::MotionModeState;
use crate::view::planner::state::PlannerState;
use crate::view::plot_marks::{NorthArrow, PlotMarksPlugin, ScaleBarLine};
use crate::view::projection::ProjectionPlugin;
use crate::view::scale::{km_to_render, km_to_render_vec3};
use crate::view::screen::{ScreenPlugin, ScreenPositions};
use crate::view::selection::Selection;
use crate::view::shared_orbit::SharedOrbitPlugin;
use crate::view::symbols::draw::Tint;
use crate::view::targets::{GroupPillFor, Hovered, MarkerFor};
use crate::view::topbar::TOP_BAR_HEIGHT_PX;
use crate::view::topbar::breadcrumb::{BreadcrumbPlugin, BreadcrumbRoot};
use crate::view::true_size::{TrueSizePlugin, projected_radius_px};
use crate::view::tunables::{Tunables, TunablesPlugin};
use crate::view::world::WorldPositions;
use crate::{Boot, Stage};

mod brackets;
mod camera;
mod frame_cue;
mod grouping;
mod inset;
mod markers;
mod paths;
mod planner;
mod rings;
mod symbols;
mod systems;
mod transfers;

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
        .add_plugins((ScreenPlugin, TrueSizePlugin))
        .init_resource::<Selection>()
        .init_resource::<PlannerState>()
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

fn blueprint(name: &'static str, class: BodyClass, radius_km: f64) -> BodyBlueprint {
    BodyBlueprint {
        name,
        class,
        affiliation: Affiliation::Neutral,
        record: None,
        propulsion: None,
        group: None,
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

/// Frames in one `focus_jump_ms` move, plus a few to settle.
fn move_frames() -> usize {
    let ms = Tunables::default().focus_jump_ms;
    (ms / 1000.0 / FRAME.as_secs_f32()).ceil() as usize + 5
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

/// The cheapest Intercept of Iver's High band for Orison, searching departure
/// days over one synodic period.
fn cheapest_intercept_of_iver(app: &mut App, orison: Entity, iver: Entity) -> Order {
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
            arrival: Arrival::Intercept { band: Band::High },
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

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_element() < 0.01
}

/// Corvane in a headless app with markers, pills and labels.
fn symbols_app_with(extra: impl FnOnce(&mut App)) -> App {
    use crate::view::markers::labels::LabelsPlugin;
    camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            EdgePlugin,
            GroupingPlugin,
            MotionPlugin,
            MarkersPlugin,
            PillsPlugin,
            HoveredPlugin,
            LabelsPlugin,
        ));
        extra(app);
    })
}

/// The marker node that stands for `body` (not its label).
fn marker_of(app: &mut App, body: Entity) -> Entity {
    use crate::view::markers::MarkerShape;
    let mut markers = app
        .world_mut()
        .query_filtered::<(Entity, &MarkerFor), With<MarkerShape>>();
    let found: Vec<Entity> = markers
        .iter(app.world())
        .filter(|(_, marker_for)| marker_for.0 == body)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(found.len(), 1, "one marker per body");
    found[0]
}

/// The texts under `root` that carry the marker component `T`.
fn texts_with<T: Component>(app: &mut App, root: Entity) -> Vec<String> {
    descendants(app, root)
        .into_iter()
        .filter(|&node| app.world().get::<T>(node).is_some())
        .filter_map(|node| app.world().get::<Text>(node).map(|t| t.0.clone()))
        .collect()
}

/// No layout runs headless: give every marker the size its node asks
/// for and every label `label_size`, and let the labels settle.
fn settle_labels(app: &mut App, label_size: Vec2) {
    use crate::view::markers::MarkerLabel;
    for _ in 0..3 {
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
            computed.size = label_size;
        }
        run(app, 2);
    }
}

/// The planner's search for `ship` and `request`, as the simulation
/// stands.
fn search(
    app: &mut App,
    ship: Entity,
    request: crate::sim::transfer::search::Request,
) -> (
    crate::sim::transfer::search::SearchContext,
    Result<crate::sim::transfer::search::Search, Infeasible>,
) {
    use bevy::ecs::system::RunSystemOnce;
    let context = app
        .world_mut()
        .run_system_once(move |planner: Planner| planner.search_context(ship))
        .unwrap()
        .expect("the ship can be planned for");
    let found = context.search(request);
    (context, found)
}

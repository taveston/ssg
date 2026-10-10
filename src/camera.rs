//! Hand-rolled orbit/zoom/pan camera.
//!
//! Zoom is stepped and animated (`docs/spec/system-view.md`, ZOOM-01 to
//! ZOOM-12): each wheel notch multiplies the view scale by a
//! fixed factor over an ease-out animation, anchored on the cursor (or on
//! the selection), resisting slightly at stops that frame something
//! meaningful and refusing to go past the system or satellite frames.
//!
//! Tilt (right drag) and pan (middle drag) are unchanged from iteration 1.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

use crate::Stage;
use crate::hover_inspect::Inspections;
use crate::system_data::BodyClass;
use crate::tunables::Tunables;
use crate::zoom::{
    animated_distance, first_stop_crossed, notches_from_scroll, step_duration_ms, stepped_distance,
};
use crate::zoom_stops::{ZoomStops, distance_for_radius, radius_for_distance};

#[derive(Resource, Clone, Copy)]
pub enum CameraFocus {
    Fixed(Vec3),
    Following(Entity),
}

impl Default for CameraFocus {
    fn default() -> Self {
        CameraFocus::Fixed(Vec3::ZERO)
    }
}

#[derive(Resource)]
struct OrbitCameraState {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for OrbitCameraState {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            // Steep, near-top-down by default.
            pitch: 1.15,
            // Replaced at startup by the system frame, once the stops have
            // been computed from the real orbits.
            distance: 300.0,
        }
    }
}

/// A request to frame a body's satellites, raised by a double-click or the
/// focus key and consumed by the camera next frame. Keeps the pointer
/// observers from needing the camera's whole system-parameter set.
#[derive(Resource, Default)]
pub struct FocusJumpRequest(pub Option<Entity>);

/// Set when the player double-clicks a body that's already inspected. The
/// planetary view doesn't exist yet; this is the hook it will hang from.
#[derive(Resource, Default)]
pub struct PlanetaryViewRequest(pub Option<Entity>);

/// Saved views: scale plus centre (or followed body), per the spec.
#[derive(Resource, Default)]
pub struct Bookmarks(pub [Option<(f32, CameraFocus)>; 4]);

/// An in-flight zoom (or focus jump). New input retargets this from the
/// *current* interpolated values rather than queueing behind it.
#[derive(Resource, Default)]
pub struct ZoomAnim {
    start_distance: f32,
    target_distance: f32,
    start_focus: Vec3,
    target_focus: CameraFocus,
    elapsed: f32,
    duration: f32,
}

impl ZoomAnim {
    fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }
}

/// Absolute safety rails, independent of the computed stops.
const MIN_DISTANCE: f32 = 0.002;
const MAX_DISTANCE: f32 = 5_000.0;
const PITCH_LIMIT: f32 = 1.55;

#[derive(Component)]
struct OrbitCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraFocus>()
            .init_resource::<OrbitCameraState>()
            .init_resource::<ZoomAnim>()
            .init_resource::<FocusJumpRequest>()
            .init_resource::<PlanetaryViewRequest>()
            .init_resource::<Bookmarks>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Startup,
                start_at_system_frame
                    .after(spawn_camera)
                    .after(crate::zoom_stops::compute_stops),
            )
            .add_systems(
                Update,
                (
                    handle_orbit_input,
                    handle_zoom_input,
                    handle_navigation_keys,
                    handle_focus_jump,
                    handle_bookmarks,
                    handle_pan_input,
                    advance_zoom_anim,
                    update_camera_transform,
                )
                    .chain()
                    .in_set(Stage::Camera),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        OrbitCamera,
        Camera3d::default(),
        // The zoom range spans a moon's orbit up to the whole system, so
        // the default near/far pair (0.1 / 1000) would clip at both ends.
        Projection::Perspective(PerspectiveProjection {
            near: 0.001,
            far: 20_000.0,
            ..default()
        }),
        Transform::from_xyz(0.0, 40.0, 70.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn camera_fov(projection: &Projection) -> f32 {
    match projection {
        Projection::Perspective(perspective) => perspective.fov,
        _ => std::f32::consts::FRAC_PI_4,
    }
}

fn focus_position(focus: &CameraFocus, transforms: &Query<&GlobalTransform>) -> Vec3 {
    match *focus {
        CameraFocus::Fixed(pos) => pos,
        CameraFocus::Following(entity) => transforms
            .get(entity)
            .map(GlobalTransform::translation)
            .unwrap_or(Vec3::ZERO),
    }
}

fn start_at_system_frame(
    stops: Res<ZoomStops>,
    tunables: Res<Tunables>,
    mut state: ResMut<OrbitCameraState>,
    camera: Query<&Projection, With<OrbitCamera>>,
) {
    let Ok(projection) = camera.single() else {
        return;
    };
    state.distance = distance_for_radius(
        stops.system_radius,
        camera_fov(projection),
        state.pitch,
        tunables.zoom_fit_margin,
    );
}

fn handle_orbit_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut state: ResMut<OrbitCameraState>,
) {
    if !mouse_buttons.pressed(MouseButton::Right) {
        return;
    }
    let delta = mouse_motion.delta;
    state.yaw -= delta.x * 0.005;
    state.pitch = (state.pitch - delta.y * 0.005).clamp(-PITCH_LIMIT, PITCH_LIMIT);
}

/// Where the world point under the cursor is, on the orbital plane.
fn cursor_anchor(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
) -> Option<Vec3> {
    let cursor = window.cursor_position()?;
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
}

/// Every distance that counts as a stop, given what's selected.
fn stop_distances(
    stops: &ZoomStops,
    tunables: &Tunables,
    fov: f32,
    pitch: f32,
    selected: Option<Entity>,
) -> Vec<f32> {
    let to_distance =
        |radius: f32| distance_for_radius(radius, fov, pitch, tunables.zoom_fit_margin);

    let mut distances = vec![
        to_distance(stops.system_radius),
        to_distance(stops.inner_system_radius),
    ];
    if let Some(radius) = selected.and_then(|entity| stops.satellite_radius(entity)) {
        distances.push(to_distance(radius));
    }
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    distances
}

/// The innermost distance the player may reach: the satellite frame of the
/// selected body, or of the body nearest the cursor when nothing is
/// selected.
fn inner_limit(
    stops: &ZoomStops,
    tunables: &Tunables,
    fov: f32,
    pitch: f32,
    selected: Option<Entity>,
    anchor: Option<Vec3>,
    transforms: &Query<&GlobalTransform>,
) -> f32 {
    let radius = selected
        .and_then(|entity| stops.satellite_radius(entity))
        .or_else(|| {
            let anchor = anchor?;
            stops
                .per_body_satellite_radius
                .iter()
                .filter_map(|(entity, radius)| {
                    let position = transforms.get(*entity).ok()?.translation();
                    Some((position.distance(anchor), *radius))
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, radius)| radius)
        });

    match radius {
        Some(radius) => {
            distance_for_radius(radius, fov, pitch, tunables.zoom_fit_margin).max(MIN_DISTANCE)
        }
        None => MIN_DISTANCE,
    }
}

/// How far into the target frame a body has to be for a zoom to latch
/// onto it, as a fraction of the frame radius. Keeps zooming over empty
/// space from grabbing a distant planet.
const LATCH_FRACTION: f32 = 0.25;

/// The nearest body to `anchor` within `max_distance`, if any.
fn nearest_body(anchor: Vec3, max_distance: f32, candidates: &[(Entity, Vec3)]) -> Option<Entity> {
    candidates
        .iter()
        .map(|(entity, position)| (*entity, position.distance(anchor)))
        .filter(|(_, distance)| *distance <= max_distance)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(entity, _)| entity)
}

/// Point the camera at a new distance (and optionally a new focus),
/// restarting the animation from wherever it currently is.
fn retarget(
    anim: &mut ZoomAnim,
    state: &OrbitCameraState,
    current_focus: Vec3,
    target_distance: f32,
    target_focus: CameraFocus,
    duration_ms: f32,
) {
    anim.start_distance = state.distance;
    anim.target_distance = target_distance;
    anim.start_focus = current_focus;
    anim.target_focus = target_focus;
    anim.elapsed = 0.0;
    anim.duration = (duration_ms / 1000.0).max(f32::EPSILON);
}

#[allow(clippy::too_many_arguments)]
fn handle_zoom_input(
    mouse_scroll: Res<AccumulatedMouseScroll>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    inspections: Res<Inspections>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform, &Projection), With<OrbitCamera>>,
    transforms: Query<&GlobalTransform>,
    bodies: Query<(Entity, &GlobalTransform, &BodyClass)>,
    state: Res<OrbitCameraState>,
    focus: Res<CameraFocus>,
    mut anim: ResMut<ZoomAnim>,
) {
    // ZOOM-01: whole notches only, converted from the device's own units -
    // a raw pixel-unit delta would zoom by many steps per event. Continuous
    // (trackpad pinch) input would hook in here instead, settling to the
    // nearest stop on release; deferred (ZOOM-07) until there's hardware to
    // test on.
    let notches = notches_from_scroll(
        mouse_scroll.delta.y,
        mouse_scroll.unit == MouseScrollUnit::Pixel,
    );
    if notches == 0.0 {
        return;
    }
    let Ok((camera, camera_transform, projection)) = camera_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };

    let fov = camera_fov(projection);
    let selected = inspections.primary;
    let current_focus = focus_position(&focus, &transforms);

    // Zoom towards the selected object if there is one, else the cursor.
    let anchor = selected
        .and_then(|entity| transforms.get(entity).ok())
        .map(GlobalTransform::translation)
        .or_else(|| cursor_anchor(camera, camera_transform, window));

    // Anything the view can centre on. Gates are excluded: they're edge
    // indicators sitting far out, not somewhere to fly to.
    let followable: Vec<(Entity, Vec3)> = bodies
        .iter()
        .filter(|(_, _, class)| **class != BodyClass::Gate)
        .map(|(entity, transform, _)| (entity, transform.translation()))
        .collect();

    // ZOOM-01: each notch multiplies the scale by the step factor.
    let naive_target = stepped_distance(state.distance, notches, tunables.zoom_step_factor);

    let outer = distance_for_radius(
        stops.system_radius,
        fov,
        state.pitch,
        tunables.zoom_fit_margin,
    )
    .min(MAX_DISTANCE);
    let inner = inner_limit(
        &stops,
        &tunables,
        fov,
        state.pitch,
        selected,
        anchor,
        &transforms,
    );

    // ZOOM-05: resist at any stop the step would skip over - settle exactly
    // on it instead, and take a little longer doing it.
    let crossed = first_stop_crossed(
        state.distance,
        naive_target,
        &stop_distances(&stops, &tunables, fov, state.pitch, selected),
    );
    let duration = step_duration_ms(
        tunables.zoom_anim_ms,
        tunables.zoom_stop_resistance,
        crossed.is_some(),
    );
    let target = crossed
        .unwrap_or(naive_target)
        .clamp(inner.min(outer), outer);

    let target_focus = match anchor {
        Some(anchor) if state.distance > f32::EPSILON => {
            // Zooming in towards a body latches onto it. Bodies keep
            // orbiting while you zoom, and at satellite scale a planet
            // crosses the whole view in about a second - a fixed centre
            // would simply lose it.
            let zooming_in = target < state.distance;
            let view_radius =
                radius_for_distance(target, fov, state.pitch, tunables.zoom_fit_margin);
            let latched = zooming_in
                .then(|| nearest_body(anchor, view_radius * LATCH_FRACTION, &followable))
                .flatten();

            match latched {
                Some(entity) => CameraFocus::Following(entity),
                // Otherwise hold the anchor point still on screen as the
                // distance changes.
                None => CameraFocus::Fixed(
                    anchor + (current_focus - anchor) * (target / state.distance),
                ),
            }
        }
        _ => *focus,
    };

    retarget(
        &mut anim,
        &state,
        current_focus,
        target,
        target_focus,
        duration,
    );
}

#[allow(clippy::too_many_arguments)]
fn handle_navigation_keys(
    keys: Res<ButtonInput<KeyCode>>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    camera_q: Query<&Projection, With<OrbitCamera>>,
    transforms: Query<&GlobalTransform>,
    state: Res<OrbitCameraState>,
    focus: Res<CameraFocus>,
    mut anim: ResMut<ZoomAnim>,
) {
    if !keys.just_pressed(KeyCode::Home) {
        return;
    }
    let Ok(projection) = camera_q.single() else {
        return;
    };
    let target = distance_for_radius(
        stops.system_radius,
        camera_fov(projection),
        state.pitch,
        tunables.zoom_fit_margin,
    );
    retarget(
        &mut anim,
        &state,
        focus_position(&focus, &transforms),
        target,
        CameraFocus::Fixed(Vec3::ZERO),
        tunables.focus_jump_ms,
    );
}

/// Frame a body's satellites in one pan-and-zoom. Raised by a double-click
/// on a body, or by the focus key for the current selection.
#[allow(clippy::too_many_arguments)]
fn handle_focus_jump(
    keys: Res<ButtonInput<KeyCode>>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    inspections: Res<Inspections>,
    camera_q: Query<&Projection, With<OrbitCamera>>,
    transforms: Query<&GlobalTransform>,
    state: Res<OrbitCameraState>,
    focus: Res<CameraFocus>,
    mut request: ResMut<FocusJumpRequest>,
    mut anim: ResMut<ZoomAnim>,
) {
    // The focus key does the same thing for whatever's selected.
    let target_body = request.0.take().or_else(|| {
        keys.just_pressed(KeyCode::KeyF)
            .then_some(inspections.primary)
            .flatten()
    });
    let Some(body) = target_body else {
        return;
    };
    let Ok(projection) = camera_q.single() else {
        return;
    };

    // A body with no satellites still gets a sensible close-up rather than
    // nothing happening.
    let radius = stops
        .satellite_radius(body)
        .unwrap_or(stops.inner_system_radius * 0.05);
    let distance = distance_for_radius(
        radius,
        camera_fov(projection),
        state.pitch,
        tunables.zoom_fit_margin,
    );

    retarget(
        &mut anim,
        &state,
        focus_position(&focus, &transforms),
        distance.max(MIN_DISTANCE),
        CameraFocus::Following(body),
        tunables.focus_jump_ms,
    );
}

/// Ctrl+1..4 save the current view; 1..4 recall it.
#[allow(clippy::too_many_arguments)]
fn handle_bookmarks(
    keys: Res<ButtonInput<KeyCode>>,
    tunables: Res<Tunables>,
    transforms: Query<&GlobalTransform>,
    state: Res<OrbitCameraState>,
    focus: Res<CameraFocus>,
    mut bookmarks: ResMut<Bookmarks>,
    mut anim: ResMut<ZoomAnim>,
) {
    const SLOTS: [KeyCode; 4] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ];
    let saving = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    for (index, key) in SLOTS.iter().enumerate() {
        if !keys.just_pressed(*key) {
            continue;
        }
        if saving {
            // Scale and centre (or followed body) only - not the tilt.
            bookmarks.0[index] = Some((state.distance, *focus));
        } else if let Some((distance, saved_focus)) = bookmarks.0[index] {
            retarget(
                &mut anim,
                &state,
                focus_position(&focus, &transforms),
                distance,
                saved_focus,
                tunables.focus_jump_ms,
            );
        }
    }
}

fn advance_zoom_anim(
    time: Res<Time>,
    transforms: Query<&GlobalTransform>,
    mut anim: ResMut<ZoomAnim>,
    mut state: ResMut<OrbitCameraState>,
    mut focus: ResMut<CameraFocus>,
) {
    if anim.duration <= 0.0 || anim.finished() {
        return;
    }
    anim.elapsed += time.delta_secs();
    let t = (anim.elapsed / anim.duration).clamp(0.0, 1.0);
    let eased = crate::anim::ease_out_cubic(t);

    // ZOOM-01: the step eases out onto its target distance.
    state.distance = animated_distance(
        anim.start_distance,
        anim.target_distance,
        anim.elapsed,
        anim.duration,
    )
    .clamp(MIN_DISTANCE, MAX_DISTANCE);

    // Blend towards where the target focus is *now*, so following a moving
    // body works, then hand over the real focus once we arrive.
    let target_position = focus_position(&anim.target_focus, &transforms);
    if t >= 1.0 {
        *focus = anim.target_focus;
    } else {
        *focus = CameraFocus::Fixed(anim.start_focus.lerp(target_position, eased));
    }
}

fn handle_pan_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    state: Res<OrbitCameraState>,
    mut focus: ResMut<CameraFocus>,
    mut anim: ResMut<ZoomAnim>,
    transforms: Query<&GlobalTransform>,
) {
    if !mouse_buttons.pressed(MouseButton::Middle) {
        return;
    }
    let current = focus_position(&focus, &transforms);
    let delta = mouse_motion.delta;
    if delta == Vec2::ZERO {
        // Still dropping a moving focus to a fixed point the moment panning starts.
        if !matches!(*focus, CameraFocus::Fixed(_)) {
            *focus = CameraFocus::Fixed(current);
        }
        return;
    }

    // Panning takes over from any in-flight zoom's focus travel.
    anim.target_focus = CameraFocus::Fixed(current);
    anim.start_focus = current;

    let yaw_rot = Quat::from_rotation_y(state.yaw);
    let right = yaw_rot * Vec3::X;
    let forward_flat = yaw_rot * Vec3::Z;
    let pan_speed = state.distance * 0.0015;
    let pan = (-right * delta.x + forward_flat * delta.y) * pan_speed;
    *focus = CameraFocus::Fixed(current + pan);
}

fn update_camera_transform(
    focus: Res<CameraFocus>,
    state: Res<OrbitCameraState>,
    transforms: Query<&GlobalTransform>,
    mut camera: Query<&mut Transform, With<OrbitCamera>>,
) {
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };
    let target = focus_position(&focus, &transforms);
    let offset = Vec3::new(
        state.distance * state.pitch.cos() * state.yaw.sin(),
        state.distance * state.pitch.sin(),
        state.distance * state.pitch.cos() * state.yaw.cos(),
    );
    camera_transform.translation = target + offset;
    camera_transform.look_at(target, Vec3::Y);
}

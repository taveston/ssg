//! Camera input: wheel, keys and mouse drags turned into camera moves.
//! Everything here reads input and calls into `state.rs`; the camera
//! iteration (CAM-10 to CAM-18) replaces this module without touching the
//! rest of the camera.
//!
//! Current bindings, from iteration 3: wheel zooms (ZOOM-01, ZOOM-02,
//! ZOOM-05); right drag tilts and turns; middle drag pans; Home returns to
//! the system frame (ZOOM-10); F frames the selection's satellites;
//! Ctrl+1 to Ctrl+4 save and 1 to 4 recall bookmarks (ZOOM-11).

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

use super::requests::FocusJumpRequest;
use super::state::{
    Bookmarks, CameraFocus, MAX_DISTANCE, MIN_DISTANCE, OrbitCamera, OrbitCameraState, PITCH_LIMIT,
    ZoomAnim, camera_fov, focus_position, retarget,
};
use super::stops::{ZoomStops, distance_for_radius, radius_for_distance};
use super::zoom::{first_stop_crossed, notches_from_scroll, step_duration_ms, stepped_distance};
use crate::sim::body::BodyClass;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;

pub fn handle_orbit_input(
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

#[allow(clippy::too_many_arguments)]
pub fn handle_zoom_input(
    mouse_scroll: Res<AccumulatedMouseScroll>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    selection: Res<Selection>,
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
    let selected = selection.primary;
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
pub fn handle_navigation_keys(
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
pub fn handle_focus_jump(
    keys: Res<ButtonInput<KeyCode>>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    selection: Res<Selection>,
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
            .then_some(selection.primary)
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
pub fn handle_bookmarks(
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

pub fn handle_pan_input(
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

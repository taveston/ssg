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
use super::zoom::{StepSettings, ZoomStep, anchored_centre, notches_from_scroll, plan_zoom_step};
use crate::sim::body::BodyClass;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;

/// Right drag: radians of yaw or pitch per pixel dragged (iteration 1
/// behaviour; CAM-12 replaces it with `rotate_deg_per_px`).
const DRAG_ROTATE_RAD_PER_PX: f32 = 0.005;
/// Middle drag: plane distance per pixel dragged, as a fraction of the
/// camera distance, so a pan feels the same at every zoom (iteration 1
/// behaviour; CAM-10 replaces it with an exact cursor-locked pan).
const DRAG_PAN_PER_PX: f32 = 0.0015;
/// A body with no satellites still gets a sensible close-up when framed:
/// this fraction of the region frame.
const BARE_BODY_FRAME_FRACTION: f32 = 0.05;

pub fn handle_orbit_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut state: ResMut<OrbitCameraState>,
) {
    if !mouse_buttons.pressed(MouseButton::Right) {
        return;
    }
    let delta = mouse_motion.delta;
    state.yaw -= delta.x * DRAG_ROTATE_RAD_PER_PX;
    state.pitch = (state.pitch - delta.y * DRAG_ROTATE_RAD_PER_PX).clamp(-PITCH_LIMIT, PITCH_LIMIT);
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

/// Every distance that counts as a stop, given what's selected: the system
/// frame, the region frame and the selection's satellite frame. That is the
/// withdrawn ZOOM-04's choice; ZOOM-15 moves it to focus (F4-13).
fn stop_distances(
    stops: &ZoomStops,
    selected: Option<Entity>,
    fit: impl Fn(f32) -> f32,
) -> Vec<f32> {
    let mut distances = vec![fit(stops.system_radius), fit(stops.inner_system_radius)];
    if let Some(radius) = selected.and_then(|entity| stops.satellite_radius(entity)) {
        distances.push(fit(radius));
    }
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    distances
}

/// The innermost distance the player may reach: the satellite frame of the
/// selected body, or of the body nearest the anchor when nothing is
/// selected. That is the withdrawn ZOOM-06; ZOOM-14 moves it to focus
/// (F4-13).
fn inner_limit(
    stops: &ZoomStops,
    selected: Option<Entity>,
    anchor: Option<Vec3>,
    transforms: &Query<&GlobalTransform>,
    fit: impl Fn(f32) -> f32,
) -> f32 {
    let nearest_frame = || {
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
    };
    selected
        .and_then(|entity| stops.satellite_radius(entity))
        .or_else(nearest_frame)
        .map_or(MIN_DISTANCE, |radius| fit(radius).max(MIN_DISTANCE))
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

/// ZOOM-01, ZOOM-02, ZOOM-05: each wheel notch is one animated zoom step,
/// retargeted from wherever the view is now. Gathers the inputs, plans the
/// step (`zoom::plan_zoom_step`), picks the centre and starts the move.
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
    // ZOOM-01: whole notches only, never the raw delta (ZOOM-07's pinch
    // would hook in here).
    let notches = notches_from_scroll(
        mouse_scroll.delta.y,
        mouse_scroll.unit == MouseScrollUnit::Pixel,
    );
    if notches == 0.0 {
        return;
    }
    let (Ok((camera, camera_transform, projection)), Ok(window)) =
        (camera_q.single(), windows.single())
    else {
        return;
    };

    let fov = camera_fov(projection);
    let selected = selection.primary;
    // The withdrawn ZOOM-03: towards the selected object if there is one,
    // else the cursor (F4-13).
    let anchor = selected
        .and_then(|entity| transforms.get(entity).ok())
        .map(GlobalTransform::translation)
        .or_else(|| cursor_anchor(camera, camera_transform, window));
    let zoom = ZoomAnchoring { selected, anchor };
    let step = plan_step(notches, fov, &state, &stops, &tunables, &zoom, &transforms);

    let current_focus = focus_position(&focus, &transforms);
    let from = ZoomFrom {
        anchor,
        centre: current_focus,
        focus: *focus,
        distance: state.distance,
    };
    let view_radius = radius_for_distance(step.target, fov, state.pitch, tunables.zoom_fit_margin);
    let target_focus =
        zoom_target_focus(from, step.target, view_radius, &followable_bodies(&bodies));
    retarget(
        &mut anim,
        &state,
        current_focus,
        step.target,
        target_focus,
        step.duration_ms,
    );
}

/// What a zoom step is anchored on and limited by.
struct ZoomAnchoring {
    selected: Option<Entity>,
    anchor: Option<Vec3>,
}

/// ZOOM-01 / ZOOM-05: the step from the current distance, with this
/// system's stops and limits at the current pitch.
fn plan_step(
    notches: f32,
    fov: f32,
    state: &OrbitCameraState,
    stops: &ZoomStops,
    tunables: &Tunables,
    zoom: &ZoomAnchoring,
    transforms: &Query<&GlobalTransform>,
) -> ZoomStep {
    let fit = |radius: f32| distance_for_radius(radius, fov, state.pitch, tunables.zoom_fit_margin);
    plan_zoom_step(
        state.distance,
        notches,
        &stop_distances(stops, zoom.selected, fit),
        inner_limit(stops, zoom.selected, zoom.anchor, transforms, fit),
        fit(stops.system_radius).min(MAX_DISTANCE),
        StepSettings {
            step_factor: tunables.zoom_step_factor,
            anim_ms: tunables.zoom_anim_ms,
            stop_resistance: tunables.zoom_stop_resistance,
        },
    )
}

/// Anything the view can centre on. Gates are excluded: they're edge
/// indicators sitting far out, not somewhere to fly to.
fn followable_bodies(
    bodies: &Query<(Entity, &GlobalTransform, &BodyClass)>,
) -> Vec<(Entity, Vec3)> {
    bodies
        .iter()
        .filter(|(_, _, class)| **class != BodyClass::Gate)
        .map(|(entity, transform, _)| (entity, transform.translation()))
        .collect()
}

/// Where a zoom step starts from.
struct ZoomFrom {
    anchor: Option<Vec3>,
    centre: Vec3,
    focus: CameraFocus,
    distance: f32,
}

/// Where a zoom step centres the view. Zooming in towards a body latches
/// onto it: bodies keep orbiting while you zoom, and at satellite scale a
/// planet crosses the whole view in about a second, so a fixed centre would
/// simply lose it. Otherwise the anchor holds still on screen. This is the
/// iteration 3 behaviour, close to CAM-21; the camera iteration replaces it
/// (F4-13).
fn zoom_target_focus(
    from: ZoomFrom,
    target: f32,
    target_view_radius: f32,
    followable: &[(Entity, Vec3)],
) -> CameraFocus {
    let Some(anchor) = from.anchor.filter(|_| from.distance > f32::EPSILON) else {
        return from.focus;
    };
    let zooming_in = target < from.distance;
    let latched = zooming_in
        .then(|| nearest_body(anchor, target_view_radius * LATCH_FRACTION, followable))
        .flatten();
    match latched {
        Some(entity) => CameraFocus::Following(entity),
        None => CameraFocus::Fixed(anchored_centre(anchor, from.centre, from.distance, target)),
    }
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
        .unwrap_or(stops.inner_system_radius * BARE_BODY_FRAME_FRACTION);
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
    let pan_speed = state.distance * DRAG_PAN_PER_PX;
    let pan = (-right * delta.x + forward_flat * delta.y) * pan_speed;
    *focus = CameraFocus::Fixed(current + pan);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bodies() -> (Entity, Entity) {
        let mut world = World::new();
        (world.spawn_empty().id(), world.spawn_empty().id())
    }

    fn from(anchor: Option<Vec3>, centre: Vec3, distance: f32) -> ZoomFrom {
        ZoomFrom {
            anchor,
            centre,
            focus: CameraFocus::Fixed(centre),
            distance,
        }
    }

    /// Zooming in near a body latches onto it; out of the latch's reach,
    /// the anchor holds still on screen instead.
    #[test]
    fn zoom_in_near_a_body_latches_onto_it() {
        let (near, far) = bodies();
        let followable = [
            (near, Vec3::new(1.0, 0.0, 0.0)),
            (far, Vec3::new(50.0, 0.0, 0.0)),
        ];
        let latched = zoom_target_focus(
            from(Some(Vec3::ZERO), Vec3::ZERO, 100.0),
            50.0,
            10.0,
            &followable,
        );
        assert!(matches!(latched, CameraFocus::Following(e) if e == near));
        let held = zoom_target_focus(
            from(Some(Vec3::ZERO), Vec3::ZERO, 100.0),
            50.0,
            1.0,
            &followable,
        );
        assert!(matches!(held, CameraFocus::Fixed(_)));
    }

    /// Zooming out never latches, and with no anchor the focus is kept.
    #[test]
    fn zoom_out_never_latches_and_no_anchor_keeps_focus() {
        let (near, _) = bodies();
        let followable = [(near, Vec3::ZERO)];
        let out = zoom_target_focus(
            from(Some(Vec3::ZERO), Vec3::ONE, 50.0),
            100.0,
            100.0,
            &followable,
        );
        assert!(matches!(out, CameraFocus::Fixed(_)));
        let kept = zoom_target_focus(from(None, Vec3::ONE, 100.0), 50.0, 100.0, &followable);
        assert!(matches!(kept, CameraFocus::Fixed(p) if p == Vec3::ONE));
    }

    #[test]
    fn nearest_body_respects_the_reach() {
        let (a, b) = bodies();
        let candidates = [(a, Vec3::new(3.0, 0.0, 0.0)), (b, Vec3::new(1.0, 0.0, 0.0))];
        assert_eq!(nearest_body(Vec3::ZERO, 5.0, &candidates), Some(b));
        assert_eq!(nearest_body(Vec3::ZERO, 0.5, &candidates), None);
        assert_eq!(nearest_body(Vec3::ZERO, 5.0, &[]), None);
    }
}

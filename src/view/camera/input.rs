//! Camera input: wheel, keys and mouse drags turned into camera moves.
//! Everything here reads input through the bindings (CAM-18) and calls
//! into `state.rs` and `rig.rs`.
//!
//! Still the iteration 3 behaviours, being replaced through iteration 5:
//! the wheel zoom anchors on the selection and latches onto the nearest
//! body (the withdrawn ZOOM-03, ZOOM-06; F4-13); right drag turns and tilts
//! (now clamped by CAM-01); middle drag pans (bounded by CAM-19); Home and
//! F jump (ZOOM-10, the withdrawn ZOOM-08); Ctrl+1 to 4 save and 1 to 4
//! recall bookmarks (ZOOM-11).

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

use super::requests::FocusJumpRequest;
use super::rig::{body_position, start_move};
use super::rules::{clamp_tilt, plane_axes, plane_point};
use super::state::{
    ActiveMove, BOOKMARK_SLOTS, Bookmark, Bookmarks, CameraRig, MAX_DISTANCE, MIN_DISTANCE,
    MoveTarget, OrbitCamera, PivotTarget, camera_fov,
};
use super::stops::{ZoomStops, distance_for_radius, radius_for_distance};
use super::zoom::{StepSettings, ZoomStep, anchored_centre, notches_from_scroll, plan_zoom_step};
use crate::sim::body::BodyClass;
use crate::view::bindings::{Bindings, held, pressed};
use crate::view::focus::{Focus, clamp_to_pan_bound};
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// Right drag: radians of heading or tilt per pixel dragged (iteration 1
/// behaviour; CAM-12 replaces it with `rotate_deg_per_px`).
const DRAG_ROTATE_RAD_PER_PX: f32 = 0.005;
/// Middle drag: plane distance per pixel dragged, as a fraction of the
/// camera distance, so a pan feels the same at every zoom (iteration 1
/// behaviour; CAM-10 replaces it with an exact cursor-locked pan).
const DRAG_PAN_PER_PX: f32 = 0.0015;
/// A body with no satellites still gets a sensible close-up when framed:
/// this fraction of the region frame.
const BARE_BODY_FRAME_FRACTION: f32 = 0.05;

/// Right drag turns the heading and tilts, the tilt clamped (CAM-01).
pub fn handle_orbit_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut rig: ResMut<CameraRig>,
) {
    if !held(&mouse_buttons, &[bindings.rotate_button]) {
        return;
    }
    let delta = mouse_motion.delta;
    rig.heading -= delta.x * DRAG_ROTATE_RAD_PER_PX;
    rig.tilt = clamp_tilt(
        rig.tilt + delta.y * DRAG_ROTATE_RAD_PER_PX,
        tunables.max_pitch_deg,
    );
}

/// Where the point under the cursor is, on the orbital plane.
fn cursor_anchor(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
) -> Option<Vec3> {
    let cursor = window.cursor_position()?;
    plane_point(camera.viewport_to_world(camera_transform, cursor).ok()?)
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
    positions: &WorldPositions,
    fit: impl Fn(f32) -> f32,
) -> f32 {
    let nearest_frame = || {
        let anchor = anchor?;
        stops
            .per_body_satellite_radius
            .iter()
            .filter_map(|(entity, radius)| {
                let position = *positions.0.get(entity)?;
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
    camera_q: Query<(&Camera, &Transform, &Projection), With<OrbitCamera>>,
    positions: Res<WorldPositions>,
    bodies: Query<(Entity, &BodyClass)>,
    mut view: CameraControl,
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
    // The camera as the player sees it: placed by the rig last frame.
    let camera_transform = GlobalTransform::from(*camera_transform);

    let fov = camera_fov(projection);
    let selected = selection.primary;
    // The withdrawn ZOOM-03: towards the selected object if there is one,
    // else the cursor (F4-13).
    let anchor = selected
        .and_then(|entity| body_position(entity, &positions))
        .or_else(|| cursor_anchor(camera, &camera_transform, window));
    let zoom = ZoomAnchoring { selected, anchor };
    let rig = *view.rig;
    let step = plan_step(notches, fov, &rig, &stops, &tunables, &zoom, &positions);

    let from = ZoomFrom {
        anchor,
        centre: rig.pivot,
        focus: *view.focus,
        distance: rig.distance,
    };
    let view_radius = radius_for_distance(step.target, fov, rig.tilt, tunables.zoom_fit_margin);
    let followable = followable_bodies(&bodies, &positions);
    let pivot = zoom_target_pivot(from, step.target, view_radius, &followable);
    let target = MoveTarget::keeping_view(&rig, pivot, step.target);
    view.start(target, step.duration_ms);
}

/// The rig, the focus and the move in flight: what every camera action
/// changes together.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CameraControl<'w> {
    pub rig: ResMut<'w, CameraRig>,
    pub focus: ResMut<'w, Focus>,
    pub active: ResMut<'w, ActiveMove>,
}

impl CameraControl<'_> {
    /// Start a move from the current rig.
    pub fn start(&mut self, target: MoveTarget, duration_ms: f32) {
        let rig = *self.rig;
        start_move(&mut self.active, &rig, &mut self.focus, target, duration_ms);
    }
}

/// What a zoom step is anchored on and limited by.
struct ZoomAnchoring {
    selected: Option<Entity>,
    anchor: Option<Vec3>,
}

/// ZOOM-01 / ZOOM-05: the step from the current distance, with this
/// system's stops and limits at the current tilt.
fn plan_step(
    notches: f32,
    fov: f32,
    rig: &CameraRig,
    stops: &ZoomStops,
    tunables: &Tunables,
    zoom: &ZoomAnchoring,
    positions: &WorldPositions,
) -> ZoomStep {
    let fit = |radius: f32| distance_for_radius(radius, fov, rig.tilt, tunables.zoom_fit_margin);
    plan_zoom_step(
        rig.distance,
        notches,
        &stop_distances(stops, zoom.selected, fit),
        inner_limit(stops, zoom.selected, zoom.anchor, positions, fit),
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
    bodies: &Query<(Entity, &BodyClass)>,
    positions: &WorldPositions,
) -> Vec<(Entity, Vec3)> {
    bodies
        .iter()
        .filter(|(_, class)| **class != BodyClass::Gate)
        .filter_map(|(entity, _)| Some((entity, body_position(entity, positions)?)))
        .collect()
}

/// Where a zoom step starts from.
struct ZoomFrom {
    anchor: Option<Vec3>,
    centre: Vec3,
    focus: Focus,
    distance: f32,
}

/// Where a zoom step centres the view. Zooming in towards a body latches
/// onto it; otherwise the anchor holds still on screen. This is the
/// iteration 3 behaviour, close to CAM-21; step 4 replaces it (F4-13).
fn zoom_target_pivot(
    from: ZoomFrom,
    target: f32,
    target_view_radius: f32,
    followable: &[(Entity, Vec3)],
) -> PivotTarget {
    let Some(anchor) = from.anchor.filter(|_| from.distance > f32::EPSILON) else {
        return match from.focus.body {
            Some(body) => PivotTarget::Body {
                body,
                offset: from.focus.offset,
            },
            None => PivotTarget::Fixed(from.centre),
        };
    };
    let zooming_in = target < from.distance;
    let latched = zooming_in
        .then(|| nearest_body(anchor, target_view_radius * LATCH_FRACTION, followable))
        .flatten();
    match latched {
        Some(body) => PivotTarget::Body {
            body,
            offset: Vec3::ZERO,
        },
        None => PivotTarget::Fixed(anchored_centre(anchor, from.centre, from.distance, target)),
    }
}

/// ZOOM-10: Home returns to the system frame.
pub fn handle_navigation_keys(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    camera_q: Query<&Projection, With<OrbitCamera>>,
    mut view: CameraControl,
) {
    if !pressed(&keys, &bindings.home) {
        return;
    }
    let Ok(projection) = camera_q.single() else {
        return;
    };
    let rig = *view.rig;
    let distance = distance_for_radius(
        stops.system_radius,
        camera_fov(projection),
        rig.tilt,
        tunables.zoom_fit_margin,
    );
    let target = MoveTarget::keeping_view(&rig, PivotTarget::Fixed(Vec3::ZERO), distance);
    view.start(target, tunables.focus_jump_ms);
}

/// Frame a body's satellites in one pan-and-zoom. Raised by a double-click
/// on a body, or by the focus key for the current selection.
#[allow(clippy::too_many_arguments)]
pub fn handle_focus_jump(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    selection: Res<Selection>,
    camera_q: Query<&Projection, With<OrbitCamera>>,
    mut request: ResMut<FocusJumpRequest>,
    mut view: CameraControl,
) {
    // The focus key does the same thing for whatever's selected.
    let target_body = request.0.take().or_else(|| {
        pressed(&keys, &bindings.focus_selection)
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
    let rig = *view.rig;
    let distance = distance_for_radius(
        radius,
        camera_fov(projection),
        rig.tilt,
        tunables.zoom_fit_margin,
    );
    let pivot = PivotTarget::Body {
        body,
        offset: Vec3::ZERO,
    };
    let target = MoveTarget::keeping_view(&rig, pivot, distance.max(MIN_DISTANCE));
    view.start(target, tunables.focus_jump_ms);
}

/// ZOOM-11: Ctrl+1..4 save the current view; 1..4 recall it - the whole
/// rig, and the focus with its offset (CAM-06).
pub fn handle_bookmarks(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    mut bookmarks: ResMut<Bookmarks>,
    mut view: CameraControl,
) {
    let saving = held(&keys, &bindings.bookmark_save);
    let slots = bindings
        .bookmark_slots
        .iter()
        .enumerate()
        .take(BOOKMARK_SLOTS);
    for (index, key) in slots {
        if !keys.just_pressed(*key) {
            continue;
        }
        if saving {
            bookmarks.0[index] = Some(Bookmark {
                rig: *view.rig,
                focus: *view.focus,
            });
        } else if let Some(bookmark) = bookmarks.0[index] {
            view.start(bookmark_target(&bookmark), tunables.focus_jump_ms);
        }
    }
}

/// Where recalling a bookmark goes: its focus with its offset, or its
/// fixed pivot.
fn bookmark_target(bookmark: &Bookmark) -> MoveTarget {
    let pivot = match bookmark.focus.body {
        Some(body) => PivotTarget::Body {
            body,
            offset: bookmark.focus.offset,
        },
        None => PivotTarget::Fixed(bookmark.rig.pivot),
    };
    MoveTarget {
        pivot,
        heading: bookmark.rig.heading,
        tilt: bookmark.rig.tilt,
        distance: bookmark.rig.distance,
    }
}

/// Middle drag pans (iteration 1 behaviour): it ends any move and drops the
/// focus, and the pivot stays within the pan bound (CAM-19).
pub fn handle_pan_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    mut view: CameraControl,
) {
    if !held(&mouse_buttons, &bindings.pan_buttons) {
        return;
    }
    // The pivot is already wherever the focus or the move put it.
    view.active.0 = None;
    *view.focus = Focus::default();

    let delta = mouse_motion.delta;
    let (right, up) = plane_axes(view.rig.heading);
    let pan = (-right * delta.x + up * delta.y) * view.rig.distance * DRAG_PAN_PER_PX;
    let bound = tunables.pan_bound_factor * stops.system_radius;
    view.rig.pivot = clamp_to_pan_bound(view.rig.pivot + pan, bound);
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
            focus: Focus::default(),
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
        let latched = zoom_target_pivot(
            from(Some(Vec3::ZERO), Vec3::ZERO, 100.0),
            50.0,
            10.0,
            &followable,
        );
        assert!(matches!(latched, PivotTarget::Body { body, .. } if body == near));
        let held = zoom_target_pivot(
            from(Some(Vec3::ZERO), Vec3::ZERO, 100.0),
            50.0,
            1.0,
            &followable,
        );
        assert!(matches!(held, PivotTarget::Fixed(_)));
    }

    /// Zooming out never latches, and with no anchor the focus is kept.
    #[test]
    fn zoom_out_never_latches_and_no_anchor_keeps_focus() {
        let (near, _) = bodies();
        let followable = [(near, Vec3::ZERO)];
        let out = zoom_target_pivot(
            from(Some(Vec3::ZERO), Vec3::ONE, 50.0),
            100.0,
            100.0,
            &followable,
        );
        assert!(matches!(out, PivotTarget::Fixed(_)));
        let kept = zoom_target_pivot(from(None, Vec3::ONE, 100.0), 50.0, 100.0, &followable);
        assert!(matches!(kept, PivotTarget::Fixed(p) if p == Vec3::ONE));
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

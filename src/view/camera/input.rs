//! Camera input: wheel, keys and mouse drags turned into camera moves.
//! Everything here reads input through the bindings (CAM-18) and calls
//! into `state.rs` and `rig.rs`.
//!
//! Zoom is in `zoom_input.rs`. Still the iteration 3 behaviours, being
//! replaced in steps 5 and 6: right drag turns and tilts (clamped by
//! CAM-01); middle drag pans (bounded by CAM-19); Home and F jump (ZOOM-10,
//! the withdrawn ZOOM-08); Ctrl+1 to 4 save and 1 to 4 recall bookmarks
//! (ZOOM-11).

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use super::requests::FocusJumpRequest;
use super::rig::{settle_focus, start_move};
use super::rules::{clamp_tilt, plane_axes};
use super::state::{
    ActiveMove, BOOKMARK_SLOTS, Bookmark, Bookmarks, CameraRig, MIN_DISTANCE, MoveTarget,
    OrbitCamera, PivotTarget, camera_fov,
};
use super::stops::{ZoomStops, distance_for_radius};
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

/// The rig, the focus and the move in flight: what every camera action
/// changes together.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CameraControl<'w> {
    pub rig: ResMut<'w, CameraRig>,
    pub focus: ResMut<'w, Focus>,
    pub active: ResMut<'w, ActiveMove>,
}

impl CameraControl<'_> {
    /// Before a new move replaces one in flight: a focus kept through the
    /// old move takes the offset of where the pivot is now, so nothing
    /// jumps (ZOOM-02, CAM-21).
    pub fn settle(&mut self, positions: &WorldPositions) {
        if self.active.0.is_none() {
            return;
        }
        settle_focus(&mut self.focus, &self.rig, positions);
    }

    /// Start a move from the current rig.
    pub fn start(&mut self, target: MoveTarget, duration_ms: f32) {
        let rig = *self.rig;
        start_move(&mut self.active, &rig, &mut self.focus, target, duration_ms);
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
    // CAM-08: Home ends focus.
    *view.focus = Focus::default();
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
    parents: Query<&ChildOf>,
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
    let radius = stops.satellite_frame(body, parents.get(body).ok().map(ChildOf::parent));
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
            // CAM-08: a bookmark saved without focus ends it.
            *view.focus = bookmark.focus;
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

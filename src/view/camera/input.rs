//! Camera moves from keys: Home (ZOOM-10), F (the withdrawn ZOOM-08,
//! replaced in step 6), and Ctrl+1 to 4 save and 1 to 4 recall bookmarks
//! (ZOOM-11). Zoom is in `zoom_input.rs`, drags in `drag.rs`, panning and
//! turning keys in `keys.rs`; all read input through the bindings (CAM-18)
//! and change the camera through `control.rs`.

use bevy::prelude::*;

use super::control::CameraControl;
use super::requests::FocusJumpRequest;
use super::state::{
    BOOKMARK_SLOTS, Bookmark, Bookmarks, MIN_DISTANCE, MoveTarget, OrbitCamera, PivotTarget,
    camera_fov,
};
use super::stops::{ZoomStops, distance_for_radius};
use crate::view::bindings::{Bindings, held, pressed};
use crate::view::focus::Focus;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;

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

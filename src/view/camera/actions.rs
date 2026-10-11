//! Camera actions: the animated moves that aren't zoom steps. Home
//! (ZOOM-10, CAM-04), N (CAM-03), focusing a body and framing its
//! satellites (ZOOM-16, CAM-06), and bookmarks (ZOOM-11, CAM-06).
//!
//! Zoom is in `zoom_input.rs`, drags in `drag.rs`, panning and turning keys
//! in `keys.rs`; all read input through the bindings (CAM-18) and change
//! the camera through `control.rs`.

use bevy::prelude::*;

use super::control::CameraControl;
use super::requests::FocusRequest;
use super::rules::nearest_heading;
use super::state::{
    BOOKMARK_SLOTS, Bookmark, Bookmarks, CameraRig, MIN_DISTANCE, MoveTarget, OrbitCamera,
    PivotTarget, camera_fov,
};
use super::stops::{ZoomStops, distance_for_radius};
use crate::view::bindings::{Bindings, held, pressed};
use crate::view::focus::Focus;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// Level (tilt 0) and north up.
const LEVEL: f32 = 0.0;
const NORTH: f32 = 0.0;

/// ZOOM-10 / CAM-04: Home returns to the system frame over the star, level
/// and north up, in one move, and ends focus (CAM-08).
#[allow(clippy::too_many_arguments)]
pub fn handle_home(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    camera_q: Query<&Projection, With<OrbitCamera>>,
    positions: Res<WorldPositions>,
    mut view: CameraControl,
) {
    if !pressed(&keys, &bindings.home) {
        return;
    }
    let Ok(projection) = camera_q.single() else {
        return;
    };
    let fov = camera_fov(projection);
    let distance = distance_for_radius(stops.system_radius, fov, LEVEL, tunables.zoom_fit_margin);
    view.settle(&positions);
    let target = MoveTarget {
        pivot: PivotTarget::Fixed(Vec3::ZERO),
        heading: nearest_heading(view.rig.heading, NORTH),
        tilt: LEVEL,
        distance,
    };
    *view.focus = Focus::default();
    view.start(target, tunables.focus_jump_ms);
}

/// CAM-03: N levels the view and turns north up, keeping the pivot (and the
/// focus with its offset) and the scale.
pub fn handle_north_up(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    positions: Res<WorldPositions>,
    mut view: CameraControl,
) {
    if !pressed(&keys, &bindings.north_up) {
        return;
    }
    view.settle(&positions);
    let rig: CameraRig = *view.rig;
    let pivot = match view.focus.body {
        Some(body) => PivotTarget::Body {
            body,
            offset: view.focus.offset,
        },
        None => PivotTarget::Fixed(rig.pivot),
    };
    let target = MoveTarget {
        pivot,
        heading: nearest_heading(rig.heading, NORTH),
        tilt: LEVEL,
        distance: rig.distance,
    };
    view.start(target, tunables.focus_jump_ms);
}

/// ZOOM-16 / CAM-06: focus a body with zero offset and frame its
/// satellites in one move of `focus_jump_ms`, keeping heading and tilt.
/// Asked for by a double-click on a body that isn't focused or the inset's
/// Focus button (`FocusRequest`), or by F for the selection. The asker has
/// already selected the body.
#[allow(clippy::too_many_arguments)]
pub fn handle_focus_requests(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    selection: Res<Selection>,
    camera_q: Query<&Projection, With<OrbitCamera>>,
    parents: Query<&ChildOf>,
    positions: Res<WorldPositions>,
    mut request: ResMut<FocusRequest>,
    mut view: CameraControl,
) {
    let key = pressed(&keys, &bindings.focus_selection).then_some(selection.primary);
    let Some(body) = request.0.take().or(key.flatten()) else {
        return;
    };
    let Ok(projection) = camera_q.single() else {
        return;
    };
    let radius = stops.satellite_frame(body, parents.get(body).ok().map(ChildOf::parent));
    let fov = camera_fov(projection);
    let distance = distance_for_radius(radius, fov, view.rig.tilt, tunables.zoom_fit_margin);
    view.settle(&positions);
    let pivot = PivotTarget::Body {
        body,
        offset: Vec3::ZERO,
    };
    let target = MoveTarget::keeping_view(&view.rig, pivot, distance.max(MIN_DISTANCE));
    view.start(target, tunables.focus_jump_ms);
}

/// ZOOM-11 / CAM-06: Ctrl+1..4 save the view; 1..4 recall it - scale,
/// pivot, heading, tilt, and any focus with its offset. A bookmark saved
/// without focus ends it (CAM-08).
pub fn handle_bookmarks(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    positions: Res<WorldPositions>,
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
        view.settle(&positions);
        if saving {
            bookmarks.0[index] = Some(Bookmark {
                rig: *view.rig,
                focus: *view.focus,
            });
        } else if let Some(bookmark) = bookmarks.0[index] {
            *view.focus = bookmark.focus;
            let target = bookmark_target(&bookmark, view.rig.heading);
            view.start(target, tunables.focus_jump_ms);
        }
    }
}

/// Where recalling a bookmark goes: its focus with its offset, or its fixed
/// pivot; its heading reached the short way round from `heading`.
fn bookmark_target(bookmark: &Bookmark, heading: f32) -> MoveTarget {
    let pivot = match bookmark.focus.body {
        Some(body) => PivotTarget::Body {
            body,
            offset: bookmark.focus.offset,
        },
        None => PivotTarget::Fixed(bookmark.rig.pivot),
    };
    MoveTarget {
        pivot,
        heading: nearest_heading(heading, bookmark.rig.heading),
        tilt: bookmark.rig.tilt,
        distance: bookmark.rig.distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CAM-06: a bookmark restores the whole view, and any focus with its
    /// offset.
    #[test]
    fn cam_06_bookmark_restores_heading_tilt_and_focus() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let rig = CameraRig {
            pivot: Vec3::new(3.0, 0.0, 1.0),
            heading: 1.0,
            tilt: 0.6,
            distance: 12.0,
        };
        let focused = Bookmark {
            rig,
            focus: Focus {
                body: Some(body),
                offset: Vec3::X,
            },
        };
        let target = bookmark_target(&focused, 1.2);
        assert_eq!(
            target.pivot,
            PivotTarget::Body {
                body,
                offset: Vec3::X
            }
        );
        assert!((target.heading - 1.0).abs() < 1e-5);
        assert_eq!((target.tilt, target.distance), (0.6, 12.0));

        let free = Bookmark {
            rig,
            focus: Focus::default(),
        };
        assert_eq!(
            bookmark_target(&free, 1.0).pivot,
            PivotTarget::Fixed(rig.pivot)
        );
    }
}

//! Mouse drags (CAM-07, CAM-10 to CAM-13, CAM-20).
//!
//! Left and middle drag pan: the plane point under the cursor when the drag
//! starts stays under the cursor throughout, also when tilted, and stops
//! dead on release. Right drag rotates about the pivot. A press that moves
//! less than `drag_threshold_px` is a click: the left button's selects (in
//! `hover.rs`, which checks `PressStart`), the right button's is the
//! context-menu hook. Shift+left drag, and any drag that starts on a panel,
//! does nothing.

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;

use super::control::CameraControl;
use super::requests::ContextMenuRequest;
use super::rig::body_position;
use super::rules::{camera_transform, plane_point};
use super::state::{CameraRig, OrbitCamera};
use super::stops::ZoomStops;
use crate::view::bindings::{Bindings, held};
use crate::view::gesture::{PressStart, is_drag};
use crate::view::targets::MapInputBlocker;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// What a press has turned into so far.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DragMode {
    /// Not yet past the drag threshold: still a click.
    Pending,
    /// CAM-10 / CAM-11: panning, holding this plane point under the cursor.
    /// While focused it is held relative to the focused body.
    Pan {
        grab: Vec3,
        relative_to: Option<Entity>,
    },
    /// CAM-12: rotating about the pivot.
    Rotate,
    /// CAM-10: Shift+left, or a press on a panel; does nothing.
    Ignored,
}

#[derive(Debug, Clone, Copy)]
struct Press {
    button: MouseButton,
    start: Vec2,
    mode: DragMode,
}

/// The mouse press in progress, if any.
#[derive(Resource, Default)]
pub struct MouseDrag(Option<Press>);

/// The cursor, the camera that projects it, and what UI it is over.
#[derive(SystemParam)]
pub struct Pointer<'w, 's> {
    windows: Query<'w, 's, &'static Window>,
    camera: Query<'w, 's, &'static Camera, With<OrbitCamera>>,
    hover: Option<Res<'w, HoverMap>>,
    parents: Query<'w, 's, &'static ChildOf>,
    blockers: Query<'w, 's, (), With<MapInputBlocker>>,
}

impl Pointer<'_, '_> {
    fn cursor(&self) -> Option<Vec2> {
        self.windows.single().ok()?.cursor_position()
    }

    /// Whether the pointer is over a panel the map's drags don't start on.
    fn over_panel(&self) -> bool {
        let Some(hits) = self
            .hover
            .as_ref()
            .and_then(|map| map.0.get(&PointerId::Mouse))
        else {
            return false;
        };
        hits.keys().any(|&hit| {
            std::iter::successors(Some(hit), |&node| {
                self.parents.get(node).ok().map(ChildOf::parent)
            })
            .any(|node| self.blockers.contains(node))
        })
    }

    /// CAM-10: the plane point under a screen position, for the camera at
    /// `pivot` with the rig's heading, tilt and distance.
    fn plane_point_at(&self, screen: Vec2, rig: &CameraRig, pivot: Vec3) -> Option<Vec3> {
        let camera = self.camera.single().ok()?;
        let transform = camera_transform(pivot, rig.heading, rig.tilt, rig.distance);
        let ray = camera
            .viewport_to_world(&GlobalTransform::from(transform), screen)
            .ok()?;
        plane_point(ray)
    }
}

/// A press of any camera button starts a gesture; where it started is
/// shared with the click handlers (CAM-10).
pub fn begin_press(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    pointer: Pointer,
    mut press_start: ResMut<PressStart>,
    mut drag: ResMut<MouseDrag>,
) {
    let mut camera_buttons = bindings.pan_buttons.clone();
    camera_buttons.extend([bindings.rotate_button, bindings.select_button]);
    let Some(button) = camera_buttons
        .into_iter()
        .find(|b| buttons.just_pressed(*b))
    else {
        return;
    };
    let Some(start) = pointer.cursor() else {
        return;
    };
    press_start.0 = Some(start);
    let reserved = button == bindings.select_button && held(&keys, &bindings.select_drag_reserved);
    let mode = if reserved || pointer.over_panel() {
        DragMode::Ignored
    } else {
        DragMode::Pending
    };
    drag.0 = Some(Press {
        button,
        start,
        mode,
    });
}

/// A press past the threshold becomes a pan or a rotation, which then
/// follows the cursor every frame.
#[allow(clippy::too_many_arguments)]
pub fn update_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    motion: Res<AccumulatedMouseMotion>,
    pointer: Pointer,
    positions: Res<WorldPositions>,
    mut drag: ResMut<MouseDrag>,
    mut view: CameraControl,
) {
    let Some(mut press) = drag.0.filter(|press| buttons.pressed(press.button)) else {
        return;
    };
    let Some(cursor) = pointer.cursor() else {
        return;
    };
    if press.mode == DragMode::Pending && is_drag(press.start, cursor, tunables.drag_threshold_px) {
        press.mode = start_drag(&press, &bindings, &pointer, &positions, &mut view);
        drag.0 = Some(press);
    }
    match press.mode {
        DragMode::Pan { grab, relative_to } => {
            let grab_now = match relative_to.and_then(|body| body_position(body, &positions)) {
                Some(position) => position + grab,
                None => grab,
            };
            let pivot = view.pivot_now(&positions);
            if let Some(under) = pointer.plane_point_at(cursor, &view.rig, pivot) {
                let bound = tunables.pan_bound_factor * stops.system_radius;
                view.pan(grab_now - under, &positions, bound);
            }
        }
        DragMode::Rotate => {
            let rate = tunables.rotate_deg_per_px.to_radians();
            let delta = motion.delta;
            view.rotate(
                -delta.x * rate,
                delta.y * rate,
                tunables.max_pitch_deg,
                &positions,
            );
        }
        DragMode::Pending | DragMode::Ignored => {}
    }
}

/// What a press becomes once it moves past the threshold. Taking over ends
/// any animated move (CAM-20).
fn start_drag(
    press: &Press,
    bindings: &Bindings,
    pointer: &Pointer,
    positions: &WorldPositions,
    view: &mut CameraControl,
) -> DragMode {
    view.interrupt(positions);
    if press.button == bindings.rotate_button {
        return DragMode::Rotate;
    }
    if !bindings.pan_buttons.contains(&press.button) {
        return DragMode::Ignored;
    }
    let pivot = view.pivot_now(positions);
    let Some(grab) = pointer.plane_point_at(press.start, &view.rig, pivot) else {
        return DragMode::Ignored;
    };
    // While focused, hold the grabbed point relative to the focused body,
    // which the view is following (CAM-07).
    match view
        .focus
        .body
        .and_then(|body| Some((body, body_position(body, positions)?)))
    {
        Some((body, position)) => DragMode::Pan {
            grab: grab - position,
            relative_to: Some(body),
        },
        None => DragMode::Pan {
            grab,
            relative_to: None,
        },
    }
}

/// On release: a right press that never became a drag is a right-click
/// (CAM-13). No inertia either way: the drag simply stops.
pub fn end_press(
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    mut drag: ResMut<MouseDrag>,
    mut context_menu: ResMut<ContextMenuRequest>,
) {
    let Some(press) = drag.0.filter(|press| buttons.just_released(press.button)) else {
        return;
    };
    if press.mode == DragMode::Pending && press.button == bindings.rotate_button {
        context_menu.0 = Some(press.start);
    }
    drag.0 = None;
}

//! Zoom input (ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-13 to ZOOM-15, CAM-08,
//! CAM-16, CAM-23): wheel notches and the + / - keys, each one animated
//! step, anchored and limited by the focus. Zooming never sets focus
//! (CAM-06); a zoom out to the system frame ends it and eases the view
//! back to the standard orientation (CAM-23).
//!
//! The rules are plain functions in `zoom.rs` (the step, the anchor) and
//! `frames.rs` (the distances); this module gathers their inputs and
//! starts the move.

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

use super::control::CameraControl;
use super::frames::Frames;
use super::rig::body_position;
use super::rules::plane_point;
use super::state::{CameraRig, MoveTarget, OrbitCamera, PivotTarget};
use super::zoom::{
    StepSettings, ZoomStep, anchored_centre, notches_from_scroll, plan_zoom_step,
    reaches_system_frame, zoom_anchor,
};
use crate::view::bindings::{Bindings, pressed};
use crate::view::focus::{Focus, clamp_to_pan_bound};
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// The camera, as zoom reads it.
type ZoomCamera<'w, 's> = Query<'w, 's, (&'static Camera, &'static Transform), With<OrbitCamera>>;

/// One zoom request this frame: how many notches, and whether it came from
/// the wheel (anchored on the cursor) or the keys (on the screen centre).
struct ZoomRequest {
    notches: f32,
    from_wheel: bool,
}

/// What a step is judged against: the frames as distances at this tilt.
struct StepFrames {
    system: f32,
    /// The focused body's satellite frame, if it has satellites.
    satellite: Option<f32>,
    inner: f32,
}

/// Everything a zoom step reads.
#[derive(SystemParam)]
pub struct ZoomContext<'w, 's> {
    scroll: Res<'w, AccumulatedMouseScroll>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    bindings: Res<'w, Bindings>,
    tunables: Res<'w, Tunables>,
    windows: Query<'w, 's, &'static Window>,
    camera: ZoomCamera<'w, 's>,
    positions: Res<'w, WorldPositions>,
    frames: Frames<'w, 's>,
    stops: Res<'w, super::stops::ZoomStops>,
}

impl ZoomContext<'_, '_> {
    /// ZOOM-01 / CAM-16: wheel notches, else one step per + or - press.
    fn request(&self) -> Option<ZoomRequest> {
        // Whole notches only, never the raw delta (ZOOM-07's pinch would
        // hook in here).
        let unit_is_pixels = self.scroll.unit == MouseScrollUnit::Pixel;
        let wheel = notches_from_scroll(self.scroll.delta.y, unit_is_pixels);
        if wheel != 0.0 {
            return Some(ZoomRequest {
                notches: wheel,
                from_wheel: true,
            });
        }
        let zoom_in = pressed(&self.keys, &self.bindings.zoom_in) as i32;
        let zoom_out = pressed(&self.keys, &self.bindings.zoom_out) as i32;
        (zoom_in != zoom_out).then_some(ZoomRequest {
            notches: (zoom_in - zoom_out) as f32,
            from_wheel: false,
        })
    }

    /// Where the cursor's ray meets the orbital plane, seen through the
    /// camera as the player sees it (placed by the rig last frame).
    fn cursor_plane_point(&self) -> Option<Vec3> {
        let cursor = self.windows.single().ok()?.cursor_position()?;
        let (camera, transform) = self.camera.single().ok()?;
        let transform = GlobalTransform::from(*transform);
        plane_point(camera.viewport_to_world(&transform, cursor).ok()?)
    }

    /// ZOOM-14 / ZOOM-15: the system and satellite frames and the inner
    /// limit for this step, by the focus.
    fn step_frames(&self, focused: Option<Entity>, tilt: f32) -> StepFrames {
        StepFrames {
            system: self.frames.system(tilt),
            satellite: focused.and_then(|body| self.frames.satellite(body, tilt)),
            inner: self.frames.inner_limit(focused, tilt),
        }
    }

    fn pan_bound(&self) -> f32 {
        self.tunables.pan_bound_factor * self.stops.system_radius
    }
}

/// ZOOM-01 / ZOOM-05 / ZOOM-14: the step, settling on the system and
/// satellite stops, inside the limits.
fn plan_step(current: f32, notches: f32, frames: &StepFrames, tunables: &Tunables) -> ZoomStep {
    let stops: Vec<f32> = std::iter::once(frames.system)
        .chain(frames.satellite)
        .collect();
    plan_zoom_step(
        current,
        notches,
        &stops,
        frames.inner,
        frames.system,
        StepSettings {
            step_factor: tunables.zoom_step_factor,
            anim_ms: tunables.zoom_anim_ms,
            stop_resistance: tunables.zoom_stop_resistance,
        },
    )
}

/// One zoom step per notch or key press: gather the anchor and frames,
/// plan the step, decide the focus, start the move.
pub fn handle_zoom_input(context: ZoomContext, mut view: CameraControl) {
    let Some(request) = context.request() else {
        return;
    };
    view.settle(&context.positions);
    let rig = *view.rig;
    let focused = view
        .focus
        .body
        .and_then(|body| Some((body, body_position(body, &context.positions)?)));

    // ZOOM-13: the focus, else the cursor's plane point, else the centre.
    let cursor_point = request
        .from_wheel
        .then(|| context.cursor_plane_point())
        .flatten();
    let anchor = zoom_anchor(focused.map(|(_, at)| at), cursor_point, rig.pivot);

    let frames = context.step_frames(focused.map(|(body, _)| body), rig.tilt);
    let step = plan_step(rig.distance, request.notches, &frames, &context.tunables);

    // CAM-23 / CAM-08: a step out to the system frame ends focus and eases
    // the pivot to the star and the tilt to the standard one.
    if request.notches < 0.0 && reaches_system_frame(step.target, frames.system) {
        let tilt = context.tunables.standard_tilt_deg.to_radians();
        *view.focus = Focus::default();
        let target = MoveTarget {
            pivot: PivotTarget::Fixed(Vec3::ZERO),
            heading: rig.heading,
            tilt,
            distance: context.frames.system(tilt),
        };
        view.start(target, step.duration_ms);
        return;
    }

    let pivot = step_pivot(&view, &rig, anchor, step.target, context.pan_bound());
    let target = MoveTarget::keeping_view(&rig, pivot, step.target);
    view.start(target, step.duration_ms);
}

/// Where a step takes the pivot. Focused, the step scales the pan offset
/// about the focused object, so the object keeps its place on screen
/// (ZOOM-13); otherwise it scales the view about the anchor, inside the
/// pan bound (CAM-19).
fn step_pivot(
    view: &CameraControl,
    rig: &CameraRig,
    anchor: Vec3,
    target: f32,
    bound: f32,
) -> PivotTarget {
    match view.focus.body {
        Some(body) => PivotTarget::Body {
            body,
            offset: view.focus.offset * (target / rig.distance.max(f32::EPSILON)),
        },
        None => PivotTarget::Fixed(clamp_to_pan_bound(
            anchored_centre(anchor, rig.pivot, rig.distance, target),
            bound,
        )),
    }
}

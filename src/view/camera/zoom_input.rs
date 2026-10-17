//! Zoom input (ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-13 to ZOOM-15, CAM-08,
//! CAM-16, CAM-23): wheel notches and the + / - keys, each one animated
//! step, anchored and limited by the frame. Zooming never changes the
//! frame (CAM-06), except that a zoom out to the system fit returns it to
//! the star and eases the view back to the standard tilt (CAM-23). In the
//! sector view the same steps run between the sector fit and its inner
//! limit (SECT-11), and never change the layer.
//!
//! The rules are plain functions in `zoom.rs` (the step, the anchor) and
//! `fits.rs` (the distances); this module gathers their inputs and starts
//! the move.

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

use super::control::CameraControl;
use super::fits::Fits;
use super::rules::plane_point;
use super::state::{CameraRig, MoveTarget, OrbitCamera, PivotTarget};
use super::zoom::{
    StepSettings, ZoomStep, anchored_centre, notches_from_scroll, plan_zoom_step,
    reaches_system_fit, zoom_anchor,
};
use crate::view::bindings::{Bindings, pressed};
use crate::view::frame::{body_position, clamp_to_pan_bound};
use crate::view::map_layer::MapLayer;
use crate::view::targets::PointerTargets;
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

/// What a step is judged against: the fits as distances at this tilt.
struct StepFits {
    system: f32,
    /// The frame body's satellite fit, if it has satellites.
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
    fits: Fits<'w, 's>,
    layer: Res<'w, MapLayer>,
    targets: PointerTargets<'w, 's>,
}

impl ZoomContext<'_, '_> {
    /// ZOOM-01 / CAM-16: wheel notches, else one step per + or - press.
    fn request(&self) -> Option<ZoomRequest> {
        // Whole notches only, never the raw delta.
        let unit_is_pixels = self.scroll.unit == MouseScrollUnit::Pixel;
        let wheel = notches_from_scroll(self.scroll.delta.y, unit_is_pixels);
        // INS-07: the wheel over a panel scrolls the panel, not the map.
        if wheel != 0.0 && self.targets.panel().is_none() {
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

    /// ZOOM-14 / ZOOM-15: the system and satellite fits and the inner
    /// limit for this step, by the frame body (`None` for the star).
    fn step_fits(&self, frame_body: Option<Entity>, (heading, tilt): (f32, f32)) -> StepFits {
        StepFits {
            system: self.fits.system_fit(heading, tilt),
            satellite: frame_body.and_then(|body| self.fits.satellite_fit(body, tilt)),
            inner: self.fits.inner_limit(frame_body, tilt),
        }
    }

    fn pan_bound(&self) -> f32 {
        self.tunables.pan_bound_factor * self.fits.outer_radius()
    }
}

/// ZOOM-01 / ZOOM-05 / ZOOM-14: the step, settling on the system and
/// satellite stops, inside the limits.
fn plan_step(current: f32, notches: f32, fits: &StepFits, tunables: &Tunables) -> ZoomStep {
    let stops: Vec<f32> = std::iter::once(fits.system).chain(fits.satellite).collect();
    plan_zoom_step(
        current,
        notches,
        &stops,
        fits.inner,
        fits.system,
        StepSettings {
            step_factor: tunables.zoom_step_factor,
            anim_ms: tunables.zoom_anim_ms,
            stop_resistance: tunables.zoom_stop_resistance,
        },
    )
}

/// One zoom step per notch or key press: gather the anchor and fits, plan
/// the step, decide the frame, start the move.
pub fn handle_zoom_input(context: ZoomContext, mut view: CameraControl) {
    let Some(request) = context.request() else {
        return;
    };
    view.settle(&context.positions);
    let rig = *view.rig;
    let frame_body = view.frame.non_star_body();

    // ZOOM-13: the frame body, else the cursor's plane point, else the
    // centre.
    let cursor_point = request
        .from_wheel
        .then(|| context.cursor_plane_point())
        .flatten();
    let body_at = frame_body.map(|body| body_position(body, &context.positions));
    let anchor = zoom_anchor(body_at, cursor_point, rig.pivot);

    let fits = context.step_fits(frame_body, (rig.heading, rig.tilt));
    let step = plan_step(rig.distance, request.notches, &fits, &context.tunables);

    // CAM-23 / CAM-08: a step out to the system fit sets the star's frame
    // and eases the pivot to the star and the tilt to the standard one.
    // The sector view has no such rule: its fit is only the outer limit
    // (SECT-11).
    let outermost = reaches_system_fit(step.target, fits.system);
    if request.notches < 0.0 && outermost && !context.layer.is_sector() {
        let tilt = context.tunables.standard_tilt_deg.to_radians();
        let target = MoveTarget {
            pivot: view.star_pivot(Vec3::ZERO),
            heading: rig.heading,
            tilt,
            distance: context.fits.system_fit(rig.heading, tilt),
        };
        view.start(target, step.duration_ms);
        return;
    }

    let pivot = step_pivot(&view, &rig, anchor, step.target, context.pan_bound());
    let target = MoveTarget::keeping_view(&rig, pivot, step.target);
    view.start(target, step.duration_ms);
}

/// Where a step takes the pivot. In a body's frame the step scales the pan
/// offset about the frame body, so the body keeps its place on screen
/// (ZOOM-13); in the star's frame it scales the view about the anchor,
/// inside the pan bound (CAM-19).
fn step_pivot(
    view: &CameraControl,
    rig: &CameraRig,
    anchor: Vec3,
    target: f32,
    bound: f32,
) -> PivotTarget {
    if view.frame.in_star_frame() {
        let centre = anchored_centre(anchor, rig.pivot, rig.distance, target);
        let from_star = centre - view.origin.star;
        return view.star_pivot(clamp_to_pan_bound(from_star, bound));
    }
    PivotTarget {
        body: view.frame.body,
        offset: view.frame.offset * (target / rig.distance.max(f32::EPSILON)),
    }
}

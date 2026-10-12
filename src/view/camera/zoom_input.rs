//! Zoom input (ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-13 to ZOOM-15, CAM-08,
//! CAM-16, CAM-21): wheel notches and the + / − keys, each one animated
//! step, anchored and limited by the focus.
//!
//! The rules are plain functions in `zoom.rs` (the step, the anchor, focus
//! changes) and `stops.rs` (the frames); this module gathers their inputs
//! and starts the move.

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

use super::control::CameraControl;
use super::rig::body_position;
use super::rules::plane_point;
use super::state::{
    CameraRig, MAX_DISTANCE, MIN_DISTANCE, MoveTarget, OrbitCamera, PivotTarget, camera_fov,
};
use super::stops::{ZoomStops, distance_for_radius};
use super::zoom::{
    FocusChange, StepSettings, ZoomFrames, ZoomStep, anchored_centre, nearest_body,
    notches_from_scroll, plan_zoom_step, zoom_anchor, zoom_focus_change,
};
use crate::sim::body::BodyClass;
use crate::sim::orbit::Orbit;
use crate::view::bindings::{Bindings, pressed};
use crate::view::focus::{Focus, clamp_to_pan_bound};
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// Orbiting bodies, with what they orbit.
type Orbiting<'w, 's> =
    Query<'w, 's, (Entity, &'static BodyClass, Option<&'static ChildOf>), With<Orbit>>;

/// The camera, as zoom reads it.
type ZoomCamera<'w, 's> =
    Query<'w, 's, (&'static Camera, &'static Transform, &'static Projection), With<OrbitCamera>>;

/// One zoom request this frame: how many notches, and whether it came from
/// the wheel (anchored on the cursor) or the keys (on the screen centre).
struct ZoomRequest {
    notches: f32,
    from_wheel: bool,
}

/// What a step is judged against: the frames as distances at this tilt.
struct StepFrames {
    system: f32,
    region: f32,
    satellite: f32,
    /// The body whose satellite frame is the inner limit (ZOOM-14).
    limit_body: Option<Entity>,
}

/// Everything a zoom step reads.
#[derive(SystemParam)]
pub struct ZoomContext<'w, 's> {
    scroll: Res<'w, AccumulatedMouseScroll>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    bindings: Res<'w, Bindings>,
    tunables: Res<'w, Tunables>,
    stops: Res<'w, ZoomStops>,
    windows: Query<'w, 's, &'static Window>,
    camera: ZoomCamera<'w, 's>,
    positions: Res<'w, WorldPositions>,
    bodies: Orbiting<'w, 's>,
}

impl ZoomContext<'_, '_> {
    /// ZOOM-01 / CAM-16: wheel notches, else one step per + or − press.
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
        let (camera, transform, _) = self.camera.single().ok()?;
        let transform = GlobalTransform::from(*transform);
        plane_point(camera.viewport_to_world(&transform, cursor).ok()?)
    }

    /// The camera distance that frames `radius` at `tilt`.
    fn fit(&self, radius: f32, tilt: f32) -> f32 {
        let fov = self
            .camera
            .single()
            .map_or(std::f32::consts::FRAC_PI_4, |(_, _, projection)| {
                camera_fov(projection)
            });
        distance_for_radius(radius, fov, tilt, self.tunables.zoom_fit_margin)
    }

    fn parent_of(&self, body: Entity) -> Option<Entity> {
        let (_, _, child_of) = self.bodies.get(body).ok()?;
        child_of.map(ChildOf::parent)
    }

    /// ZOOM-14 / ZOOM-15: the system, region and satellite frames for this
    /// step, by the focus, else by the body nearest the anchor.
    fn frames(&self, focused: Option<Entity>, anchor: Vec3, tilt: f32) -> StepFrames {
        // Candidates when nothing is focused: whatever orbits the star, but
        // not the gates, which stay edge indicators.
        let candidates: Vec<(Entity, Vec3)> = self
            .bodies
            .iter()
            .filter(|(_, class, child_of)| child_of.is_none() && **class != BodyClass::Gate)
            .filter_map(|(body, _, _)| Some((body, body_position(body, &self.positions)?)))
            .collect();
        let limit_body = focused.or_else(|| nearest_body(anchor, &candidates));
        let satellite = limit_body.map_or(MIN_DISTANCE, |body| {
            self.fit(self.stops.satellite_frame(body, self.parent_of(body)), tilt)
        });
        // The focused body's planet: itself if it orbits the star.
        let focused_planet = focused.map(|body| self.parent_of(body).unwrap_or(body));
        StepFrames {
            system: self.fit(self.stops.system_radius, tilt).min(MAX_DISTANCE),
            region: self.fit(self.stops.region_frame(focused_planet), tilt),
            satellite,
            limit_body,
        }
    }

    fn pan_bound(&self) -> f32 {
        self.tunables.pan_bound_factor * self.stops.system_radius
    }
}

/// ZOOM-01 / ZOOM-05 / ZOOM-14: the step, settling on the system, region
/// and satellite stops, inside the limits.
fn plan_step(current: f32, notches: f32, frames: &StepFrames, tunables: &Tunables) -> ZoomStep {
    plan_zoom_step(
        current,
        notches,
        &[frames.system, frames.region, frames.satellite],
        frames.satellite.max(MIN_DISTANCE),
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

    let frames = context.frames(focused.map(|(body, _)| body), anchor, rig.tilt);
    let step = plan_step(rig.distance, request.notches, &frames, &context.tunables);
    let decision = StepDecision {
        anchor,
        target: step.target,
        bound: context.pan_bound(),
    };
    let pivot = step_pivot(&mut view, &rig, &context.positions, &decision, &frames);
    let target = MoveTarget::keeping_view(&rig, pivot, step.target);
    view.start(target, step.duration_ms);
}

/// A planned step's anchor, target distance and pan bound.
struct StepDecision {
    anchor: Vec3,
    target: f32,
    bound: f32,
}

/// Where a step takes the pivot, and what it does to the focus (CAM-08,
/// CAM-21). Focused, the step scales the pan offset about the focused
/// object, so the object keeps its place on screen (ZOOM-13).
fn step_pivot(
    view: &mut CameraControl,
    rig: &CameraRig,
    positions: &WorldPositions,
    step: &StepDecision,
    frames: &StepFrames,
) -> PivotTarget {
    let scale = step.target / rig.distance.max(f32::EPSILON);
    let anchored = anchored_centre(step.anchor, rig.pivot, rig.distance, step.target);
    // CAM-19: a fixed pivot stays inside the pan bound.
    let fixed = PivotTarget::Fixed(clamp_to_pan_bound(anchored, step.bound));
    let zoom_frames = ZoomFrames {
        system: frames.system,
        region: frames.region,
    };
    let focused = view.focus.body.is_some();
    let change = zoom_focus_change(
        focused,
        rig.distance,
        step.target,
        zoom_frames,
        frames.limit_body,
    );
    match (change, view.focus.body) {
        (FocusChange::Keep, Some(body)) => PivotTarget::Body {
            body,
            offset: view.focus.offset * scale,
        },
        (FocusChange::Keep, None) => fixed,
        (FocusChange::End, _) => {
            *view.focus = Focus::default();
            fixed
        }
        // CAM-21: focus with the offset that keeps the view where it is;
        // this step keeps its anchor, and the offset is settled when it
        // ends.
        (FocusChange::Start(body), _) => {
            if let Some(position) = body_position(body, positions) {
                *view.focus = Focus {
                    body: Some(body),
                    offset: rig.pivot - position,
                };
            }
            fixed
        }
    }
}

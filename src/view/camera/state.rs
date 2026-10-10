//! Camera state: where the camera is, what it is centred on, and the move
//! in flight (ZOOM-01, ZOOM-02, ZOOM-10, ZOOM-11). Shared by the input
//! handlers (`input.rs`) and the rig that applies it (`rig.rs`).

use bevy::prelude::*;

use crate::view::anim::MS_PER_SEC;

#[derive(Resource, Clone, Copy)]
pub enum CameraFocus {
    Fixed(Vec3),
    Following(Entity),
}

impl Default for CameraFocus {
    fn default() -> Self {
        CameraFocus::Fixed(Vec3::ZERO)
    }
}

/// The camera's orientation and zoom. **Tilt** is the spec's: 0 looks
/// straight down at the orbital plane. **Heading** turns the view about
/// the plane's normal. Both in radians.
#[derive(Resource)]
pub struct OrbitCameraState {
    pub heading: f32,
    pub tilt: f32,
    pub distance: f32,
}

/// The opening heading, from iteration 1. CAM-02 (not built) makes it
/// north up.
const START_HEADING: f32 = 0.6;
/// Steep, near-top-down by default (about 24 degrees from straight down).
/// CAM-02 (not built) makes it straight down.
const START_TILT: f32 = 0.42;
/// Replaced at startup by the system frame, once the stops have been
/// computed from the real orbits.
const PLACEHOLDER_DISTANCE: f32 = 300.0;

impl Default for OrbitCameraState {
    fn default() -> Self {
        Self {
            heading: START_HEADING,
            tilt: START_TILT,
            distance: PLACEHOLDER_DISTANCE,
        }
    }
}

/// Saved views: scale plus centre (or followed body), per the spec.
#[derive(Resource, Default)]
pub struct Bookmarks(pub [Option<(f32, CameraFocus)>; BOOKMARK_SLOTS]);

/// ZOOM-11: Ctrl+1 to Ctrl+4.
pub const BOOKMARK_SLOTS: usize = 4;

/// An in-flight zoom (or focus jump). New input retargets this from the
/// *current* interpolated values rather than queueing behind it.
#[derive(Resource, Default)]
pub struct ZoomAnim {
    pub start_distance: f32,
    pub target_distance: f32,
    pub start_focus: Vec3,
    pub target_focus: CameraFocus,
    pub elapsed: f32,
    pub duration: f32,
}

impl ZoomAnim {
    pub fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }
}

/// Absolute safety rails, independent of the computed stops.
pub const MIN_DISTANCE: f32 = 0.002;
pub const MAX_DISTANCE: f32 = 5_000.0;
/// The iteration 1 tilt limits, which let the camera below the plane.
/// CAM-01 (not built) replaces them with 0 to `max_pitch_deg`.
pub const MIN_TILT: f32 = 0.02;
pub const MAX_TILT: f32 = 3.12;

#[derive(Component)]
pub struct OrbitCamera;

pub fn camera_fov(projection: &Projection) -> f32 {
    match projection {
        Projection::Perspective(perspective) => perspective.fov,
        _ => std::f32::consts::FRAC_PI_4,
    }
}

pub fn focus_position(focus: &CameraFocus, transforms: &Query<&GlobalTransform>) -> Vec3 {
    match *focus {
        CameraFocus::Fixed(pos) => pos,
        CameraFocus::Following(entity) => transforms
            .get(entity)
            .map(GlobalTransform::translation)
            .unwrap_or(Vec3::ZERO),
    }
}

/// Point the camera at a new distance (and optionally a new focus),
/// restarting the animation from wherever it currently is.
pub fn retarget(
    anim: &mut ZoomAnim,
    state: &OrbitCameraState,
    current_focus: Vec3,
    target_distance: f32,
    target_focus: CameraFocus,
    duration_ms: f32,
) {
    anim.start_distance = state.distance;
    anim.target_distance = target_distance;
    anim.start_focus = current_focus;
    anim.target_focus = target_focus;
    anim.elapsed = 0.0;
    anim.duration = (duration_ms / MS_PER_SEC).max(f32::EPSILON);
}

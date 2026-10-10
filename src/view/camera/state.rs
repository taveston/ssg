//! Camera state: where the camera is, what it is centred on, and the move
//! in flight (ZOOM-01, ZOOM-02, ZOOM-10, ZOOM-11). Shared by the input
//! handlers (`input.rs`) and the rig that applies it (`rig.rs`).

use bevy::prelude::*;

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

#[derive(Resource)]
pub struct OrbitCameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for OrbitCameraState {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            // Steep, near-top-down by default.
            pitch: 1.15,
            // Replaced at startup by the system frame, once the stops have
            // been computed from the real orbits.
            distance: 300.0,
        }
    }
}

/// Saved views: scale plus centre (or followed body), per the spec.
#[derive(Resource, Default)]
pub struct Bookmarks(pub [Option<(f32, CameraFocus)>; 4]);

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
pub const PITCH_LIMIT: f32 = 1.55;

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
    anim.duration = (duration_ms / 1000.0).max(f32::EPSILON);
}

//! The camera rig: spawns the camera, starts it at the system frame
//! (CAM-02), runs the animated move (ZOOM-01), keeps the pivot on the
//! focused object (CAM-05) and places the camera from the rig each frame.

use bevy::prelude::*;

use super::rules::camera_transform;
use super::state::{
    ActiveMove, CameraMove, CameraRig, MAX_DISTANCE, MIN_DISTANCE, MoveTarget, OrbitCamera,
    PivotTarget, blend_rig, camera_fov,
};
use super::stops::{ZoomStops, distance_for_radius};
use crate::view::focus::{Focus, focused_pivot};
use crate::view::tunables::Tunables;

/// Render-unit clip planes wide enough for a moon's orbit up close and the
/// whole system with its gates.
const NEAR_CLIP: f32 = 0.001;
const FAR_CLIP: f32 = 20_000.0;
/// Where the camera is spawned, in render units, before its first update.
const SPAWN_POSITION: Vec3 = Vec3::new(0.0, 40.0, 70.0);

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        OrbitCamera,
        Camera3d::default(),
        // The zoom range spans a moon's orbit up to the whole system, so
        // the default near/far pair (0.1 / 1000) would clip at both ends.
        Projection::Perspective(PerspectiveProjection {
            near: NEAR_CLIP,
            far: FAR_CLIP,
            ..default()
        }),
        // Placed properly by `place_camera`, but systems on the first frame
        // see this pose: far out and looking at the star, so nothing
        // measures as unfolded before the camera has settled.
        Transform::from_translation(SPAWN_POSITION).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// CAM-02: the camera starts at the system frame, level and north up.
pub fn start_at_system_frame(
    stops: Res<ZoomStops>,
    tunables: Res<Tunables>,
    mut rig: ResMut<CameraRig>,
    camera: Query<&Projection, With<OrbitCamera>>,
) {
    let Ok(projection) = camera.single() else {
        return;
    };
    rig.distance = distance_for_radius(
        stops.system_radius,
        camera_fov(projection),
        rig.tilt,
        tunables.zoom_fit_margin,
    );
}

/// Where a body is in render space.
pub fn body_position(body: Entity, transforms: &Query<&GlobalTransform>) -> Option<Vec3> {
    transforms.get(body).ok().map(GlobalTransform::translation)
}

/// Where a move's target pivot is now.
fn target_pivot_now(
    target: &MoveTarget,
    transforms: &Query<&GlobalTransform>,
    fallback: Vec3,
) -> Vec3 {
    match target.pivot {
        PivotTarget::Fixed(pivot) => pivot,
        PivotTarget::Body { body, offset } => body_position(body, transforms)
            .map_or(fallback, |position| focused_pivot(position, offset)),
    }
}

/// Start a move from the current rig (ZOOM-02), replacing any in flight.
/// The focus is set at once to what the move ends on, so a focus jump
/// shows as focused from its first frame.
pub fn start_move(
    active: &mut ActiveMove,
    rig: &CameraRig,
    focus: &mut Focus,
    target: MoveTarget,
    duration_ms: f32,
) {
    *focus = target.focus();
    active.0 = Some(CameraMove::new(*rig, target, duration_ms));
}

/// ZOOM-01: advance the move in flight and write the rig from it.
pub fn advance_move(
    time: Res<Time>,
    transforms: Query<&GlobalTransform>,
    mut active: ResMut<ActiveMove>,
    mut rig: ResMut<CameraRig>,
) {
    let Some(mut camera_move) = active.0 else {
        return;
    };
    camera_move.elapsed += time.delta_secs();
    let t = (camera_move.elapsed / camera_move.duration).clamp(0.0, 1.0);
    let target_pivot = target_pivot_now(&camera_move.target, &transforms, rig.pivot);
    let mut next = blend_rig(&camera_move.start, &camera_move.target, target_pivot, t);
    next.distance = next.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    *rig = next;
    active.0 = (!camera_move.finished()).then_some(camera_move);
}

/// CAM-05: with no move in flight, the pivot is the focused object's
/// position plus the offset, every frame.
pub fn follow_focus(
    active: Res<ActiveMove>,
    focus: Res<Focus>,
    transforms: Query<&GlobalTransform>,
    mut rig: ResMut<CameraRig>,
) {
    if active.0.is_some() {
        return;
    }
    let Some(position) = focus.body.and_then(|body| body_position(body, &transforms)) else {
        return;
    };
    rig.pivot = focused_pivot(position, focus.offset);
}

/// Place the camera from the rig.
pub fn place_camera(rig: Res<CameraRig>, mut camera: Query<&mut Transform, With<OrbitCamera>>) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    *transform = camera_transform(rig.pivot, rig.heading, rig.tilt, rig.distance);
}

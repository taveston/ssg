//! The camera rig: spawns the camera, starts it at the system frame, runs
//! the animated move (ZOOM-01) and places the camera from its state each
//! frame.

use bevy::prelude::*;

use super::state::{
    CameraFocus, MAX_DISTANCE, MIN_DISTANCE, OrbitCamera, OrbitCameraState, ZoomAnim, camera_fov,
    focus_position,
};
use super::stops::{ZoomStops, distance_for_radius};
use super::zoom::animated_distance;
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
        // Placed properly by `update_camera_transform`, but systems on the
        // first frame see this pose: far out and looking at the star, so
        // nothing measures as unfolded before the camera has settled.
        Transform::from_translation(SPAWN_POSITION).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub fn start_at_system_frame(
    stops: Res<ZoomStops>,
    tunables: Res<Tunables>,
    mut state: ResMut<OrbitCameraState>,
    camera: Query<&Projection, With<OrbitCamera>>,
) {
    let Ok(projection) = camera.single() else {
        return;
    };
    state.distance = distance_for_radius(
        stops.system_radius,
        camera_fov(projection),
        state.pitch,
        tunables.zoom_fit_margin,
    );
}

pub fn advance_zoom_anim(
    time: Res<Time>,
    transforms: Query<&GlobalTransform>,
    mut anim: ResMut<ZoomAnim>,
    mut state: ResMut<OrbitCameraState>,
    mut focus: ResMut<CameraFocus>,
) {
    if anim.duration <= 0.0 || anim.finished() {
        return;
    }
    anim.elapsed += time.delta_secs();
    let t = (anim.elapsed / anim.duration).clamp(0.0, 1.0);
    let eased = crate::view::anim::ease_out_cubic(t);

    // ZOOM-01: the step eases out onto its target distance.
    state.distance = animated_distance(
        anim.start_distance,
        anim.target_distance,
        anim.elapsed,
        anim.duration,
    )
    .clamp(MIN_DISTANCE, MAX_DISTANCE);

    // Blend towards where the target focus is *now*, so following a moving
    // body works, then hand over the real focus once we arrive.
    let target_position = focus_position(&anim.target_focus, &transforms);
    if t >= 1.0 {
        *focus = anim.target_focus;
    } else {
        *focus = CameraFocus::Fixed(anim.start_focus.lerp(target_position, eased));
    }
}

pub fn update_camera_transform(
    focus: Res<CameraFocus>,
    state: Res<OrbitCameraState>,
    transforms: Query<&GlobalTransform>,
    mut camera: Query<&mut Transform, With<OrbitCamera>>,
) {
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };
    let target = focus_position(&focus, &transforms);
    let offset = Vec3::new(
        state.distance * state.pitch.cos() * state.yaw.sin(),
        state.distance * state.pitch.sin(),
        state.distance * state.pitch.cos() * state.yaw.cos(),
    );
    camera_transform.translation = target + offset;
    camera_transform.look_at(target, Vec3::Y);
}

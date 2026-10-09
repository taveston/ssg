//! Hand-rolled orbit/zoom/pan camera. The camera only refocuses on a body
//! when `CameraFocus` is explicitly set to `Following` (from the inset's
//! Focus action in `hover_inspect.rs`) - clicking a marker inspects it, it
//! does not move the camera.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
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
struct OrbitCameraState {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for OrbitCameraState {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            // Steep, near-top-down by default.
            pitch: 1.15,
            distance: 300.0,
        }
    }
}

// The render-space span runs from a moon's orbit (a small fraction of a
// render unit) out to the gates (several hundred units), hence the huge
// zoom range and the multiplicative (logarithmic-feeling) zoom step below.
const MIN_DISTANCE: f32 = 0.05;
const MAX_DISTANCE: f32 = 900.0;
const PITCH_LIMIT: f32 = 1.55;

#[derive(Component)]
struct OrbitCamera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraFocus>()
            .init_resource::<OrbitCameraState>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    handle_orbit_input,
                    handle_zoom_input,
                    handle_pan_input,
                    update_camera_transform,
                )
                    .chain(),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        OrbitCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 40.0, 70.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn focus_position(focus: &CameraFocus, transforms: &Query<&GlobalTransform>) -> Vec3 {
    match *focus {
        CameraFocus::Fixed(pos) => pos,
        CameraFocus::Following(entity) => transforms
            .get(entity)
            .map(GlobalTransform::translation)
            .unwrap_or(Vec3::ZERO),
    }
}

fn handle_orbit_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut state: ResMut<OrbitCameraState>,
) {
    if !mouse_buttons.pressed(MouseButton::Right) {
        return;
    }
    let delta = mouse_motion.delta;
    state.yaw -= delta.x * 0.005;
    state.pitch = (state.pitch - delta.y * 0.005).clamp(-PITCH_LIMIT, PITCH_LIMIT);
}

fn handle_zoom_input(mouse_scroll: Res<AccumulatedMouseScroll>, mut state: ResMut<OrbitCameraState>) {
    if mouse_scroll.delta.y == 0.0 {
        return;
    }
    let zoom_factor = 1.0 - mouse_scroll.delta.y * 0.1;
    state.distance = (state.distance * zoom_factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
}

fn handle_pan_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    state: Res<OrbitCameraState>,
    mut focus: ResMut<CameraFocus>,
    transforms: Query<&GlobalTransform>,
) {
    if !mouse_buttons.pressed(MouseButton::Middle) {
        return;
    }
    let current = focus_position(&focus, &transforms);
    let delta = mouse_motion.delta;
    if delta == Vec2::ZERO {
        // Still dropping a moving focus to a fixed point the moment panning starts.
        if !matches!(*focus, CameraFocus::Fixed(_)) {
            *focus = CameraFocus::Fixed(current);
        }
        return;
    }

    let yaw_rot = Quat::from_rotation_y(state.yaw);
    let right = yaw_rot * Vec3::X;
    let forward_flat = yaw_rot * Vec3::Z;
    let pan_speed = state.distance * 0.0015;
    let pan = (-right * delta.x + forward_flat * delta.y) * pan_speed;
    *focus = CameraFocus::Fixed(current + pan);
}

fn update_camera_transform(
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

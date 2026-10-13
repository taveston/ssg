//! Keyboard camera (CAM-14, CAM-15, CAM-17, CAM-20): W, A, S, D and the
//! arrows pan relative to the screen, easing in and out, faster with
//! Shift; a cursor at the window edge pans the same way when edge
//! scrolling is on; Q and E turn, Page Up and Page Down tilt.

use bevy::prelude::*;

use super::control::CameraControl;
use super::rules::{
    KeyPanSettings, edge_scroll_direction, key_pan_velocity, plane_axes, ramp_velocity,
    visible_height_at_pivot,
};
use super::state::{OrbitCamera, camera_fov};
use super::stops::ZoomStops;
use crate::view::bindings::{Bindings, held};
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// The key pan's current velocity, along the screen's right (x) and up (y)
/// in plane units per second, so it can ramp (CAM-14).
#[derive(Resource, Default)]
pub struct KeyPan(Vec2);

/// The direction the pan keys and the window edge ask for.
fn pan_direction(
    keys: &ButtonInput<KeyCode>,
    bindings: &Bindings,
    tunables: &Tunables,
    window: Option<&Window>,
) -> Vec2 {
    let axis =
        |plus: &[KeyCode], minus: &[KeyCode]| held(keys, plus) as i32 - held(keys, minus) as i32;
    let mut direction = Vec2::new(
        axis(&bindings.pan_right, &bindings.pan_left) as f32,
        axis(&bindings.pan_up, &bindings.pan_down) as f32,
    );
    // CAM-17: off by default.
    if tunables.edge_scroll_enabled
        && let Some(window) = window
        && let Some(cursor) = window.cursor_position()
    {
        let size = Vec2::new(window.width(), window.height());
        direction += edge_scroll_direction(cursor, size, tunables.edge_scroll_px);
    }
    direction
}

/// CAM-14 / CAM-17: pan at a fraction of the visible height per second,
/// ramping in and out; panning moves the frame's offset (CAM-07).
#[allow(clippy::too_many_arguments)]
pub fn handle_key_pan(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    stops: Res<ZoomStops>,
    windows: Query<&Window>,
    camera: Query<&Projection, With<OrbitCamera>>,
    positions: Res<WorldPositions>,
    mut key_pan: ResMut<KeyPan>,
    mut view: CameraControl,
) {
    let direction = pan_direction(&keys, &bindings, &tunables, windows.single().ok());
    // Only keys actually held are pan input (CAM-20): a ramp still easing
    // out gives way to a move started meanwhile.
    if direction == Vec2::ZERO && view.active.0.is_some() {
        key_pan.0 = Vec2::ZERO;
        return;
    }
    let fov = camera
        .single()
        .map_or(std::f32::consts::FRAC_PI_4, camera_fov);
    let visible_height = visible_height_at_pivot(view.rig.distance, fov);
    let fast = held(&keys, &bindings.pan_fast);
    let settings = KeyPanSettings {
        screen_per_s: tunables.key_pan_screen_per_s,
        fast_factor: tunables.key_pan_fast_factor,
    };
    let target = key_pan_velocity(direction, visible_height, view.rig.tilt, fast, settings);
    // Full speed is the fastest the keys ask for, which sets the ramp's rate.
    let full = key_pan_velocity(Vec2::Y, visible_height, view.rig.tilt, fast, settings).length();
    let dt = time.delta_secs();
    let ramp_s = tunables.key_pan_ramp_ms / crate::view::anim::MS_PER_SEC;
    key_pan.0 = ramp_velocity(key_pan.0, target, full, dt, ramp_s);
    if key_pan.0 == Vec2::ZERO {
        return;
    }
    let (right, up) = plane_axes(view.rig.heading);
    let delta = (right * key_pan.0.x + up * key_pan.0.y) * dt;
    view.pan(
        delta,
        &positions,
        tunables.pan_bound_factor * stops.system_radius,
    );
}

/// CAM-15: Q and E turn, Page Up tilts towards the horizon and Page Down
/// towards straight down, at `key_rotate_deg_per_s`, tilt clamped (CAM-01).
pub fn handle_key_rotate(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    positions: Res<WorldPositions>,
    mut view: CameraControl,
) {
    let axis =
        |plus: &[KeyCode], minus: &[KeyCode]| held(&keys, plus) as i32 - held(&keys, minus) as i32;
    let rate = tunables.key_rotate_deg_per_s.to_radians() * time.delta_secs();
    let heading = axis(&bindings.turn_left, &bindings.turn_right) as f32 * rate;
    let tilt = axis(&bindings.tilt_up, &bindings.tilt_down) as f32 * rate;
    view.rotate(heading, tilt, tunables.max_pitch_deg, &positions);
}

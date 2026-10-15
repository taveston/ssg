//! The camera rig: spawns the camera, starts it at the system fit
//! (CAM-02), runs the animated move (ZOOM-01), keeps the pivot on the frame
//! body (CAM-05) and places the camera from the rig each frame.

use bevy::camera::Viewport;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::fits::FALLBACK_VIEWPORT;
use super::rules::camera_transform;
use super::state::{
    ActiveMove, CameraMove, CameraRig, MAX_DISTANCE, MIN_DISTANCE, MoveTarget, OrbitCamera,
    blend_rig, camera_fov,
};
use super::stops::{ZoomStops, distance_for_radius};
use crate::view::anim::ease_out_cubic;
use crate::view::frame::{Frame, body_position, frame_pivot};
use crate::view::screen::MapView;
use crate::view::tunables::Tunables;
use crate::view::world::{RenderOrigin, WorldPositions};

/// Render-unit clip planes wide enough for a moon's orbit up close and the
/// whole system with its gates, from the farthest the camera goes
/// (`MAX_DISTANCE`) with the pivot at the pan bound (CAM-19).
const NEAR_CLIP: f32 = 0.001;
const FAR_CLIP: f32 = 20_000.0;
/// Where the camera is spawned, in render units, before its first update.
const SPAWN_POSITION: Vec3 = Vec3::new(0.0, 40.0, 70.0);

/// The map camera, and a second camera for the overlay: the map camera
/// draws in the window less the sidebar (INS-07), while the overlay (top
/// bar, markers, sidebar) is laid out over the whole window, so it is
/// drawn by a camera of its own that clears nothing and draws after.
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
        // Nothing of the map (rings are gizmos) is drawn twice: this
        // camera renders the overlay only.
        RenderLayers::none(),
    ));
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

/// CAM-02: the camera starts at the system fit in the standard
/// orientation: north up, tilted by `standard_tilt_deg`.
pub fn start_at_system_fit(
    stops: Res<ZoomStops>,
    tunables: Res<Tunables>,
    mut rig: ResMut<CameraRig>,
    camera: Query<&Projection, With<OrbitCamera>>,
    map: MapView,
) {
    let Ok(projection) = camera.single() else {
        return;
    };
    let viewport = map.viewport().unwrap_or(FALLBACK_VIEWPORT);
    rig.tilt = tunables.standard_tilt_deg.to_radians();
    rig.distance = distance_for_radius(
        stops.system_radius,
        camera_fov(projection),
        rig.tilt,
        tunables.zoom_fit_margin,
        viewport,
    );
}

/// SCL-06: render space moved with the frame body since last frame, so the
/// points the camera holds from then move with it and stay where they were
/// in the system.
pub fn rebase_on_origin(
    origin: Res<RenderOrigin>,
    mut rig: ResMut<CameraRig>,
    mut active: ResMut<ActiveMove>,
) {
    rig.pivot += origin.shift;
    if let Some(camera_move) = active.0.as_mut() {
        camera_move.start.pivot += origin.shift;
    }
}

/// Where a move's target pivot is now: its body's position plus offset.
fn target_pivot_now(target: &MoveTarget, positions: &WorldPositions) -> Vec3 {
    frame_pivot(
        body_position(target.pivot.body, positions),
        target.pivot.offset,
    )
}

/// Start a move from the current rig (ZOOM-02), replacing any in flight.
/// The move puts the camera in its target body's frame at once, so a frame
/// jump shows the new frame from its first frame.
pub fn start_move(
    active: &mut ActiveMove,
    rig: &CameraRig,
    frame: &mut Frame,
    target: MoveTarget,
    duration_ms: f32,
) {
    let mut camera_move = CameraMove::new(*rig, target, duration_ms);
    // Already in this body's frame: the pivot started at its offset,
    // exactly, however far the body has moved since the rig was set.
    if frame.is_on(target.pivot.body) {
        camera_move.origin_offset = Some(frame.offset);
    }
    *frame = target.frame(frame);
    active.0 = Some(camera_move);
}

/// ZOOM-01: advance the move in flight and write the rig from it. The move
/// rides along with its target body: the pivot is the body's position now
/// plus an offset blended from where the move began.
pub fn advance_move(
    time: Res<Time>,
    positions: Res<WorldPositions>,
    mut active: ResMut<ActiveMove>,
    mut rig: ResMut<CameraRig>,
) {
    let Some(mut camera_move) = active.0 else {
        return;
    };
    camera_move.elapsed += time.delta_secs();
    let t = (camera_move.elapsed / camera_move.duration).clamp(0.0, 1.0);
    let target_pivot = target_pivot_now(&camera_move.target, &positions);
    let mut next = blend_rig(&camera_move.start, &camera_move.target, target_pivot, t);
    let position = body_position(camera_move.target.pivot.body, &positions);
    let from = *camera_move
        .origin_offset
        .get_or_insert(camera_move.start.pivot - position);
    next.pivot = position + from.lerp(camera_move.target.pivot.offset, ease_out_cubic(t));
    next.distance = next.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    *rig = next;
    active.0 = (!camera_move.finished()).then_some(camera_move);
}

/// The offset that keeps the frame body's pivot where the rig is now, so
/// taking up the frame again doesn't jump.
pub fn settle_frame(frame: &mut Frame, rig: &CameraRig, positions: &WorldPositions) {
    frame.offset = rig.pivot - body_position(frame.body, positions);
}

/// CAM-05: with no move in flight, the pivot is the frame body's position
/// plus the offset, every frame.
pub fn follow_frame(
    active: Res<ActiveMove>,
    frame: Res<Frame>,
    positions: Res<WorldPositions>,
    mut rig: ResMut<CameraRig>,
) {
    if active.0.is_some() {
        return;
    }
    rig.pivot = frame_pivot(body_position(frame.body, &positions), frame.offset);
}

/// INS-07: the camera draws in the window less the sidebar, so the pivot
/// (the frame body, CAM-05) sits at the centre of the visible map and the
/// scale, set by the height, is unchanged. The viewport follows the slide
/// frame by frame.
pub fn fit_camera_to_map(map: MapView, mut cameras: Query<&mut Camera, With<OrbitCamera>>) {
    let (Some(window), Some(viewport)) = (map.window(), map.viewport()) else {
        return;
    };
    let wanted = sidebar_viewport(window, viewport, map.scale_factor());
    let same = |a: &Option<Viewport>, b: &Option<Viewport>| match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.physical_position == b.physical_position && a.physical_size == b.physical_size
        }
        _ => false,
    };
    for mut camera in &mut cameras {
        if !same(&camera.viewport, &wanted) {
            camera.viewport = wanted.clone();
        }
    }
}

/// INS-07: the camera's viewport rectangle in physical pixels: the whole
/// window (`None`) when no sidebar shows, else the part left of it.
pub fn sidebar_viewport(window: Vec2, viewport: Vec2, scale: f32) -> Option<Viewport> {
    if viewport.x >= window.x {
        return None;
    }
    Some(Viewport {
        physical_position: UVec2::ZERO,
        physical_size: (viewport * scale).round().as_uvec2().max(UVec2::ONE),
        ..default()
    })
}

/// Place the camera from the rig.
pub fn place_camera(rig: Res<CameraRig>, mut camera: Query<&mut Transform, With<OrbitCamera>>) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    *transform = camera_transform(rig.pivot, rig.heading, rig.tilt, rig.distance);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::sol::sol_system;
    use crate::sim::system::OrbitPlacement;
    use crate::view::scale::km_to_render;

    /// ZOOM-15 / CAM-19: Sol, the widest system, fits the view inside the
    /// distance rail even straight down with the sidebar open, and from
    /// that distance with the pivot at the pan bound the far side of
    /// Neptune's orbit is still inside the far clip plane.
    #[test]
    fn the_widest_system_fits_inside_the_distance_rail_and_the_far_clip() {
        let tunables = Tunables::default();
        let widest = sol_system()
            .planets
            .iter()
            .map(|planet| match planet.orbit {
                OrbitPlacement::Own { radius_km } => km_to_render(radius_km),
                OrbitPlacement::Named(_) => 0.0,
            })
            .fold(0.0, f32::max);
        let with_sidebar = Vec2::new(1280.0 - tunables.inset_width_px, 720.0);
        let fit = distance_for_radius(
            widest,
            PerspectiveProjection::default().fov,
            0.0,
            tunables.zoom_fit_margin,
            with_sidebar,
        );
        assert!(fit < MAX_DISTANCE, "Sol's fit is {fit}");
        let farthest = MAX_DISTANCE + (1.0 + tunables.pan_bound_factor) * widest;
        assert!(farthest < FAR_CLIP, "{farthest} beyond the far clip");
    }
}

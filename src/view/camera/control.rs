//! The camera's one mutation interface: the rig, the focus and the move in
//! flight, changed together by every kind of camera input (zoom, drags,
//! keys, focus actions). Panning respects focus and the pan bound (CAM-07,
//! CAM-19), rotating the tilt clamp (CAM-01), and pan or rotate input ends
//! an animated move from where the camera is now (CAM-20).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::rig::{body_position, settle_focus, start_move};
use super::rules::clamp_tilt;
use super::state::{ActiveMove, CameraRig, MoveTarget};
use crate::view::focus::{Focus, clamp_offset_to_pan_bound, clamp_to_pan_bound, focused_pivot};
use crate::view::world::WorldPositions;

#[derive(SystemParam)]
pub struct CameraControl<'w> {
    pub rig: ResMut<'w, CameraRig>,
    pub focus: ResMut<'w, Focus>,
    pub active: ResMut<'w, ActiveMove>,
}

impl CameraControl<'_> {
    /// Before a new move replaces one in flight: a focus kept through the
    /// old move takes the offset of where the pivot is now, so nothing
    /// jumps (ZOOM-02).
    pub fn settle(&mut self, positions: &WorldPositions) {
        if self.active.0.is_none() {
            return;
        }
        settle_focus(&mut self.focus, &self.rig, positions);
    }

    /// Start a move from the current rig.
    pub fn start(&mut self, target: MoveTarget, duration_ms: f32) {
        let rig = *self.rig;
        start_move(&mut self.active, &rig, &mut self.focus, target, duration_ms);
    }

    /// CAM-20: pan or rotate input ends a move in flight and takes over from
    /// the camera's current state, with no jump.
    pub fn interrupt(&mut self, positions: &WorldPositions) {
        self.settle(positions);
        self.active.0 = None;
    }

    /// The pivot as it stands this frame: on the focused body plus the
    /// offset (CAM-05), else the rig's.
    pub fn pivot_now(&self, positions: &WorldPositions) -> Vec3 {
        match self
            .focus
            .body
            .and_then(|body| body_position(body, positions))
        {
            Some(position) if self.active.0.is_none() => focused_pivot(position, self.focus.offset),
            _ => self.rig.pivot,
        }
    }

    /// CAM-07 / CAM-19: pan the view by `delta` in the plane. Focused, it
    /// changes the offset and keeps the focus; either way the pivot stays
    /// inside the pan bound.
    pub fn pan(&mut self, delta: Vec3, positions: &WorldPositions, bound: f32) {
        self.interrupt(positions);
        let focused = self
            .focus
            .body
            .and_then(|body| body_position(body, positions));
        match focused {
            Some(position) => {
                let offset = clamp_offset_to_pan_bound(position, self.focus.offset + delta, bound);
                self.focus.offset = offset;
                self.rig.pivot = focused_pivot(position, offset);
            }
            None => self.rig.pivot = clamp_to_pan_bound(self.rig.pivot + delta, bound),
        }
    }

    /// CAM-01 / CAM-12 / CAM-15: turn the heading (unlimited) and tilt
    /// (clamped), about the pivot.
    pub fn rotate(
        &mut self,
        heading: f32,
        tilt: f32,
        max_tilt_deg: f32,
        positions: &WorldPositions,
    ) {
        if heading == 0.0 && tilt == 0.0 {
            return;
        }
        self.interrupt(positions);
        self.rig.heading += heading;
        self.rig.tilt = clamp_tilt(self.rig.tilt + tilt, max_tilt_deg);
    }
}

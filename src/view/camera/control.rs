//! The camera's one mutation interface: the rig, the frame and the move in
//! flight, changed together by every kind of camera input (zoom, drags,
//! keys, frame actions). Panning changes the frame's offset, never the
//! frame, inside the pan bound (CAM-07, CAM-19); rotating keeps the tilt
//! clamp (CAM-01); pan or rotate input ends an animated move from where the
//! camera is now (CAM-20).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::rig::{settle_frame, start_move};
use super::rules::clamp_tilt;
use super::state::{ActiveMove, CameraRig, MoveTarget, PivotTarget};
use super::stops::{SectorFit, ZoomStops, outer_radius};
use crate::view::bindings::Bindings;
use crate::view::frame::{Frame, body_position, clamp_offset_to_pan_bound, frame_pivot};
use crate::view::map_layer::MapLayer;
use crate::view::tunables::Tunables;
use crate::view::world::{RenderOrigin, WorldPositions};

/// What camera input reads besides the input itself: the keys, their
/// bindings (CAM-18), the tunables, the zoom stops and the layer shown.
#[derive(SystemParam)]
pub struct CameraSettings<'w> {
    pub keys: Res<'w, ButtonInput<KeyCode>>,
    pub bindings: Res<'w, Bindings>,
    pub tunables: Res<'w, Tunables>,
    pub stops: Res<'w, ZoomStops>,
    pub layer: Res<'w, MapLayer>,
    pub sector: Res<'w, SectorFit>,
}

impl CameraSettings<'_> {
    /// CAM-19 / SECT-10: how far from the star, or the sector's centre,
    /// the pivot may be panned.
    pub fn pan_bound(&self) -> f32 {
        self.tunables.pan_bound_factor * outer_radius(*self.layer, &self.stops, &self.sector)
    }
}

#[derive(SystemParam)]
pub struct CameraControl<'w> {
    pub origin: Res<'w, RenderOrigin>,
    pub rig: ResMut<'w, CameraRig>,
    pub frame: ResMut<'w, Frame>,
    pub active: ResMut<'w, ActiveMove>,
}

impl CameraControl<'_> {
    /// Before a new move replaces one in flight: the frame takes the offset
    /// of where the pivot is now, so nothing jumps (ZOOM-02).
    pub fn settle(&mut self, positions: &WorldPositions) {
        if self.active.0.is_none() {
            return;
        }
        // A move onto the frame body rides along with it, so its current
        // offset is the one the move has blended to; the rig's pivot is a
        // frame old, and the body has moved since.
        if let Some(offset) = self
            .active
            .0
            .as_ref()
            .and_then(|m| m.offset_now(self.frame.body))
        {
            self.frame.offset = offset;
            return;
        }
        settle_frame(&mut self.frame, &self.rig, positions);
    }

    /// Start a move from the current rig.
    pub fn start(&mut self, target: MoveTarget, duration_ms: f32) {
        let rig = *self.rig;
        start_move(&mut self.active, &rig, &mut self.frame, target, duration_ms);
    }

    /// A move target in the star's frame, with the pivot at `point` from
    /// the star.
    pub fn star_pivot(&self, point: Vec3) -> PivotTarget {
        PivotTarget {
            body: self.frame.star,
            offset: point,
        }
    }

    /// The current frame as a move target: the frame body and its offset.
    pub fn frame_pivot(&self) -> PivotTarget {
        PivotTarget {
            body: self.frame.body,
            offset: self.frame.offset,
        }
    }

    /// CAM-20: pan or rotate input ends a move in flight and takes over from
    /// the camera's current state, with no jump.
    pub fn interrupt(&mut self, positions: &WorldPositions) {
        self.settle(positions);
        self.active.0 = None;
    }

    /// The pivot as it stands this frame: the frame body plus the offset
    /// (CAM-05), or the rig's while a move is in flight.
    pub fn pivot_now(&self, positions: &WorldPositions) -> Vec3 {
        if self.active.0.is_some() {
            return self.rig.pivot;
        }
        frame_pivot(body_position(self.frame.body, positions), self.frame.offset)
    }

    /// CAM-07 / CAM-19: pan the view by `delta` in the plane. It changes the
    /// frame's offset and never the frame, and the pivot stays inside the
    /// pan bound.
    pub fn pan(&mut self, delta: Vec3, positions: &WorldPositions, bound: f32) {
        self.interrupt(positions);
        let position = body_position(self.frame.body, positions);
        let from_star = position - self.origin.star;
        let offset = clamp_offset_to_pan_bound(from_star, self.frame.offset + delta, bound);
        self.frame.offset = offset;
        self.rig.pivot = frame_pivot(position, offset);
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

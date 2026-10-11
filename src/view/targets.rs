//! Links from a pickable UI node back to the body it stands for. Shared by
//! everything that draws a clickable node for a body (markers, pills, ring
//! hit targets) and by the pointer handlers in `hover.rs`, so neither side
//! has to import the other (ARCH-03).

use bevy::prelude::*;

/// A marker (or exposure-ring hit target) -> the body it represents.
/// Hovering and clicking it target that body (HOV-01, INS-01, MOT-05).
#[derive(Component)]
pub struct MarkerFor(pub Entity);

/// A UI panel the map's drags don't start on: pressing on the top bar or
/// an inset and dragging doesn't pan or rotate the camera (CAM-10).
#[derive(Component)]
pub struct MapInputBlocker;

/// A "+N" count pill -> the parent whose folded satellites it counts.
/// Clicking it does what clicking the parent does (GRP-07).
#[derive(Component)]
pub struct GroupPillFor(pub Entity);

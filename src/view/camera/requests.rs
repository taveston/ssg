//! What the rest of the view can ask of the camera, without reaching into
//! its state: set by pointer handlers, consumed by the camera next frame.
//! This is the seam the camera iteration (ZOOM-16, ZOOM-17, CAM-06)
//! rebuilds behind.

use bevy::prelude::*;

/// A request to frame a body's satellites, raised by a double-click or the
/// focus key and consumed by the camera next frame. Keeps the pointer
/// observers from needing the camera's whole system-parameter set.
#[derive(Resource, Default)]
pub struct FocusJumpRequest(pub Option<Entity>);

/// Set when the player double-clicks a body that's already inspected. The
/// planetary view doesn't exist yet; this is the hook it will hang from.
#[derive(Resource, Default)]
pub struct PlanetaryViewRequest(pub Option<Entity>);

//! What the rest of the view can ask of the camera, without reaching into
//! its state: set by pointer handlers, consumed by the camera next frame.
//! ZOOM-16, ZOOM-17 and CAM-13 meet the camera here.

use bevy::prelude::*;

/// ZOOM-16: a request to focus a body and frame its satellites, raised by a
/// double-click on a body that isn't focused or by the inset's Focus button,
/// and taken by the camera next frame. Keeps the pointer observers from
/// needing the camera's whole system-parameter set.
#[derive(Resource, Default)]
pub struct FocusRequest(pub Option<Entity>);

/// Set when the planetary view is asked for (ZOOM-17). The view doesn't
/// exist yet; `planetary_view_hook` takes the request.
#[derive(Resource, Default)]
pub struct PlanetaryViewRequest(pub Option<Entity>);

/// CAM-13: a right-click (a right press and release under the drag
/// threshold), at this screen position. Reserved for context menus and
/// orders; until those exist, `context_menu_hook` takes it and does
/// nothing visible.
#[derive(Resource, Default)]
pub struct ContextMenuRequest(pub Option<Vec2>);

/// CAM-13: the hook context menus and orders will hang from. Nothing
/// visible happens yet.
pub fn context_menu_hook(mut request: ResMut<ContextMenuRequest>) {
    if let Some(position) = request.0.take() {
        debug!("context menu requested at {position} (not built)");
    }
}

/// ZOOM-17: the hook the planetary view will hang from. Nothing visible
/// happens yet.
pub fn planetary_view_hook(mut request: ResMut<PlanetaryViewRequest>) {
    if let Some(body) = request.0.take() {
        debug!("planetary view requested for {body} (not built)");
    }
}

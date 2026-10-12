//! The camera (`docs/spec/system-view.md`, section 5 and 5a: ZOOM-01,
//! ZOOM-02, ZOOM-05, ZOOM-10 to ZOOM-17, CAM-01 to CAM-22). It looks at a
//! pivot on the orbital plane from a heading, a tilt and a distance, and
//! tracks the focused object (`view/focus.rs`); selection never moves it.
//!
//! - `state.rs`: the rig, the move in flight, bookmarks.
//! - `rules.rs`, `zoom.rs`, `stops.rs`: the geometry and rules, as plain
//!   functions.
//! - `control.rs`: the one way input changes the camera.
//! - `zoom_input.rs`, `drag.rs`, `keys.rs`, `actions.rs`: the inputs,
//!   all read through `view/bindings.rs` (CAM-18).
//! - `requests.rs`: what the rest of the view can ask of the camera, and the
//!   context-menu and planetary-view hooks.
//! - `rig.rs`: runs the move, follows the focus, places the camera.

use bevy::prelude::*;

use crate::{Boot, Stage};

pub mod actions;
pub mod control;
pub mod drag;
pub mod frames;
pub mod keys;
pub mod requests;
pub mod rig;
pub mod rules;
pub mod state;
pub mod stops;
pub mod zoom;
pub mod zoom_input;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(stops::StopsPlugin)
            .init_resource::<state::CameraRig>()
            .init_resource::<state::ActiveMove>()
            .init_resource::<state::Bookmarks>()
            .init_resource::<requests::FocusRequest>()
            .init_resource::<requests::PlanetaryViewRequest>()
            .init_resource::<requests::ContextMenuRequest>()
            .init_resource::<drag::MouseDrag>()
            .init_resource::<keys::KeyPan>()
            .add_systems(
                Startup,
                (rig::spawn_camera, rig::start_at_system_frame)
                    .chain()
                    .in_set(Boot::Overlay),
            )
            .add_systems(
                Update,
                (
                    drag::begin_press,
                    drag::update_drag,
                    drag::end_press,
                    keys::handle_key_pan,
                    keys::handle_key_rotate,
                    zoom_input::handle_zoom_input,
                    actions::handle_home,
                    actions::handle_north_up,
                    actions::handle_focus_requests,
                    actions::handle_bookmarks,
                    rig::advance_move,
                    rig::follow_focus,
                    rig::place_camera,
                    requests::context_menu_hook,
                    requests::planetary_view_hook,
                )
                    .chain()
                    .in_set(Stage::Camera),
            );
    }
}

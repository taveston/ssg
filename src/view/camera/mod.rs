//! The camera (`docs/spec/system-view.md`, section 5 and 5a: ZOOM-01,
//! ZOOM-02, ZOOM-05, ZOOM-10 to ZOOM-17, CAM-01 to CAM-25). It looks at a
//! pivot on the orbital plane from a heading, a tilt and a distance, and
//! sits in the reference frame of one body (`view/frame.rs`), tracking it;
//! selection never moves it. The sector view uses the same camera, in the
//! star's frame with the sector's centre for the star, its fits and limits
//! the sector's (SECT-10 to SECT-12).
//!
//! - `state.rs`: the rig, the move in flight, bookmarks.
//! - `rules.rs`, `zoom.rs`, `stops.rs`: the geometry and rules, as plain
//!   functions; `fits.rs`: the distances they give for this window.
//! - `control.rs`: the one way input changes the camera.
//! - `zoom_input.rs`, `drag.rs`, `keys.rs`, `actions.rs`: the inputs,
//!   all read through `view/bindings.rs` (CAM-18).
//! - `requests.rs`: what the rest of the view can ask of the camera, and the
//!   context-menu and planetary-view hooks.
//! - `rig.rs`: runs the move, follows the frame body, places the camera.
//! - `keep.rs`, `release.rs`: keep the frame body on screen, and move the
//!   frame off a body that turns into an exposure ring.

use bevy::prelude::*;

use crate::{Boot, Stage};

pub mod actions;
pub mod control;
pub mod drag;
pub mod fits;
pub mod keep;
pub mod keys;
pub mod release;
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
            .init_resource::<requests::FrameRequest>()
            .init_resource::<requests::RecentreRequest>()
            .init_resource::<requests::NorthUpRequest>()
            .init_resource::<requests::PlanetaryViewRequest>()
            .init_resource::<requests::ContextMenuRequest>()
            .init_resource::<drag::MouseDrag>()
            .init_resource::<keys::KeyPan>()
            // CAM-14: the camera reads whether a tray has the arrow keys.
            .init_resource::<crate::view::multi::OpenTray>()
            // SECT-10: the fits and the pan bound follow the layer shown.
            .init_resource::<crate::view::map_layer::MapLayer>()
            .add_systems(
                Startup,
                (rig::spawn_camera, rig::start_at_system_fit)
                    .chain()
                    .in_set(Boot::Overlay),
            )
            .add_systems(
                Update,
                (
                    rig::rebase_on_origin,
                    drag::begin_press,
                    drag::update_drag,
                    drag::end_press,
                    keys::handle_key_pan,
                    keys::handle_key_rotate,
                    zoom_input::handle_zoom_input,
                    actions::handle_home,
                    actions::handle_north_up,
                    actions::handle_frame_requests,
                    actions::handle_recentre_request,
                    actions::handle_bookmarks,
                    keep::keep_frame_body_in_view,
                    release::release_ring_mode_frame,
                    rig::advance_move,
                    rig::follow_frame,
                    rig::fit_camera_to_map,
                    rig::place_camera,
                    requests::context_menu_hook,
                    requests::planetary_view_hook,
                )
                    .chain()
                    .in_set(Stage::Camera),
            );
    }
}

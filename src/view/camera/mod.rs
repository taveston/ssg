//! The camera: a hand-rolled orbit/zoom/pan rig (`docs/spec/system-view.md`,
//! ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-10, ZOOM-11, ZOOM-15).
//!
//! Zoom is stepped and animated: each wheel notch multiplies the view scale
//! by a fixed factor over an ease-out animation, resisting slightly at stops
//! that frame something meaningful (`stops.rs`, `zoom.rs`) and refusing to
//! go past the system or satellite frames.
//!
//! The anchoring, limits, latching and double-click behaviour still follow
//! the withdrawn ZOOM-03, ZOOM-04, ZOOM-06, ZOOM-08, ZOOM-09 and ZOOM-12
//! (F4-13): the camera iteration replaces them. Input handling is kept in
//! `input.rs` so that rework can replace it without touching the rest.

use bevy::prelude::*;

use crate::{Boot, Stage};

pub mod input;
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
            .init_resource::<requests::FocusJumpRequest>()
            .init_resource::<requests::PlanetaryViewRequest>()
            .add_systems(
                Startup,
                (rig::spawn_camera, rig::start_at_system_frame)
                    .chain()
                    .in_set(Boot::Overlay),
            )
            .add_systems(
                Update,
                (
                    input::handle_orbit_input,
                    zoom_input::handle_zoom_input,
                    input::handle_navigation_keys,
                    input::handle_focus_jump,
                    input::handle_bookmarks,
                    input::handle_pan_input,
                    rig::advance_move,
                    rig::follow_focus,
                    rig::place_camera,
                )
                    .chain()
                    .in_set(Stage::Camera),
            );
    }
}

//! View (ARCH-01): everything drawn - the km to render conversion, the
//! camera, markers, rings, pills, grouping, the motion rule, gate edge
//! arrows, the hover tooltip, inspection insets, the top bar, the palette,
//! fonts, fidelity images and the presentation tunables.
//!
//! The view reads the simulation and changes it only through its API
//! (ARCH-14). State that several view modules share lives in its own small
//! module - `selection`, `fold`, `motion_mode`, `targets`, `screen` - so no
//! two feature modules import each other (ARCH-03).

use bevy::prelude::*;

pub mod anchors;
pub mod anim;
pub mod belt;
pub mod bindings;
pub mod breadcrumb;
pub mod camera;
pub mod colors;
pub mod date;
pub mod edge;
pub mod fidelity;
pub mod fold;
pub mod fonts;
pub mod frame;
pub mod frame_marks;
pub mod gesture;
pub mod grouping;
pub mod hover;
pub mod in_orbit;
pub mod inspect;
pub mod marker_images;
pub mod markers;
pub mod motion;
pub mod motion_mode;
pub mod pills;
pub mod projection;
pub mod ring_picking;
pub mod rings;
pub mod scale;
pub mod screen;
pub mod selection;
pub mod targets;
pub mod topbar;
pub mod true_size;
pub mod tunables;
pub mod world;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        // Shared state with no single owner: selection is written by clicks
        // and the inset's buttons, the frame by the camera, and both are
        // read across the view.
        app.init_resource::<selection::Selection>()
            .init_resource::<gesture::PressStart>()
            .add_plugins((
                frame::FramePlugin,
                tunables::TunablesPlugin,
                bindings::BindingsPlugin,
                fonts::FontsPlugin,
                fidelity::FidelityPlugin,
                projection::ProjectionPlugin,
                belt::BeltPlugin,
                camera::CameraPlugin,
                breadcrumb::BreadcrumbPlugin,
            ))
            .add_plugins((
                grouping::GroupingPlugin,
                motion::MotionPlugin,
                rings::RingsPlugin,
                ring_picking::RingPickingPlugin,
                screen::ScreenPlugin,
                anchors::AnchorsPlugin,
                frame_marks::FrameMarksPlugin,
                edge::EdgePlugin,
                marker_images::MarkerImagesPlugin,
                markers::MarkersPlugin,
                pills::PillsPlugin,
                hover::HoverPlugin,
                inspect::InspectPlugin,
                in_orbit::InOrbitPlugin,
                topbar::TopBarPlugin,
            ));
    }
}

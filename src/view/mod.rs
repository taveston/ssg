//! View (ARCH-01): everything drawn - the km to render conversion, the
//! camera, markers, rings, multi-markers, grouping, the motion rule, gate
//! edge arrows, inspection insets, the top bar, the palette, fonts, the
//! label halo, the frame cue, fidelity images and the presentation
//! tunables, and a debug screenshot option (`debug_shot`).
//!
//! The view reads the simulation and changes it only through its API
//! (ARCH-14). State that several view modules share lives in its own small
//! module at this level - `selection`, `fold`, `multi`, `motion_mode`,
//! `targets`, `screen`, `shared_orbit`, `sidebar` and others - or, for the
//! planner, `planner/state.rs`, so no two feature modules import each
//! other (ARCH-03).
//!
//! Features with several modules have a folder each: `camera/`, `markers/`
//! (markers, their picking, labels and brackets), `multi_markers/`
//! (several objects on one marker), `tray/` (the list a multi-marker
//! opens), `rings/` (orbit rings, their marks,
//! picking and near arcs), `symbols/` (ship symbols,
//! their drawing and shader), `edge/` (edge arrows and tabs),
//! `inset/` (the inspection sidebar's contents), `topbar/`, `frame_cue/`,
//! `planner/` (the transfer planner) and `paths/` (transfers drawn on the
//! map).

use bevy::prelude::*;

pub mod anim;
pub mod belt;
pub mod bindings;
pub mod buttons;
pub mod camera;
pub mod colors;
pub mod countdown;
pub mod date;
pub mod debug_shot;
pub mod edge;
pub mod fold;
pub mod fonts;
pub mod frame;
pub mod frame_cue;
pub mod gesture;
pub mod grouping;
pub mod halo;
pub mod hover;
pub mod hovered;
pub mod inset;
pub mod layers;
pub mod list_rows;
pub mod markers;
pub mod motion;
pub mod motion_mode;
pub mod multi;
pub mod multi_markers;
pub mod node_box;
pub mod paths;
pub mod planner;
pub mod plot_marks;
pub mod projection;
pub mod readout;
pub mod rings;
pub mod scale;
pub mod screen;
pub mod selection;
pub mod shared_orbit;
pub mod ship_groups;
pub mod sidebar;
pub mod symbols;
pub mod targets;
pub mod topbar;
pub mod tray;
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
            .init_resource::<planner::state::PlannerState>()
            .init_resource::<multi::OpenTray>()
            .init_resource::<multi::TrayRequest>()
            .init_resource::<multi::MultiMarkers>()
            .init_resource::<gesture::PressStart>()
            .add_plugins((
                frame::FramePlugin,
                hovered::HoveredPlugin,
                sidebar::SidebarPlugin,
                tunables::TunablesPlugin,
                bindings::BindingsPlugin,
                fonts::FontsPlugin,
                inset::fidelity::FidelityPlugin,
                projection::ProjectionPlugin,
                belt::BeltPlugin,
                camera::CameraPlugin,
                topbar::breadcrumb::BreadcrumbPlugin,
                buttons::ButtonsPlugin,
                plot_marks::PlotMarksPlugin,
                markers::picking::MarkerPickingPlugin,
                edge::parent_tab::ParentTabPlugin,
            ))
            .add_plugins((
                shared_orbit::SharedOrbitPlugin,
                rings::marks::RingMarksPlugin,
                rings::near_arcs::NearArcsPlugin,
            ))
            .add_plugins((
                grouping::GroupingPlugin,
                motion::MotionPlugin,
                rings::RingsPlugin,
                rings::picking::RingPickingPlugin,
                screen::ScreenPlugin,
                edge::EdgePlugin,
                symbols::shape::SymbolShapePlugin,
                markers::MarkersPlugin,
                multi_markers::MultiMarkersPlugin,
                hover::HoverPlugin,
                inset::inspect::InspectPlugin,
                inset::outline::OutlinePlugin,
                topbar::TopBarPlugin,
            ))
            .add_plugins((
                markers::labels::LabelsPlugin,
                halo::HaloPlugin,
                markers::brackets::BracketsPlugin,
                true_size::TrueSizePlugin,
                debug_shot::DebugShotPlugin,
                planner::PlannerPlugin,
                paths::PathsPlugin,
                paths::ghost::GhostMaterialPlugin,
                frame_cue::FrameCuePlugin,
                frame_cue::stroke::CueStrokeMaterialPlugin,
                tray::TrayPlugin,
            ));
    }
}

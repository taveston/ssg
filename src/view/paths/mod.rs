//! Transfers on the map (`docs/spec/planner.md` section 6, PLNR-60 to
//! PLNR-69): the path of the leg being planned, of the hovered place
//! while choosing, and of each own ship's order, drawn at true scale in
//! the geometry of its own dates; the ghosts of the objects at its ends;
//! the arrival ghost's label; and the coast after an Intercept up to the
//! first thing that happens on it.
//!
//! - `look.rs`: which paths and ghosts show, and how strongly (PLNR-62,
//!   PLNR-64).
//! - `gather.rs`: this frame's paths from the planner's state and the
//!   ships' plans (PLNR-62 to PLNR-67).
//! - `geometry.rs`: sampling a rail, splitting it at today, dashes on
//!   screen (PLNR-60, PLNR-61).
//! - `strokes.rs`: the lines, as world gizmos (PLNR-60, PLNR-61).
//! - `marks.rs`: start dots, ghosts and labels, as UI nodes (PLNR-61,
//!   PLNR-63 to PLNR-65, PLNR-67, PLNR-69).
//! - `ghost.rs`: the ghost's material (PLNR-63).
//!
//! Ships in flight themselves are drawn as any ship is, as a dot: the
//! motion rule and grouping leave them out (PLNR-68, `motion.rs`,
//! `grouping.rs`). Nothing here is a hover or click target (PLNR-69).

use bevy::prelude::*;

use super::markers::MarkersPlaced;
use super::planner::PlannerUpdated;
use crate::{Boot, Stage};

pub mod gather;
pub mod geometry;
pub mod ghost;
pub mod look;
pub mod marks;
pub mod strokes;

/// The paths and their marks; the ghost's material is
/// `ghost::GhostMaterialPlugin`, which needs a renderer.
pub struct PathsPlugin;

impl Plugin for PathsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<gather::MapPaths>()
            .init_resource::<strokes::DrawnPaths>()
            .add_systems(Startup, strokes::spawn_path_strokes.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    gather::gather_paths,
                    strokes::draw_paths,
                    marks::place_path_marks,
                )
                    .chain()
                    .after(PlannerUpdated)
                    .in_set(MarkersPlaced)
                    .in_set(Stage::Overlay),
            );
    }
}

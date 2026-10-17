//! The sector view (`docs/spec/sector-view.md`): the top layer of the map,
//! the generated sector drawn in the system view's style with the same
//! camera. It is a truth view: every star and every link, whatever anyone
//! knows, and it says so on screen (SECT-08).
//!
//! It shows only while `MapLayer` is `Sector` (SECT-01); while the system
//! view is shown every part of it is hidden and takes no input.
//!
//! - `nodes.rs`: a node per catalogue entry, placed and layered by depth
//!   each frame (SECT-03, SECT-05), and the sector fit (SECT-11).
//! - `links.rs`: the links, drawn by state (SECT-04).
//! - `labels.rs`: the nodes' labels (SECT-06).
//! - `input.rs`: hover, click and the selection marks (SECT-07, SECT-20).
//! - `readout.rs`: the truth readout (SECT-08).
//! - `inset/`: the inspection inset (SECT-20 to SECT-24).
//!
//! The camera, the scale bar and the north dial are the system view's,
//! told by `MapLayer` which layer they serve (SECT-09 to SECT-12).

use bevy::prelude::*;

use crate::{Boot, Stage};

pub mod input;
pub mod inset;
pub mod labels;
pub mod links;
pub mod nodes;
pub mod readout;

/// What a node stands for: a gated system, an ungated star system (a giant
/// among them) or a brown dwarf (SECT-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Gated,
    Ungated,
    BrownDwarf,
}

/// A node of the sector view: the catalogue entry it stands for, what
/// kind it is, and where it sits in render space (SECT-03).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SectorNode {
    pub id: usize,
    pub kind: NodeKind,
    pub world: Vec3,
}

/// On anything that stands for a node to the pointer, its hit target or
/// its label: the catalogue entry it stands for (SECT-07, SECT-20).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForEntry(pub usize);

/// SECT-21 / SECT-30: a gated system the player has asked to enter, by
/// entry id: set by the inset's Enter system button, taken by whatever
/// enters the system.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EnterRequest(pub Option<usize>);

/// Every node's place on screen this frame has been worked out.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodesPlaced;

pub struct SectorPlugin;

impl Plugin for SectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<nodes::NodeScreens>()
            .init_resource::<EnterRequest>()
            .init_resource::<input::NodeSpots>()
            .init_resource::<links::LinkPieces>()
            .add_systems(Startup, nodes::fit_the_sector.in_set(Boot::Derive))
            .add_systems(
                Startup,
                (
                    nodes::spawn_nodes,
                    labels::spawn_labels,
                    links::spawn_link_strokes,
                    readout::spawn_readout,
                    inset::spawn_panel,
                )
                    .chain()
                    .in_set(Boot::Overlay),
            )
            .add_systems(
                Update,
                nodes::place_nodes.in_set(NodesPlaced).in_set(Stage::Screen),
            )
            .add_systems(
                Update,
                (
                    input::find_node_spots,
                    input::mark_nodes,
                    labels::place_node_labels,
                )
                    .chain()
                    .after(NodesPlaced)
                    .in_set(Stage::Layout),
            )
            .add_systems(
                Update,
                (
                    links::draw_links,
                    readout::show_readout,
                    input::clear_on_empty_click,
                    input::handle_escape,
                    inset::rebuild_inset,
                    inset::slide_panel,
                    inset::scroll_panel,
                )
                    .chain()
                    .in_set(Stage::Overlay),
            )
            .add_systems(
                PreUpdate,
                input::pick_nodes.in_set(bevy::picking::PickingSystems::Backend),
            );
    }
}

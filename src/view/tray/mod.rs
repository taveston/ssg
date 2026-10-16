//! A multi-marker's **tray** (MUL-07 to MUL-10, MUL-12): a small panel
//! beside the marker listing what it holds, so any one of them can be
//! picked without leaving the map. A click on a multi-marker, or on a
//! shared orbit's ring or identity mark, opens its tray (the click
//! handlers ask through `TrayRequest`); a click anywhere else, Esc, or the
//! marker ceasing to exist closes it. At most one is open (`OpenTray`).
//!
//! `rows.rs` says what a tray lists and says, as plain functions;
//! `contents.rs` gathers that from the world each frame; `place.rs`
//! places it beside its marker inside the map area; `draw.rs` builds and
//! places the panel; `input.rs` opens and closes it and takes its keys
//! and row clicks.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::view::markers::MarkersPlaced;
use crate::view::multi::{OpenTray, TrayRequest};
use crate::view::multi_markers::MultiGathered;
use crate::{Boot, Stage};

pub mod contents;
pub mod draw;
pub mod input;
pub mod place;
pub mod rows;

/// The open tray's own state: the selected row and the rows opened in
/// place. Reset when another tray opens.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct TrayState {
    /// The selected row (MUL-09, MUL-10). Usually the selection itself;
    /// for a group's member, the member, while its group is selected.
    pub cursor: Option<Entity>,
    /// Rows opened to list their members (MUL-09).
    pub open_rows: HashSet<Entity>,
}

pub struct TrayPlugin;

impl Plugin for TrayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OpenTray>()
            .init_resource::<TrayRequest>()
            .init_resource::<TrayState>()
            .init_resource::<contents::TrayContents>()
            .add_systems(Startup, draw::spawn_tray.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    input::open_or_close,
                    input::tray_keys,
                    contents::gather_tray,
                    draw::fill_tray,
                    draw::place_tray_panel,
                    draw::dim_rows,
                    input::scroll_tray,
                )
                    .chain()
                    .in_set(Stage::Overlay)
                    .after(MultiGathered)
                    .after(MarkersPlaced),
            );
    }
}

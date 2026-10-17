//! Which layer of the map is shown, the sector view or the system view
//! (SECT-01), and the sector view's selection. Exactly one layer is shown
//! at a time; every part of the view that differs between them reads
//! `MapLayer`, so the two never draw at once. Shared state, so the camera,
//! the sidebar, the plot marks and both views read it without importing
//! one another (ARCH-03).
//!
//! The game starts in the sector view when it was started with a sector
//! (`content::start::LoadedSector`), and in the system view otherwise.

use bevy::prelude::*;

use crate::Boot;
use crate::content::start::LoadedSector;

/// SECT-01: the layer shown.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MapLayer {
    /// One system, its bodies round its star (`docs/spec/system-view.md`).
    #[default]
    System,
    /// The whole sector, its stars and the links between them.
    Sector,
}

impl MapLayer {
    /// Whether the sector view is shown.
    pub fn is_sector(self) -> bool {
        self == MapLayer::Sector
    }
}

/// SECT-20: the sector view's selection: the catalogue entry whose node
/// is selected and whose inset is open, by entry id. There is at most
/// one. It is kept apart from the system view's `Selection`, so it stays
/// as it was left while a system is shown.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SectorSelection(pub Option<usize>);

impl SectorSelection {
    /// Whether the entry `id` is selected.
    pub fn is_selected(&self, id: usize) -> bool {
        self.0 == Some(id)
    }
}

/// Whether anything the shown layer can select is selected: the sidebar
/// is open while it is (INS-07, SECT-20).
pub fn layer_has_selection(layer: MapLayer, system: bool, sector: &SectorSelection) -> bool {
    match layer {
        MapLayer::System => system,
        MapLayer::Sector => sector.0.is_some(),
    }
}

pub struct MapLayerPlugin;

impl Plugin for MapLayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapLayer>()
            .init_resource::<SectorSelection>()
            .add_systems(Startup, start_in_the_sector.in_set(Boot::World));
    }
}

/// SECT-01 / SECT-40: a game started with a sector starts in the sector
/// view.
fn start_in_the_sector(sector: Option<Res<LoadedSector>>, mut layer: ResMut<MapLayer>) {
    *layer = if sector.is_some() {
        MapLayer::Sector
    } else {
        MapLayer::System
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SECT-01 / INS-07: the sidebar follows the shown layer's selection
    /// only: a system selection doesn't open it over the sector view, nor
    /// a sector one over the system view.
    #[test]
    fn the_sidebar_follows_the_shown_layers_selection() {
        let picked = SectorSelection(Some(3));
        let none = SectorSelection::default();
        assert!(layer_has_selection(MapLayer::Sector, false, &picked));
        assert!(!layer_has_selection(MapLayer::Sector, true, &none));
        assert!(layer_has_selection(MapLayer::System, true, &none));
        assert!(!layer_has_selection(MapLayer::System, false, &picked));
        assert!(picked.is_selected(3) && !picked.is_selected(4));
    }
}

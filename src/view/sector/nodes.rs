//! The sector's nodes (SECT-03, SECT-05): one per catalogue entry, at the
//! entry's position. A gated system's node is a star marker, an ungated
//! system's a plain circle outline, a brown dwarf's the same circle
//! fainter. Marker sizes are fixed in pixels, never from a star's size or
//! distance (MRK-03). Each frame every node is projected with this
//! frame's camera, snapped to a whole pixel (SYM-16) and layered by its
//! depth, nearer over farther.
//!
//! The sector fit's slab is worked out here too, from the same render
//! positions (SECT-11).

use std::collections::HashMap;

use bevy::prelude::*;

use super::{ForEntry, NodeKind, SectorNode, input};
use crate::content::generator::Sector;
use crate::content::generator::catalogue::Entry;
use crate::content::start::LoadedSector;
use crate::sim::body::{Affiliation, BodyClass};
use crate::view::camera::stops::SectorFit;
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::layers;
use crate::view::map_layer::MapLayer;
use crate::view::markers::draw::{BodyLook, spawn_icon};
use crate::view::scale::sector_km_to_render_vec3;
use crate::view::screen::{MapView, in_viewport, snap_to_pixel};
use crate::view::tunables::Tunables;
use crate::view::world::ViewNow;

/// Where a node is drawn this frame: its node entity, its centre on
/// screen, half its drawn size, and its depth from the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeScreen {
    pub node: Entity,
    pub kind: NodeKind,
    pub centre: Vec2,
    pub half: f32,
    pub depth: f32,
}

/// Every node drawn this frame, by entry id. A node behind the camera, or
/// any node while the system view is shown, has none.
#[derive(Resource, Default, Debug)]
pub struct NodeScreens(pub HashMap<usize, NodeScreen>);

/// SECT-03: what kind of node an entry gets: a gated system's, an
/// ungated star system's (giants included), or a brown dwarf's.
pub fn node_kind(sector: &Sector, entry: &Entry) -> NodeKind {
    if sector.gated_system(entry.id).is_some() {
        NodeKind::Gated
    } else if entry.system().is_some() {
        NodeKind::Ungated
    } else {
        NodeKind::BrownDwarf
    }
}

/// SECT-06 / SECT-21: what a node is called: a gated system's name, or
/// any other entry's designation.
pub fn node_name(sector: &Sector, id: usize) -> String {
    match sector.gated_system(id) {
        Some(gated) => gated.names.system.clone(),
        None => sector.entries[id].designation.clone(),
    }
}

/// SECT-03: a node's size across, in pixels, by kind.
pub fn node_px(kind: NodeKind, tunables: &Tunables) -> f32 {
    match kind {
        NodeKind::Gated => tunables.sector_node_px,
        NodeKind::Ungated | NodeKind::BrownDwarf => tunables.sector_dot_px,
    }
}

/// SECT-03: how strongly a node is drawn, by kind.
pub fn node_alpha(kind: NodeKind, tunables: &Tunables) -> f32 {
    match kind {
        NodeKind::Gated => 1.0,
        NodeKind::Ungated => tunables.sector_dot_alpha,
        NodeKind::BrownDwarf => tunables.sector_faint_alpha,
    }
}

/// SECT-10 / SECT-11: the sector fit takes in every node, and the pan
/// bound reaches past the farthest.
pub fn fit_the_sector(sector: Option<Res<LoadedSector>>, mut fit: ResMut<SectorFit>) {
    let Some(loaded) = sector else {
        return;
    };
    let points: Vec<Vec3> = loaded
        .sector
        .entries
        .iter()
        .map(|entry| sector_km_to_render_vec3(entry.position_km))
        .collect();
    *fit = SectorFit::of(points);
}

/// SECT-03: a node and its hit target for every catalogue entry, hidden
/// until placed.
pub fn spawn_nodes(
    mut commands: Commands,
    sector: Option<Res<LoadedSector>>,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
) {
    let Some(loaded) = sector else {
        return;
    };
    let sector = &loaded.sector;
    for entry in &sector.entries {
        let kind = node_kind(sector, entry);
        let node = SectorNode {
            id: entry.id,
            kind,
            world: sector_km_to_render_vec3(entry.position_km),
        };
        spawn_node(&mut commands, (&fonts, &tunables), node);
        input::spawn_target(&mut commands, ForEntry(entry.id));
    }
}

/// SECT-03: one node: a star marker (MRK-13) for a gated system, else a
/// plain circle outline in the neutral colour at its kind's strength. It
/// takes no clicks itself; its hit target does (`input.rs`).
fn spawn_node(commands: &mut Commands, assets: (&Fonts, &Tunables), node: SectorNode) {
    let (_, tunables) = assets;
    let size = node_px(node.kind, tunables);
    let gated = node.kind == NodeKind::Gated;
    let mut entity = commands.spawn((
        node,
        Node {
            position_type: PositionType::Absolute,
            width: px(size),
            height: px(size),
            border: UiRect::all(px(if gated { 0.0 } else { tunables.body_rim_px })),
            border_radius: BorderRadius::MAX,
            display: Display::None,
            ..default()
        },
        GlobalZIndex(layers::BODIES),
        Pickable::IGNORE,
    ));
    if gated {
        let star = BodyLook {
            class: BodyClass::Star,
            affiliation: Affiliation::Neutral,
            record: None,
            group: None,
        };
        entity.with_children(|parent| spawn_icon(parent, assets, star, size / 2.0));
    } else {
        let rim = colors::NEUTRAL.with_alpha(node_alpha(node.kind, tunables));
        entity.insert((BorderColor::all(rim), BackgroundColor(Color::NONE)));
    }
}

/// SECT-03 / SECT-05: place every node in the viewport on its projected
/// position this frame, nearer nodes layered over farther ones, and note
/// where each is drawn. While the system view is shown every node is
/// hidden.
pub fn place_nodes(
    layer: Res<MapLayer>,
    now: ViewNow,
    map: MapView,
    tunables: Res<Tunables>,
    mut nodes: Query<(Entity, &SectorNode, &mut Node, &mut GlobalZIndex)>,
    mut screens: ResMut<NodeScreens>,
) {
    screens.0.clear();
    let camera = now.camera().map(|(_, transform)| transform.translation());
    let scale = map.scale_factor();
    let viewport = map.viewport().filter(|_| layer.is_sector());
    if let (Some(eye), Some(viewport)) = (camera, viewport) {
        for (entity, node, ..) in &nodes {
            // A node beyond the map's edge, under the sidebar, isn't
            // drawn, as a marker there isn't.
            let Some(at) = now
                .project(node.world)
                .filter(|&at| in_viewport(at, viewport))
            else {
                continue;
            };
            let screen = NodeScreen {
                node: entity,
                kind: node.kind,
                centre: snap_to_pixel(at, scale),
                half: node_px(node.kind, &tunables) / 2.0,
                depth: eye.distance(node.world),
            };
            screens.0.insert(node.id, screen);
        }
    }
    let depths: Vec<(usize, f32)> = screens.0.iter().map(|(&id, s)| (id, s.depth)).collect();
    let order: HashMap<usize, i32> = layers::depth_layers(&depths).into_iter().collect();
    for (_, node, mut ui, mut z) in &mut nodes {
        let Some(screen) = screens.0.get(&node.id) else {
            if ui.display != Display::None {
                ui.display = Display::None;
            }
            continue;
        };
        let corner = screen.centre - Vec2::splat(screen.half);
        ui.display = Display::Flex;
        ui.left = px(corner.x);
        ui.top = px(corner.y);
        let wanted = GlobalZIndex(order[&node.id]);
        if *z != wanted {
            *z = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::testing::sector;

    /// SECT-03: every entry is a node: gated systems are gated nodes,
    /// other star systems (giants among them) ungated, brown dwarfs their
    /// own kind; a gated node is named for its system, any other by its
    /// designation.
    #[test]
    fn every_entry_is_a_node_of_its_kind() {
        let sector = sector(30, 7);
        let kinds: Vec<NodeKind> = sector
            .entries
            .iter()
            .map(|e| node_kind(&sector, e))
            .collect();
        let count = |kind| kinds.iter().filter(|&&k| k == kind).count();
        assert_eq!(count(NodeKind::Gated), sector.gated.len());
        let dwarfs = sector.entries.iter().filter(|e| e.system().is_none());
        assert_eq!(count(NodeKind::BrownDwarf), dwarfs.count());
        assert!(count(NodeKind::Ungated) > 0);
        let gated = &sector.gated[0];
        assert_eq!(node_name(&sector, gated.id), gated.names.system);
        let other = sector
            .entries
            .iter()
            .find(|e| node_kind(&sector, e) != NodeKind::Gated)
            .unwrap();
        assert_eq!(node_name(&sector, other.id), other.designation);
    }

    /// SECT-03: a gated node is `sector_node_px` across at full strength;
    /// an ungated one `sector_dot_px` at `sector_dot_alpha`, a brown
    /// dwarf's the same size at `sector_faint_alpha`.
    #[test]
    fn node_sizes_and_strengths_follow_their_kind() {
        let t = Tunables::default();
        assert_eq!(node_px(NodeKind::Gated, &t), t.sector_node_px);
        assert_eq!(node_px(NodeKind::Ungated, &t), t.sector_dot_px);
        assert_eq!(node_px(NodeKind::BrownDwarf, &t), t.sector_dot_px);
        assert_eq!(node_alpha(NodeKind::Gated, &t), 1.0);
        assert_eq!(node_alpha(NodeKind::Ungated, &t), t.sector_dot_alpha);
        assert_eq!(node_alpha(NodeKind::BrownDwarf, &t), t.sector_faint_alpha);
    }
}

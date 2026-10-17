//! Hover and click in the sector view (SECT-07, SECT-20). Each node has an
//! invisible hit target, reached by a small picking backend that hits the
//! node whose centre is nearest the pointer within its hit radius, as
//! markers are (MRK-08); a node's label is a target too. Hovering a node
//! marks it with the hover brackets and selecting it with the selection
//! brackets (HOV-06), round its box (HOV-07), drawn by the system view's
//! bracket pool. A click on a node selects it; a click on empty space and
//! Esc clear the selection. Links are not targets (SECT-04). A
//! double-click on a gated system's node, or Enter with one selected, asks
//! to enter it (SECT-30); the second click of a double-click counts on
//! the node the first one hit, through the map's shared click record, as
//! the system view's does (`double_click.rs`).
//!
//! Everything here is idle while the system view is shown.

use bevy::ecs::system::SystemParam;
use bevy::picking::PickingSettings;
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;

use super::nodes::NodeScreens;
use super::{EnterRequest, ForEntry};
use crate::content::start::LoadedSector;
use crate::view::bindings::{Bindings, pressed};
use crate::view::double_click::{MapClicks, double_click_interval};
use crate::view::hover::{ClickCheck, on_marker_out, on_marker_over};
use crate::view::map_layer::{MapLayer, SectorSelection};
use crate::view::markers::brackets::{BracketBox, LayerMarks, Mark, mark_strength};
use crate::view::markers::picking::{HitSpot, hit_radius, marker_under};
use crate::view::targets::{Hovered, PointerTargets};
use crate::view::tunables::Tunables;

/// Between the rings' order and the UI's, as the markers' hits are, so a
/// label or a panel wins over a node (HOV-02).
const NODE_ORDER_OFFSET: f32 = 0.25;

/// A node's invisible hit target.
#[derive(Component)]
pub struct NodeTarget;

/// Every drawn node's hit area this frame, with the entry it stands for.
#[derive(Resource, Default, Debug)]
pub struct NodeSpots(pub Vec<(HitSpot, usize)>);

/// SECT-20: a hit target for the node of `entry`, which a hover marks and
/// a click selects.
pub fn spawn_target(commands: &mut Commands, entry: ForEntry) {
    commands
        .spawn((NodeTarget, entry))
        .observe(on_marker_over)
        .observe(on_marker_out)
        .observe(on_node_click);
}

/// MRK-08 / SECT-20: each drawn node's hit area: its drawn half-size, but
/// never less than `min_hit_radius_px`, round its centre.
pub fn find_node_spots(
    screens: Res<NodeScreens>,
    tunables: Res<Tunables>,
    targets: Query<(Entity, &ForEntry), With<NodeTarget>>,
    mut spots: ResMut<NodeSpots>,
) {
    spots.0.clear();
    for (target, entry) in &targets {
        let Some(screen) = screens.0.get(&entry.0) else {
            continue;
        };
        let spot = HitSpot {
            target,
            centre: screen.centre,
            radius: hit_radius(screen.half, tunables.min_hit_radius_px),
            frames: None,
        };
        spots.0.push((spot, entry.0));
    }
}

/// The backend: report a hit on the nearest node for each pointer,
/// against the overlay's camera, as markers' hits are.
pub fn pick_nodes(
    pointers: Query<(&PointerId, &PointerLocation)>,
    camera: Query<(Entity, &Camera), With<IsDefaultUiCamera>>,
    spots: Res<NodeSpots>,
    mut output: MessageWriter<PointerHits>,
) {
    let Ok((camera_entity, camera)) = camera.single() else {
        return;
    };
    let hit_spots: Vec<HitSpot> = spots.0.iter().map(|(spot, _)| *spot).collect();
    for (pointer, location) in &pointers {
        let Some(location) = &location.location else {
            continue;
        };
        let Some((target, distance)) = marker_under(location.position, &hit_spots) else {
            continue;
        };
        let hit = HitData::new(camera_entity, distance, None, None);
        output.write(PointerHits::new(
            *pointer,
            vec![(target, hit)],
            camera.order as f32 + NODE_ORDER_OFFSET,
        ));
    }
}

/// What a click on the sector map reads and sets: the layer and the
/// sector shown, the selection, the map's record of recent clicks (shared
/// with the system view, `double_click.rs`) and the request to enter a
/// system.
#[derive(SystemParam)]
pub struct NodeClick<'w, 's> {
    layer: Res<'w, MapLayer>,
    loaded: Option<Res<'w, LoadedSector>>,
    entries: Query<'w, 's, &'static ForEntry>,
    selection: ResMut<'w, SectorSelection>,
    enter: ResMut<'w, EnterRequest>,
    clicks: ResMut<'w, MapClicks>,
    time: Res<'w, Time<Real>>,
    picking: Option<Res<'w, PickingSettings>>,
    tunables: Res<'w, Tunables>,
}

impl NodeClick<'_, '_> {
    /// SECT-30: if the click now ending is the second of a double-click
    /// whose first click hit a node, that node, wherever the map has slid
    /// since (the inset opening narrows it, INS-07).
    fn doubled_node(&mut self) -> Option<usize> {
        let interval = double_click_interval(self.picking.as_deref());
        let double = self
            .clicks
            .take_double(interval, self.tunables.drag_threshold_px)?;
        self.entries.get(double.body).ok().map(|entry| entry.0)
    }

    /// SECT-30: a double-click on a node selects it, and on a gated
    /// system's node also asks to enter it.
    fn double_click(&mut self, id: usize) {
        self.selection.0 = Some(id);
        if is_gated(self.loaded.as_deref(), id) {
            self.enter.0 = Some(id);
        }
    }

    /// SECT-20 / SECT-30: a click on the node of `id` through its target
    /// `target`, as Bevy counted it. A single click selects the node and
    /// may start a double-click; a second click soon after, on this node
    /// or where the first one's node was, is a double-click on the first.
    /// The click after a double-click is a single click.
    fn on_node(&mut self, target: Entity, id: usize, count: u8) {
        let count = if self.clicks.begin() { 1 } else { count };
        if count >= 2 {
            self.clicks.doubled();
            self.double_click(id);
            return;
        }
        if let Some(first) = self.doubled_node() {
            self.double_click(first);
            return;
        }
        self.selection.0 = Some(id);
        self.clicks.remember(target, self.time.elapsed());
    }

    /// SECT-20 / SECT-30: a click on empty space clears the selection,
    /// unless it completes a double-click on the node it slid away from.
    fn on_empty(&mut self) {
        let after_double = self.clicks.begin();
        if !after_double && let Some(first) = self.doubled_node() {
            self.double_click(first);
            return;
        }
        self.clicks.forget();
        self.selection.0 = None;
    }
}

/// SECT-30: whether the entry `id` is a gated system, the only kind that
/// can be entered.
pub fn is_gated(loaded: Option<&LoadedSector>, id: usize) -> bool {
    loaded.is_some_and(|loaded| loaded.sector.gated_system(id).is_some())
}

/// SECT-20 / SECT-30: a click on a node, or its label, selects it and
/// opens its inset, in place of any other; a double-click on a gated
/// system's node enters it.
pub fn on_node_click(click: On<PointerClick>, check: ClickCheck, mut node_click: NodeClick) {
    if !node_click.layer.is_sector() || !check.selects(&click) {
        return;
    }
    if let Ok(entry) = node_click.entries.get(click.entity) {
        let id = entry.0;
        node_click.on_node(click.entity, id, click.count);
    }
}

/// SECT-20: a click on empty space clears the selection: a primary press
/// and release within the drag threshold that hits no node, label or
/// panel.
pub fn clear_on_empty_click(
    (buttons, bindings): (Res<ButtonInput<MouseButton>>, Res<Bindings>),
    check: ClickCheck,
    targets: PointerTargets,
    mut node_click: NodeClick,
) {
    let released = buttons.just_released(bindings.select_button);
    if node_click.layer.is_sector()
        && released
        && check.released_as_click()
        && targets.over_nothing()
    {
        node_click.on_empty();
    }
}

/// SECT-30: the Enter key enters the selected node's system, if it is a
/// gated one.
pub fn enter_on_key(
    (layer, keys, bindings): (Res<MapLayer>, Res<ButtonInput<KeyCode>>, Res<Bindings>),
    loaded: Option<Res<LoadedSector>>,
    selection: Res<SectorSelection>,
    mut enter: ResMut<EnterRequest>,
) {
    if !layer.is_sector() || !pressed(&keys, &bindings.enter_system) {
        return;
    }
    if let Some(id) = selection.0.filter(|&id| is_gated(loaded.as_deref(), id)) {
        enter.0 = Some(id);
    }
}

/// SECT-20: Esc clears the selection and closes the inset.
pub fn handle_escape(
    layer: Res<MapLayer>,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    mut selection: ResMut<SectorSelection>,
) {
    if layer.is_sector() && pressed(&keys, &bindings.close_inset) {
        selection.0 = None;
    }
}

/// SECT-07 / HOV-06 / HOV-07: the marks round the hovered and selected
/// nodes: brackets round each node's box, its drawn square plus
/// `bracket_margin_px` on every side, fully for the selection and at
/// `bracket_hover_alpha` for a hovered node.
pub fn node_marks(
    screens: &NodeScreens,
    selected: Option<usize>,
    hovered: &[usize],
    tunables: &Tunables,
) -> Vec<Mark> {
    let mut ids: Vec<usize> = hovered.iter().copied().chain(selected).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| {
            let screen = screens.0.get(&id)?;
            let alpha = mark_strength(selected == Some(id), hovered.contains(&id), tunables)?;
            let bbox =
                BracketBox::around_square(screen.centre, screen.half, tunables.bracket_margin_px);
            Some(Mark { bbox, alpha })
        })
        .collect()
}

/// SECT-07: ask the bracket pool for this frame's marks round the hovered
/// and selected nodes; none while the system view is shown.
pub fn mark_nodes(
    layer: Res<MapLayer>,
    screens: Res<NodeScreens>,
    selection: Res<SectorSelection>,
    tunables: Res<Tunables>,
    hovered: Query<&ForEntry, With<Hovered>>,
    mut marks: ResMut<LayerMarks>,
) {
    marks.0.clear();
    if !layer.is_sector() {
        return;
    }
    let hovered: Vec<usize> = hovered.iter().map(|entry| entry.0).collect();
    marks.0 = node_marks(&screens, selection.0, &hovered, &tunables);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::sector::NodeKind;
    use crate::view::sector::nodes::NodeScreen;

    fn screens(world: &mut World) -> NodeScreens {
        let mut screens = NodeScreens::default();
        for (id, x) in [(0, 100.0), (1, 300.0)] {
            screens.0.insert(
                id,
                NodeScreen {
                    node: world.spawn_empty().id(),
                    kind: NodeKind::Gated,
                    centre: Vec2::new(x, 200.0),
                    half: 7.0,
                    depth: 1.0,
                },
            );
        }
        screens
    }

    /// SECT-07 / HOV-06 / HOV-07: the selected node takes the full mark
    /// and a hovered one the hover mark, each round the node's square
    /// plus the margin; a node both hovered and selected takes the
    /// selection's; a node off screen has none.
    #[test]
    fn hovered_and_selected_nodes_take_their_marks() {
        let mut world = World::new();
        let screens = screens(&mut world);
        let t = Tunables::default();
        let marks = node_marks(&screens, Some(0), &[1, 0, 9], &t);
        assert_eq!(marks.len(), 2);
        let selected = marks.iter().find(|m| m.bbox.centre.x == 100.0).unwrap();
        let hovered = marks.iter().find(|m| m.bbox.centre.x == 300.0).unwrap();
        assert_eq!(selected.alpha, 1.0);
        assert_eq!(hovered.alpha, t.bracket_hover_alpha);
        assert_eq!(selected.bbox.half, Vec2::splat(7.0 + t.bracket_margin_px));
        assert!(node_marks(&screens, None, &[], &t).is_empty());
    }
}

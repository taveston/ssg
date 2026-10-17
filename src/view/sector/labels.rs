//! The nodes' labels (SECT-06): a gated node is labelled with its
//! system's name, in the label face in the neutral colour; an ungated
//! node, or a brown dwarf's, with its designation only while it is
//! hovered or selected. Labels are placed as the system view's are
//! (LBL-01 to LBL-04): beside the node's box, clear of every other node
//! and label, keeping their place while it stays free and fading when it
//! changes. The selection's label goes first, then the hovered node's,
//! then gated nodes', so a gated node's label is kept before another's
//! when there isn't room for both; the selection's and the hovered node's
//! never hide.

use bevy::prelude::*;

use super::nodes::{NodeScreens, node_name};
use super::{ForEntry, NodeKind};
use crate::content::start::LoadedSector;
use crate::view::anim::step_per_frame;
use crate::view::colors;
use crate::view::fonts::{Face, Fonts};
use crate::view::halo::TextHalo;
use crate::view::hover::{on_marker_out, on_marker_over};
use crate::view::layers;
use crate::view::map_layer::{MapLayer, SectorSelection};
use crate::view::markers::labels::{
    Candidate, LabelState, Placement, SLOTS, advance_fade, label_rect, place_labels,
};
use crate::view::node_box::node_size;
use crate::view::targets::Hovered;
use crate::view::text_size::TextSize;
use crate::view::tunables::Tunables;

/// A node's label, for the entry it names.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeLabel;

/// SECT-06: a label's rank in placement: the selection first, then the
/// hovered node, then gated nodes, then the rest.
pub fn sector_label_rank(selected: bool, hovered: bool, kind: NodeKind) -> u8 {
    match (selected, hovered, kind) {
        (true, ..) => 0,
        (_, true, _) => 1,
        (_, _, NodeKind::Gated) => 2,
        _ => 3,
    }
}

/// SECT-06: whether a node's label is wanted at all: always for a gated
/// node, else only while it is hovered or selected.
pub fn label_wanted(kind: NodeKind, selected: bool, hovered: bool) -> bool {
    kind == NodeKind::Gated || selected || hovered
}

/// SECT-06: a label for every node, hidden until placed. It is a hover
/// and click target for its node, as a marker's label is (HOV-02).
pub fn spawn_labels(
    mut commands: Commands,
    sector: Option<Res<LoadedSector>>,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
) {
    let Some(loaded) = sector else {
        return;
    };
    for entry in &loaded.sector.entries {
        commands
            .spawn((
                NodeLabel,
                ForEntry(entry.id),
                LabelState::default(),
                Text::new(node_name(&loaded.sector, entry.id)),
                TextLayout::linebreak(LineBreak::NoWrap),
                TextSize::MapLabel.font(&fonts, Face::Label, &tunables),
                TextColor(colors::NEUTRAL.with_alpha(0.0)),
                TextHalo::ground(),
                GlobalZIndex(layers::LABELS),
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
            ))
            .observe(on_marker_over)
            .observe(on_marker_out)
            .observe(super::input::on_node_click);
    }
}

/// What a label looks like this frame, as placement reads it.
type LabelNodes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ForEntry,
        &'static ComputedNode,
        &'static mut LabelState,
        &'static mut Node,
        &'static mut TextColor,
    ),
    With<NodeLabel>,
>;

/// SECT-06 / LBL-01 to LBL-04: place this frame's labels clear of every
/// node and of each other, fade each towards its place, and hide the
/// rest. Nothing shows while the system view is shown.
pub fn place_node_labels(
    layer: Res<MapLayer>,
    screens: Res<NodeScreens>,
    (selection, tunables, time): (Res<SectorSelection>, Res<Tunables>, Res<Time>),
    hovered: Query<&ForEntry, With<Hovered>>,
    mut labels: LabelNodes,
) {
    let hovered: Vec<usize> = hovered.iter().map(|entry| entry.0).collect();
    let gap = tunables.label_gap_px;
    let margin = tunables.bracket_margin_px;
    let obstacles: Vec<Rect> = screens
        .0
        .values()
        .map(|s| Rect::from_center_half_size(s.centre, Vec2::splat(s.half + margin)))
        .collect();
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut owners: Vec<Entity> = Vec::new();
    for (label, entry, computed, state, ..) in &labels {
        let Some(screen) = screens.0.get(&entry.0).filter(|_| layer.is_sector()) else {
            continue;
        };
        let (selected, over) = (selection.is_selected(entry.0), hovered.contains(&entry.0));
        if !label_wanted(screen.kind, selected, over) {
            continue;
        }
        let size = node_size(computed);
        candidates.push(Candidate {
            body: label,
            rank: sector_label_rank(selected, over, screen.kind),
            depth: screen.depth,
            marker: Rect::from_center_half_size(screen.centre, Vec2::splat(screen.half + margin)),
            slots: SLOTS,
            full_size: size,
            name_size: size,
            pinned: selected || over,
            previous: state.shown,
        });
        owners.push(label);
    }
    let placed = place_labels(&candidates, &obstacles, gap);
    let step = step_per_frame(time.delta_secs(), tunables.label_fade_ms);
    for (label, _, _, mut state, mut node, mut color) in &mut labels {
        let at = owners.iter().position(|&owner| owner == label);
        let wanted: Option<Placement> = at.and_then(|index| placed[index]);
        let (alpha, shown) = advance_fade(state.alpha, state.shown, wanted, step);
        *state = LabelState { shown, alpha };
        let place = at.zip(shown).map(|(index, placement)| {
            let candidate = &candidates[index];
            label_rect(candidate.marker, candidate.full_size, placement.slot, gap).min
        });
        write_label(&mut node, &mut color, place, alpha);
    }
}

/// A label's node and colour from its place and fade: hidden with no
/// place or once faded out.
fn write_label(node: &mut Node, color: &mut TextColor, place: Option<Vec2>, alpha: f32) {
    let Some(corner) = place.filter(|_| alpha > 0.0) else {
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    };
    node.display = Display::Flex;
    node.left = px(corner.x);
    node.top = px(corner.y);
    let faded = colors::NEUTRAL.with_alpha(alpha);
    if color.0 != faded {
        color.0 = faded;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SECT-06: a gated node's label is always wanted; another node's
    /// only while it is hovered or selected.
    #[test]
    fn ungated_labels_show_only_while_hovered_or_selected() {
        assert!(label_wanted(NodeKind::Gated, false, false));
        for kind in [NodeKind::Ungated, NodeKind::BrownDwarf] {
            assert!(!label_wanted(kind, false, false));
            assert!(label_wanted(kind, true, false));
            assert!(label_wanted(kind, false, true));
        }
    }

    /// SECT-06 / LBL-02: the selection's label is placed first, then the
    /// hovered node's, then gated nodes' before any other.
    #[test]
    fn gated_labels_are_placed_before_others() {
        let rank = sector_label_rank;
        assert!(rank(true, false, NodeKind::Ungated) < rank(false, true, NodeKind::Gated));
        assert!(rank(false, true, NodeKind::Ungated) < rank(false, false, NodeKind::Gated));
        assert!(rank(false, false, NodeKind::Gated) < rank(false, false, NodeKind::Ungated));
    }
}

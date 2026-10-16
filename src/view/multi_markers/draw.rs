//! Draws multi-markers (MUL-01, MUL-03, MUL-06, SYM-16, SYM-17): a front
//! frame with the count where an icon would be, and a back frame behind
//! it, offset right and up. Every part is vector geometry at its screen
//! size (`symbols/shape.rs`), so its outline is as sharp as a symbol's.
//!
//! Fold, together and overlap markers come and go as the view changes,
//! so they are drawn by a pool of nodes, each kept for one marker while
//! that marker lasts so a hover on it stays put. A station marker is the
//! station's own marker: it carries a back frame and a count from the
//! start, shown while something is docked (MUL-03). An exposure ring's
//! identity mark builds its multi-marker from the same parts (MOT-14).

use std::collections::HashMap;

use bevy::prelude::*;

use super::look::{FrameLook, MultiLook};
use crate::view::fonts::Fonts;
use crate::view::hover;
use crate::view::layers;
use crate::view::multi::{
    BackFrame, MultiKey, MultiMarker, MultiMarkers, MultiNode, MultiTarget, back_offset,
};
use crate::view::symbols::FrameShape;
use crate::view::symbols::draw::{Tint, TintSlot};
use crate::view::symbols::shape::{SymbolMask, SymbolPart};
use crate::view::targets::MarkerFor;
use crate::view::tunables::Tunables;

/// Pool nodes spawned up front; more are added when a frame wants more.
const POOL_SIZE: usize = 8;

/// One coloured part of a multi-marker's frames.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiPart {
    BackFill,
    BackFrame,
    FrontFill,
    FrontFrame,
}

/// The node holding a back frame's parts, offset from the front frame.
#[derive(Component)]
pub struct BackHolder;

/// The node holding a multi-marker's count, centred on the front frame.
#[derive(Component)]
pub struct CountHolder;

/// A multi-marker's count text (MUL-01).
#[derive(Component)]
pub struct MultiCount;

/// On a station's marker: it becomes a station marker while something is
/// docked at it (MUL-03).
#[derive(Component)]
pub struct StationMarker;

/// A node covering the whole frame square.
fn full() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: percent(100),
        height: percent(100),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// The mask and full-strength colour of one part, from the look.
fn part_look(part: MultiPart, look: &MultiLook) -> (SymbolMask, Color) {
    let frame = |f: &FrameLook| SymbolMask::Frame {
        shape: f.shape,
        dashed: false,
    };
    match part {
        MultiPart::BackFill => (SymbolMask::Fill(look.back.shape), look.back.fill()),
        MultiPart::BackFrame => (frame(&look.back), look.back.color),
        MultiPart::FrontFill => (SymbolMask::Fill(look.front.shape), look.front.fill()),
        MultiPart::FrontFrame => (frame(&look.front), look.front.color),
    }
}

/// One frame part, filling its holder.
fn spawn_part(parent: &mut ChildSpawnerCommands, part: MultiPart, look: &MultiLook, size: f32) {
    let (mask, color) = part_look(part, look);
    parent.spawn((
        part,
        full(),
        SymbolPart {
            mask,
            color,
            size_px: size,
        },
        Tint::new(color, TintSlot::Shape),
        Pickable::IGNORE,
    ));
}

/// MUL-01: the back frame, `size` across and `offset` from the front
/// frame's square, under anything spawned after it; hidden unless
/// `shown`.
pub fn spawn_back_frame(
    parent: &mut ChildSpawnerCommands,
    look: &MultiLook,
    (size, offset): (f32, Vec2),
    shown: bool,
) {
    parent
        .spawn((
            BackHolder,
            Node {
                position_type: PositionType::Absolute,
                left: px(offset.x),
                top: px(offset.y),
                width: px(size),
                height: px(size),
                display: if shown { Display::Flex } else { Display::None },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|holder| {
            spawn_part(holder, MultiPart::BackFill, look, size);
            spawn_part(holder, MultiPart::BackFrame, look, size);
        });
}

/// MUL-01: the count, in the data face at `font_px` in the front frame's
/// colour, centred where an icon would be; hidden unless `shown`.
pub fn spawn_count(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    look: &MultiLook,
    font_px: f32,
    shown: bool,
) {
    let color = look.front.color;
    parent
        .spawn((
            CountHolder,
            Node {
                display: if shown { Display::Flex } else { Display::None },
                ..full()
            },
            Pickable::IGNORE,
        ))
        .with_children(|holder| {
            holder.spawn((
                MultiCount,
                Text::new(look.count.clone()),
                // Inside a frame a few pixels wide, the figures must not
                // wrap.
                TextLayout::linebreak(LineBreak::NoWrap),
                fonts.data(font_px),
                TextColor(color),
                Tint::new(color, TintSlot::Text),
                Pickable::IGNORE,
            ));
        });
}

/// MUL-01: a whole multi-marker's parts under `parent`, the square its
/// front frame fills: back frame, front frame, count.
pub fn spawn_multi_parts(
    parent: &mut ChildSpawnerCommands,
    (fonts, tunables): (&Fonts, &Tunables),
    look: &MultiLook,
) {
    let size = tunables.ship_symbol_px;
    let offset = back_offset(tunables.back_frame_offset_px);
    spawn_back_frame(parent, look, (size, offset), true);
    spawn_part(parent, MultiPart::FrontFill, look, size);
    spawn_part(parent, MultiPart::FrontFrame, look, size);
    spawn_count(parent, fonts, look, tunables.multi_count_px, true);
}

/// A placeholder look for parts spawned before anything is known.
pub fn blank_look() -> MultiLook {
    let frame = FrameLook {
        shape: FrameShape::Circle,
        color: Color::NONE,
        fill_mix: 0.0,
    };
    MultiLook {
        front: frame,
        back: frame,
        count: String::new(),
    }
}

/// The parts of a multi-marker that change with its look.
type FrameParts<'w, 's> = Query<
    'w,
    's,
    (
        &'static MultiPart,
        &'static mut Tint,
        &'static mut SymbolPart,
    ),
>;

/// A multi-marker's count, as text, colour and tint.
type CountParts<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor, &'static mut Tint),
    (With<MultiCount>, Without<MultiPart>),
>;

/// Rewrite the parts under `root` to show `look` at `alpha`, each in the
/// one colour slot it is drawn with.
pub fn write_look(
    root: Entity,
    look: &MultiLook,
    alpha: f32,
    children: &Query<&Children>,
    (frames, counts): (&mut FrameParts, &mut CountParts),
) {
    for node in children.iter_descendants(root) {
        if let Ok((part, mut tint, mut shape)) = frames.get_mut(node) {
            let (mask, color) = part_look(*part, look);
            tint.set_if_neq(Tint::new(color, TintSlot::Shape));
            shape.set_if_neq(SymbolPart {
                mask,
                color: color.with_alpha(color.alpha() * alpha),
                ..*shape
            });
        } else if let Ok((mut text, mut text_color, mut tint)) = counts.get_mut(node) {
            if text.0 != look.count {
                text.0.clone_from(&look.count);
            }
            let color = look.front.color;
            tint.set_if_neq(Tint::new(color, TintSlot::Text));
            text_color.set_if_neq(TextColor(color.with_alpha(color.alpha() * alpha)));
        }
    }
}

/// A pool node and the invisible entity that takes its pointer events.
fn spawn_pool_node(commands: &mut Commands, assets: (&Fonts, &Tunables)) {
    let target = commands
        .spawn(MultiTarget(None))
        // MUL-11 / HOV-01: hovering marks it; a click is MUL-07's.
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(super::click::on_multi_click)
        .id();
    let size = assets.1.ship_symbol_px;
    commands
        .spawn((
            MultiNode { target },
            BackFrame(None),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                width: px(size),
                height: px(size),
                ..default()
            },
            // MRK-12: with the symbols, above every body.
            GlobalZIndex(layers::SYMBOLS),
            // Hover and click go through the hit area (MUL-11).
            Pickable::IGNORE,
        ))
        .with_children(|root| spawn_multi_parts(root, assets, &blank_look()));
}

pub fn spawn_pool(mut commands: Commands, fonts: Res<Fonts>, tunables: Res<Tunables>) {
    for _ in 0..POOL_SIZE {
        spawn_pool_node(&mut commands, (&fonts, &tunables));
    }
}

/// The pool's nodes, with the key each showed last frame.
type PoolNodes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MultiNode,
        &'static mut Node,
        &'static mut BackFrame,
    ),
    Without<MarkerFor>,
>;

/// Which wanted marker each pool node draws this frame, given the key
/// each showed last frame: a marker keeps its node, so a hover on it stays
/// put, and new markers take free nodes in order. Returns, per node, the
/// index of the marker it draws, and how many more nodes are needed.
pub fn assign_slots(
    shown: &[Option<MultiKey>],
    wanted: &[MultiKey],
) -> (Vec<Option<usize>>, usize) {
    let index: HashMap<MultiKey, usize> = wanted.iter().enumerate().map(|(i, &k)| (k, i)).collect();
    let mut slots: Vec<Option<usize>> = shown
        .iter()
        .map(|key| key.and_then(|key| index.get(&key).copied()))
        .collect();
    let mut placed = vec![false; wanted.len()];
    for &i in slots.iter().flatten() {
        placed[i] = true;
    }
    let mut free = (0..slots.len())
        .filter(|&n| slots[n].is_none())
        .collect::<Vec<_>>()
        .into_iter();
    let mut short = 0;
    for index in (0..wanted.len()).filter(|&i| !placed[i]) {
        match free.next() {
            Some(node) => slots[node] = Some(index),
            None => short += 1,
        }
    }
    (slots, short)
}

/// MUL-01 / MUL-06 / SYM-16: draw this frame's fold, together and
/// overlap markers with the pool, each on the node it had last frame
/// where it can, centred on its whole-pixel centre and faded with its
/// things; hide the rest, and grow the pool when it is short. A station
/// marker is its station's own marker.
pub fn place_multi_markers(
    mut commands: Commands,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    multis: Res<MultiMarkers>,
    (mut pool, mut targets): (PoolNodes, Query<&mut MultiTarget>),
    children: Query<&Children>,
    mut parts: (FrameParts, CountParts),
) {
    let wanted: Vec<&MultiMarker> = multis
        .markers
        .iter()
        .filter(|m| m.drawn && !matches!(m.key, MultiKey::Station(_)))
        .collect();
    let mut nodes: Vec<(Entity, Entity)> = pool.iter().map(|(e, n, ..)| (e, n.target)).collect();
    nodes.sort_by_key(|(e, _)| *e);
    let shown: Vec<Option<MultiKey>> = nodes
        .iter()
        .map(|&(_, target)| targets.get(target).ok().and_then(|t| t.0))
        .collect();
    let keys: Vec<MultiKey> = wanted.iter().map(|m| m.key).collect();
    let (slots, short) = assign_slots(&shown, &keys);
    for _ in 0..short {
        spawn_pool_node(&mut commands, (&fonts, &tunables));
    }
    let half = tunables.ship_symbol_px / 2.0;
    let back = back_offset(tunables.back_frame_offset_px);
    for ((node_entity, target), slot) in nodes.into_iter().zip(slots) {
        let marker = slot.map(|i| wanted[i]);
        if let Ok(mut target) = targets.get_mut(target) {
            target.set_if_neq(MultiTarget(marker.map(|m| m.key)));
        }
        let Ok((_, _, mut node, mut back_frame)) = pool.get_mut(node_entity) else {
            continue;
        };
        // A node is written only when it changes, so still markers cost
        // no layout.
        let Some(marker) = marker else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            back_frame.set_if_neq(BackFrame(None));
            continue;
        };
        let (left, top) = (px(marker.centre.x - half), px(marker.centre.y - half));
        if (node.display, node.left, node.top) != (Display::Flex, left, top) {
            node.display = Display::Flex;
            node.left = left;
            node.top = top;
        }
        back_frame.set_if_neq(BackFrame(Some(back)));
        let alpha = marker.alpha;
        write_look(
            node_entity,
            &marker.look,
            alpha,
            &children,
            (&mut parts.0, &mut parts.1),
        );
    }
}

/// The nodes holding a station marker's back frame and count.
type Holders<'w, 's> = Query<'w, 's, &'static mut Node, Or<(With<BackHolder>, With<CountHolder>)>>;

/// The station markers' nodes.
type StationNodes<'w, 's> =
    Query<'w, 's, (Entity, &'static MarkerFor, &'static mut BackFrame), With<StationMarker>>;

/// MUL-03: a station with things docked shows its back frame and its
/// count; without, it is the plain station symbol. The station's marker
/// fades the parts with the rest of it (`markers/mod.rs`).
pub fn show_station_markers(
    tunables: Res<Tunables>,
    multis: Res<MultiMarkers>,
    mut stations: StationNodes,
    children: Query<&Children>,
    mut holders: Holders,
    mut parts: (FrameParts, CountParts),
) {
    let back = back_offset(tunables.back_frame_offset_px);
    for (root, marker_for, mut back_frame) in &mut stations {
        let marker = multis.get(MultiKey::Station(marker_for.0));
        back_frame.set_if_neq(BackFrame(marker.map(|_| back)));
        let display = if marker.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        for node in children.iter_descendants(root) {
            if let Ok(mut holder) = holders.get_mut(node)
                && holder.display != display
            {
                holder.display = display;
            }
        }
        if let Some(marker) = marker {
            write_look(
                root,
                &marker.look,
                1.0,
                &children,
                (&mut parts.0, &mut parts.1),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MUL-01: a marker keeps the pool node it had, so a hover on it
    /// stays put; new markers take free nodes in order, and the pool says
    /// how many more nodes it needs.
    #[test]
    fn markers_keep_their_pool_nodes() {
        let mut world = World::new();
        let [a, b, c, d] = [(); 4].map(|_| MultiKey::Overlap(world.spawn_empty().id()));
        let shown = [Some(a), None, Some(b)];
        assert_eq!(
            assign_slots(&shown, &[b, c, a]),
            (vec![Some(2), Some(1), Some(0)], 0)
        );
        assert_eq!(
            assign_slots(&shown, &[d, c]),
            (vec![Some(0), Some(1), None], 0)
        );
        assert_eq!(assign_slots(&[None], &[a, b, c]).1, 2);
    }

    /// MUL-01: the back and front frames are drawn as the look's shapes,
    /// solid, each with its opaque inside.
    #[test]
    fn parts_follow_the_look() {
        use crate::view::multi_markers::look::affiliation_frame;
        use crate::view::symbols::SymbolSettings;
        let settings = SymbolSettings {
            fill_faction: 0.3,
            fill_other: 0.16,
            stale_alpha: 0.55,
            condition_warn: 0.5,
        };
        let look = MultiLook {
            front: affiliation_frame(crate::sim::body::Affiliation::Hostile, settings),
            back: affiliation_frame(crate::sim::body::Affiliation::Own, settings),
            count: "3".to_string(),
        };
        let frame = |shape| SymbolMask::Frame {
            shape,
            dashed: false,
        };
        assert_eq!(
            part_look(MultiPart::FrontFrame, &look),
            (frame(FrameShape::Diamond), look.front.color)
        );
        assert_eq!(
            part_look(MultiPart::BackFrame, &look),
            (frame(FrameShape::Circle), look.back.color)
        );
        assert_eq!(
            part_look(MultiPart::FrontFill, &look),
            (SymbolMask::Fill(FrameShape::Diamond), look.front.fill())
        );
    }
}

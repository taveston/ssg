//! A parent's **moon ring** and moon count (GRP-10, GRP-11): while
//! natural bodies are folded into a body, its marker carries a second
//! circle behind it, offset right and up, always in the neutral colour,
//! and its label a dim "+N" counting them. Ships, groups and stations
//! folded into it are its object marker's (`multi_markers/pop.rs`), and
//! neither affects the other (GRP-05).
//!
//! The ring is part of the parent's marker: a child of its node, drawn
//! under the marker's fill by a cover of the same fill above it, faded
//! with the marker and in and out as it comes and goes, sliding out from
//! behind the marker as it fades in. Its offset is the marker's
//! `BackFrame`, which the ring is drawn at, so the marker's box, hit area
//! and label take in just what is drawn (HOV-07, MRK-08, LBL-01).

use bevy::prelude::*;

use crate::view::anim::{advance_towards, step_per_frame};
use crate::view::colors;
use crate::view::fold::FoldSummary;
use crate::view::multi::{BackFrame, back_offset};
use crate::view::symbols::draw::{Tint, TintSlot};
use crate::view::targets::MarkerFor;
use crate::view::tunables::Tunables;

/// On a body's marker: its moon ring's state.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct MoonRing {
    /// How far the ring has faded in, 0 to 1 (GRP-03).
    pub t: f32,
    /// How many natural bodies are folded into the body (GRP-11).
    pub count: usize,
}

/// The ring's node under a marker: a circle the marker's size, with its
/// own fill and rim.
#[derive(Component)]
pub struct MoonRingNode;

/// On a body's marker: its ring node and the cover over it, found once
/// at spawn so nothing searches the marker's children each frame.
#[derive(Component, Debug, Clone, Copy)]
pub struct MoonRingParts {
    pub ring: Entity,
    pub cover: Entity,
}

/// The cover over the ring: the marker's fill again, so the ring shows
/// only where it reaches past the marker (GRP-10).
#[derive(Component)]
pub struct MoonRingCover;

/// A label's "+N" moon count line, for the body it counts for (GRP-11).
#[derive(Component)]
pub struct MoonCountLine(pub Entity);

/// GRP-10: the ring after one frame of `step`: towards shown while
/// `count` natural bodies are folded in, towards gone while none are.
pub fn next_ring(ring: MoonRing, count: usize, step: f32) -> MoonRing {
    let target = if count > 0 { 1.0 } else { 0.0 };
    MoonRing {
        t: advance_towards(ring.t, target, step),
        count,
    }
}

/// GRP-10 / HOV-07: where the ring sits from the marker's centre, as far
/// as it has faded in, so the box grows and shrinks with it; none while
/// it is gone.
pub fn ring_offset(ring: MoonRing, offset_px: f32) -> Option<Vec2> {
    (ring.t > 0.0).then(|| back_offset(offset_px * ring.t))
}

/// GRP-11: the moon count's text: "+N", or nothing without folded
/// natural bodies.
pub fn count_text(count: usize) -> String {
    if count > 0 {
        format!("+{count}")
    } else {
        String::new()
    }
}

/// GRP-10: the ring and its cover under a body marker, spawned before the
/// marker's class detail and rim so they draw under them: the ring behind
/// the marker, hidden until something folds in (the marker's painting
/// slides it out to its back frame as it fades in), and over it the
/// marker's fill with its ground-coloured gap (MRK-11).
pub fn spawn_moon_ring(
    parent: &mut ChildSpawnerCommands,
    fill: Color,
    gap: Outline,
    tunables: &Tunables,
) -> MoonRingParts {
    let full = || Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: percent(100),
        height: percent(100),
        border_radius: BorderRadius::MAX,
        ..default()
    };
    let ring = parent
        .spawn((
            MoonRingNode,
            Node {
                display: Display::None,
                ..full()
            },
            BackgroundColor(fill),
            Tint::new(fill, TintSlot::Background),
            Pickable::IGNORE,
        ))
        .with_children(|ring| {
            ring.spawn((
                Node {
                    border: UiRect::all(px(tunables.body_rim_px)),
                    ..full()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(colors::NEUTRAL),
                Tint::new(colors::NEUTRAL, TintSlot::Border),
                Pickable::IGNORE,
            ));
        })
        .id();
    let cover = parent
        .spawn((
            MoonRingCover,
            full(),
            BackgroundColor(fill),
            Tint::new(fill, TintSlot::Background),
            gap,
            Pickable::IGNORE,
        ))
        .id();
    MoonRingParts { ring, cover }
}

/// GRP-11: the label's second line for the moon count: the data face at
/// `moon_count_px`, the secondary text colour at `moon_count_alpha`; empty
/// until something folds in.
pub fn moon_count_line(body: Entity, font: TextFont, tunables: &Tunables) -> impl Bundle {
    let color = colors::SECONDARY.with_alpha(tunables.moon_count_alpha);
    (
        MoonCountLine(body),
        Text::default(),
        TextLayout::linebreak(LineBreak::NoWrap),
        font,
        TextColor(color),
        Tint::new(color, TintSlot::Text),
    )
}

/// GRP-10 / GRP-11: each body marker's ring follows the natural bodies
/// folded into its body, fading in and out over `fold_unfold_anim_ms`;
/// its offset is the marker's back frame, and its label's count reads
/// "+N".
pub fn update_moon_rings(
    time: Res<Time>,
    tunables: Res<Tunables>,
    folds: Res<FoldSummary>,
    mut rings: Query<(&MarkerFor, &mut MoonRing, &mut BackFrame)>,
    mut counts: Query<(&MoonCountLine, &mut Text)>,
) {
    let step = step_per_frame(time.delta_secs(), tunables.fold_unfold_anim_ms);
    let count_of = |body: Entity| folds.0.get(&body).map_or(0, |group| group.bodies.len());
    for (marker_for, mut ring, mut back) in &mut rings {
        let next = next_ring(*ring, count_of(marker_for.0), step);
        ring.set_if_neq(next);
        back.set_if_neq(BackFrame(ring_offset(next, tunables.moon_ring_offset_px)));
    }
    for (line, mut text) in &mut counts {
        let wanted = count_text(count_of(line.0));
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GRP-10 / GRP-03: the ring fades in while natural bodies are folded
    /// in and out once none are, a step at a time.
    #[test]
    fn the_ring_fades_in_and_out_with_its_folded_bodies() {
        let shown = next_ring(MoonRing::default(), 2, 0.5);
        assert_eq!(shown, MoonRing { t: 0.5, count: 2 });
        let full = next_ring(shown, 2, 0.5);
        assert_eq!(full.t, 1.0);
        let going = next_ring(full, 0, 0.25);
        assert_eq!(going, MoonRing { t: 0.75, count: 0 });
    }

    /// GRP-10 / HOV-07: the ring sits `moon_ring_offset_px` right and up
    /// once shown, part way while fading, and adds nothing once gone.
    #[test]
    fn the_rings_offset_follows_its_fade() {
        let full = MoonRing { t: 1.0, count: 1 };
        assert_eq!(ring_offset(full, 3.0), Some(Vec2::new(3.0, -3.0)));
        let half = MoonRing { t: 0.5, count: 1 };
        assert_eq!(ring_offset(half, 3.0), Some(Vec2::new(1.5, -1.5)));
        assert_eq!(ring_offset(MoonRing::default(), 3.0), None);
    }

    /// GRP-11: the count reads "+N", and nothing without folded bodies.
    #[test]
    fn the_moon_count_reads_plus_n() {
        assert_eq!(count_text(5), "+5");
        assert_eq!(count_text(0), "");
    }
}

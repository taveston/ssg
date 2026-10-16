//! The selection and hover marks (HOV-06, HOV-07): four white corner
//! brackets round whatever stands for the selected object on the map, and
//! the same brackets dimmer round each hovered one. What stands for an
//! object is its marker, symbol or true-size disc; the multi-marker that
//! holds it while it is folded or joined, the box sliding between the two
//! as it folds, unfolds, joins or parts (INS-06, MUL-05); or its exposure
//! ring's identity mark (MOT-14). A multi-marker's box takes in both its
//! frames (HOV-07). A multi-marker under the pointer is bracketed too
//! (MUL-11, GRP-07), and a shared orbit's identity mark while the orbit or
//! one of its members is hovered or selected (HOV-05).
//!
//! Each frame the wanted marks are gathered as boxes with an alpha, then
//! drawn by a small pool of bracket nodes in the labels layer, above every
//! marker (MRK-12). The frame body has no mark of its own.

use bevy::prelude::*;

use super::{MarkerDrawn, MarkersPlaced};
use crate::sim::body::BodyClass;
use crate::view::colors;
use crate::view::fold::{FoldSummary, SatelliteFoldState};
use crate::view::hovered::HoveredBodies;
use crate::view::layers;
use crate::view::motion_mode::MotionModeState;
use crate::view::multi::{
    BackFrame, JoinSlide, MultiKey, MultiMarkers, MultiTarget, back_offset, frames_rect,
};
use crate::view::rings::marks::RingMarkFor;
use crate::view::selection::Selection;
use crate::view::shared_orbit::OrbitMembers;
use crate::view::targets::{Hovered, MarkerFor, OrbitFor};
use crate::view::tunables::Tunables;
use crate::{Boot, Stage};

/// Two marks whose boxes differ by less than this, in pixels, are one.
const SAME_BOX_PX: f32 = 0.5;
/// Bracket nodes spawned up front; more are added if ever needed.
const POOL_SIZE: usize = 4;

/// HOV-07: the box a mark's brackets sit on, centred on the object, in
/// screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BracketBox {
    pub centre: Vec2,
    pub half: Vec2,
}

impl BracketBox {
    /// HOV-07: the smallest square that clears an object drawn `drawn_half`
    /// pixels from its centre by `margin` on every side.
    pub fn around_square(centre: Vec2, drawn_half: f32, margin: f32) -> Self {
        Self {
            centre,
            half: Vec2::splat(drawn_half + margin),
        }
    }

    /// HOV-07: the box that clears what is drawn in `rect` by `margin` on
    /// every side: for a multi-marker, the square round both its frames.
    pub fn around_rect(rect: Rect, margin: f32) -> Self {
        Self {
            centre: rect.center(),
            half: rect.half_size() + Vec2::splat(margin),
        }
    }

    /// INS-06: part way from `self` to `to`, so a mark moving between a
    /// multi-marker and a marker never jumps.
    pub fn lerp(self, to: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            centre: self.centre.lerp(to.centre, t),
            half: self.half.lerp(to.half, t),
        }
    }

    /// HOV-06: how long each bracket arm is: `fraction` of the box's
    /// shorter side, but never more than `max_px`, so the corners stay
    /// small round a close disc's large box.
    pub fn arm_px(&self, fraction: f32, max_px: f32) -> f32 {
        (fraction * 2.0 * self.half.min_element()).min(max_px)
    }

    fn same_as(&self, other: &Self) -> bool {
        (self.centre - other.centre).abs().max_element() < SAME_BOX_PX
            && (self.half - other.half).abs().max_element() < SAME_BOX_PX
    }
}

/// A mark to draw this frame: its box and how strongly it is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    pub bbox: BracketBox,
    pub alpha: f32,
}

/// HOV-06: how strongly an object's mark is drawn: fully when selected,
/// at `bracket_hover_alpha` when only hovered, not at all otherwise.
pub fn mark_strength(selected: bool, hovered: bool, tunables: &Tunables) -> Option<f32> {
    if selected {
        Some(1.0)
    } else if hovered {
        Some(tunables.bracket_hover_alpha)
    } else {
        None
    }
}

/// INS-06 / HOV-06: where a thing's mark goes: on the multi-marker that
/// holds it while folded or joined, on its marker once out, and part way
/// while it slides (`out` 0 to 1). With only one of the two, that one.
pub fn shown_box(
    marker: Option<BracketBox>,
    holder: Option<BracketBox>,
    out: f32,
) -> Option<BracketBox> {
    match (marker, holder) {
        (Some(marker), Some(holder)) => Some(holder.lerp(marker, out)),
        (marker, holder) => marker.or(holder),
    }
}

/// HOV-06: one mark per box: where two land on the same box (a hovered
/// multi-marker that holds the selection), the stronger stays.
pub fn merge_marks(marks: Vec<Mark>) -> Vec<Mark> {
    let mut merged: Vec<Mark> = Vec::new();
    for mark in marks {
        match merged.iter_mut().find(|m| m.bbox.same_as(&mark.bbox)) {
            Some(existing) => existing.alpha = existing.alpha.max(mark.alpha),
            None => merged.push(mark),
        }
    }
    merged
}

/// One set of four brackets, from the pool.
#[derive(Component)]
pub struct Bracket;

/// One corner of a bracket set: two arms along its top or bottom edge and
/// its left or right edge.
#[derive(Component, Clone, Copy)]
pub struct BracketCorner {
    pub top: bool,
    pub right: bool,
}

pub struct BracketsPlugin;

impl Plugin for BracketsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_pool.in_set(Boot::Overlay))
            .add_systems(
                Update,
                draw_marks.in_set(Stage::Overlay).after(MarkersPlaced),
            );
    }
}

fn spawn_pool(mut commands: Commands) {
    for _ in 0..POOL_SIZE {
        spawn_bracket(&mut commands);
    }
}

/// A hidden bracket set: a box node with a corner at each corner, drawn
/// above all markers and taking no clicks from what it surrounds.
fn spawn_bracket(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Bracket,
            GlobalZIndex(layers::LABELS),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|bracket| {
            for (top, right) in [(true, false), (true, true), (false, false), (false, true)] {
                bracket.spawn((
                    BracketCorner { top, right },
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(Color::NONE),
                    Pickable::IGNORE,
                ));
            }
        })
        .id()
}

/// HOV-06: a corner's node: `arm` pixels each way from the box's corner,
/// stroked `stroke` wide along the two outer edges, inside the box.
fn corner_node(corner: BracketCorner, arm: f32, stroke: f32) -> Node {
    let line = |on: bool| px(if on { stroke } else { 0.0 });
    let edge = |on: bool| if on { px(0) } else { Val::Auto };
    Node {
        position_type: PositionType::Absolute,
        width: px(arm),
        height: px(arm),
        top: edge(corner.top),
        bottom: edge(!corner.top),
        left: edge(!corner.right),
        right: edge(corner.right),
        border: UiRect {
            top: line(corner.top),
            bottom: line(!corner.top),
            left: line(!corner.right),
            right: line(corner.right),
        },
        ..default()
    }
}

/// Where each marker is drawn, by body, with a station marker's back
/// frame.
type Markers<'w, 's> = Query<
    'w,
    's,
    (
        &'static MarkerFor,
        &'static MarkerDrawn,
        Option<&'static BackFrame>,
    ),
>;

/// What a body's mark depends on besides its marker.
type BodyStates<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static ChildOf>,
        Option<&'static SatelliteFoldState>,
        Option<&'static MotionModeState>,
        Option<&'static JoinSlide>,
    ),
    With<BodyClass>,
>;

/// The exposure rings' identity marks.
type RingMarks<'w, 's> = Query<
    'w,
    's,
    (
        &'static RingMarkFor,
        &'static Node,
        Option<&'static MarkerFor>,
        Option<&'static OrbitFor>,
    ),
    (Without<Bracket>, Without<BracketCorner>),
>;

/// The parts of the scene a mark can go round.
#[derive(bevy::ecs::system::SystemParam)]
pub struct MarkSources<'w, 's> {
    selection: Res<'w, Selection>,
    hovered: Res<'w, HoveredBodies>,
    folds: Res<'w, FoldSummary>,
    multis: Res<'w, MultiMarkers>,
    members: Res<'w, OrbitMembers>,
    markers: Markers<'w, 's>,
    bodies: BodyStates<'w, 's>,
    hovered_multis: Query<'w, 's, &'static MultiTarget, With<Hovered>>,
    ring_marks: RingMarks<'w, 's>,
}

impl MarkSources<'_, '_> {
    fn strength(&self, body: Entity, tunables: &Tunables) -> Option<f32> {
        mark_strength(
            self.selection.is_selected(body),
            self.hovered.contains(body),
            tunables,
        )
    }

    /// HOV-07: the box round a body's marker as drawn, both frames of a
    /// station marker included.
    fn marker_box(&self, body: Entity, margin: f32) -> Option<BracketBox> {
        let (_, drawn, back) = self.markers.iter().find(|(m, ..)| m.0 == body)?;
        let (centre, half) = drawn.0?;
        Some(match back.and_then(|b| b.0) {
            Some(back) => BracketBox::around_rect(frames_rect(centre, half, Some(back)), margin),
            None => BracketBox::around_square(centre, half, margin),
        })
    }

    /// HOV-07: the box round a multi-marker's two frames, while it is
    /// drawn; a station marker's is its station's marker's.
    fn multi_box(&self, key: MultiKey, tunables: &Tunables) -> Option<BracketBox> {
        let margin = tunables.bracket_margin_px;
        if let MultiKey::Station(station) = key {
            return self.marker_box(station, margin);
        }
        let marker = self.multis.get(key).filter(|m| m.drawn)?;
        let back = back_offset(tunables.back_frame_offset_px);
        let rect = frames_rect(marker.centre, tunables.ship_symbol_px / 2.0, Some(back));
        Some(BracketBox::around_rect(rect, margin))
    }

    /// INS-06: the parent whose fold marker holds `body` while it is
    /// folded: its own, or, while its parent is folded too, the one that
    /// holds the parent.
    fn holding_fold(&self, body: Entity) -> Option<Entity> {
        let mut at = body;
        loop {
            let parent = self.bodies.get(at).ok()?.0?.parent();
            let held = self
                .folds
                .0
                .get(&parent)
                .is_some_and(|group| group.members.contains(&at));
            if held {
                return Some(parent);
            }
            at = parent;
        }
    }

    /// INS-06: the multi-marker box a body's mark slides from or to, and
    /// how far out of it the body is (0 in it, 1 on its own): the fold
    /// marker that holds it, or that it is unfolding out of; or the
    /// marker it is joined into, or parting from (MUL-05).
    fn holder_box(&self, body: Entity, tunables: &Tunables) -> (Option<BracketBox>, f32) {
        let Ok((child_of, fold, _, slide)) = self.bodies.get(body) else {
            return (None, 1.0);
        };
        let unfold = fold.map_or(1.0, |fold| fold.anim_t);
        if let Some(parent) = self.holding_fold(body) {
            return (self.multi_box(MultiKey::Fold(parent), tunables), unfold);
        }
        if unfold < 1.0 {
            // Unfolding out of a fold marker it has already left the count of.
            let parent = child_of.map(ChildOf::parent);
            let held = parent.and_then(|p| self.multi_box(MultiKey::Fold(p), tunables));
            return (held, unfold);
        }
        match slide {
            Some(slide) if slide.t > 0.0 => {
                let key = self.multis.joined.get(&body).copied();
                (key.and_then(|k| self.multi_box(k, tunables)), 1.0 - slide.t)
            }
            _ => (None, 1.0),
        }
    }

    /// HOV-06 / INS-06: the mark round a body's marker, or the
    /// multi-marker that holds it, sliding between the two as it folds,
    /// unfolds, joins or parts. A marker faded into its exposure ring
    /// fades its mark with it (MOT-11).
    fn body_mark(&self, body: Entity, strength: f32, tunables: &Tunables) -> Option<Mark> {
        let (_, _, motion, _) = self.bodies.get(body).ok()?;
        let marker = self.marker_box(body, tunables.bracket_margin_px);
        let (holder, out) = self.holder_box(body, tunables);
        let fade = match (marker, holder) {
            (Some(_), _) => motion.map_or(1.0, MotionModeState::dot_alpha),
            _ => 1.0,
        };
        let bbox = shown_box(marker, holder, out)?;
        Some(Mark {
            bbox,
            alpha: strength * fade,
        })
    }

    /// MUL-11 / HOV-01: a multi-marker under the pointer is bracketed as a
    /// marker is.
    fn hovered_multis(&self, tunables: &Tunables) -> Vec<Mark> {
        self.hovered_multis
            .iter()
            .filter_map(|target| self.multi_box(target.0?, tunables))
            .map(|bbox| Mark {
                bbox,
                alpha: tunables.bracket_hover_alpha,
            })
            .collect()
    }

    /// HOV-06 / HOV-05 / MOT-14: the marks round exposure rings' identity
    /// marks: a lone body's mark for that body, a shared orbit's
    /// multi-marker for the orbit and every member on it. They fade with
    /// the mark.
    fn ring_marks(&self, tunables: &Tunables) -> Vec<Mark> {
        let margin = tunables.bracket_margin_px;
        let mut marks = Vec::new();
        for (mark, node, marker_for, orbit_for) in &self.ring_marks {
            let Some(rect) = mark.mark.filter(|_| node.display != Display::None) else {
                continue;
            };
            let strength = match (marker_for, orbit_for) {
                (Some(body), _) => self.strength(body.0, tunables),
                (None, Some(orbit)) => self.orbit_strength(orbit.0, tunables),
                (None, None) => None,
            };
            if let Some(strength) = strength {
                marks.push(Mark {
                    bbox: BracketBox::around_rect(rect, margin),
                    alpha: strength * mark.shown,
                });
            }
        }
        marks
    }

    /// HOV-05 / HOV-06: a shared orbit's multi-marker stands for the
    /// orbit and for each member on it, so it takes the strongest of their
    /// marks.
    fn orbit_strength(&self, orbit: Entity, tunables: &Tunables) -> Option<f32> {
        let members = self.members.0.get(&orbit).into_iter().flatten();
        std::iter::once(self.strength(orbit, tunables))
            .chain(members.map(|&m| self.strength(m, tunables)))
            .flatten()
            .reduce(f32::max)
    }

    /// Every mark wanted this frame.
    pub fn marks(&self, tunables: &Tunables) -> Vec<Mark> {
        let mut bodies: Vec<Entity> = self.hovered.0.iter().copied().collect();
        bodies.extend(self.selection.selected);
        bodies.sort();
        bodies.dedup();
        let mut marks: Vec<Mark> = bodies
            .into_iter()
            .filter_map(|body| {
                let strength = self.strength(body, tunables)?;
                self.body_mark(body, strength, tunables)
            })
            .collect();
        marks.extend(self.hovered_multis(tunables));
        marks.extend(self.ring_marks(tunables));
        marks.retain(|mark| mark.alpha > 0.0);
        merge_marks(marks)
    }
}

/// HOV-06: draw this frame's marks with the bracket pool, adding to it if
/// ever more are wanted, and hide the rest.
fn draw_marks(
    mut commands: Commands,
    tunables: Res<Tunables>,
    sources: MarkSources,
    mut brackets: Query<(Entity, &mut Node, &Children), With<Bracket>>,
    mut corners: Query<(&BracketCorner, &mut Node, &mut BorderColor), Without<Bracket>>,
) {
    let marks = sources.marks(&tunables);
    let mut pool: Vec<_> = brackets.iter_mut().collect();
    pool.sort_by_key(|(entity, ..)| entity.index_u32());
    for _ in pool.len()..marks.len() {
        spawn_bracket(&mut commands);
    }
    for (index, (_, node, children)) in pool.iter_mut().enumerate() {
        let Some(mark) = marks.get(index) else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        let BracketBox { centre, half } = mark.bbox;
        node.display = Display::Flex;
        node.left = px(centre.x - half.x);
        node.top = px(centre.y - half.y);
        node.width = px(2.0 * half.x);
        node.height = px(2.0 * half.y);
        let arm = mark
            .bbox
            .arm_px(tunables.bracket_arm_fraction, tunables.bracket_arm_max_px);
        let color = colors::SELECTION.with_alpha(mark.alpha);
        for child in children.iter() {
            if let Ok((corner, mut corner_node_now, mut border)) = corners.get_mut(child) {
                *corner_node_now = corner_node(*corner, arm, tunables.bracket_stroke_px);
                border.set_all(color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HOV-07 / SYM-14: a symbol's box is its `ship_symbol_px` square plus
    /// the margin on every side, centred on it.
    #[test]
    fn a_symbols_box_clears_its_square_by_the_margin() {
        let tunables = Tunables::default();
        let centre = Vec2::new(200.0, 100.0);
        let bbox = BracketBox::around_square(
            centre,
            tunables.ship_symbol_px / 2.0,
            tunables.bracket_margin_px,
        );
        assert_eq!(bbox.centre, centre);
        let side = tunables.ship_symbol_px + 2.0 * tunables.bracket_margin_px;
        assert_eq!(bbox.half * 2.0, Vec2::splat(side));
    }

    /// HOV-07: a multi-marker's box is the smallest square that clears
    /// both its frames by the margin.
    #[test]
    fn a_multi_markers_box_clears_both_frames() {
        let tunables = Tunables::default();
        let half = tunables.ship_symbol_px / 2.0;
        let offset = tunables.back_frame_offset_px;
        let margin = tunables.bracket_margin_px;
        let centre = Vec2::new(100.0, 50.0);
        let frames = frames_rect(centre, half, Some(back_offset(offset)));
        let bbox = BracketBox::around_rect(frames, margin);
        let side = 2.0 * half + offset + 2.0 * margin;
        assert_eq!(bbox.half * 2.0, Vec2::splat(side), "a square");
        assert_eq!(bbox.centre, centre + Vec2::new(offset, -offset) / 2.0);
        // Its left and bottom edges clear the front frame by the margin,
        // its right and top edges the back frame.
        assert_eq!(bbox.centre.x - bbox.half.x, centre.x - half - margin);
        assert_eq!(
            bbox.centre.y - bbox.half.y,
            centre.y - offset - half - margin
        );
    }

    /// HOV-06: each arm is `bracket_arm_fraction` of the box's shorter
    /// side, up to `bracket_arm_max_px`: round a close disc's large box the
    /// corners stay that small.
    #[test]
    fn arms_are_a_share_of_the_shorter_side_up_to_a_cap() {
        let tunables = Tunables::default();
        let arm = |half: Vec2| {
            BracketBox {
                centre: Vec2::ZERO,
                half,
            }
            .arm_px(tunables.bracket_arm_fraction, tunables.bracket_arm_max_px)
        };
        let small = arm(Vec2::new(19.0, 12.0));
        assert!((small - tunables.bracket_arm_fraction * 24.0).abs() < 1e-5);
        assert!(small < tunables.bracket_arm_max_px);
        // A disc 300 px across.
        assert_eq!(arm(Vec2::splat(154.0)), tunables.bracket_arm_max_px);
    }

    /// HOV-06: the selection mark is full white, the hover mark dimmer, and
    /// an object that is both takes the selection mark.
    #[test]
    fn selection_outranks_hover() {
        let tunables = Tunables::default();
        assert_eq!(mark_strength(true, false, &tunables), Some(1.0));
        assert_eq!(mark_strength(true, true, &tunables), Some(1.0));
        assert_eq!(
            mark_strength(false, true, &tunables),
            Some(tunables.bracket_hover_alpha)
        );
        assert_eq!(mark_strength(false, false, &tunables), None);
    }

    /// INS-06: folded or joined, the mark is on the multi-marker; out,
    /// on the marker; part way, part way, so it never jumps.
    #[test]
    fn the_mark_slides_between_the_multi_marker_and_the_marker() {
        let marker = BracketBox::around_square(Vec2::new(100.0, 0.0), 11.0, 4.0);
        let held = BracketBox::around_rect(
            frames_rect(Vec2::ZERO, 11.0, Some(Vec2::new(4.0, -4.0))),
            4.0,
        );
        assert_eq!(shown_box(Some(marker), Some(held), 0.0), Some(held));
        assert_eq!(shown_box(Some(marker), Some(held), 1.0), Some(marker));
        let half = shown_box(Some(marker), Some(held), 0.5).unwrap();
        assert_eq!(half.centre, held.centre.lerp(marker.centre, 0.5));
        assert_eq!(shown_box(None, Some(held), 0.3), Some(held));
        assert_eq!(shown_box(Some(marker), None, 0.3), Some(marker));
        assert_eq!(shown_box(None, None, 1.0), None);
    }

    /// HOV-06: two marks on one box (a hovered multi-marker holding the
    /// selection) draw once, at the stronger alpha.
    #[test]
    fn marks_on_one_box_merge_to_the_stronger() {
        let bbox = BracketBox::around_square(Vec2::ZERO, 6.0, 4.0);
        let other = BracketBox::around_square(Vec2::new(50.0, 0.0), 6.0, 4.0);
        let merged = merge_marks(vec![
            Mark { bbox, alpha: 0.6 },
            Mark { bbox, alpha: 1.0 },
            Mark {
                bbox: other,
                alpha: 0.6,
            },
        ]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].alpha, 1.0);
    }

    /// HOV-06: a corner's arms run along the box's outer edges, inside it,
    /// `bracket_stroke_px` wide.
    #[test]
    fn a_corner_strokes_its_two_outer_edges() {
        let node = corner_node(
            BracketCorner {
                top: true,
                right: false,
            },
            9.0,
            2.0,
        );
        assert_eq!((node.top, node.left), (px(0), px(0)));
        assert_eq!((node.bottom, node.right), (Val::Auto, Val::Auto));
        assert_eq!((node.width, node.height), (px(9.0), px(9.0)));
        assert_eq!((node.border.top, node.border.left), (px(2.0), px(2.0)));
        assert_eq!((node.border.bottom, node.border.right), (px(0), px(0)));
    }
}

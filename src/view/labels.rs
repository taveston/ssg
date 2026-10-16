//! Label placement (LBL-01 to LBL-04): where each marker's label sits so
//! that labels never overprint one another or any marker, symbol or pill.
//!
//! Each label tries the positions right, left, above and below its
//! marker, `label_gap_px` from the edge of the marker's box (the box the
//! selection brackets go round, HOV-07), and takes the first that is
//! free. The gap is measured from the box whether or not the brackets
//! show, so a label never moves when its object is hovered or selected.
//! A symbol's box takes in its amplifiers too (SYM-13), which also keep
//! other labels off them. Labels are placed in priority order (the
//! selection, the hovered object, the frame body, the star and planets,
//! ships by
//! affiliation, moons), nearer the camera first within a rank, so a
//! label that matters more is placed first and the rest give way. A
//! label with no free position drops its second line and tries again,
//! then hides; the selection's and the hovered object's never hide. A
//! label keeps its position while it stays free, and any change fades
//! over `label_fade_ms`.
//!
//! An exposure ring's label is placed the same way beside the ring's
//! identity mark, whose direction chevron counts as part of the mark
//! (MOT-02, MOT-14); its period line is its second line.
//!
//! The placement is a pure function of the candidates and the obstacles
//! (`place_labels`); the system gathers them from the markers and pills
//! laid out this frame and writes the result back to the label nodes.

use bevy::prelude::*;

use super::anim::{advance_towards, step_per_frame};
use super::frame::Frame;
use super::hovered::HoveredBodies;
use super::markers::{MarkerHalfSize, MarkerLabel, MarkersPlaced, SecondLineFor};
use super::node_box::{node_rect, node_size};
use super::paths::marks::PathLabel;
use super::ring_marks::{PeriodLine, RingLabelBox, RingMarkFor};
use super::selection::Selection;
use super::shared_orbit::OrbitMembers;
use super::symbol_draw::AmplifierReach;
use super::symbols::own_flag_civilian;
use super::targets::{GroupPillFor, MarkerFor, OrbitFor};
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::Stage;
use crate::sim::body::{Affiliation, BodyClass};
use crate::sim::track::ShipRecord;

/// LBL-01: the positions a label may take, in the order tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Right,
    Left,
    Above,
    Below,
}

const SLOTS: [Slot; 4] = [Slot::Right, Slot::Left, Slot::Above, Slot::Below];

/// Where a label sits: its slot, and whether it keeps its second line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub slot: Slot,
    /// With its second line (LBL-03).
    pub full: bool,
}

/// A label waiting to be placed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub body: Entity,
    /// LBL-02: lower goes first.
    pub rank: u8,
    /// LBL-02: within a rank, nearer the camera (smaller) goes first.
    pub depth: f32,
    /// The marker's box on screen.
    pub marker: Rect,
    /// The label's size with its second line, and with the name alone.
    pub full_size: Vec2,
    pub name_size: Vec2,
    /// LBL-03: never hidden.
    pub pinned: bool,
    /// LBL-04: where it sat last frame.
    pub previous: Option<Placement>,
}

/// LBL-01: the label's top-left relative to its marker's top-left, for a
/// label of `label` pixels beside a marker of `marker` pixels: centred
/// along the marker's side, `gap` from its edge.
pub fn label_offset(marker: Vec2, label: Vec2, slot: Slot, gap: f32) -> Vec2 {
    match slot {
        Slot::Right => Vec2::new(marker.x + gap, (marker.y - label.y) / 2.0),
        Slot::Left => Vec2::new(-gap - label.x, (marker.y - label.y) / 2.0),
        Slot::Above => Vec2::new((marker.x - label.x) / 2.0, -gap - label.y),
        Slot::Below => Vec2::new((marker.x - label.x) / 2.0, marker.y + gap),
    }
}

/// LBL-01: the box a label of `size` takes in `slot` beside `marker`.
pub fn label_rect(marker: Rect, size: Vec2, slot: Slot, gap: f32) -> Rect {
    let min = marker.min + label_offset(marker.size(), size, slot, gap);
    Rect::from_corners(min, min + size)
}

/// Whether two boxes overprint: they share some area, not just an edge.
pub fn overlaps(a: Rect, b: Rect) -> bool {
    let shared = a.intersect(b);
    shared.width() > 0.0 && shared.height() > 0.0
}

/// LBL-02: a label's rank: the selection, the hovered object, the frame
/// body, the star and planets, then ships, groups and stations by
/// affiliation (hostile, unknown, own commanded, own-flag civilian,
/// allied and neutral), then moons; anything else after.
pub fn label_rank(
    (selected, hovered, frame_body): (bool, bool, bool),
    class: BodyClass,
    affiliation: Affiliation,
    record: Option<&ShipRecord>,
) -> u8 {
    if selected {
        return 0;
    }
    if hovered {
        return 1;
    }
    if frame_body {
        return 2;
    }
    match (class, record) {
        (BodyClass::Star | BodyClass::Planet, _) => 3,
        (_, Some(record)) => match affiliation {
            Affiliation::Hostile => 4,
            Affiliation::Unknown => 5,
            Affiliation::Own if own_flag_civilian(affiliation, record) => 7,
            Affiliation::Own => 6,
            Affiliation::Allied | Affiliation::Neutral => 8,
        },
        (BodyClass::Moon, _) => 9,
        _ => 10,
    }
}

/// LBL-01 to LBL-04: where each candidate's label goes, given the boxes
/// it must keep clear of (every marker, symbol and pill). Candidates are
/// taken in rank order, nearer first within a rank; each keeps its
/// previous place if still free, else takes the first free slot with its
/// second line, then without, else hides - unless pinned, when it takes
/// the right slot regardless. Returns a placement per candidate, in the
/// candidates' order.
pub fn place_labels(
    candidates: &[Candidate],
    obstacles: &[Rect],
    gap: f32,
) -> Vec<Option<Placement>> {
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (&candidates[a], &candidates[b]);
        a.rank.cmp(&b.rank).then(a.depth.total_cmp(&b.depth))
    });
    let mut taken: Vec<Rect> = obstacles.to_vec();
    let mut placements = vec![None; candidates.len()];
    for index in order {
        let candidate = &candidates[index];
        let rect_for = |placement: Placement| {
            let size = if placement.full {
                candidate.full_size
            } else {
                candidate.name_size
            };
            label_rect(candidate.marker, size, placement.slot, gap)
        };
        let free = |rect: Rect| !taken.iter().any(|&other| overlaps(other, rect));
        let has_second_line = candidate.full_size != candidate.name_size;
        let tries = candidate.previous.into_iter().chain(
            [true, false]
                .into_iter()
                .filter(|&full| full || has_second_line)
                .flat_map(|full| SLOTS.into_iter().map(move |slot| Placement { slot, full })),
        );
        let mut chosen = tries
            .map(|placement| (placement, rect_for(placement)))
            .find(|&(_, rect)| free(rect));
        if chosen.is_none() && candidate.pinned {
            let placement = Placement {
                slot: Slot::Right,
                full: true,
            };
            chosen = Some((placement, rect_for(placement)));
        }
        if let Some((placement, rect)) = chosen {
            taken.push(rect);
            placements[index] = Some(placement);
        }
    }
    placements
}

/// LBL-04: a label's fade after one frame of `step`: towards full while
/// its shown place is the wanted one, towards nothing while it isn't,
/// and once it is out, the wanted place becomes the shown one. So a
/// change of place fades out where it was and in where it goes, and a
/// hidden label fades out.
pub fn advance_fade(
    alpha: f32,
    shown: Option<Placement>,
    wanted: Option<Placement>,
    step: f32,
) -> (f32, Option<Placement>) {
    if shown == wanted {
        let target = if wanted.is_some() { 1.0 } else { 0.0 };
        return (advance_towards(alpha, target, step), shown);
    }
    let alpha = advance_towards(alpha, 0.0, step);
    if alpha <= 0.0 {
        (0.0, wanted)
    } else {
        (alpha, shown)
    }
}

/// A label's placement and fade (LBL-04).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct LabelState {
    /// Where it is drawn now.
    pub shown: Option<Placement>,
    /// How far it has faded in at that place, 0 to 1.
    pub alpha: f32,
}

pub struct LabelsPlugin;

impl Plugin for LabelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attach_label_state, place_marker_labels)
                .chain()
                .in_set(Stage::Overlay)
                .after(MarkersPlaced),
        );
    }
}

/// PLNR-65: the paths' labels are placed after every label LBL-02 ranks
/// (whose ranks run to 10), so they give way to all of them.
pub const PATH_LABEL_RANK: u8 = 11;

/// Labels, of markers, paths or exposure rings, that have no placement
/// state yet.
type UnplacedLabels = (
    Or<(With<MarkerLabel>, With<PathLabel>, With<RingLabelBox>)>,
    Without<LabelState>,
);

/// Every label gets placement state, hidden until placed.
fn attach_label_state(mut commands: Commands, labels: Query<Entity, UnplacedLabels>) {
    for label in &labels {
        commands.entity(label).insert(LabelState::default());
    }
}

/// What a label's rank is read from.
type RankedBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static BodyClass,
        &'static Affiliation,
        Option<&'static ShipRecord>,
    ),
>;

/// The markers, by node and body, with how far a symbol's amplifiers
/// reach beyond it.
type Markers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MarkerFor,
        &'static Node,
        &'static ComputedNode,
        Option<&'static AmplifierReach>,
    ),
    (With<MarkerHalfSize>, Without<MarkerLabel>),
>;

/// A marker shown this frame: its node, its body, the box it takes with
/// its amplifiers (SYM-13), and where that box's top-left sits relative
/// to the node's, which is what a label's offset is measured from.
type MarkerBox = (Entity, Entity, Rect, Vec2);

/// LBL-01 / SYM-13: the box a marker's label is placed beside and other
/// labels keep clear of: the marker's box (`margin` round what is drawn,
/// HOV-07), with a symbol's amplifiers. Returns the box and the shift
/// from the node's top-left to the box's.
pub fn marker_box(drawn: Rect, reach: Option<&AmplifierReach>, margin: f32) -> (Rect, Vec2) {
    let grown = reach.map_or(drawn.inflate(margin), |reach| {
        reach.label_box(drawn, margin)
    });
    (grown, grown.min - drawn.min)
}

/// The label nodes under the markers.
type Labels<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ChildOf,
        &'static ComputedNode,
        &'static mut Node,
        &'static mut LabelState,
        &'static mut Visibility,
    ),
    (With<MarkerLabel>, Without<MarkerHalfSize>),
>;

/// The paths' labels (PLNR-65, PLNR-67): each with the box it sits beside
/// this frame, if it is used.
type PathLabels<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PathLabel,
        &'static ComputedNode,
        &'static mut Node,
        &'static mut LabelState,
        &'static mut Visibility,
    ),
    (
        Without<MarkerLabel>,
        Without<MarkerHalfSize>,
        Without<SecondLineFor>,
        Without<GroupPillFor>,
        Without<RingMarkFor>,
        Without<RingLabelBox>,
        Without<PeriodLine>,
    ),
>;

/// The exposure rings' labels (MOT-02), each under its ring's mark.
type RingLabels<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ChildOf,
        &'static ComputedNode,
        &'static mut Node,
        &'static mut LabelState,
        &'static mut Visibility,
    ),
    (
        With<RingLabelBox>,
        Without<MarkerLabel>,
        Without<MarkerHalfSize>,
        Without<GroupPillFor>,
        Without<RingMarkFor>,
    ),
>;

/// A label's second line (a symbol's, or a ring's period line): its
/// node, for showing or hiding it, and its size.
type SecondLines<'w, 's> = Query<
    'w,
    's,
    (
        &'static ChildOf,
        &'static ComputedNode,
        &'static mut Node,
        &'static Text,
    ),
    (
        Or<(With<SecondLineFor>, With<PeriodLine>)>,
        Without<MarkerLabel>,
        Without<MarkerHalfSize>,
        Without<RingLabelBox>,
        Without<GroupPillFor>,
        Without<RingMarkFor>,
    ),
>;

/// The other things a label must keep clear of: fold pills and ring marks.
type Obstacles<'w, 's> = (
    Query<
        'w,
        's,
        (&'static Node, &'static ComputedNode),
        (
            With<GroupPillFor>,
            Without<MarkerLabel>,
            Without<SecondLineFor>,
        ),
    >,
    RingMarkNodes<'w, 's>,
);

/// The exposure rings' marks, with the body or shared orbit each stands
/// for.
type RingMarkNodes<'w, 's> = Query<
    'w,
    's,
    (
        &'static Node,
        &'static RingMarkFor,
        Option<&'static MarkerFor>,
        Option<&'static OrbitFor>,
    ),
    (Without<MarkerLabel>, Without<SecondLineFor>),
>;

/// What decides a label's rank this frame.
#[derive(bevy::ecs::system::SystemParam)]
struct Ranking<'w, 's> {
    selection: Res<'w, Selection>,
    hovered: Res<'w, HoveredBodies>,
    frame: Res<'w, Frame>,
    view: ViewNow<'w, 's>,
    bodies: RankedBodies<'w, 's>,
    members: Option<Res<'w, OrbitMembers>>,
}

impl Ranking<'_, '_> {
    /// LBL-02: a body's rank and depth, and whether its label is pinned
    /// (LBL-03).
    fn of(&self, body: Entity) -> Option<(u8, f32, bool)> {
        let (class, affiliation, record) = self.bodies.get(body).ok()?;
        let selected = self.selection.is_selected(body);
        let hovered = self.hovered.contains(body);
        let rank = label_rank(
            (selected, hovered, self.frame.body == body),
            *class,
            *affiliation,
            record,
        );
        let depth = match (self.view.position(body), self.view.camera()) {
            (Some(position), Some((_, camera))) => {
                (position - camera.translation()).dot(*camera.forward())
            }
            _ => f32::MAX,
        };
        Some((rank, depth, selected || hovered))
    }

    /// LBL-02 / HOV-05: a shared orbit's label ranks as the highest of its
    /// members, or as hovered while the orbit itself is, and is pinned
    /// while any of them is (LBL-03).
    fn of_orbit(&self, orbit: Entity) -> Option<(u8, f32, bool)> {
        let members = self.members.as_ref()?.0.get(&orbit)?;
        let (mut rank, depth, mut pinned) = members
            .iter()
            .filter_map(|&member| self.of(member))
            .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2 || b.2))?;
        if self.hovered.contains(orbit) {
            rank = rank.min(1);
            pinned = true;
        }
        Some((rank, depth, pinned))
    }
}

/// LBL-01: the boxes every marker, pill and ring mark takes this frame;
/// a ring mark's is its box (HOV-07, `margin` round it).
fn gather_obstacles(
    markers: &[MarkerBox],
    (pills, ring_marks): &Obstacles,
    margin: f32,
) -> Vec<Rect> {
    let mut obstacles: Vec<Rect> = markers.iter().map(|&(_, _, rect, _)| rect).collect();
    obstacles.extend(
        pills
            .iter()
            .filter_map(|(node, computed)| node_rect(node, computed)),
    );
    for (node, mark, ..) in ring_marks {
        if node.display == Display::None {
            continue;
        }
        obstacles.extend(mark.mark.map(|rect| rect.inflate(margin)));
        // MOT-14: a ring's direction chevron counts as part of its mark.
        obstacles.extend(mark.chevron);
    }
    obstacles
}

/// LBL-01 to LBL-04: gather this frame's markers and their labels, place
/// the labels, and write each label's place, second line, fade and
/// pickability.
fn place_marker_labels(
    (time, tunables): (Res<Time>, Res<Tunables>),
    ranking: Ranking,
    markers: Markers,
    (mut labels, mut path_labels, mut ring_labels): (Labels, PathLabels, RingLabels),
    mut second_lines: SecondLines,
    obstacles: Obstacles,
    mut colors: Query<(&ChildOf, &mut TextColor)>,
) {
    let marker_rects: Vec<MarkerBox> = markers
        .iter()
        .filter_map(|(entity, marker_for, node, computed, reach)| {
            let (rect, shift) = marker_box(
                node_rect(node, computed)?,
                reach,
                tunables.bracket_margin_px,
            );
            Some((entity, marker_for.0, rect, shift))
        })
        .collect();
    let margin = tunables.bracket_margin_px;
    let obstacle_rects = gather_obstacles(&marker_rects, &obstacles, margin);

    // Each shown marker's label is a candidate; its sizes are last
    // frame's layout, in logical pixels.
    let mut entries: Vec<(Entity, Candidate, Vec2)> = Vec::new();
    for (label, child_of, computed, _, state, _) in &labels {
        let Some(&(_, body, marker, shift)) =
            marker_rects.iter().find(|(m, ..)| *m == child_of.parent())
        else {
            continue;
        };
        let Some((rank, depth, pinned)) = ranking.of(body) else {
            continue;
        };
        let (full_size, second) = label_sizes(label, computed, &second_lines);
        entries.push((
            label,
            Candidate {
                body,
                rank,
                depth,
                marker,
                full_size,
                name_size: full_size - Vec2::new(0.0, second),
                pinned,
                previous: state.shown,
            },
            shift,
        ));
    }
    entries.extend(ring_label_entries(
        (&ring_labels, &second_lines),
        &obstacles.1,
        &ranking,
        margin,
    ));
    entries.extend(path_label_entries(&path_labels));
    let candidates: Vec<Candidate> = entries.iter().map(|(_, c, _)| *c).collect();
    let placements = place_labels(&candidates, &obstacle_rects, tunables.label_gap_px);
    let step = step_per_frame(time.delta_secs(), tunables.label_fade_ms);
    hide_unused_path_labels(&mut path_labels);

    for ((label, candidate, shift), wanted) in entries.into_iter().zip(placements) {
        let found = if let Ok((.., node, state, visibility)) = labels.get_mut(label) {
            Some((node, state, visibility))
        } else if let Ok((.., node, state, visibility)) = ring_labels.get_mut(label) {
            Some((node, state, visibility))
        } else {
            path_labels
                .get_mut(label)
                .ok()
                .map(|(.., node, state, visibility)| (node, state, visibility))
        };
        let Some((mut node, mut state, mut visibility)) = found else {
            continue;
        };
        let (alpha, shown) = advance_fade(state.alpha, state.shown, wanted, step);
        if (state.alpha, state.shown) != (alpha, shown) {
            *state = LabelState { shown, alpha };
        }
        let full = shown.is_none_or(|p| p.full);
        if let Some(placement) = shown {
            let size = if placement.full {
                candidate.full_size
            } else {
                candidate.name_size
            };
            let offset = shift
                + label_offset(
                    candidate.marker.size(),
                    size,
                    placement.slot,
                    tunables.label_gap_px,
                );
            node.left = px(offset.x);
            node.top = px(offset.y);
        }
        // A hidden label is neither drawn nor picked.
        visibility.set_if_neq(if alpha > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        // SYM-13: an empty second line (nothing applies) takes no room.
        for (line_of, _, mut line, text) in &mut second_lines {
            if line_of.parent() == label {
                let display = if full && !text.0.is_empty() {
                    Display::Flex
                } else {
                    Display::None
                };
                if line.display != display {
                    line.display = display;
                }
            }
        }
        // The marker's fade has already been written to the text; the
        // label's own fade multiplies it.
        for (text_of, mut color) in &mut colors {
            if text_of.parent() == label {
                let faded = color.0.with_alpha(color.0.alpha() * alpha);
                color.0 = faded;
            }
        }
    }
}

/// The size of a label laid out last frame, and of its second line, in
/// logical pixels; zero for a label with no second line.
fn label_sizes(label: Entity, computed: &ComputedNode, second_lines: &SecondLines) -> (Vec2, f32) {
    let second = second_lines
        .iter()
        .find(|(line_of, ..)| line_of.parent() == label)
        .map_or(0.0, |(_, c, ..)| node_size(c).y);
    (node_size(computed), second)
}

/// MOT-02 / MOT-14: each shown exposure ring's label as a candidate
/// beside its mark's box (`margin` round the mark, HOV-07), ranked as the
/// body or shared orbit the ring stands for, with its period line as its
/// second line. Its node sits under the mark's node, so its offset is
/// measured from that node's corner.
fn ring_label_entries(
    (ring_labels, second_lines): (&RingLabels, &SecondLines),
    ring_marks: &RingMarkNodes,
    ranking: &Ranking,
    margin: f32,
) -> Vec<(Entity, Candidate, Vec2)> {
    let mut entries = Vec::new();
    for (label, child_of, computed, _, state, _) in ring_labels {
        let Ok((node, mark, body, orbit)) = ring_marks.get(child_of.parent()) else {
            continue;
        };
        let (Some(drawn), Val::Px(left), Val::Px(top)) = (mark.mark, node.left, node.top) else {
            continue;
        };
        if node.display == Display::None {
            continue;
        }
        let ranked = match (body, orbit) {
            (Some(body), _) => ranking.of(body.0),
            (None, Some(orbit)) => ranking.of_orbit(orbit.0),
            (None, None) => None,
        };
        let Some((rank, depth, pinned)) = ranked else {
            continue;
        };
        let (full_size, second) = label_sizes(label, computed, second_lines);
        let bbox = drawn.inflate(margin);
        entries.push((
            label,
            Candidate {
                body: mark.owner,
                rank,
                depth,
                marker: bbox,
                full_size,
                name_size: full_size - Vec2::new(0.0, second),
                pinned,
                previous: state.shown,
            },
            bbox.min - Vec2::new(left, top),
        ));
    }
    entries
}

/// PLNR-65 / PLNR-67: each used path label as a candidate after every
/// ranked label, in the order the paths list them, beside its anchor box
/// on screen. Its node sits on the screen, not under a marker, so its
/// offset is measured from the screen's corner.
fn path_label_entries(path_labels: &PathLabels) -> Vec<(Entity, Candidate, Vec2)> {
    let mut used: Vec<(Entity, Rect, Vec2, Option<Placement>)> = path_labels
        .iter()
        .filter_map(|(label, path, computed, _, state, _)| {
            let size = node_size(computed);
            Some((label, path.anchor?, size, state.shown))
        })
        .collect();
    used.sort_by_key(|(label, ..)| label.index_u32());
    used.into_iter()
        .enumerate()
        .map(|(order, (label, anchor, size, previous))| {
            let candidate = Candidate {
                body: label,
                rank: PATH_LABEL_RANK,
                depth: order as f32,
                marker: anchor,
                full_size: size,
                name_size: size,
                pinned: false,
                previous,
            };
            (label, candidate, anchor.min)
        })
        .collect()
}

/// A path label with nothing to say this frame is hidden at once, and
/// starts afresh when it is used again.
fn hide_unused_path_labels(path_labels: &mut PathLabels) {
    for (_, path, _, _, mut state, mut visibility) in path_labels {
        if path.anchor.is_none() {
            visibility.set_if_neq(Visibility::Hidden);
            state.set_if_neq(LabelState::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(x: f32, y: f32, size: f32) -> Rect {
        Rect::from_corners(Vec2::new(x, y), Vec2::new(x + size, y + size))
    }

    fn candidate(world: &mut World, marker: Rect, rank: u8) -> Candidate {
        Candidate {
            body: world.spawn_empty().id(),
            rank,
            depth: 1.0,
            marker,
            full_size: Vec2::new(60.0, 26.0),
            name_size: Vec2::new(60.0, 14.0),
            pinned: false,
            previous: None,
        }
    }

    /// LBL-01: each slot sits `gap` from the marker's edge, centred along
    /// that side.
    #[test]
    fn slots_sit_a_gap_from_the_marker() {
        let marker = marker(100.0, 100.0, 20.0);
        let size = Vec2::new(40.0, 10.0);
        let right = label_rect(marker, size, Slot::Right, 6.0);
        assert_eq!(right.min, Vec2::new(126.0, 105.0));
        let left = label_rect(marker, size, Slot::Left, 6.0);
        assert_eq!(left.max, Vec2::new(94.0, 115.0));
        let above = label_rect(marker, size, Slot::Above, 6.0);
        assert_eq!(above.max.y, 94.0);
        assert_eq!(above.min.x, 90.0);
        let below = label_rect(marker, size, Slot::Below, 6.0);
        assert_eq!(below.min.y, 126.0);
        for rect in [right, left, above, below] {
            assert!(!overlaps(rect, marker));
        }
        assert!(overlaps(marker, marker.inflate(-1.0)));
        assert!(
            !overlaps(
                marker,
                Rect::from_corners(Vec2::new(120.0, 100.0), Vec2::new(140.0, 120.0))
            ),
            "touching edges don't overlap"
        );
    }

    /// PLNR-65: a path's label is placed after every label LBL-02 ranks,
    /// so it gives way to them, and is hidden with no free position.
    #[test]
    fn a_path_label_gives_way_to_every_ranked_label() {
        let mut world = World::new();
        // A ghost's label beside a moon's marker, listed first; the moon's
        // own label, the lowest LBL-02 rank, still goes first.
        let spot = marker(100.0, 100.0, 20.0);
        let path = candidate(&mut world, spot, PATH_LABEL_RANK);
        let moon = candidate(
            &mut world,
            spot,
            label_rank(
                (false, false, false),
                BodyClass::Moon,
                Affiliation::Neutral,
                None,
            ),
        );
        let placements = place_labels(&[path, moon], &[], 6.0);
        assert_eq!(placements[1].unwrap().slot, Slot::Right);
        assert_eq!(placements[0].unwrap().slot, Slot::Left);
        // Boxed in on every side, the path label is hidden.
        let walls: Vec<Rect> = SLOTS
            .iter()
            .map(|&slot| label_rect(spot, Vec2::new(60.0, 26.0), slot, 6.0))
            .collect();
        assert_eq!(place_labels(&[path], &walls, 6.0), vec![None]);
    }

    /// LBL-01 / LBL-02: a label whose right slot is taken by a higher
    /// label moves to the next free slot; lower ranks are placed after
    /// higher ones whatever their order in the list.
    #[test]
    fn labels_take_the_first_free_slot_in_rank_order() {
        let mut world = World::new();
        let a = candidate(&mut world, marker(100.0, 100.0, 20.0), 3);
        // Just left of a, so b's right label would run under a's.
        let b = candidate(&mut world, marker(60.0, 104.0, 12.0), 9);
        let placed = place_labels(&[b, a], &[a.marker, b.marker], 6.0);
        assert_eq!(
            placed[1],
            Some(Placement {
                slot: Slot::Right,
                full: true
            }),
            "the planet is placed first"
        );
        let b_placement = placed[0].unwrap();
        assert_ne!(
            b_placement.slot,
            Slot::Right,
            "the moon's right slot is under the planet's label"
        );
        let a_rect = label_rect(a.marker, a.full_size, Slot::Right, 6.0);
        let b_rect = label_rect(b.marker, b.full_size, b_placement.slot, 6.0);
        assert!(!overlaps(a_rect, b_rect));
        assert!(!overlaps(b_rect, a.marker));
    }

    /// LBL-02: within a rank the nearer body goes first.
    #[test]
    fn nearer_bodies_are_placed_first_within_a_rank() {
        let mut world = World::new();
        let mut far = candidate(&mut world, marker(100.0, 100.0, 20.0), 9);
        far.depth = 10.0;
        let mut near = candidate(&mut world, marker(60.0, 104.0, 12.0), 9);
        near.depth = 1.0;
        let placed = place_labels(&[far, near], &[], 6.0);
        assert_eq!(placed[1].unwrap().slot, Slot::Right);
        assert_ne!(placed[0].unwrap().slot, Slot::Right);
    }

    /// LBL-03: boxed in, a label drops its second line; boxed in still, it
    /// hides, unless it is the selection's or the hovered object's, which
    /// takes the right slot regardless.
    #[test]
    fn a_boxed_in_label_loses_its_second_line_then_hides() {
        let mut world = World::new();
        let m = marker(100.0, 100.0, 20.0);
        let c = candidate(&mut world, m, 6);
        // Walls `inner` px above and below the marker, and close in at the
        // sides, so only a name-only label fits above or below.
        let ring = |inner: f32| {
            vec![
                Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(500.0, 100.0 - inner)),
                Rect::from_corners(Vec2::new(0.0, 120.0 + inner), Vec2::new(500.0, 500.0)),
                Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(70.0, 500.0)),
                Rect::from_corners(Vec2::new(150.0, 0.0), Vec2::new(500.0, 500.0)),
            ]
        };
        let placed = place_labels(&[c], &ring(21.0), 6.0);
        assert_eq!(
            placed[0],
            Some(Placement {
                slot: Slot::Above,
                full: false
            })
        );
        let placed = place_labels(&[c], &ring(10.0), 6.0);
        assert_eq!(placed[0], None);
        let pinned = Candidate { pinned: true, ..c };
        let placed = place_labels(&[pinned], &ring(10.0), 6.0);
        assert_eq!(
            placed[0],
            Some(Placement {
                slot: Slot::Right,
                full: true
            })
        );
        // Without a second line there is nothing to drop.
        let plain = Candidate {
            name_size: c.full_size,
            ..c
        };
        assert_eq!(place_labels(&[plain], &ring(21.0), 6.0)[0], None);
    }

    /// LBL-04: a label keeps its place while that place stays free, even
    /// if an earlier slot has come free; once its place is taken it moves.
    #[test]
    fn a_label_keeps_its_place_while_it_is_free() {
        let mut world = World::new();
        let m = marker(100.0, 100.0, 20.0);
        let settled = Candidate {
            previous: Some(Placement {
                slot: Slot::Below,
                full: true,
            }),
            ..candidate(&mut world, m, 9)
        };
        assert_eq!(
            place_labels(&[settled], &[], 6.0)[0].unwrap().slot,
            Slot::Below
        );
        let below = label_rect(m, settled.full_size, Slot::Below, 6.0);
        assert_eq!(
            place_labels(&[settled], &[below], 6.0)[0].unwrap().slot,
            Slot::Right
        );
    }

    /// LBL-02: the rank order.
    #[test]
    fn ranks_run_from_the_selection_to_the_moons() {
        use crate::sim::track::ShipClass;
        let warship = ShipRecord::new(ShipClass::Warship, true);
        let civilian = ShipRecord::new(ShipClass::Cargo, false);
        let rank =
            |flags, class, affiliation, record| label_rank(flags, class, affiliation, record);
        let none = (false, false, false);
        assert_eq!(
            rank((true, true, true), BodyClass::Moon, Affiliation::Own, None),
            0
        );
        assert_eq!(
            rank((false, true, true), BodyClass::Moon, Affiliation::Own, None),
            1
        );
        assert_eq!(
            rank(
                (false, false, true),
                BodyClass::Moon,
                Affiliation::Own,
                None
            ),
            2
        );
        assert_eq!(rank(none, BodyClass::Star, Affiliation::Neutral, None), 3);
        assert_eq!(rank(none, BodyClass::Planet, Affiliation::Own, None), 3);
        assert_eq!(
            rank(none, BodyClass::Ship, Affiliation::Hostile, Some(&warship)),
            4
        );
        assert_eq!(
            rank(none, BodyClass::Ship, Affiliation::Unknown, Some(&warship)),
            5
        );
        assert_eq!(
            rank(none, BodyClass::Ship, Affiliation::Own, Some(&warship)),
            6
        );
        assert_eq!(
            rank(none, BodyClass::Ship, Affiliation::Own, Some(&civilian)),
            7
        );
        assert_eq!(
            rank(
                none,
                BodyClass::Station,
                Affiliation::Allied,
                Some(&warship)
            ),
            8
        );
        assert_eq!(
            rank(none, BodyClass::Ship, Affiliation::Neutral, Some(&civilian)),
            8
        );
        assert_eq!(rank(none, BodyClass::Moon, Affiliation::Own, None), 9);
        assert_eq!(rank(none, BodyClass::Gate, Affiliation::Neutral, None), 10);
    }

    /// LBL-04: a shown label fades in; a hidden one fades out; a change of
    /// place fades out where it was, then in where it goes.
    #[test]
    fn fades_run_through_zero_on_a_change_of_place() {
        let right = Some(Placement {
            slot: Slot::Right,
            full: true,
        });
        let left = Some(Placement {
            slot: Slot::Left,
            full: true,
        });
        assert_eq!(advance_fade(0.0, right, right, 0.25), (0.25, right));
        assert_eq!(advance_fade(1.0, right, None, 0.25), (0.75, right));
        let (alpha, shown) = advance_fade(0.5, right, left, 0.25);
        assert_eq!(
            (alpha, shown),
            (0.25, right),
            "still fading out where it was"
        );
        let (alpha, shown) = advance_fade(0.25, right, left, 0.25);
        assert_eq!((alpha, shown), (0.0, left), "now at its new place");
        assert_eq!(advance_fade(0.0, left, left, 0.25), (0.25, left));
        assert_eq!(advance_fade(0.0, None, None, 0.25), (0.0, None));
    }
}

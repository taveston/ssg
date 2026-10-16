//! Exposure-ring labels and identity marks (MOT-02, MOT-14, HOV-05): on a
//! ring that has closed into an exposure ring, a mark at a fixed spot says
//! what is on it without reading the label beside it. A lone ship or
//! station shows its symbol at `ship_symbol_px`, a lone natural body its
//! body marker, and a shared orbit a "+N" pill styled as a fold pill, with
//! the orbit's name as its label. The mark shows only once the tail has
//! closed to a full circle and the dot has gone (MOT-11), fading in and
//! out over `identity_mark_fade_ms`, so a ring held short of closing never
//! shows a moving dot and a still mark together.
//!
//! Under the name, a second line gives the orbit's period, so the ring
//! says it is a fast orbit. Beside the mark, a direction chevron on the
//! ring points the way the bodies circle (`ring_chevron.rs` places it);
//! it fades with the mark. The label is placed beside the mark like any
//! other label (`labels.rs`, LBL-01 to LBL-04): the chevron is part of the
//! mark there, so the label takes another side when the chevron would run
//! under it, and the period line is the first part to go when there is no
//! room.
//!
//! The mark is placed with its centre on a whole pixel (SYM-16) and is a
//! target for the same thing the ring is: the body, or the shared orbit
//! (HOV-05). The brackets round a hovered or selected mark are
//! `brackets.rs` (HOV-06).

use bevy::prelude::*;

use super::anim::{advance_towards, step_per_frame};
use super::colors;
use super::fonts::Fonts;
use super::hover;
use super::layers;
use super::markers::{
    BodyLook, MarkersPlaced, TintedParts, fade_parts, marker_half_size, spawn_icon,
};
use super::motion_mode::MotionModeState;
use super::pills::{PILL_HALF_HEIGHT_PX, PillFace, pill_bundle, pill_node};
use super::plot_marks::screen_angle;
use super::readout::format_duration;
use super::ring_chevron::{ChevronFit, ChevronSpot, chevron_box, chevron_spot};
use super::screen::{MapView, in_viewport, snap_to_pixel};
use super::selection::Selection;
use super::shared_orbit::{OrbitRings, Ring, ring_screen_point};
use super::ship_groups::GroupFaces;
use super::symbol_draw::{Tint, TintSlot};
use super::symbol_shape::{SymbolMask, SymbolPart};
use super::targets::{MarkerFor, OrbitFor};
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{Affiliation, BodyClass};
use crate::sim::track::ShipRecord;
use crate::{Boot, Stage};

/// Below this much fade a ring's mark and label aren't drawn.
const MIN_MARK_ALPHA: f32 = 0.05;
const LABEL_FONT_PX: f32 = 11.0;
/// The period line under the name, in the data face (MOT-02).
const PERIOD_FONT_PX: f32 = 10.0;

/// A ring's mark and label, for the body or named orbit the ring stands
/// for; `half` is how far the mark reaches from the ring point, so the
/// node can be placed with the mark centred there. `shown` is how far it
/// has faded in, 0 to 1. `mark` and `chevron` are the boxes the mark and
/// its direction chevron take on screen this frame, while they are drawn,
/// for the labels to be placed beside and keep clear of (LBL-01).
#[derive(Component)]
pub struct RingMarkFor {
    pub owner: Entity,
    pub half: f32,
    pub shown: f32,
    pub mark: Option<Rect>,
    pub chevron: Option<Rect>,
}

/// The box holding a ring's name and period, beside its mark, placed by
/// `labels.rs` (LBL-01).
#[derive(Component)]
pub struct RingLabelBox;

/// The period line under a ring's name (MOT-02).
#[derive(Component)]
pub struct PeriodLine;

/// The direction chevron beside a ring's mark (MOT-14).
#[derive(Component)]
pub struct RingChevron;

/// The label beside a ring's mark, for the ring it names, and its colour
/// when nothing on the ring is selected.
#[derive(Component)]
pub struct RingLabelFor {
    pub owner: Entity,
    pub color: Color,
}

/// The "+N" pill of a shared orbit's mark, for the orbit (HOV-05).
#[derive(Component)]
pub struct IdentityPill(pub Entity);

pub struct RingMarksPlugin;

impl Plugin for RingMarksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ring_marks.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (colour_ring_labels, place_ring_marks.in_set(MarkersPlaced))
                    .chain()
                    .in_set(Stage::Overlay),
            );
    }
}

/// MOT-14: whether a ring's mark and label should show: only once the
/// tail has closed to a full circle, so the dot has fully faded (MOT-11).
/// A ring with nothing drawn on it (every body folded) shows none.
pub fn ring_mark_wanted(state: &MotionModeState, nothing_drawn: bool) -> bool {
    !nothing_drawn && state.closing >= 1.0
}

/// MOT-14: what a ring's identity mark is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityMark {
    /// A lone body: its symbol or body marker, `half` pixels from centre.
    Body(Entity),
    /// A shared orbit: a "+N" pill.
    Pill,
}

/// MOT-14: a shared orbit's mark is a "+N" pill; a lone body's is its own
/// symbol or marker.
pub fn identity_mark(ring: &Ring) -> Option<IdentityMark> {
    if ring.shared {
        Some(IdentityMark::Pill)
    } else {
        ring.members.first().map(|m| IdentityMark::Body(m.body))
    }
}

/// What a lone body's mark looks like.
type Looks<'w, 's> = Query<
    'w,
    's,
    (
        &'static BodyClass,
        &'static Affiliation,
        Option<&'static ShipRecord>,
    ),
>;

/// MOT-02 / MOT-14: a mark and label for every ring.
fn spawn_ring_marks(
    mut commands: Commands,
    fonts: Res<Fonts>,
    tunables: Res<Tunables>,
    rings: OrbitRings,
    looks: Looks,
    groups: GroupFaces,
) {
    for owner in rings.owners() {
        let Some(ring) = rings.ring(owner) else {
            continue;
        };
        match identity_mark(&ring) {
            Some(IdentityMark::Pill) => spawn_pill_mark(&mut commands, (&fonts, &tunables), &ring),
            Some(IdentityMark::Body(body)) => {
                let Ok((class, affiliation, record)) = looks.get(body) else {
                    continue;
                };
                let look = BodyLook {
                    class: *class,
                    affiliation: *affiliation,
                    record,
                    group: groups.face(body),
                };
                spawn_body_mark(&mut commands, (&fonts, &tunables), &ring, body, look);
            }
            None => {}
        }
    }
}

/// The node a mark and its label sit in: a row, hidden until placed.
fn mark_root(owner: Entity, half: f32) -> impl Bundle {
    (
        RingMarkFor {
            owner,
            half,
            shown: 0.0,
            mark: None,
            chevron: None,
        },
        GlobalZIndex(layers::LABELS),
        mark_root_node(half),
    )
}

/// MOT-14: the row is exactly as tall as the mark, so the mark's centre
/// is `half` below the row's top, where `place_ring_marks` and the
/// brackets put the ring point. The label and chevron are placed
/// absolutely beside it, so neither makes the row taller and pushes the
/// mark down off the ring.
fn mark_root_node(half: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        display: Display::None,
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        height: px(2.0 * half),
        ..default()
    }
}

/// MOT-02: the label beside a mark: the name in its colour, and under it
/// the orbit's period in the data face in the secondary colour.
/// `labels.rs` places it beside the mark (LBL-01).
fn spawn_label(root: &mut ChildSpawnerCommands, fonts: &Fonts, ring: &Ring, color: Color) {
    root.spawn((
        RingLabelBox,
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        // Hidden until `labels.rs` has placed it.
        Visibility::Hidden,
    ))
    .with_children(|label| {
        label.spawn((
            RingLabelFor {
                owner: ring.owner,
                color,
            },
            Text::new(ring.name.clone()),
            TextLayout::linebreak(LineBreak::NoWrap),
            fonts.label(LABEL_FONT_PX),
            TextColor(color),
            Tint::new(color, TintSlot::Text),
        ));
        label.spawn((
            PeriodLine,
            Text::new(period_line(ring)),
            TextLayout::linebreak(LineBreak::NoWrap),
            fonts.data(PERIOD_FONT_PX),
            TextColor(colors::SECONDARY),
            Tint::new(colors::SECONDARY, TintSlot::Text),
        ));
    });
}

/// MOT-02: the period line's text: the orbit's period as a duration
/// (UNIT-02).
pub fn period_line(ring: &Ring) -> String {
    format_duration(ring.period_days().abs())
}

/// MOT-14: the direction chevron, placed and turned each frame; it takes
/// no clicks.
fn spawn_chevron(root: &mut ChildSpawnerCommands, tunables: &Tunables) {
    let size = tunables.chevron_px;
    root.spawn((
        RingChevron,
        Node {
            position_type: PositionType::Absolute,
            width: px(size),
            height: px(size),
            display: Display::None,
            ..default()
        },
        UiTransform::IDENTITY,
        SymbolPart {
            mask: SymbolMask::Chevron,
            color: colors::NEUTRAL,
            size_px: size,
        },
        Tint::new(colors::NEUTRAL, TintSlot::Shape),
        Pickable::IGNORE,
    ));
}

/// MOT-14 / HOV-05: a shared orbit's mark: a "+N" pill styled as a fold
/// pill, labelled with the orbit's name, targeting the orbit.
fn spawn_pill_mark(commands: &mut Commands, (fonts, tunables): (&Fonts, &Tunables), ring: &Ring) {
    let (count, flags) = ring.pill();
    let face = PillFace { count, flags };
    let node = Node {
        position_type: PositionType::Relative,
        display: Display::Flex,
        ..pill_node()
    };
    commands
        .spawn((
            mark_root(ring.owner, PILL_HALF_HEIGHT_PX),
            OrbitFor(ring.owner),
        ))
        .with_children(|root| {
            root.spawn((IdentityPill(ring.owner), pill_bundle(fonts, face, node)));
            spawn_label(root, fonts, ring, colors::NEUTRAL);
            spawn_chevron(root, tunables);
        })
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(hover::on_orbit_click);
}

/// MOT-14: a lone body's mark: its symbol at `ship_symbol_px`, or its body
/// marker (MRK-13), labelled with its name, targeting the body.
fn spawn_body_mark(
    commands: &mut Commands,
    (fonts, tunables): (&Fonts, &Tunables),
    ring: &Ring,
    body: Entity,
    look: BodyLook,
) {
    let half = marker_half_size(look.class, tunables);
    let color = look.color(tunables);
    commands
        .spawn((mark_root(ring.owner, half), MarkerFor(body)))
        .with_children(|root| {
            spawn_icon(root, (fonts, tunables), look, half);
            spawn_label(root, fonts, ring, color);
            spawn_chevron(root, tunables);
        })
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(hover::on_marker_click);
}

/// MOT-02: a ring's label is white while a body on it is selected, and
/// its own colour otherwise.
fn colour_ring_labels(
    selection: Res<Selection>,
    rings: OrbitRings,
    mut labels: Query<(&RingLabelFor, &mut Tint)>,
) {
    for (label_for, mut tint) in &mut labels {
        let Some(ring) = rings.ring(label_for.owner) else {
            continue;
        };
        let selected = ring.members.iter().any(|m| selection.is_selected(m.body));
        let color = if selected {
            colors::SELECTION
        } else {
            label_for.color
        };
        tint.set_if_neq(Tint::new(color, TintSlot::Text));
    }
}

/// A mark's chevron, placed and turned each frame.
type Chevrons<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut UiTransform),
    (With<RingChevron>, Without<RingMarkFor>),
>;

/// The parts of a mark that are placed beside it each frame.
#[derive(bevy::ecs::system::SystemParam)]
struct MarkParts<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    pills: Query<'w, 's, &'static ComputedNode, With<IdentityPill>>,
    chevrons: Chevrons<'w, 's>,
}

/// A laid-out node's size in logical pixels.
fn node_size(computed: &ComputedNode) -> Vec2 {
    computed.size() * computed.inverse_scale_factor()
}

impl MarkParts<'_, '_> {
    /// MOT-14: the box the mark takes, relative to the ring point: a
    /// symbol or marker centred on it, or a pill starting half its height
    /// before it.
    fn mark_box(&self, root: Entity, half: f32) -> Rect {
        let width = self
            .children
            .iter_descendants(root)
            .find_map(|part| self.pills.get(part).ok())
            .map_or(2.0 * half, |pill| node_size(pill).x.max(2.0 * half));
        Rect::new(-half, -half, width - half, half)
    }

    /// MOT-14: place and turn the chevron (its centre and direction on
    /// screen), relative to the mark's top-left `origin`, or hide it.
    fn place_chevron(&mut self, root: Entity, origin: Vec2, spot: Option<ChevronSpot>, size: f32) {
        let Some(part) = self
            .children
            .iter_descendants(root)
            .find(|&part| self.chevrons.contains(part))
        else {
            return;
        };
        let Ok((mut node, mut transform)) = self.chevrons.get_mut(part) else {
            return;
        };
        let Some(spot) = spot else {
            node.display = Display::None;
            return;
        };
        let at = spot.centre - origin - Vec2::splat(size / 2.0);
        node.display = Display::Flex;
        node.left = px(at.x);
        node.top = px(at.y);
        let rotation = Rot2::radians(screen_angle(Vec2::ZERO, spot.dir));
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
    }
}

/// MOT-02 / MOT-14 / SYM-16: place each ring's mark with its centre on the
/// ring at the label spot, on a whole pixel, fading it in once the ring
/// has closed and out when it opens. A pill's node is as tall as the pill,
/// so its left edge sits half a pill-height before the ring point. The
/// chevron goes on the ring ahead of the mark; the boxes of both are kept
/// for the labels, which are placed after (LBL-01).
fn place_ring_marks(
    (view, time, tunables): (ViewNow, Res<Time>, Res<Tunables>),
    map: MapView,
    rings: OrbitRings,
    mut marks: Query<(Entity, &mut RingMarkFor, &mut Node)>,
    (mut mark_parts, mut parts): (MarkParts, TintedParts),
) {
    let Some(viewport) = map.viewport() else {
        return;
    };
    let scale = map.scale_factor();
    let step = step_per_frame(time.delta_secs(), tunables.identity_mark_fade_ms);
    for (entity, mut mark, mut node) in &mut marks {
        let ring = rings.ring(mark.owner);
        let wanted = ring
            .as_ref()
            .is_some_and(|ring| ring_mark_wanted(&ring.mode(), ring.drawn().next().is_none()));
        mark.shown = advance_towards(mark.shown, if wanted { 1.0 } else { 0.0 }, step);
        let alpha = mark.shown;
        let placed = ring.as_ref().and_then(|ring| {
            if alpha < MIN_MARK_ALPHA {
                return None;
            }
            let pos = ring_screen_point(ring, ring.label_angle(), &view)?;
            let pos = snap_to_pixel(pos, scale);
            in_viewport(pos, viewport).then_some((ring, pos))
        });
        let Some((ring, pos)) = placed else {
            node.display = Display::None;
            mark.mark = None;
            mark.chevron = None;
            continue;
        };
        let half = mark.half;
        node.display = Display::Flex;
        node.left = px(pos.x - half);
        node.top = px(pos.y - half);

        let mark_box = mark_parts.mark_box(entity, half);
        let fit = ChevronFit {
            mark: mark_box,
            gap_px: tunables.chevron_gap_px,
            chevron_px: tunables.chevron_px,
        };
        let spot = chevron_spot(
            |angle| ring_screen_point(ring, angle, &view),
            ring.label_angle(),
            ring.sense(),
            fit,
        );
        mark.mark = Some(Rect {
            min: mark_box.min + pos,
            max: mark_box.max + pos,
        });
        mark.chevron = spot.map(|spot| chevron_box(spot, tunables.chevron_px));
        mark_parts.place_chevron(entity, pos - Vec2::splat(half), spot, tunables.chevron_px);
        fade_parts(entity, alpha, &mark_parts.children, &mut parts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::shared_orbit::Member;

    /// MOT-14: the mark shows only once the tail is a full circle, never
    /// beside a fading dot, and not on a ring with nothing drawn.
    #[test]
    fn the_ring_mark_waits_for_a_closed_ring() {
        let state = |closing: f32| MotionModeState {
            tail_degrees: 300.0 + 60.0 * closing,
            closing,
            ring_mode: closing >= 1.0,
        };
        assert!(!ring_mark_wanted(&state(0.0), false));
        assert!(!ring_mark_wanted(&state(0.95), false));
        assert!(ring_mark_wanted(&state(1.0), false));
        assert!(!ring_mark_wanted(&state(1.0), true));
    }

    /// MOT-14 / HOV-06: a moon's mark is shorter than its label's line, yet
    /// its row is exactly the mark's height with the mark centred in it, so
    /// the mark's centre is `half` below the row's top: on the ring point,
    /// where the brackets centre their box. A row sized by its label would
    /// push the mark below the ring and off its brackets.
    #[test]
    fn the_mark_row_is_the_marks_height_so_the_mark_sits_on_the_ring_point() {
        let tunables = Tunables::default();
        let moon_half = marker_half_size(BodyClass::Moon, &tunables);
        assert!(2.0 * moon_half < LABEL_FONT_PX, "the label is the taller");
        for half in [
            moon_half,
            PILL_HALF_HEIGHT_PX,
            tunables.ship_symbol_px / 2.0,
        ] {
            let node = mark_root_node(half);
            assert_eq!(node.height, px(2.0 * half));
            assert_eq!(node.align_items, AlignItems::Center);
        }
    }

    /// MOT-14: a shared orbit's mark is a pill; a lone body's is its own
    /// symbol or marker; an empty ring has none.
    #[test]
    fn shared_orbits_get_a_pill_and_lone_bodies_their_own_mark() {
        let mut world = World::new();
        let member = |world: &mut World| Member {
            body: world.spawn_empty().id(),
            class: BodyClass::Ship,
            affiliation: Affiliation::Own,
            angle: 0.0,
            phase0: 0.0,
            period_days: 1.0,
            hidden: false,
            mode: MotionModeState::default(),
        };
        let ring = |world: &mut World, members: Vec<Member>| Ring {
            owner: world.spawn_empty().id(),
            centre: None,
            radius_km: 1.0,
            name: "High".to_string(),
            shared: members.len() > 1,
            members,
        };
        let a = member(&mut world);
        let b = member(&mut world);
        assert_eq!(
            identity_mark(&ring(&mut world, vec![a, b])),
            Some(IdentityMark::Pill)
        );
        assert_eq!(
            identity_mark(&ring(&mut world, vec![a])),
            Some(IdentityMark::Body(a.body))
        );
        assert_eq!(identity_mark(&ring(&mut world, vec![])), None);
    }
}

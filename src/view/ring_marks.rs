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
//! The mark is placed with its centre on a whole pixel (SYM-16) and is a
//! target for the same thing the ring is: the body, or the shared orbit
//! (HOV-05), whose pill is ringed in white while the orbit is hovered.

use bevy::prelude::*;

use super::anim::{advance_towards, step_per_frame};
use super::colors;
use super::fonts::Fonts;
use super::hover;
use super::hovered::HoveredBodies;
use super::layers;
use super::markers::{BodyLook, TintedParts, fade_parts, marker_half_size, spawn_icon};
use super::motion_mode::MotionModeState;
use super::pills::{PILL_HALF_HEIGHT_PX, PillFace, pill_bundle, pill_node, selection_outline};
use super::screen::{in_viewport, snap_to_pixel};
use super::selection::Selection;
use super::shared_orbit::{OrbitRings, Ring, ring_screen_point};
use super::symbol_draw::{Tint, TintSlot};
use super::targets::{MarkerFor, OrbitFor};
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{Affiliation, BodyClass};
use crate::sim::track::ShipRecord;
use crate::{Boot, Stage};

/// Below this much fade a ring's mark and label aren't drawn.
const MIN_MARK_ALPHA: f32 = 0.05;
/// Gap between the mark and its label, which sits to its right.
const LABEL_GAP_PX: f32 = 4.0;
const LABEL_FONT_PX: f32 = 11.0;

/// A ring's mark and label, for the body or named orbit the ring stands
/// for; `half` is how far the mark reaches from the ring point, so the
/// node can be placed with the mark centred there. `shown` is how far it
/// has faded in, 0 to 1.
#[derive(Component)]
pub struct RingMarkFor {
    pub owner: Entity,
    pub half: f32,
    pub shown: f32,
}

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
                (colour_ring_labels, ring_identity_pills, place_ring_marks)
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
) {
    for owner in rings.owners() {
        let Some(ring) = rings.ring(owner) else {
            continue;
        };
        match identity_mark(&ring) {
            Some(IdentityMark::Pill) => spawn_pill_mark(&mut commands, &fonts, &ring),
            Some(IdentityMark::Body(body)) => {
                let Ok((class, affiliation, record)) = looks.get(body) else {
                    continue;
                };
                let look = BodyLook {
                    class: *class,
                    affiliation: *affiliation,
                    record,
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
        },
        GlobalZIndex(layers::LABELS),
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(LABEL_GAP_PX),
            ..default()
        },
    )
}

/// MOT-02: the label beside a mark, in its colour.
fn label(fonts: &Fonts, owner: Entity, text: &str, color: Color) -> impl Bundle {
    (
        RingLabelFor { owner, color },
        Text::new(text),
        TextLayout::linebreak(LineBreak::NoWrap),
        fonts.label(LABEL_FONT_PX),
        TextColor(color),
        Tint::new(color, TintSlot::Text),
    )
}

/// MOT-14 / HOV-05: a shared orbit's mark: a "+N" pill styled as a fold
/// pill, labelled with the orbit's name, targeting the orbit.
fn spawn_pill_mark(commands: &mut Commands, fonts: &Fonts, ring: &Ring) {
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
            root.spawn(label(fonts, ring.owner, &ring.name, colors::NEUTRAL));
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
            root.spawn(label(fonts, ring.owner, &ring.name, color));
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
            colors::SELECTION_RING
        } else {
            label_for.color
        };
        tint.set_if_neq(Tint::new(color, TintSlot::Text));
    }
}

/// HOV-05: a shared orbit's identity pill is ringed in white while the
/// orbit is hovered.
fn ring_identity_pills(
    hovered: Res<HoveredBodies>,
    mut pills: Query<(&IdentityPill, &mut Outline)>,
) {
    for (pill, mut outline) in &mut pills {
        outline.set_if_neq(selection_outline(hovered.contains(pill.0)));
    }
}

/// MOT-02 / MOT-14 / SYM-16: place each ring's mark with its centre on the
/// ring at the label spot, on a whole pixel, fading it in once the ring
/// has closed and out when it opens. A pill's node is as tall as the pill,
/// so its left edge sits half a pill-height before the ring point.
fn place_ring_marks(
    (view, time, tunables): (ViewNow, Res<Time>, Res<Tunables>),
    windows: Query<&Window>,
    rings: OrbitRings,
    mut marks: Query<(Entity, &mut RingMarkFor, &mut Node)>,
    (children, mut parts): (Query<&Children>, TintedParts),
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Vec2::new(window.width(), window.height());
    let scale = window.scale_factor();
    let step = step_per_frame(time.delta_secs(), tunables.identity_mark_fade_ms);
    for (entity, mut mark, mut node) in &mut marks {
        let wanted = rings
            .ring(mark.owner)
            .is_some_and(|ring| ring_mark_wanted(&ring.mode(), ring.drawn().next().is_none()));
        mark.shown = advance_towards(mark.shown, if wanted { 1.0 } else { 0.0 }, step);
        let alpha = mark.shown;
        let placed = rings.ring(mark.owner).and_then(|ring| {
            if alpha < MIN_MARK_ALPHA {
                return None;
            }
            let pos = ring_screen_point(&ring, ring.label_angle(), &view)?;
            let pos = snap_to_pixel(pos, scale);
            in_viewport(pos, viewport).then_some((pos, alpha))
        });
        let Some((pos, alpha)) = placed else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x - mark.half);
        node.top = px(pos.y - mark.half);
        fade_parts(entity, alpha, &children, &mut parts);
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

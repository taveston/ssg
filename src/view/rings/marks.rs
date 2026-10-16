//! Exposure-ring labels and identity marks (MOT-02, MOT-14, HOV-05): on a
//! ring that has closed into an exposure ring, a mark at a fixed spot says
//! what is on it without reading the label beside it. A lone ship or
//! station shows its symbol at `ship_symbol_px`, a lone natural body its
//! body marker, and a shared orbit a multi-marker holding its members
//! (MUL-01, MUL-02), with the orbit's name as its label. The mark shows
//! only once the tail has closed to a full circle and the dot has gone
//! (MOT-11), fading in and out over `identity_mark_fade_ms`, so a ring
//! held short of closing never shows a moving dot and a still mark
//! together. The mark follows the ring's members as they move, dock and
//! leave: a shared orbit's multi-marker is redrawn when its count, frames
//! or docked split change, and a mark that changes kind is built again.
//!
//! Under the name, a second line gives the orbit's period, so the ring
//! says it is a fast orbit. Beside the mark, a direction chevron on the
//! ring points the way the bodies circle (`rings/chevron.rs` places it);
//! it fades with the mark. The label is placed beside the mark like any
//! other label (`markers/labels.rs`, LBL-01 to LBL-04): the chevron is part of the
//! mark there, so the label takes another side when the chevron would run
//! under it, and the period line is the first part to go when there is no
//! room.
//!
//! The mark is placed with its centre on a whole pixel (SYM-16) and is a
//! target for the same thing the ring is: the body, or the shared orbit
//! (HOV-05). The brackets round a hovered or selected mark are
//! `markers/brackets.rs` (HOV-06).

use bevy::prelude::*;

use super::chevron::{ChevronFit, ChevronSpot, chevron_box, chevron_spot};
use crate::sim::body::{Affiliation, BodyClass};
use crate::sim::track::ShipRecord;
use crate::sim::tunables::SimTunables;
use crate::view::anim::{advance_towards, step_per_frame};
use crate::view::colors;
use crate::view::fonts::{Face, Fonts};
use crate::view::halo::TextHalo;
use crate::view::hover;
use crate::view::layers;
use crate::view::markers::draw::{BodyLook, spawn_icon};
use crate::view::markers::{
    MarkersPlaced, TintedParts, fade_parts, marker_half_size, symbol_settings,
};
use crate::view::motion_mode::MotionModeState;
use crate::view::multi::{BackFrame, back_offset, frames_rect};
use crate::view::multi_markers::draw::{spawn_back_frame, spawn_count, spawn_multi_parts};
use crate::view::multi_markers::link::{OrbitSlot, together_sets};
use crate::view::multi_markers::look::MultiLook;
use crate::view::readout::format_duration;
use crate::view::screen::{MapView, in_viewport, screen_angle, snap_to_pixel};
use crate::view::selection::Selection;
use crate::view::shared_orbit::{OrbitRings, Ring, ring_screen_point};
use crate::view::ship_groups::GroupFaces;
use crate::view::symbols::draw::{
    AmplifierStyle, Tint, TintSlot, amplifier_gap, spawn_symbol_parts,
};
use crate::view::symbols::shape::{SymbolMask, SymbolPart};
use crate::view::symbols::{GroupFace, SymbolLook, symbol_look};
use crate::view::targets::{MarkerFor, OrbitFor};
use crate::view::text_size::TextSize;
use crate::view::tunables::Tunables;
use crate::view::world::ViewNow;
use crate::{Boot, Stage};

/// Below this much fade a ring's mark and label aren't drawn.
const MIN_MARK_ALPHA: f32 = 0.05;

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
/// `markers/labels.rs` (LBL-01).
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

pub struct RingMarksPlugin;

impl Plugin for RingMarksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, sync_ring_marks.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    sync_ring_marks,
                    colour_ring_labels,
                    place_ring_marks.in_set(MarkersPlaced),
                )
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
    /// A shared orbit: a multi-marker holding its members.
    Multi,
}

/// MOT-14: a shared orbit's mark is a multi-marker; a lone body's is its
/// own symbol or marker.
pub fn identity_mark(ring: &Ring) -> Option<IdentityMark> {
    if ring.shared {
        Some(IdentityMark::Multi)
    } else {
        ring.members.first().map(|m| IdentityMark::Body(m.body))
    }
}

/// MUL-04 / MOT-14: a shared orbit's members split into the things its
/// multi-marker counts and, for each station among them, the members
/// docked there (together with it, PHAS-02), which are not counted.
pub fn ring_things(ring: &Ring, together_km: f64) -> (Vec<Entity>, Vec<(Entity, Vec<Entity>)>) {
    let slots: Vec<OrbitSlot> = ring
        .members
        .iter()
        .map(|m| OrbitSlot {
            thing: m.body,
            orbit: ring.owner,
            radius_km: ring.radius_km,
            phase_rad: m.phase0,
        })
        .collect();
    let is_station = |body: Entity| {
        ring.members
            .iter()
            .any(|m| m.body == body && m.class == BodyClass::Station)
    };
    let mut docked: Vec<(Entity, Vec<Entity>)> = Vec::new();
    for set in together_sets(&slots, together_km) {
        if let Some(station) = set.iter().copied().find(|&b| is_station(b)) {
            docked.push((station, set.into_iter().filter(|&b| b != station).collect()));
        }
    }
    let things = ring
        .members
        .iter()
        .map(|m| m.body)
        .filter(|b| !docked.iter().any(|(_, d)| d.contains(b)))
        .collect();
    (things, docked)
}

/// What a shared orbit's identity multi-marker shows in front: a plain
/// frame, or, when its only thing is a station with things docked, that
/// station's own symbol as a station marker (MUL-03).
#[derive(Debug, Clone, PartialEq)]
pub enum IdentityFront {
    Frame,
    Station(SymbolLook),
}

/// MOT-14: what a ring's identity mark shows, kept on the mark so it is
/// rebuilt only when that changes: a lone body's own symbol or marker
/// with everything it is drawn from, or a shared orbit's multi-marker
/// with its front and look.
#[derive(Component, Debug, Clone, PartialEq)]
pub enum MarkShows {
    Body(Entity, BodyMarkLook),
    Multi(IdentityFront, MultiLook),
}

/// What a lone body's mark is drawn from: its class, affiliation, record
/// and group face. Kept whole, so a change in any of them redraws the
/// mark.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyMarkLook {
    pub class: BodyClass,
    pub affiliation: Affiliation,
    pub record: Option<ShipRecord>,
    pub group: Option<GroupFace>,
}

impl BodyMarkLook {
    fn body_look(&self) -> BodyLook<'_> {
        BodyLook {
            class: self.class,
            affiliation: self.affiliation,
            record: self.record.as_ref(),
            group: self.group,
        }
    }
}

/// The square under a mark's root that holds its symbol, marker or
/// multi-marker.
#[derive(Component)]
struct MarkIcon;

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

/// MOT-14: what a ring's identity mark should show now, from its members
/// as they are this frame; `None` for a ring with no mark.
fn mark_shows(ring: &Ring, assets: &MarkAssets) -> Option<MarkShows> {
    match identity_mark(ring)? {
        IdentityMark::Multi => {
            let together_km = assets.sim.together_km;
            let (front, look) = identity_multi(ring, together_km, &assets.looks, &assets.tunables);
            Some(MarkShows::Multi(front, look))
        }
        IdentityMark::Body(body) => {
            let (class, affiliation, record) = assets.looks.get(body).ok()?;
            let look = BodyMarkLook {
                class: *class,
                affiliation: *affiliation,
                record: record.cloned(),
                group: assets.groups.face(body),
            };
            Some(MarkShows::Body(body, look))
        }
    }
}

/// The marks there are, by the ring they stand for.
type Marks<'w, 's> = Query<'w, 's, (Entity, &'static RingMarkFor, &'static MarkShows)>;

/// What a mark is drawn with.
#[derive(bevy::ecs::system::SystemParam)]
struct MarkAssets<'w, 's> {
    fonts: Res<'w, Fonts>,
    tunables: Res<'w, Tunables>,
    sim: Res<'w, SimTunables>,
    looks: Looks<'w, 's>,
    groups: GroupFaces<'w, 's>,
}

/// MOT-02 / MOT-14 / MUL-04: a mark and label for every ring, kept true
/// as ships come, go and dock. A shared orbit's multi-marker is redrawn
/// when its count, frames or docked split change; a lone body's mark is
/// built again when its look does, and so is a mark that changes kind (a
/// shared orbit down to one member, or back), keeping how far it has
/// faded in. A named orbit whose last member leaves keeps its mark, which
/// fades out while it is placed (RAIL-09), so a ship coming back fades it
/// in from where it was; a mark is removed only once what it stands for
/// is gone.
fn sync_ring_marks(
    mut commands: Commands,
    assets: MarkAssets,
    rings: OrbitRings,
    marks: Marks,
    icons: Query<(Entity, &ChildOf), With<MarkIcon>>,
) {
    let tunables = &assets.tunables;
    for (root, mark, _) in &marks {
        if !rings.stands(mark.owner) {
            commands.entity(root).despawn();
        }
    }
    for owner in rings.owners() {
        let Some(ring) = rings.ring(owner) else {
            continue;
        };
        let Some(wanted) = mark_shows(&ring, &assets) else {
            continue;
        };
        let current = marks.iter().find(|(_, mark, _)| mark.owner == owner);
        let shown = match (current, &wanted) {
            (Some((_, _, shows)), _) if *shows == wanted => continue,
            (Some((root, _, MarkShows::Multi(..))), MarkShows::Multi(front, look)) => {
                for (icon, _) in icons.iter().filter(|(_, c)| c.parent() == root) {
                    commands
                        .entity(icon)
                        .despawn_children()
                        .with_children(|icon| {
                            spawn_multi_icon(icon, (&assets.fonts, tunables), front, look);
                        });
                }
                commands.entity(root).insert(wanted.clone());
                continue;
            }
            (Some((root, mark, _)), _) => {
                commands.entity(root).despawn();
                mark.shown
            }
            (None, _) => 0.0,
        };
        spawn_mark(&mut commands, &assets, &ring, (wanted, shown));
    }
}

/// MOT-14: a ring's mark, showing `wanted`, faded in `shown` of the way.
fn spawn_mark(
    commands: &mut Commands,
    assets: &MarkAssets,
    ring: &Ring,
    (wanted, shown): (MarkShows, f32),
) {
    let draw = (&*assets.fonts, &*assets.tunables);
    let root = match &wanted {
        MarkShows::Multi(front, look) => {
            spawn_multi_mark(commands, draw, ring, (front, look), shown)
        }
        MarkShows::Body(body, look) => {
            spawn_body_mark(commands, draw, ring, (*body, look.body_look()), shown)
        }
    };
    commands.entity(root).insert(wanted);
}

/// The node a mark and its label sit in: a row, hidden until placed,
/// faded in `shown` of the way.
fn mark_root(owner: Entity, half: f32, shown: f32) -> impl Bundle {
    (
        RingMarkFor {
            owner,
            half,
            shown,
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
/// `markers/labels.rs` places it beside the mark (LBL-01).
fn spawn_label(
    root: &mut ChildSpawnerCommands,
    (fonts, tunables): (&Fonts, &Tunables),
    ring: &Ring,
    color: Color,
) {
    root.spawn((
        RingLabelBox,
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        // Hidden until `markers/labels.rs` has placed it.
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
            TextSize::MapLabel.font(fonts, Face::Label, tunables),
            TextColor(color),
            Tint::new(color, TintSlot::Text),
            TextHalo::ground(),
        ));
        label.spawn((
            PeriodLine,
            Text::new(period_line(ring)),
            TextLayout::linebreak(LineBreak::NoWrap),
            TextSize::SecondLine.font(fonts, Face::Data, tunables),
            TextColor(colors::SECONDARY),
            Tint::new(colors::SECONDARY, TintSlot::Text),
            TextHalo::ground(),
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

/// MOT-14 / MUL-02 / MUL-03: what a shared orbit's multi-marker shows:
/// the frames and count of its things (MUL-04), or the station marker of
/// a station that is its only thing.
fn identity_multi(
    ring: &Ring,
    together_km: f64,
    looks: &Looks,
    tunables: &Tunables,
) -> (IdentityFront, MultiLook) {
    let settings = symbol_settings(tunables);
    let (things, docked) = ring_things(ring, together_km);
    let look_of = |body: Entity| {
        ring.members
            .iter()
            .find(|m| m.body == body)
            .map(|m| (m.class, m.affiliation))
    };
    if let ([thing], [(station, at)]) = (things.as_slice(), docked.as_slice())
        && thing == station
        && let Ok((_, affiliation, Some(record))) = looks.get(*station)
    {
        let symbol = symbol_look(*affiliation, record, settings);
        let at_looks: Vec<_> = at.iter().filter_map(|&b| look_of(b)).collect();
        let look = MultiLook::of_station(&symbol, &at_looks, at.len(), settings);
        return (IdentityFront::Station(symbol), look);
    }
    // MUL-02: docked members count for the frames, though not for the
    // count.
    let thing_looks: Vec<_> = things
        .iter()
        .chain(docked.iter().flat_map(|(_, at)| at))
        .filter_map(|&b| look_of(b))
        .collect();
    let look = MultiLook::of_things(&thing_looks, things.len(), settings);
    (IdentityFront::Frame, look)
}

/// MUL-01 / MUL-03: the parts of a shared orbit's multi-marker under
/// `icon`, the square its front frame fills: frames and count, or a
/// station's symbol with the back frame and count of what is docked.
fn spawn_multi_icon(
    icon: &mut ChildSpawnerCommands,
    (fonts, tunables): (&Fonts, &Tunables),
    front: &IdentityFront,
    look: &MultiLook,
) {
    let size = tunables.ship_symbol_px;
    match front {
        IdentityFront::Frame => spawn_multi_parts(icon, (fonts, tunables), look),
        IdentityFront::Station(symbol) => {
            let back = back_offset(tunables.back_frame_offset_px);
            spawn_back_frame(icon, look, (size, back), true);
            let style = AmplifierStyle {
                gap: amplifier_gap(tunables.bracket_margin_px, tunables.amplifier_gap_px),
                track_mix: tunables.condition_track_mix,
            };
            spawn_symbol_parts(icon, fonts, symbol, size, style);
            spawn_count(icon, fonts, look, tunables.multi_count_px, true);
        }
    }
}

/// MOT-14 / HOV-05: a shared orbit's mark: a multi-marker holding its
/// members, labelled with the orbit's name, targeting the orbit.
fn spawn_multi_mark(
    commands: &mut Commands,
    (fonts, tunables): (&Fonts, &Tunables),
    ring: &Ring,
    (front, look): (&IdentityFront, &MultiLook),
    shown: f32,
) -> Entity {
    let size = tunables.ship_symbol_px;
    let square = Node {
        width: px(size),
        height: px(size),
        ..default()
    };
    commands
        .spawn((
            mark_root(ring.owner, size / 2.0, shown),
            OrbitFor(ring.owner),
            BackFrame(Some(back_offset(tunables.back_frame_offset_px))),
        ))
        .with_children(|root| {
            root.spawn((MarkIcon, square, Pickable::IGNORE))
                .with_children(|icon| spawn_multi_icon(icon, (fonts, tunables), front, look));
            spawn_label(root, (fonts, tunables), ring, colors::NEUTRAL);
            spawn_chevron(root, tunables);
        })
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(hover::on_orbit_click)
        .id()
}

/// MOT-14: a lone body's mark: its symbol at `ship_symbol_px`, or its body
/// marker (MRK-13), labelled with its name, targeting the body.
fn spawn_body_mark(
    commands: &mut Commands,
    (fonts, tunables): (&Fonts, &Tunables),
    ring: &Ring,
    (body, look): (Entity, BodyLook),
    shown: f32,
) -> Entity {
    let half = marker_half_size(look.class, tunables);
    let color = look.color(tunables);
    commands
        .spawn((mark_root(ring.owner, half, shown), MarkerFor(body)))
        .with_children(|root| {
            spawn_icon(root, (fonts, tunables), look, half);
            spawn_label(root, (fonts, tunables), ring, color);
            spawn_chevron(root, tunables);
        })
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(hover::on_marker_click)
        .id()
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
    chevrons: Chevrons<'w, 's>,
}

/// MOT-14 / HOV-07: the box a mark takes, relative to the ring point: a
/// symbol or marker centred on it, with a multi-marker's back frame.
pub fn mark_box(half: f32, back: Option<&BackFrame>) -> Rect {
    frames_rect(Vec2::ZERO, half, back.and_then(|b| b.0))
}

impl MarkParts<'_, '_> {
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
/// has closed and out when it opens. A multi-marker's front frame is
/// centred there, its back frame behind it. The chevron goes on the ring
/// ahead of the mark; the boxes of both are kept for the labels, which are
/// placed after (LBL-01).
fn place_ring_marks(
    (view, time, tunables): (ViewNow, Res<Time>, Res<Tunables>),
    map: MapView,
    rings: OrbitRings,
    mut marks: Query<(Entity, &mut RingMarkFor, &mut Node, Option<&BackFrame>)>,
    (mut mark_parts, mut parts): (MarkParts, TintedParts),
) {
    let Some(viewport) = map.viewport() else {
        return;
    };
    let scale = map.scale_factor();
    let step = step_per_frame(time.delta_secs(), tunables.identity_mark_fade_ms);
    for (entity, mut mark, mut node, back) in &mut marks {
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

        let mark_box = mark_box(half, back);
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
        assert!(
            2.0 * moon_half < tunables.map_label_px,
            "the label is the taller"
        );
        for half in [moon_half, tunables.ship_symbol_px / 2.0] {
            let node = mark_root_node(half);
            assert_eq!(node.height, px(2.0 * half));
            assert_eq!(node.align_items, AlignItems::Center);
        }
    }

    /// MOT-14: a shared orbit's mark is a multi-marker; a lone body's is
    /// its own symbol or marker; an empty ring has none.
    #[test]
    fn shared_orbits_get_a_multi_marker_and_lone_bodies_their_own_mark() {
        let mut world = World::new();
        let member = |world: &mut World| Member {
            body: world.spawn_empty().id(),
            class: BodyClass::Ship,
            affiliation: Affiliation::Own,
            angle: 0.0,
            phase0: 0.0,
            period_days: 1.0,
            hidden: false,
            joined: false,
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
            Some(IdentityMark::Multi)
        );
        assert_eq!(
            identity_mark(&ring(&mut world, vec![a])),
            Some(IdentityMark::Body(a.body))
        );
        assert_eq!(identity_mark(&ring(&mut world, vec![])), None);
    }

    /// MUL-04 / MOT-14: a shared orbit's multi-marker counts a station
    /// and the ships apart from it; ships at the station's phase are
    /// docked, listed under it and not counted.
    #[test]
    fn docked_members_are_not_counted_on_the_identity_mark() {
        let mut world = World::new();
        let at = |world: &mut World, class, phase0| Member {
            body: world.spawn_empty().id(),
            class,
            affiliation: Affiliation::Own,
            angle: 0.0,
            phase0,
            period_days: 1.0,
            hidden: false,
            joined: false,
            mode: MotionModeState::default(),
        };
        let station = at(&mut world, BodyClass::Station, 1.0);
        let docked = at(&mut world, BodyClass::Ship, 1.0);
        let apart = at(&mut world, BodyClass::Ship, 2.0);
        let ring = Ring {
            owner: world.spawn_empty().id(),
            centre: None,
            radius_km: 40_000.0,
            name: "High".to_string(),
            shared: true,
            members: vec![station, docked, apart],
        };
        let (things, at_station) = ring_things(&ring, 10.0);
        assert_eq!(things, [station.body, apart.body]);
        assert_eq!(at_station, [(station.body, vec![docked.body])]);
    }

    /// MUL-04 / MUL-05 / MOT-14: the identity mark decides what is docked
    /// with the map's test at full precision. A ship just over
    /// `together_km` from a station is apart on the map, and so on the
    /// mark too, though its phase rounded to f32 would put it inside.
    #[test]
    fn the_identity_mark_and_the_map_agree_on_what_is_docked_near_together_km() {
        use crate::sim::transfer::phase::{gap_rad, together};
        let mut world = World::new();
        let (radius_km, together_km) = (40_000.0, 10.0);
        let at = |world: &mut World, class, phase0| Member {
            body: world.spawn_empty().id(),
            class,
            affiliation: Affiliation::Own,
            angle: 0.0,
            phase0,
            period_days: 1.0,
            hidden: false,
            joined: false,
            mode: MotionModeState::default(),
        };
        // 10 km at 40,000 km is 2.5e-4 rad; this gap is 10.002 km.
        let (station_phase, ship_phase) = (6.0, 6.0 + 2.5005e-4);
        let station = at(&mut world, BodyClass::Station, station_phase);
        let ship = at(&mut world, BodyClass::Ship, ship_phase);
        let as_f32 = |phase: f64| phase as f32 as f64;
        assert!(
            together(
                radius_km,
                gap_rad(as_f32(station_phase), as_f32(ship_phase)),
                together_km
            ),
            "rounded to f32, the two would be together"
        );
        let ring = Ring {
            owner: world.spawn_empty().id(),
            centre: None,
            radius_km,
            name: "High".to_string(),
            shared: true,
            members: vec![station, ship],
        };
        let slots = [station, ship].map(|m| OrbitSlot {
            thing: m.body,
            orbit: ring.owner,
            radius_km,
            phase_rad: m.phase0,
        });
        assert!(
            together_sets(&slots, together_km).is_empty(),
            "apart on the map"
        );
        let (things, docked) = ring_things(&ring, together_km);
        assert_eq!(things, [station.body, ship.body]);
        assert!(docked.is_empty());
    }
}

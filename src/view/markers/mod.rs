//! The vector-display overlay: one fixed-size UI marker per body, placed on
//! its projected screen position each frame (MRK-03, MRK-04, MRK-06), with
//! its label beside it (SYM-13; where exactly is `markers/labels.rs`, LBL-01).
//!
//! Natural bodies are body markers: outlined circles with a ground-colour
//! inside, ticked for the star and dotted for a planet (MRK-13). Close in
//! they grow into true-size discs, filled and ringed by a dark gap
//! (MRK-07, MRK-11). They are drawn nearer over farther (MRK-12); a
//! salvage site is an X and a gate on
//! screen a hollow circle; ships and stations are symbols (section 3a of
//! the spec, built in `symbols/mod.rs` and `symbols/draw.rs`). What's drawn is
//! decided elsewhere - `fold.rs` says whether a satellite is folded into
//! its parent (GRP-01, GRP-03), `multi.rs` whether a symbol is joined into
//! a multi-marker (MUL-05), `motion_mode.rs` how far its dot has faded
//! into the exposure ring (MOT-11), and `edge/mod.rs` whether a gate is
//! pinned to the screen edge instead (EDGE-01). This module turns those
//! into a position and a strength, which fades every coloured part
//! (`Tint`) as one.
//!
//! MRK-06 holds by construction: only satellites fold, so every planet
//! keeps its own marker at every zoom.
//!
//! The markers and their labels are built in `markers/draw.rs`; this
//! module places, fades and layers them each frame.

use bevy::color::Mix;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::colors;
use super::countdown::Countdowns;
use super::edge::GateEdgeTargets;
use super::fold::SatelliteFoldState;
use super::layers;
use super::motion_mode::MotionModeState;
use super::multi::{JoinSlide, MultiMarkers};
use super::multi_markers::MultiGathered;
use super::screen::ScreenPositions;
use super::ship_groups::GroupFaces;
use super::symbols::draw::{Tint, TintSlot};
use super::symbols::shape::SymbolPart;
use super::symbols::{SecondLineFacts, SymbolSettings, label_second_line, with_countdown};
use super::targets::MarkerFor;
use super::true_size::{DiscRadii, disc_half_px};
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{Affiliation, BodyClass};
use crate::sim::clock::GameClock;
use crate::sim::track::ShipRecord;
use crate::{Boot, Stage};
use draw::spawn_markers;

pub mod brackets;
pub mod draw;
pub mod labels;
pub mod moon_ring;
pub mod picking;

/// Half-size in pixels, so the update system can centre the node on its
/// projected screen position without re-deriving size from class.
#[derive(Component)]
pub struct MarkerHalfSize(pub Vec2);

/// Where a marker is drawn this frame, for whatever is placed round it
/// (the selection and hover brackets, HOV-07): its centre and its drawn
/// half-size, which for a true-size disc is the disc's (MRK-07). `None`
/// while the marker isn't drawn.
#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct MarkerDrawn(pub Option<(Vec2, f32)>);

/// A marker's label, a child of the marker placed beside it by
/// `markers/labels.rs` (LBL-01).
#[derive(Component)]
pub struct MarkerLabel;

/// The markers, multi-markers and ring marks have been laid out for this
/// frame; the labels and brackets are placed after (LBL-01, HOV-06).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarkersPlaced;

/// A symbol label's second line, for the body it describes (SYM-13). It
/// is rewritten as game time passes, so a stale track's age stays true.
#[derive(Component)]
pub struct SecondLineFor(pub Entity);

/// MRK-04: the shape a body's marker takes.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkerShape {
    /// Body marker: a natural body. It alone grows to a true-size disc
    /// (MRK-07, MRK-13).
    Disc,
    /// Hollow circle: a gate on screen (its edge arrow is EDGE-01).
    HollowCircle,
    Glyph(&'static str),
    /// A ship or station symbol (SYM-01 to SYM-08).
    Symbol,
}

/// Below this combined strength a marker is simply not drawn - which also
/// takes it out of picking, so a folded satellite can't be clicked.
const MIN_VISIBLE_ALPHA: f32 = 0.02;
/// A natural body's rim: a child of its disc, drawn as the child's border
/// so it fades apart from the fill (MRK-11).
#[derive(Component)]
pub struct BodyRim;

/// A body marker's class detail - the star's ticks or a planet's dot -
/// shown only while the body is drawn at its marker size (MRK-13).
#[derive(Component)]
pub struct BodyDetail;

/// MRK-04: body marker = natural body; X = salvage site; a gate on screen
/// is a hollow circle; ships and stations are symbols.
pub fn marker_shape(class: BodyClass, has_record: bool) -> MarkerShape {
    match class {
        BodyClass::Ship | BodyClass::Station if has_record => MarkerShape::Symbol,
        BodyClass::Gate => MarkerShape::HollowCircle,
        BodyClass::SalvageSite => MarkerShape::Glyph("X"),
        _ => MarkerShape::Disc,
    }
}

/// MRK-03 / SYM-04: marker size is a presentation choice by class, in
/// screen pixels, never derived from physical size. Natural bodies have
/// the largest markers; a symbol's frame fits a square of `ship_symbol_px`.
pub fn marker_half_size(class: BodyClass, tunables: &Tunables) -> f32 {
    match class {
        BodyClass::Star => tunables.star_marker_px / 2.0,
        BodyClass::Planet => tunables.planet_marker_px / 2.0,
        BodyClass::Moon => tunables.moon_marker_px / 2.0,
        BodyClass::Gate => 5.5,
        BodyClass::SalvageSite => 7.0,
        BodyClass::Ship | BodyClass::Station => tunables.ship_symbol_px / 2.0,
    }
}

/// MRK-07: the half-size a marker is drawn at, given its class's fixed
/// half-size and the body's true radius on screen. A natural body's disc
/// grows to its true size once that is larger; every other shape keeps its
/// fixed size.
pub fn drawn_half(class_half: f32, is_disc: bool, true_radius_px: f32) -> f32 {
    if is_disc {
        disc_half_px(class_half, true_radius_px)
    } else {
        class_half
    }
}

/// HOV-07: how far a marker's bracket box reaches from its centre: the
/// drawn marker (a symbol's square, SYM-14, or a disc to the outer edge of
/// its rim) and the margin.
pub fn box_half(drawn_half: f32, tunables: &Tunables) -> f32 {
    drawn_half + tunables.bracket_margin_px
}

/// HOV-07 / GRP-12: how far a parent's bracket box reaches from its
/// centre, without its moon ring: its drawn marker, grown to a true-size
/// disc when close (MRK-07), and the margin. Its object pip and popped
/// fold marker are placed from that corner.
pub fn parent_clear_px(
    class: BodyClass,
    has_record: bool,
    true_radius_px: f32,
    tunables: &Tunables,
) -> f32 {
    let is_disc = marker_shape(class, has_record) == MarkerShape::Disc;
    let drawn = drawn_half(marker_half_size(class, tunables), is_disc, true_radius_px);
    box_half(drawn, tunables)
}

/// GRP-01 / GRP-03 / MOT-11: how strongly a body's marker is drawn. A
/// fully folded satellite draws nothing; a sliding one fades with the
/// slide; and as its tail closes into the exposure ring the dot fades out.
pub fn marker_alpha(fold_anim_t: f32, fully_hidden: bool, dot_alpha: f32) -> f32 {
    if fully_hidden {
        0.0
    } else {
        fold_anim_t * dot_alpha
    }
}

/// GRP-03: while folding or unfolding, a satellite's marker slides between
/// its parent's marker (0.0) and its true position (1.0).
pub fn marker_position(own: Vec2, parent: Option<Vec2>, fold_anim_t: f32) -> Vec2 {
    match parent {
        Some(parent) if fold_anim_t < 1.0 => parent.lerp(own, fold_anim_t),
        _ => own,
    }
}

/// MUL-05: a symbol joining a multi-marker slides from its own place
/// towards the marker's centre and fades as it goes; parting, it slides
/// back out. Returns where it is drawn and how much of its strength it
/// keeps.
pub fn joined_place(own: Vec2, slide: Option<&JoinSlide>) -> (Vec2, f32) {
    match slide {
        Some(slide) => (own + slide.offset * slide.t, 1.0 - slide.t),
        None => (own, 1.0),
    }
}

/// SYM-02 / SYM-03 / SYM-08: the symbol settings from the tunables.
pub fn symbol_settings(tunables: &Tunables) -> SymbolSettings {
    SymbolSettings {
        fill_faction: tunables.symbol_fill_faction,
        fill_other: tunables.symbol_fill_other,
        stale_alpha: tunables.stale_symbol_alpha,
        condition_warn: tunables.condition_warn_fraction,
    }
}

/// MRK-11 / MRK-13: a natural body's opaque fill: the ground colour as a
/// marker, the neutral mixed into the ground as a true-size disc.
pub fn body_fill(true_size: bool, tunables: &Tunables) -> Color {
    if true_size {
        colors::GROUND.mix(&colors::NEUTRAL, tunables.body_fill_mix)
    } else {
        colors::GROUND
    }
}

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        // MUL-13: a station's second line reads its docked count. A moon
        // ring reads `FoldSummary`, which `GroupingPlugin` owns (GRP-10).
        app.init_resource::<MultiMarkers>()
            .add_systems(Startup, spawn_markers.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    (moon_ring::update_moon_rings, update_markers)
                        .chain()
                        .in_set(MarkersPlaced),
                    layer_bodies_by_depth,
                    // Reads this frame's docked counts, so none lags a frame.
                    update_second_lines.after(MultiGathered),
                )
                    .in_set(Stage::Overlay),
            );
    }
}

/// SYM-08 / SYM-13 / PLNR-66 / MUL-13: keep each second line true as
/// game time passes, so a stale track's age counts up, an order's
/// countdown counts down and a station's docked count follows what is
/// docked.
fn update_second_lines(
    clock: Res<GameClock>,
    bodies: Query<(&Affiliation, &ShipRecord)>,
    (groups, multis): (GroupFaces, Res<MultiMarkers>),
    countdowns: Countdowns,
    mut lines: Query<(&SecondLineFor, &mut Text)>,
) {
    for (line_for, mut text) in &mut lines {
        let Ok((affiliation, record)) = bodies.get(line_for.0) else {
            continue;
        };
        let members = groups.member_classes(line_for.0);
        let facts = SecondLineFacts {
            members: &members,
            docked: multis.docked_count(line_for.0),
            now_day: clock.elapsed_days(),
        };
        let line = with_countdown(
            label_second_line(*affiliation, record, facts),
            countdowns.of(line_for.0),
        )
        .unwrap_or_default();
        if text.0 != line {
            text.0 = line;
        }
    }
}

/// MRK-12: natural bodies are layered by depth every frame, so a nearer
/// disc is drawn over a farther one as the view turns.
fn layer_bodies_by_depth(
    view: ViewNow,
    mut markers: Query<(&MarkerFor, &MarkerShape, &mut GlobalZIndex)>,
) {
    let Some((_, camera)) = view.camera() else {
        return;
    };
    let depth_of = |body| {
        let position = view.position(body)?;
        Some((position - camera.translation()).dot(*camera.forward()))
    };
    let depths: Vec<(Entity, f32)> = markers
        .iter()
        .filter(|(_, shape, _)| **shape == MarkerShape::Disc)
        .filter_map(|(marker_for, ..)| Some((marker_for.0, depth_of(marker_for.0)?)))
        .collect();
    let layers: std::collections::HashMap<Entity, i32> =
        layers::depth_layers(&depths).into_iter().collect();
    for (marker_for, _, mut z) in &mut markers {
        if let Some(&layer) = layers.get(&marker_for.0)
            && z.0 != layer
        {
            z.0 = layer;
        }
    }
}

/// What a marker's look depends on, from its body.
type MarkerBodies<'w, 's> = Query<
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

/// The marker nodes themselves.
type MarkerNodes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MarkerFor,
        &'static MarkerHalfSize,
        &'static MarkerShape,
        &'static mut Node,
        &'static mut MarkerDrawn,
    ),
>;

/// The labels, by the marker they belong to.
/// Body markers' class details (MRK-13).
type DetailNodes<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (With<BodyDetail>, Without<MarkerShape>, Without<MarkerLabel>),
>;

/// MRK-13: show a body marker's ticks or dot, or hide them on a true-size
/// disc (MRK-07).
fn show_body_detail(
    root: Entity,
    shown: bool,
    children: &Query<&Children>,
    details: &mut DetailNodes,
) {
    let display = if shown { Display::Flex } else { Display::None };
    for child in children.iter_descendants(root) {
        if let Ok(mut node) = details.get_mut(child)
            && node.display != display
        {
            node.display = display;
        }
    }
}

/// Every coloured part of a marker, and where its colour lives.
pub type TintedParts<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Tint,
        Option<&'static mut BackgroundColor>,
        Option<&'static mut BorderColor>,
        Option<&'static mut SymbolPart>,
        Option<&'static mut TextColor>,
    ),
>;

/// Write each tinted part's colour at `alpha` of its full strength, to the
/// one slot it is drawn with.
pub fn fade_parts(root: Entity, alpha: f32, children: &Query<&Children>, parts: &mut TintedParts) {
    for part in std::iter::once(root).chain(children.iter_descendants(root)) {
        let Ok((tint, background, border, shape, text)) = parts.get_mut(part) else {
            continue;
        };
        let color = tint.color.with_alpha(tint.color.alpha() * alpha);
        match tint.slot {
            TintSlot::Background => {
                if let Some(mut background) = background {
                    background.0 = color;
                }
            }
            TintSlot::Border => {
                if let Some(mut border) = border {
                    border.set_all(color);
                }
            }
            TintSlot::Shape => {
                if let Some(mut shape) = shape {
                    shape.set_if_neq(SymbolPart { color, ..*shape });
                }
            }
            TintSlot::Text => {
                if let Some(mut text) = text {
                    text.0 = color;
                }
            }
        }
    }
}

/// Where markers go this frame: the bodies' screen positions, the gates
/// pinned to the map's edge, and the true-size discs' radii.
#[derive(SystemParam)]
struct MarkerScreen<'w> {
    screen: Res<'w, ScreenPositions>,
    edges: Res<'w, GateEdgeTargets>,
    discs: Res<'w, DiscRadii>,
}

impl MarkerScreen<'_> {
    /// MRK-03: a marker's centre, moving towards its parent's as it folds
    /// and towards a multi-marker's as it joins one (MUL-05); `None` while
    /// the body is off screen, faded out, or a gate pinned to the edge.
    fn centre(
        &self,
        body: Entity,
        parent: Option<Entity>,
        alpha: f32,
        (fold_anim_t, slide): (f32, Option<&JoinSlide>),
    ) -> Option<Vec2> {
        let own = self
            .screen
            .get(body)
            .filter(|pos| self.screen.in_viewport(*pos))
            .filter(|_| alpha > MIN_VISIBLE_ALPHA && !self.edges.is_pinned(body))?;
        let parent = parent.and_then(|parent| self.screen.get(parent));
        Some(joined_place(marker_position(own, parent, fold_anim_t), slide).0)
    }
}

/// A moon ring's node (GRP-10).
type RingNodes<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<moon_ring::MoonRingNode>,
        Without<MarkerShape>,
        Without<BodyDetail>,
    ),
>;

/// A marker's coloured parts, its class detail and moon ring, and the
/// tunables they are coloured by.
#[derive(SystemParam)]
struct MarkerPaint<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parts: TintedParts<'w, 's>,
    details: DetailNodes<'w, 's>,
    rings: Query<
        'w,
        's,
        (
            &'static moon_ring::MoonRing,
            &'static moon_ring::MoonRingParts,
        ),
    >,
    ring_nodes: RingNodes<'w, 's>,
    tunables: Res<'w, Tunables>,
}

impl MarkerPaint<'_, '_> {
    /// Fades a marker's parts to `alpha`. A disc (`true_size` is `Some`)
    /// grown past its marker is first filled and loses its class detail
    /// (MRK-07, MRK-11, MRK-13); its moon ring takes the same fill and
    /// fades in and out on top of the marker's fade (GRP-10).
    fn paint(&mut self, marker: Entity, true_size: Option<bool>, alpha: f32) {
        let Some(true_size) = true_size else {
            fade_parts(marker, alpha, &self.children, &mut self.parts);
            return;
        };
        let fill = body_fill(true_size, &self.tunables);
        if let Ok((mut tint, ..)) = self.parts.get_mut(marker) {
            tint.color = fill;
        }
        show_body_detail(marker, !true_size, &self.children, &mut self.details);
        let ring = self
            .rings
            .get(marker)
            .ok()
            .map(|(ring, parts)| (*ring, *parts));
        if let Some((_, parts)) = ring {
            for child in [parts.ring, parts.cover] {
                if let Ok((mut tint, ..)) = self.parts.get_mut(child) {
                    tint.color = fill;
                }
            }
        }
        fade_parts(marker, alpha, &self.children, &mut self.parts);
        let Some((ring, parts)) = ring else {
            return;
        };
        if let Ok(mut node) = self.ring_nodes.get_mut(parts.ring) {
            let display = if ring.t > 0.0 {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
        fade_parts(parts.ring, alpha * ring.t, &self.children, &mut self.parts);
    }
}

/// A marker's alpha and how far its fold has run, from its body's fold,
/// join (MUL-05) and motion mode.
fn marker_fade(
    fold: Option<&SatelliteFoldState>,
    motion: Option<&MotionModeState>,
    slide: Option<&JoinSlide>,
) -> (f32, f32) {
    let fold_anim_t = fold.map(|f| f.anim_t).unwrap_or(1.0);
    let alpha = marker_alpha(
        fold_anim_t,
        fold.map(|f| f.effective_hidden).unwrap_or(false),
        motion.map(MotionModeState::dot_alpha).unwrap_or(1.0),
    );
    (alpha * joined_place(Vec2::ZERO, slide).1, fold_anim_t)
}

/// MRK-03 / MRK-07: place, fade and colour every marker for this frame.
fn update_markers(
    place: MarkerScreen,
    bodies: MarkerBodies,
    mut markers: MarkerNodes,
    mut paint: MarkerPaint,
) {
    for (entity, marker_for, half, shape, mut node, mut drawn_at) in &mut markers {
        let body = marker_for.0;
        drawn_at.set_if_neq(MarkerDrawn(None));
        let Ok((child_of, fold, motion, slide)) = bodies.get(body) else {
            node.display = Display::None;
            continue;
        };
        let (alpha, fold_anim_t) = marker_fade(fold, motion, slide);
        let parent = child_of.map(ChildOf::parent);
        let Some(pos) = place.centre(body, parent, alpha, (fold_anim_t, slide)) else {
            node.display = Display::None;
            continue;
        };
        let disc = *shape == MarkerShape::Disc;
        let drawn = drawn_half(half.0.x, disc, place.discs.get(body));
        drawn_at.set_if_neq(MarkerDrawn(Some((pos, drawn))));
        node.display = Display::Flex;
        node.width = px(drawn * 2.0);
        node.height = px(drawn * 2.0);
        node.left = px(pos.x - drawn);
        node.top = px(pos.y - drawn);
        paint.paint(entity, disc.then_some(drawn > half.0.x), alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MRK-11 / MRK-13: a marker's inside is the ground colour; a true-size
    /// disc's is the neutral mixed into the ground, opaque either way.
    #[test]
    fn body_fills_are_opaque_ground_or_a_mix() {
        let tunables = Tunables::default();
        assert_eq!(body_fill(false, &tunables), colors::GROUND);
        let disc = body_fill(true, &tunables);
        assert_eq!(disc.alpha(), 1.0);
        assert_ne!(disc, colors::GROUND);
        assert_ne!(disc, colors::NEUTRAL);
    }

    /// MRK-04: discs for natural bodies, X for salvage, a hollow circle
    /// for a gate, and symbols for ships and stations.
    #[test]
    fn marker_shapes_by_class() {
        use BodyClass::*;
        assert_eq!(marker_shape(Planet, false), MarkerShape::Disc);
        assert_eq!(marker_shape(Moon, false), MarkerShape::Disc);
        assert_eq!(marker_shape(Star, false), MarkerShape::Disc);
        assert_eq!(marker_shape(SalvageSite, false), MarkerShape::Glyph("X"));
        assert_eq!(marker_shape(Gate, false), MarkerShape::HollowCircle);
        assert_eq!(marker_shape(Ship, true), MarkerShape::Symbol);
        assert_eq!(marker_shape(Station, true), MarkerShape::Symbol);
    }

    /// MRK-03 / SYM-04: marker size depends on class only; natural bodies
    /// are the largest, star over planet over moon, each at least a ship
    /// symbol across; a symbol fits `ship_symbol_px`.
    #[test]
    fn marker_size_is_by_class() {
        let tunables = Tunables::default();
        let size = |class| 2.0 * marker_half_size(class, &tunables);
        assert_eq!(size(BodyClass::Star), tunables.star_marker_px);
        assert_eq!(size(BodyClass::Planet), tunables.planet_marker_px);
        assert_eq!(size(BodyClass::Moon), tunables.moon_marker_px);
        assert!(size(BodyClass::Star) > size(BodyClass::Planet));
        assert!(size(BodyClass::Planet) > size(BodyClass::Moon));
        for class in [BodyClass::Ship, BodyClass::Station] {
            assert_eq!(size(class), tunables.ship_symbol_px);
        }
    }

    /// HOV-07 / MRK-07: a disc's box grows with its true size once that
    /// beats the marker; any other shape keeps its fixed size.
    #[test]
    fn the_box_follows_the_drawn_size() {
        let tunables = Tunables::default();
        let margin = tunables.bracket_margin_px;
        assert_eq!(
            box_half(drawn_half(6.0, true, 2.0), &tunables),
            6.0 + margin
        );
        assert_eq!(
            box_half(drawn_half(6.0, true, 80.0), &tunables),
            80.0 + margin
        );
        assert_eq!(
            box_half(drawn_half(11.0, false, 80.0), &tunables),
            11.0 + margin
        );
    }

    /// GRP-01: a fully folded satellite draws nothing, whatever else says.
    #[test]
    fn folded_satellite_marker_is_invisible() {
        assert_eq!(marker_alpha(1.0, true, 1.0), 0.0);
        assert_eq!(marker_alpha(0.0, false, 1.0), 0.0);
        assert_eq!(marker_alpha(1.0, false, 1.0), 1.0);
    }

    /// GRP-03 / MOT-11: the marker fades with the fold slide and as the
    /// tail closes.
    #[test]
    fn ring_mode_fades_the_dot() {
        assert_eq!(marker_alpha(1.0, false, 0.0), 0.0);
        assert!((marker_alpha(0.5, false, 0.5) - 0.25).abs() < 1e-6);
    }

    /// SYM-02 / SYM-13: fading writes each part's colour to its own slot
    /// only, so a symbol part's or a label's node background stays clear.
    #[test]
    fn fading_leaves_other_colour_slots_alone() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let color = colors::OWN;
        let image = world
            .spawn((
                SymbolPart {
                    mask: crate::view::symbols::shape::SymbolMask::Triangle,
                    color,
                    size_px: 22.0,
                },
                BackgroundColor(Color::NONE),
                Tint::new(color, TintSlot::Shape),
            ))
            .id();
        let text = world
            .spawn((
                TextColor(color),
                BackgroundColor(Color::NONE),
                Tint::new(color, TintSlot::Text),
            ))
            .id();
        let root = world.spawn_empty().add_children(&[image, text]).id();
        world
            .run_system_once(move |children: Query<&Children>, mut parts: TintedParts| {
                fade_parts(root, 0.5, &children, &mut parts);
            })
            .unwrap();
        let faded = color.with_alpha(0.5);
        assert_eq!(world.get::<SymbolPart>(image).unwrap().color, faded);
        assert_eq!(world.get::<TextColor>(text).unwrap().0, faded);
        for part in [image, text] {
            assert_eq!(world.get::<BackgroundColor>(part).unwrap().0, Color::NONE);
        }
    }

    /// MUL-05: a joining symbol slides towards the marker's centre and
    /// fades; a symbol held by none is where it is, at full strength.
    #[test]
    fn a_joining_symbol_slides_into_the_marker_and_fades() {
        let own = Vec2::new(100.0, 50.0);
        let slide = JoinSlide {
            t: 0.25,
            offset: Vec2::new(-8.0, 4.0),
            held: true,
            lead: false,
        };
        assert_eq!(
            joined_place(own, Some(&slide)),
            (Vec2::new(98.0, 51.0), 0.75)
        );
        let inside = JoinSlide { t: 1.0, ..slide };
        assert_eq!(joined_place(own, Some(&inside)), (own + slide.offset, 0.0));
        assert_eq!(joined_place(own, None), (own, 1.0));
    }

    /// GRP-12 / HOV-07: the parent's box an object marker is placed from
    /// grows with the parent's true-size disc.
    #[test]
    fn a_parents_box_grows_with_its_disc() {
        let tunables = Tunables::default();
        let margin = tunables.bracket_margin_px;
        let planet = tunables.planet_marker_px / 2.0;
        let far = parent_clear_px(BodyClass::Planet, false, 1.0, &tunables);
        assert_eq!(far, planet + margin);
        let close = parent_clear_px(BodyClass::Planet, false, 90.0, &tunables);
        assert_eq!(close, 90.0 + margin);
    }

    /// GRP-03: the marker slides from its parent to its true position.
    #[test]
    fn marker_slides_out_of_its_parent() {
        let own = Vec2::new(100.0, 0.0);
        let parent = Some(Vec2::ZERO);
        assert_eq!(marker_position(own, parent, 0.0), Vec2::ZERO);
        assert_eq!(marker_position(own, parent, 0.5), Vec2::new(50.0, 0.0));
        assert_eq!(marker_position(own, parent, 1.0), own);
        // A body with no parent on screen stays where it is.
        assert_eq!(marker_position(own, None, 0.3), own);
    }
}

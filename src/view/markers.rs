//! The vector-display overlay: one fixed-size UI marker per body, placed on
//! its projected screen position each frame (MRK-03, MRK-04, MRK-06), with
//! its label to the right (SYM-13).
//!
//! Natural bodies are filled discs, a salvage site an X and a gate on
//! screen a hollow circle; ships and stations are symbols (section 3a of
//! the spec, built in `symbols.rs` and `symbol_draw.rs`). What's drawn is
//! decided elsewhere - `fold.rs` says whether a satellite is folded into
//! its parent (GRP-01, GRP-03), `motion_mode.rs` how far its dot has faded
//! into the exposure ring (MOT-11), and `edge.rs` whether a gate is pinned
//! to the screen edge instead (EDGE-01). This module turns those into a
//! position and a strength, which fades every coloured part (`Tint`) as
//! one.
//!
//! MRK-06 holds by construction: only satellites fold, so every planet
//! keeps its own marker at every zoom.

use bevy::prelude::*;

use super::colors;
use super::edge::GateEdgeTargets;
use super::fold::SatelliteFoldState;
use super::fonts::Fonts;
use super::hover;
use super::hovered::HoveredBodies;
use super::motion_mode::MotionModeState;
use super::scale::km_to_render;
use super::screen::ScreenPositions;
use super::selection::Selection;
use super::symbol_draw::{Tint, TintSlot, spawn_symbol_parts};
use super::symbol_images::SymbolImages;
use super::symbols::{SymbolSettings, label_second_line, symbol_look};
use super::targets::MarkerFor;
use super::true_size::{disc_half_px, projected_radius_px};
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{Affiliation, BodyClass, DisplayName, PhysicalRadius};
use crate::sim::clock::GameClock;
use crate::sim::track::ShipRecord;
use crate::{Boot, Stage};

/// Half-size in pixels, so the update system can centre the node on its
/// projected screen position without re-deriving size from class.
#[derive(Component)]
pub struct MarkerHalfSize(pub Vec2);

/// A marker's label, which sits just right of the marker and moves out when
/// the marker grows to a true-size disc (MRK-07).
#[derive(Component)]
pub struct MarkerLabel;

/// A symbol label's second line, for the body it describes (SYM-13). It
/// is rewritten as game time passes, so a stale track's age stays true.
#[derive(Component)]
pub struct SecondLineFor(pub Entity);

/// MRK-04: the shape a body's marker takes.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkerShape {
    /// Filled disc: a natural body. It alone grows to true size (MRK-07).
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
/// Outline width of hollow markers.
const HOLLOW_BORDER_PX: f32 = 1.5;
/// Gap between the hover ring (an outline) and the marker it rings.
const HOVER_RING_OFFSET_PX: f32 = 2.0;
/// Width of the white hover and selection ring.
const RING_PX: f32 = 2.0;
/// Glyph markers are drawn a little larger than their box so the glyph's
/// own side bearings don't shrink it.
const GLYPH_SCALE: f32 = 1.8;
/// Gap between a marker and its label, which sits to its right.
const LABEL_GAP_PX: f32 = 4.0;
const LABEL_FONT_PX: f32 = 12.0;
const SECOND_LINE_FONT_PX: f32 = 10.0;

/// MRK-04: filled disc = natural body; X = salvage site; a gate on screen
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
/// screen pixels, never derived from physical size. A symbol's frame fits
/// a square of `symbol_px`.
pub fn marker_half_size(class: BodyClass, symbol_px: f32) -> f32 {
    match class {
        BodyClass::Star => 9.0,
        BodyClass::Planet => 6.0,
        BodyClass::Moon => 3.0,
        BodyClass::Gate => 5.5,
        BodyClass::SalvageSite => 7.0,
        BodyClass::Ship | BodyClass::Station => symbol_px / 2.0,
    }
}

/// How far the white ring reaches from a marker's centre (MRK-01), so
/// anything drawn round a marker (CAM-22's brackets) can clear it. For a
/// symbol it encloses the symbol square, not the amplifiers (SYM-14).
pub fn ring_outer_half(class: BodyClass, symbol_px: f32) -> f32 {
    marker_half_size(class, symbol_px) + HOVER_RING_OFFSET_PX + RING_PX
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

/// SYM-02 / SYM-03 / SYM-08: the symbol settings from the tunables.
pub fn symbol_settings(tunables: &Tunables) -> SymbolSettings {
    SymbolSettings {
        fill_state: tunables.symbol_fill_state,
        fill_other: tunables.symbol_fill_other,
        stale_alpha: tunables.stale_symbol_alpha,
    }
}

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_markers.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (update_markers, ring_markers, update_second_lines).in_set(Stage::Overlay),
            );
    }
}

/// What a marker needs from its body.
type SpawnBodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static BodyClass,
        &'static Affiliation,
        &'static DisplayName,
        Option<&'static ShipRecord>,
    ),
>;

fn spawn_markers(
    mut commands: Commands,
    fonts: Res<Fonts>,
    images: Res<SymbolImages>,
    tunables: Res<Tunables>,
    clock: Res<GameClock>,
    bodies: SpawnBodies,
) {
    for (body, class, affiliation, name, record) in &bodies {
        let marker = BodyLook {
            class: *class,
            affiliation: *affiliation,
            record,
        };
        let assets = (&*fonts, &*images, &*tunables);
        let label = MarkerText {
            name: &name.0,
            second_line: record
                .and_then(|r| label_second_line(*affiliation, r, clock.elapsed_days())),
        };
        spawn_marker(&mut commands, assets, body, marker, label);
    }
}

/// Who a marker stands for, as far as its look goes.
#[derive(Clone, Copy)]
pub struct BodyLook<'a> {
    pub class: BodyClass,
    pub affiliation: Affiliation,
    pub record: Option<&'a ShipRecord>,
}

impl BodyLook<'_> {
    fn shape(&self) -> MarkerShape {
        marker_shape(self.class, self.record.is_some())
    }

    /// The marker's and label's colour: a symbol's by affiliation and
    /// command (SYM-02), a body's own blue or neutral white (MRK-01).
    pub fn color(&self, tunables: &Tunables) -> Color {
        match self.record {
            Some(record) => symbol_look(self.affiliation, record, symbol_settings(tunables)).color,
            None => colors::affiliation_color(self.affiliation),
        }
    }
}

/// What a marker's label says (SYM-13).
struct MarkerText<'a> {
    name: &'a str,
    second_line: Option<String>,
}

/// The fonts, symbol masks and tunables a marker is built from.
pub type MarkerAssets<'a> = (&'a Fonts, &'a SymbolImages, &'a Tunables);

/// MRK-03 / MRK-04: the marker's box, sized by class and shaped by
/// rounding and borders.
fn marker_node(shape: MarkerShape, half: f32) -> Node {
    let mut node = Node {
        position_type: PositionType::Absolute,
        width: px(half * 2.0),
        height: px(half * 2.0),
        display: Display::None,
        ..default()
    };
    if shape == MarkerShape::HollowCircle {
        node.border = UiRect::all(px(HOLLOW_BORDER_PX));
    }
    if matches!(shape, MarkerShape::Disc | MarkerShape::HollowCircle) {
        node.border_radius = BorderRadius::MAX;
    }
    node
}

/// The shape itself, spawned on or under `marker`, at `half` pixels.
fn spawn_shape(marker: &mut EntityCommands, assets: MarkerAssets, look: BodyLook, half: f32) {
    let (fonts, images, tunables) = assets;
    let color = look.color(tunables);
    match look.shape() {
        MarkerShape::Disc => {
            marker.insert((
                BackgroundColor(color),
                Tint::new(color, TintSlot::Background),
            ));
        }
        MarkerShape::HollowCircle => {
            marker.insert((
                BackgroundColor(Color::NONE),
                BorderColor::all(color),
                Tint::new(color, TintSlot::Border),
            ));
        }
        MarkerShape::Glyph(glyph) => {
            marker.with_children(|parent| {
                parent.spawn((
                    Text::new(glyph),
                    fonts.label(half * GLYPH_SCALE),
                    TextColor(color),
                    Tint::new(color, TintSlot::Text),
                    Pickable::IGNORE,
                ));
            });
        }
        MarkerShape::Symbol => {
            if let Some(record) = look.record {
                let symbol = symbol_look(look.affiliation, record, symbol_settings(tunables));
                marker.with_children(|parent| {
                    spawn_symbol_parts(parent, (images, fonts), &symbol, half * 2.0);
                });
            }
        }
    }
}

fn spawn_marker(
    commands: &mut Commands,
    assets: MarkerAssets,
    body: Entity,
    look: BodyLook,
    text: MarkerText,
) {
    let (fonts, _, tunables) = assets;
    let half = marker_half_size(look.class, tunables.ship_symbol_px);
    let shape = look.shape();
    let mut marker = commands.spawn((
        MarkerFor(body),
        MarkerHalfSize(Vec2::splat(half)),
        shape,
        marker_node(shape, half),
        // Hover and click go through the marker's hit area instead
        // (MRK-08, `marker_picking.rs`), so the node itself takes none.
        Pickable::IGNORE,
        // The hover ring (HOV-01): an outline, invisible until hovered.
        Outline {
            width: px(0),
            offset: px(HOVER_RING_OFFSET_PX),
            color: Color::NONE,
        },
    ));
    spawn_shape(&mut marker, assets, look, half);
    let color = look.color(tunables);
    marker.with_children(|parent| spawn_label(parent, fonts, (body, half, color), text));
}

/// SYM-13: the label to the right of the marker, in its colour: the name,
/// and for a symbol a second line in the secondary text colour. It is a
/// hover and click target for its body, and wins over an orbit ring
/// (HOV-02).
fn spawn_label(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    (body, half, color): (Entity, f32, Color),
    text: MarkerText,
) {
    parent
        .spawn((
            MarkerLabel,
            MarkerFor(body),
            Node {
                position_type: PositionType::Absolute,
                left: px(half * 2.0 + LABEL_GAP_PX),
                top: px(0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|label| {
            label.spawn((
                Text::new(text.name),
                // The label sits beside a marker a few pixels wide; it must
                // not wrap to that width.
                TextLayout::linebreak(LineBreak::NoWrap),
                fonts.label(LABEL_FONT_PX),
                TextColor(color),
                Tint::new(color, TintSlot::Text),
            ));
            if let Some(line) = text.second_line {
                label.spawn((
                    SecondLineFor(body),
                    Text::new(line),
                    TextLayout::linebreak(LineBreak::NoWrap),
                    fonts.mono(SECOND_LINE_FONT_PX),
                    TextColor(colors::PANEL_BORDER),
                    Tint::new(colors::PANEL_BORDER, TintSlot::Text),
                ));
            }
        })
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(hover::on_marker_click);
}

/// SYM-08 / SYM-13: keep each second line true as game time passes, so a
/// stale track's age counts up.
fn update_second_lines(
    clock: Res<GameClock>,
    bodies: Query<(&Affiliation, &ShipRecord)>,
    mut lines: Query<(&SecondLineFor, &mut Text)>,
) {
    for (line_for, mut text) in &mut lines {
        let Ok((affiliation, record)) = bodies.get(line_for.0) else {
            continue;
        };
        let line =
            label_second_line(*affiliation, record, clock.elapsed_days()).unwrap_or_default();
        if text.0 != line {
            text.0 = line;
        }
    }
}

/// INS-05: a small copy of a body's marker, for lists: a symbol at
/// `list_symbol_px` for a ship or station, or the body's shape at
/// `body_half` pixels.
pub fn spawn_list_icon(
    parent: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    look: BodyLook,
    body_half: f32,
) {
    let (_, _, tunables) = assets;
    let half = match look.shape() {
        MarkerShape::Symbol => tunables.list_symbol_px / 2.0,
        _ => body_half,
    };
    let mut node = marker_node(look.shape(), half);
    node.position_type = PositionType::Relative;
    node.display = Display::Flex;
    let mut icon = parent.spawn(node);
    spawn_shape(&mut icon, assets, look, half);
}

/// MRK-01: the white ring - on a marker that is hovered (HOV-01) or
/// selected (INS-01, CAM-22).
pub fn shows_ring(hovered: bool, selected: bool) -> bool {
    hovered || selected
}

/// MRK-01: draw the white ring on hovered and selected markers.
fn ring_markers(
    selection: Res<Selection>,
    hovered: Res<HoveredBodies>,
    mut markers: Query<(&MarkerFor, &mut Outline), With<MarkerHalfSize>>,
) {
    for (marker_for, mut outline) in &mut markers {
        let body = marker_for.0;
        let ring = shows_ring(hovered.contains(body), selection.is_selected(body));
        outline.width = px(if ring { RING_PX } else { 0.0 });
        outline.color = if ring {
            colors::SELECTION_RING
        } else {
            Color::NONE
        };
    }
}

/// MRK-07: the half-size a body's marker is drawn at. A disc is as large as
/// its true radius on screen when that beats the class's fixed size; every
/// other shape keeps its fixed size.
fn drawn_half_size(
    view: &ViewNow,
    projection: Option<&Projection>,
    viewport_height: f32,
    (body, shape_is_disc, class_half): (Entity, bool, f32),
    radius: Option<&PhysicalRadius>,
) -> f32 {
    let (Some(radius), true, Some(Projection::Perspective(perspective))) =
        (radius, shape_is_disc, projection)
    else {
        return class_half;
    };
    let (Some((_, camera)), Some(position)) = (view.camera(), view.position(body)) else {
        return class_half;
    };
    let depth = (position - camera.translation()).dot(*camera.forward());
    let true_radius = projected_radius_px(
        km_to_render(radius.0),
        depth,
        perspective.fov,
        viewport_height,
    );
    disc_half_px(class_half, true_radius)
}

/// What a marker's look depends on, from its body.
type MarkerBodies<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static ChildOf>,
        Option<&'static SatelliteFoldState>,
        Option<&'static MotionModeState>,
        Option<&'static PhysicalRadius>,
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
    ),
>;

/// The labels, by the marker they belong to.
type LabelNodes<'w, 's> =
    Query<'w, 's, (&'static ChildOf, &'static mut Node), (With<MarkerLabel>, Without<MarkerShape>)>;

/// Every coloured part of a marker, and where its colour lives.
type TintedParts<'w, 's> = Query<
    'w,
    's,
    (
        &'static Tint,
        Option<&'static mut BackgroundColor>,
        Option<&'static mut BorderColor>,
        Option<&'static mut ImageNode>,
        Option<&'static mut TextColor>,
    ),
>;

/// Write each tinted part's colour at `alpha` of its full strength, to the
/// one slot it is drawn with.
fn fade_parts(root: Entity, alpha: f32, children: &Query<&Children>, parts: &mut TintedParts) {
    for part in std::iter::once(root).chain(children.iter_descendants(root)) {
        let Ok((tint, background, border, image, text)) = parts.get_mut(part) else {
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
            TintSlot::Image => {
                if let Some(mut image) = image {
                    image.color = color;
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

/// MRK-03 / MRK-07: place, fade and colour every marker for this frame.
#[allow(clippy::too_many_arguments)]
fn update_markers(
    screen: Res<ScreenPositions>,
    edges: Res<GateEdgeTargets>,
    view: ViewNow,
    projections: Query<&Projection, With<Camera3d>>,
    bodies: MarkerBodies,
    mut markers: MarkerNodes,
    mut labels: LabelNodes,
    (children, mut parts): (Query<&Children>, TintedParts),
) {
    let projection = projections.single().ok();
    for (entity, marker_for, half, shape, mut node) in &mut markers {
        let body = marker_for.0;
        let Ok((child_of, fold, motion, radius)) = bodies.get(body) else {
            node.display = Display::None;
            continue;
        };

        let fold_anim_t = fold.map(|f| f.anim_t).unwrap_or(1.0);
        let alpha = marker_alpha(
            fold_anim_t,
            fold.map(|f| f.effective_hidden).unwrap_or(false),
            motion.map(MotionModeState::dot_alpha).unwrap_or(1.0),
        );
        let own = screen
            .get(body)
            .filter(|pos| screen.in_viewport(*pos))
            .filter(|_| alpha > MIN_VISIBLE_ALPHA && !edges.is_pinned(body));
        let Some(own) = own else {
            node.display = Display::None;
            continue;
        };
        let parent = child_of.and_then(|child_of| screen.get(child_of.parent()));
        let pos = marker_position(own, parent, fold_anim_t);

        let drawn = drawn_half_size(
            &view,
            projection,
            screen.viewport.y,
            (body, *shape == MarkerShape::Disc, half.0.x),
            radius,
        );
        node.display = Display::Flex;
        node.width = px(drawn * 2.0);
        node.height = px(drawn * 2.0);
        node.left = px(pos.x - drawn);
        node.top = px(pos.y - drawn);
        fade_parts(entity, alpha, &children, &mut parts);
        for (label_of, mut label) in &mut labels {
            if label_of.parent() == entity {
                label.left = px(drawn * 2.0 + LABEL_GAP_PX);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// MRK-01: hovered or selected markers carry the white ring.
    #[test]
    fn ring_on_hover_or_selection() {
        assert!(shows_ring(true, false));
        assert!(shows_ring(false, true));
        assert!(!shows_ring(false, false));
    }

    /// MRK-03 / SYM-04: marker size depends on class only; a symbol fits
    /// `ship_symbol_px`.
    #[test]
    fn marker_size_is_by_class() {
        assert!(
            marker_half_size(BodyClass::Star, 22.0) > marker_half_size(BodyClass::Planet, 22.0)
        );
        assert!(
            marker_half_size(BodyClass::Planet, 22.0) > marker_half_size(BodyClass::Moon, 22.0)
        );
        assert_eq!(marker_half_size(BodyClass::Ship, 22.0), 11.0);
        assert_eq!(marker_half_size(BodyClass::Station, 30.0), 15.0);
    }

    /// SYM-14: the ring and brackets enclose the symbol square.
    #[test]
    fn the_ring_encloses_the_symbol_square() {
        assert!(ring_outer_half(BodyClass::Ship, 22.0) > 11.0);
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
    /// only, so an image's or a label's node background stays clear.
    #[test]
    fn fading_leaves_other_colour_slots_alone() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let color = colors::OWN;
        let image = world
            .spawn((
                ImageNode::default(),
                BackgroundColor(Color::NONE),
                Tint::new(color, TintSlot::Image),
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
        assert_eq!(world.get::<ImageNode>(image).unwrap().color, faded);
        assert_eq!(world.get::<TextColor>(text).unwrap().0, faded);
        for part in [image, text] {
            assert_eq!(world.get::<BackgroundColor>(part).unwrap().0, Color::NONE);
        }
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

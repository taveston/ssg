//! The vector-display overlay: one fixed-size UI marker per body, placed on
//! its projected screen position each frame (MRK-03, MRK-04, MRK-06), with
//! its label beside it (SYM-13; where exactly is `labels.rs`, LBL-01).
//!
//! Natural bodies are body markers: outlined circles with a ground-colour
//! inside, ticked for the star and dotted for a planet (MRK-13). Close in
//! they grow into true-size discs, filled and ringed by a dark gap
//! (MRK-07, MRK-11). They are drawn nearer over farther (MRK-12); a
//! salvage site is an X and a gate on
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

use bevy::color::Mix;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::colors;
use super::countdown::Countdowns;
use super::edge::GateEdgeTargets;
use super::fold::SatelliteFoldState;
use super::fonts::Fonts;
use super::hover;
use super::layers;
use super::motion_mode::MotionModeState;
use super::screen::ScreenPositions;
use super::ship_groups::GroupFaces;
use super::symbol_draw::{
    AmplifierStyle, Tint, TintSlot, amplifier_gap, amplifier_reach, spawn_symbol_parts,
};
use super::symbol_shape::SymbolPart;
use super::symbols::{
    GroupFace, SymbolSettings, label_second_line, symbol_look, with_countdown, with_group,
};
use super::targets::MarkerFor;
use super::true_size::{DiscRadii, disc_half_px};
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{Affiliation, BodyClass, DisplayName};
use crate::sim::clock::GameClock;
use crate::sim::track::ShipRecord;
use crate::view::halo::TextHalo;
use crate::{Boot, Stage};

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
/// `labels.rs` (LBL-01).
#[derive(Component)]
pub struct MarkerLabel;

/// The markers, pills and ring marks have been laid out for this frame;
/// the labels are placed after (LBL-01).
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

/// Outline width of hollow markers.
const HOLLOW_BORDER_PX: f32 = 1.5;
/// Glyph markers are drawn a little larger than their box so the glyph's
/// own side bearings don't shrink it.
const GLYPH_SCALE: f32 = 1.8;
const LABEL_FONT_PX: f32 = 12.0;
const SECOND_LINE_FONT_PX: f32 = 10.0;

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
/// its rim) and the margin. A fold pill sits clear of it (GRP-05).
pub fn box_half(drawn_half: f32, tunables: &Tunables) -> f32 {
    drawn_half + tunables.bracket_margin_px
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
        fill_faction: tunables.symbol_fill_faction,
        fill_other: tunables.symbol_fill_other,
        stale_alpha: tunables.stale_symbol_alpha,
        condition_warn: tunables.condition_warn_fraction,
    }
}

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_markers.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    update_markers.in_set(MarkersPlaced),
                    layer_bodies_by_depth,
                    update_second_lines,
                )
                    .in_set(Stage::Overlay),
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
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    clock: Res<GameClock>,
    bodies: SpawnBodies,
    groups: GroupFaces,
) {
    for (body, class, affiliation, name, record) in &bodies {
        let marker = BodyLook {
            class: *class,
            affiliation: *affiliation,
            record,
            group: groups.face(body),
        };
        let assets = (&*fonts, &*tunables);
        let members = groups.member_classes(body);
        // SYM-13: a symbol always has a second line, empty while nothing
        // applies, so a countdown can appear on it later (PLNR-66).
        let label = MarkerText {
            name: &name.0,
            second_line: record.map(|r| {
                label_second_line(*affiliation, r, &members, clock.elapsed_days())
                    .unwrap_or_default()
            }),
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
    /// A group's count and task-force mark (SYM-09); `None` for a ship.
    pub group: Option<GroupFace>,
}

impl BodyLook<'_> {
    fn shape(&self) -> MarkerShape {
        marker_shape(self.class, self.record.is_some())
    }

    /// The marker's and label's colour: a symbol's by affiliation and
    /// command (SYM-02); any other body's the neutral white, whoever owns
    /// it, as the map doesn't show a body's ownership (MRK-01, MRK-10).
    pub fn color(&self, tunables: &Tunables) -> Color {
        match self.record {
            Some(record) => symbol_look(self.affiliation, record, symbol_settings(tunables)).color,
            None => colors::NEUTRAL,
        }
    }
}

/// What a marker's label says (SYM-13).
struct MarkerText<'a> {
    name: &'a str,
    second_line: Option<String>,
}

/// The fonts and tunables a marker is built from.
pub type MarkerAssets<'a> = (&'a Fonts, &'a Tunables);

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
    let (fonts, tunables) = assets;
    let color = look.color(tunables);
    match look.shape() {
        MarkerShape::Disc => {
            // MRK-13: an opaque ground-colour inside a neutral rim;
            // `update_markers` switches the fill when it grows (MRK-11).
            let fill = body_fill(false, tunables);
            marker.insert((BackgroundColor(fill), Tint::new(fill, TintSlot::Background)));
            marker.with_children(|parent| {
                spawn_body_detail(parent, look.class, color, tunables);
                parent.spawn((
                    BodyRim,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        border: UiRect::all(px(tunables.body_rim_px)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(color),
                    Tint::new(color, TintSlot::Border),
                    Pickable::IGNORE,
                ));
            });
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
                let symbol = with_group(
                    symbol_look(look.affiliation, record, symbol_settings(tunables)),
                    look.group,
                );
                let style = AmplifierStyle {
                    gap: amplifier_gap(tunables.bracket_margin_px, tunables.amplifier_gap_px),
                    track_mix: tunables.condition_track_mix,
                };
                // SYM-13: the label keeps clear of the amplifiers too.
                marker.insert(amplifier_reach(&symbol, half * 2.0, style.gap));
                marker.with_children(|parent| {
                    spawn_symbol_parts(parent, fonts, &symbol, half * 2.0, style);
                });
            }
        }
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

/// MRK-13: the class detail inside or round a body marker: radial ticks
/// outside the star's rim, a centre dot in a planet, nothing for a moon.
fn spawn_body_detail(
    parent: &mut ChildSpawnerCommands,
    class: BodyClass,
    color: Color,
    tunables: &Tunables,
) {
    let full = || Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: percent(100),
        height: percent(100),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let part = |node: Node| {
        (
            node,
            BackgroundColor(color),
            Tint::new(color, TintSlot::Background),
            Pickable::IGNORE,
        )
    };
    match class {
        BodyClass::Planet => {
            let dot = tunables.planet_dot_px;
            parent
                .spawn((BodyDetail, full(), Pickable::IGNORE))
                .with_children(|holder| {
                    holder.spawn(part(Node {
                        width: px(dot),
                        height: px(dot),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    }));
                });
        }
        BodyClass::Star => {
            let count = tunables.star_tick_count.max(1);
            let (tick, width) = (tunables.star_tick_px, tunables.body_rim_px);
            for i in 0..count {
                // Each tick hangs above the top of a full-size holder,
                // which is turned about the marker's centre.
                let angle = std::f32::consts::TAU * i as f32 / count as f32;
                parent
                    .spawn((
                        BodyDetail,
                        full(),
                        UiTransform::from_rotation(Rot2::radians(angle)),
                        Pickable::IGNORE,
                    ))
                    .with_children(|holder| {
                        holder.spawn(part(Node {
                            position_type: PositionType::Absolute,
                            top: px(-tick),
                            left: percent(50),
                            margin: UiRect::left(px(-width / 2.0)),
                            width: px(width),
                            height: px(tick),
                            ..default()
                        }));
                    });
            }
        }
        _ => {}
    }
}

fn spawn_marker(
    commands: &mut Commands,
    assets: MarkerAssets,
    body: Entity,
    look: BodyLook,
    text: MarkerText,
) {
    let (_, tunables) = assets;
    let half = marker_half_size(look.class, tunables);
    let shape = look.shape();
    let mut marker = commands.spawn((
        MarkerFor(body),
        MarkerHalfSize(Vec2::splat(half)),
        MarkerDrawn::default(),
        shape,
        marker_node(shape, half),
        // MRK-12: bodies are re-layered by depth each frame.
        GlobalZIndex(if shape == MarkerShape::Disc {
            layers::BODIES
        } else {
            layers::SYMBOLS
        }),
        // Hover and click go through the marker's hit area instead
        // (MRK-08, `marker_picking.rs`), so the node itself takes none.
        Pickable::IGNORE,
    ));
    if shape == MarkerShape::Disc {
        marker.insert(disc_gap(tunables));
    }
    spawn_shape(&mut marker, assets, look, half);
    let color = look.color(tunables);
    marker.with_children(|parent| spawn_label(parent, assets, (body, half, color), text));
}

/// SYM-13: the label beside the marker, in its colour: the name, and for
/// a symbol a second line in the secondary text colour. It starts to the
/// right; `labels.rs` places it from then on (LBL-01). It is a hover and
/// click target for its body, and wins over an orbit ring (HOV-02).
fn spawn_label(
    parent: &mut ChildSpawnerCommands,
    (fonts, tunables): MarkerAssets,
    (body, half, color): (Entity, f32, Color),
    text: MarkerText,
) {
    parent
        .spawn((
            MarkerLabel,
            MarkerFor(body),
            // MRK-12: labels sit above every marker, not just their own.
            GlobalZIndex(layers::LABELS),
            Node {
                position_type: PositionType::Absolute,
                left: px(half * 2.0 + tunables.bracket_margin_px + tunables.label_gap_px),
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
                TextHalo::ground(),
            ));
            if let Some(line) = text.second_line {
                label.spawn((
                    SecondLineFor(body),
                    Text::new(line),
                    TextLayout::linebreak(LineBreak::NoWrap),
                    fonts.data(SECOND_LINE_FONT_PX),
                    TextColor(colors::PANEL_BORDER),
                    Tint::new(colors::PANEL_BORDER, TintSlot::Text),
                    TextHalo::ground(),
                ));
            }
        })
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(hover::on_marker_click);
}

/// SYM-08 / SYM-13 / PLNR-66: keep each second line true as game time
/// passes, so a stale track's age counts up and an order's countdown
/// counts down.
fn update_second_lines(
    clock: Res<GameClock>,
    bodies: Query<(&Affiliation, &ShipRecord)>,
    groups: GroupFaces,
    countdowns: Countdowns,
    mut lines: Query<(&SecondLineFor, &mut Text)>,
) {
    for (line_for, mut text) in &mut lines {
        let Ok((affiliation, record)) = bodies.get(line_for.0) else {
            continue;
        };
        let members = groups.member_classes(line_for.0);
        let line = with_countdown(
            label_second_line(*affiliation, record, &members, clock.elapsed_days()),
            countdowns.of(line_for.0),
        )
        .unwrap_or_default();
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
    let (_, tunables) = assets;
    let half = match look.shape() {
        MarkerShape::Symbol => tunables.list_symbol_px / 2.0,
        _ => body_half,
    };
    spawn_icon(parent, assets, look, half);
}

/// A copy of a body's marker at `half` pixels, laid out in the flow of its
/// parent rather than placed on the map: a symbol, or a body marker with
/// its class detail (MRK-13). Lists (INS-05) and exposure-ring identity
/// marks (MOT-14) are built from it.
pub fn spawn_icon(
    parent: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    look: BodyLook,
    half: f32,
) {
    let mut node = marker_node(look.shape(), half);
    node.position_type = PositionType::Relative;
    node.display = Display::Flex;
    let mut icon = parent.spawn(node);
    spawn_shape(&mut icon, assets, look, half);
}

/// MRK-11: a natural body's ring of ground colour, hard against the
/// outside of its rim, so overlapping discs stay distinct.
pub fn disc_gap(tunables: &Tunables) -> Outline {
    Outline::new(px(tunables.body_gap_px), px(0), colors::GROUND)
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
    /// MRK-03: a marker's centre, moving towards its parent's as it folds;
    /// `None` while the body is off screen, faded out, or a gate pinned
    /// to the edge.
    fn centre(
        &self,
        body: Entity,
        parent: Option<Entity>,
        alpha: f32,
        fold_anim_t: f32,
    ) -> Option<Vec2> {
        let own = self
            .screen
            .get(body)
            .filter(|pos| self.screen.in_viewport(*pos))
            .filter(|_| alpha > MIN_VISIBLE_ALPHA && !self.edges.is_pinned(body))?;
        let parent = parent.and_then(|parent| self.screen.get(parent));
        Some(marker_position(own, parent, fold_anim_t))
    }
}

/// A marker's coloured parts and its class detail, and the tunables
/// they are coloured by.
#[derive(SystemParam)]
struct MarkerPaint<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parts: TintedParts<'w, 's>,
    details: DetailNodes<'w, 's>,
    tunables: Res<'w, Tunables>,
}

impl MarkerPaint<'_, '_> {
    /// Fades a marker's parts to `alpha`. A disc (`true_size` is `Some`)
    /// grown past its marker is first filled and loses its class detail
    /// (MRK-07, MRK-11, MRK-13).
    fn paint(&mut self, marker: Entity, true_size: Option<bool>, alpha: f32) {
        if let Some(true_size) = true_size {
            if let Ok((mut tint, ..)) = self.parts.get_mut(marker) {
                tint.color = body_fill(true_size, &self.tunables);
            }
            show_body_detail(marker, !true_size, &self.children, &mut self.details);
        }
        fade_parts(marker, alpha, &self.children, &mut self.parts);
    }
}

/// A marker's alpha and how far its fold has run, from its body's fold
/// and motion mode.
fn marker_fade(fold: Option<&SatelliteFoldState>, motion: Option<&MotionModeState>) -> (f32, f32) {
    let fold_anim_t = fold.map(|f| f.anim_t).unwrap_or(1.0);
    let alpha = marker_alpha(
        fold_anim_t,
        fold.map(|f| f.effective_hidden).unwrap_or(false),
        motion.map(MotionModeState::dot_alpha).unwrap_or(1.0),
    );
    (alpha, fold_anim_t)
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
        let Ok((child_of, fold, motion)) = bodies.get(body) else {
            node.display = Display::None;
            continue;
        };
        let (alpha, fold_anim_t) = marker_fade(fold, motion);
        let parent = child_of.map(ChildOf::parent);
        let Some(pos) = place.centre(body, parent, alpha, fold_anim_t) else {
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

    /// The class details spawned for a body marker of `class`.
    fn details_of(class: BodyClass) -> (usize, usize) {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world
            .run_system_once(move |mut commands: Commands| {
                commands.spawn(Node::default()).with_children(|parent| {
                    spawn_body_detail(parent, class, colors::NEUTRAL, &Tunables::default());
                });
            })
            .unwrap();
        let holders = world
            .query_filtered::<(), With<BodyDetail>>()
            .iter(&world)
            .count();
        let parts = world
            .query::<&Tint>()
            .iter(&world)
            .filter(|tint| tint.color == colors::NEUTRAL)
            .count();
        (holders, parts)
    }

    /// MRK-13: the star has `star_tick_count` ticks, a planet one centre
    /// dot, a moon nothing beyond its circle.
    #[test]
    fn body_markers_show_their_class_in_their_shape() {
        let ticks = Tunables::default().star_tick_count as usize;
        assert_eq!(details_of(BodyClass::Star), (ticks, ticks));
        assert_eq!(details_of(BodyClass::Planet), (1, 1));
        assert_eq!(details_of(BodyClass::Moon), (0, 0));
    }

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

    /// MRK-11: a disc carries its ground-coloured gap hard against its
    /// rim, whether or not it is hovered or selected.
    #[test]
    fn a_disc_is_ringed_by_a_ground_gap() {
        let tunables = Tunables::default();
        let gap = disc_gap(&tunables);
        assert_eq!(gap.color, colors::GROUND);
        assert_eq!(gap.width, px(tunables.body_gap_px));
        assert_eq!(gap.offset, px(0));
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

    /// MRK-10: the star, planets and moons are drawn in the neutral colour
    /// whoever owns them.
    #[test]
    fn natural_bodies_are_neutral_whoever_owns_them() {
        let tunables = Tunables::default();
        for class in [BodyClass::Star, BodyClass::Planet, BodyClass::Moon] {
            for affiliation in [Affiliation::Own, Affiliation::Hostile, Affiliation::Neutral] {
                let look = BodyLook {
                    class,
                    affiliation,
                    record: None,
                    group: None,
                };
                assert_eq!(look.color(&tunables), colors::NEUTRAL);
            }
        }
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
                    mask: crate::view::symbol_shape::SymbolMask::Triangle,
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

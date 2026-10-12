//! The vector-display overlay: one fixed-size UI marker per body, placed on
//! its projected screen position each frame (MRK-03, MRK-04, MRK-06).
//!
//! What's drawn is decided elsewhere - `fold.rs` says whether a satellite
//! is folded into its parent (GRP-01, GRP-03), `motion_mode.rs` whether a
//! body reads as a dot or as an exposure ring (MOT-02), and `edge.rs`
//! whether a gate is pinned to the screen edge instead (EDGE-01). This
//! module turns those into a shape, a position and a strength.
//!
//! MRK-06 holds by construction: only satellites fold, so every planet
//! keeps its own marker at every zoom.

use bevy::prelude::*;

use super::colors;
use super::edge::GateEdgeTargets;
use super::fold::SatelliteFoldState;
use super::fonts::Fonts;
use super::hover;
use super::marker_images::MarkerImages;
use super::motion_mode::MotionModeState;
use super::screen::ScreenPositions;
use super::selection::Selection;
use super::targets::{Hovered, MarkerFor};
use crate::sim::body::{BodyClass, DisplayName, Ownership};
use crate::{Boot, Stage};

/// Half-size in pixels, so the update system can centre the node on its
/// projected screen position without re-deriving size from class.
#[derive(Component)]
pub struct MarkerHalfSize(pub Vec2);

/// The marker's full-strength colour, so fading can be recomputed from it
/// each frame rather than accumulating rounding.
#[derive(Component)]
pub struct MarkerBaseColor(pub Color);

/// Where a marker's colour lives, so fading knows what to write to.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkerStyle {
    /// Filled shape: colour on `BackgroundColor`.
    Fill,
    /// Hollow square or circle: colour on `BorderColor`.
    Border,
    /// Text glyph: colour on the child `TextColor`.
    Glyph,
    /// Generated shape image: colour on the `ImageNode`'s tint.
    Image,
}

/// MRK-04: the shape a body's marker takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkerShape {
    Disc,
    HollowSquare,
    HollowCircle,
    /// Filled triangle pointing up, from `MarkerImages`.
    FilledTriangle,
    /// Filled diamond, from `MarkerImages`.
    FilledDiamond,
    Glyph(&'static str),
}

impl MarkerShape {
    fn style(self) -> MarkerStyle {
        match self {
            MarkerShape::Disc => MarkerStyle::Fill,
            MarkerShape::HollowSquare | MarkerShape::HollowCircle => MarkerStyle::Border,
            MarkerShape::FilledTriangle | MarkerShape::FilledDiamond => MarkerStyle::Image,
            MarkerShape::Glyph(_) => MarkerStyle::Glyph,
        }
    }
}

/// Below this combined strength a marker is simply not drawn - which also
/// takes it out of picking, so a folded satellite can't be clicked.
const MIN_VISIBLE_ALPHA: f32 = 0.02;
/// Outline width of hollow markers.
const HOLLOW_BORDER_PX: f32 = 1.5;
/// Gap between the hover ring (an outline) and the marker it rings.
const HOVER_RING_OFFSET_PX: f32 = 2.0;
/// Glyph markers are drawn a little larger than their box so the glyph's
/// own side bearings don't shrink it.
const GLYPH_SCALE: f32 = 1.8;
/// Gap between a marker and its label, which sits to its right.
const LABEL_GAP_PX: f32 = 4.0;
const LABEL_FONT_PX: f32 = 12.0;

/// MRK-03: marker size is a presentation choice by class, in screen pixels,
/// never derived from physical size.
/// How far the white ring reaches from a marker's centre (MRK-01), so
/// anything drawn round a marker (CAM-22's brackets) can clear it.
pub fn ring_outer_half(class: BodyClass) -> f32 {
    marker_half_size(class).max_element() + HOVER_RING_OFFSET_PX + RING_PX
}

fn marker_half_size(class: BodyClass) -> Vec2 {
    match class {
        BodyClass::Star => Vec2::splat(9.0),
        BodyClass::Planet => Vec2::splat(6.0),
        BodyClass::Moon => Vec2::splat(3.0),
        BodyClass::Station => Vec2::splat(5.0),
        BodyClass::Gate => Vec2::splat(5.5),
        BodyClass::SalvageSite => Vec2::splat(7.0),
        BodyClass::Ship => Vec2::splat(6.0),
    }
}

/// MRK-04: filled disc = body; hollow square = station; triangle = own
/// ship; diamond = foreign contact; X = salvage site. A gate on screen is a
/// hollow circle (its edge arrow is EDGE-01). The filled triangle and
/// diamond are generated images.
pub fn marker_shape(class: BodyClass, ownership: Ownership) -> MarkerShape {
    match (class, ownership) {
        (BodyClass::Star | BodyClass::Planet | BodyClass::Moon, _) => MarkerShape::Disc,
        (BodyClass::Station, _) => MarkerShape::HollowSquare,
        (BodyClass::Gate, _) => MarkerShape::HollowCircle,
        (BodyClass::SalvageSite, _) => MarkerShape::Glyph("X"),
        (BodyClass::Ship, Ownership::Foreign) => MarkerShape::FilledDiamond,
        (BodyClass::Ship, _) => MarkerShape::FilledTriangle,
    }
}

/// GRP-01 / GRP-03 / MOT-02: how strongly a body's marker is drawn. A fully
/// folded satellite draws nothing; a sliding one fades with the slide; and
/// in exposure-ring mode the ring carries the body, so the dot fades out.
pub fn marker_alpha(fold_anim_t: f32, fully_hidden: bool, dot_strength: f32) -> f32 {
    if fully_hidden {
        0.0
    } else {
        fold_anim_t * dot_strength
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

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_markers.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (update_markers, ring_markers).in_set(Stage::Overlay),
            );
    }
}

fn spawn_markers(
    mut commands: Commands,
    fonts: Res<Fonts>,
    images: Res<MarkerImages>,
    bodies: Query<(Entity, &BodyClass, &Ownership, &DisplayName)>,
) {
    for (entity, class, ownership, name) in &bodies {
        let look = (*class, *ownership);
        spawn_marker(&mut commands, &fonts, &images, entity, look, name);
    }
}

/// MRK-03 / MRK-04: the marker's box, sized by class and shaped by
/// rounding and borders.
fn marker_node(shape: MarkerShape, half: Vec2) -> Node {
    let mut node = Node {
        position_type: PositionType::Absolute,
        width: px(half.x * 2.0),
        height: px(half.y * 2.0),
        display: Display::None,
        ..default()
    };
    if matches!(shape, MarkerShape::HollowSquare | MarkerShape::HollowCircle) {
        node.border = UiRect::all(px(HOLLOW_BORDER_PX));
    }
    if matches!(shape, MarkerShape::Disc | MarkerShape::HollowCircle) {
        node.border_radius = BorderRadius::MAX;
    }
    node
}

fn spawn_marker(
    commands: &mut Commands,
    fonts: &Fonts,
    images: &MarkerImages,
    body: Entity,
    (class, ownership): (BodyClass, Ownership),
    name: &DisplayName,
) {
    let half = marker_half_size(class);
    let color = colors::ownership_color(ownership);
    let shape = marker_shape(class, ownership);
    let style = shape.style();

    let mut marker = commands.spawn((
        MarkerFor(body),
        MarkerHalfSize(half),
        MarkerBaseColor(color),
        style,
        marker_node(shape, half),
        // The hover ring (HOV-01): an outline, invisible until hovered.
        Outline {
            width: px(0),
            offset: px(HOVER_RING_OFFSET_PX),
            color: Color::NONE,
        },
    ));
    match style {
        MarkerStyle::Fill => {
            marker.insert(BackgroundColor(color));
        }
        MarkerStyle::Border => {
            marker.insert((BackgroundColor(Color::NONE), BorderColor::all(color)));
        }
        MarkerStyle::Image => {
            let image = match shape {
                MarkerShape::FilledTriangle => images.triangle.clone(),
                _ => images.diamond.clone(),
            };
            marker.insert(ImageNode::new(image).with_color(color));
        }
        MarkerStyle::Glyph => {}
    }
    marker
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(hover::on_marker_click)
        .with_children(|parent| spawn_marker_text(parent, fonts, shape, half, color, name));
}

/// A glyph marker's glyph, and every marker's label just right of it. Both
/// are child text, coloured with the marker.
fn spawn_marker_text(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    shape: MarkerShape,
    half: Vec2,
    color: Color,
    name: &DisplayName,
) {
    if let MarkerShape::Glyph(glyph) = shape {
        parent.spawn((
            Text::new(glyph),
            fonts.label(half.x * GLYPH_SCALE),
            TextColor(color),
        ));
    }
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(half.x * 2.0 + LABEL_GAP_PX),
            top: px(0),
            ..default()
        },
        Text::new(name.0.clone()),
        fonts.label(LABEL_FONT_PX),
        TextColor(color),
    ));
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
        &'static MarkerFor,
        &'static MarkerHalfSize,
        &'static MarkerBaseColor,
        &'static MarkerStyle,
        &'static mut Node,
        Option<&'static mut BackgroundColor>,
        Option<&'static mut BorderColor>,
        Option<&'static mut ImageNode>,
        Option<&'static Children>,
    ),
>;

/// INS-05: a small copy of a body's marker, for lists - the same shape
/// and colour as on the map, at `half` pixels.
pub fn spawn_marker_icon(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    images: &MarkerImages,
    (class, ownership): (BodyClass, Ownership),
    half: f32,
) {
    let shape = marker_shape(class, ownership);
    let color = colors::ownership_color(ownership);
    let mut node = marker_node(shape, Vec2::splat(half));
    node.position_type = PositionType::Relative;
    node.display = Display::Flex;
    let mut icon = parent.spawn(node);
    match shape {
        MarkerShape::Disc => {
            icon.insert(BackgroundColor(color));
        }
        MarkerShape::HollowSquare | MarkerShape::HollowCircle => {
            icon.insert(BorderColor::all(color));
        }
        MarkerShape::FilledTriangle => {
            icon.insert(ImageNode::new(images.triangle.clone()).with_color(color));
        }
        MarkerShape::FilledDiamond => {
            icon.insert(ImageNode::new(images.diamond.clone()).with_color(color));
        }
        MarkerShape::Glyph(glyph) => {
            icon.with_children(|icon| {
                icon.spawn((
                    Text::new(glyph),
                    fonts.label(half * GLYPH_SCALE),
                    TextColor(color),
                ));
            });
        }
    }
}

/// MRK-01: the white ring - on a marker that is hovered (HOV-01) or
/// selected, pinned or not (INS-01, CAM-22).
pub fn shows_ring(hovered: bool, selected: bool) -> bool {
    hovered || selected
}

/// Width of the white hover and selection ring.
const RING_PX: f32 = 2.0;

/// MRK-01: draw the white ring on hovered and selected markers.
fn ring_markers(
    selection: Res<Selection>,
    mut markers: Query<(&MarkerFor, Has<Hovered>, &mut Outline), With<MarkerHalfSize>>,
) {
    for (marker_for, hovered, mut outline) in &mut markers {
        let ring = shows_ring(hovered, selection.is_open(marker_for.0));
        outline.width = px(if ring { RING_PX } else { 0.0 });
        outline.color = if ring {
            colors::SELECTION_RING
        } else {
            Color::NONE
        };
    }
}

/// MRK-03: place, fade and colour every marker for this frame.
fn update_markers(
    screen: Res<ScreenPositions>,
    edges: Res<GateEdgeTargets>,
    bodies: MarkerBodies,
    mut markers: MarkerNodes,
    mut text_colors: Query<&mut TextColor>,
) {
    for (marker_for, half, base_color, style, mut node, background, border, image, children) in
        &mut markers
    {
        let body = marker_for.0;
        let Ok((child_of, fold, motion)) = bodies.get(body) else {
            node.display = Display::None;
            continue;
        };

        let fold_anim_t = fold.map(|f| f.anim_t).unwrap_or(1.0);
        let alpha = marker_alpha(
            fold_anim_t,
            fold.map(|f| f.effective_hidden).unwrap_or(false),
            motion.map(MotionModeState::dot_strength).unwrap_or(1.0),
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

        node.display = Display::Flex;
        node.left = px(pos.x - half.0.x);
        node.top = px(pos.y - half.0.y);

        let color = base_color.0.with_alpha(alpha);
        match (style, background, border, image) {
            (MarkerStyle::Fill, Some(mut background), _, _) => background.0 = color,
            (MarkerStyle::Border, _, Some(mut border), _) => {
                border.set_all(color);
            }
            (MarkerStyle::Image, _, _, Some(mut image)) => image.color = color,
            _ => {}
        }
        // Labels (and glyph markers) carry their colour on child text.
        for child in children.into_iter().flat_map(|children| children.iter()) {
            if let Ok(mut text_color) = text_colors.get_mut(child) {
                text_color.0 = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MRK-04: each class gets its own shape, and ships are told apart by
    /// ownership.
    #[test]
    fn marker_shapes_by_class() {
        use BodyClass::*;
        use Ownership::*;
        assert_eq!(marker_shape(Planet, Own), MarkerShape::Disc);
        assert_eq!(marker_shape(Moon, Neutral), MarkerShape::Disc);
        assert_eq!(marker_shape(Star, Neutral), MarkerShape::Disc);
        assert_eq!(marker_shape(Station, Own), MarkerShape::HollowSquare);
        assert_eq!(marker_shape(SalvageSite, Neutral), MarkerShape::Glyph("X"));
        assert_eq!(marker_shape(Ship, Own), MarkerShape::FilledTriangle);
        assert_eq!(marker_shape(Ship, Foreign), MarkerShape::FilledDiamond);
        assert_eq!(MarkerShape::FilledTriangle.style(), MarkerStyle::Image);
    }

    /// MRK-01: hovered or selected markers carry the white ring.
    #[test]
    fn ring_on_hover_or_selection() {
        assert!(shows_ring(true, false));
        assert!(shows_ring(false, true));
        assert!(!shows_ring(false, false));
    }

    /// MRK-03: marker size depends on class only.
    #[test]
    fn marker_size_is_by_class() {
        assert!(marker_half_size(BodyClass::Star).x > marker_half_size(BodyClass::Planet).x);
        assert!(marker_half_size(BodyClass::Planet).x > marker_half_size(BodyClass::Moon).x);
    }

    /// GRP-01: a fully folded satellite draws nothing, whatever else says.
    #[test]
    fn folded_satellite_marker_is_invisible() {
        assert_eq!(marker_alpha(1.0, true, 1.0), 0.0);
        assert_eq!(marker_alpha(0.0, false, 1.0), 0.0);
        assert_eq!(marker_alpha(1.0, false, 1.0), 1.0);
    }

    /// GRP-03 / MOT-02: the marker fades with the fold slide and with the
    /// dot-to-ring cross-fade.
    #[test]
    fn ring_mode_fades_the_dot() {
        assert_eq!(marker_alpha(1.0, false, 0.0), 0.0);
        assert!((marker_alpha(0.5, false, 0.5) - 0.25).abs() < 1e-6);
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

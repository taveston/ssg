//! The vector-display overlay: one fixed-size UI marker per body,
//! projected from its true-scale 3D position each frame.
//!
//! What's drawn is decided elsewhere - `grouping.rs` says whether a
//! satellite is folded into its parent, and `motion.rs` says whether a body
//! reads as a dot or as an exposure ring. This module just places and
//! styles the nodes, carries the parent's "+N" count pill, and keeps
//! off-screen gates pinned to the viewport edge.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::Stage;
use crate::colors;
use crate::fonts::Fonts;
use crate::grouping::{FoldSummary, SatelliteFoldState};
use crate::hover_inspect;
use crate::motion::MotionModeState;
use crate::system_data::{BodyClass, DisplayName, Ownership};

/// Marker UI node -> the body entity it represents.
#[derive(Component)]
pub struct MarkerFor(pub Entity);

/// Half-size in pixels, so the update system can centre the node on its
/// projected screen position without re-deriving size from class.
#[derive(Component)]
pub struct MarkerHalfSize(pub Vec2);

/// The marker's full-strength colour, so fading can be recomputed from it
/// each frame rather than accumulating rounding.
#[derive(Component)]
pub struct MarkerBaseColor(pub Color);

/// Where a marker's colour lives, so fading knows what to write to.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum MarkerStyle {
    /// Filled disc: colour on `BackgroundColor`.
    Fill,
    /// Hollow square or circle: colour on `BorderColor`.
    Border,
    /// Text glyph: colour on the child `TextColor`.
    Glyph,
}

/// The count pill for a body's folded satellites.
#[derive(Component)]
pub struct GroupPillFor(pub Entity);

#[derive(Component)]
pub struct PillCountText;

/// The small blue dot on a pill, shown when own ships are parked among the
/// folded satellites.
#[derive(Component)]
pub struct PillOwnShipDot;

/// Edge-of-screen arrowhead for a gate that's off-screen, rotated each
/// frame to point toward it.
#[derive(Component)]
pub struct EdgeIndicatorFor(pub Entity);

/// The label next to an edge indicator - a sibling, not a child, so it
/// doesn't rotate along with the arrowhead.
#[derive(Component)]
pub struct EdgeLabelFor(pub Entity);

const EDGE_MARGIN_PX: f32 = 28.0;
/// Below this combined strength a marker is simply not drawn - which also
/// takes it out of picking, so a folded satellite can't be clicked.
const MIN_VISIBLE_ALPHA: f32 = 0.02;

/// EDGE-01: where a gate's arrow sits when the gate itself is off-screen.
///
/// The arrow is pinned to the viewport edge, `margin_px` inside it, on the
/// line from the viewport centre towards the gate's projected position -
/// which is what makes it read as "that way". `None` means the gate is
/// comfortably on screen and gets its ordinary marker instead.
pub fn clamp_to_viewport_edge(pos: Vec2, viewport: Vec2, margin_px: f32) -> Option<Vec2> {
    let centre = viewport / 2.0;
    let half_extent = (centre - Vec2::splat(margin_px)).max(Vec2::ZERO);
    let delta = pos - centre;

    // How far along `delta` the first edge is crossed. A component of zero
    // never crosses its edge, so it puts no limit on the scale.
    let axis_scale = |delta: f32, extent: f32| {
        if delta.abs() > f32::EPSILON {
            extent / delta.abs()
        } else {
            f32::INFINITY
        }
    };
    let scale = axis_scale(delta.x, half_extent.x).min(axis_scale(delta.y, half_extent.y));
    // At or inside the margin: not an edge case at all.
    if scale >= 1.0 {
        return None;
    }
    Some(centre + delta * scale.max(0.0))
}

/// EDGE-01: the rotation that turns the arrowhead glyph - which points up
/// when unrotated - towards the gate.
pub fn edge_arrow_angle(edge_pos: Vec2, viewport: Vec2) -> f32 {
    let dir = edge_pos - viewport / 2.0;
    // Screen y grows downwards, so "up" is -y.
    dir.x.atan2(-dir.y)
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

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_markers.after(crate::scene::spawn_system))
            .add_systems(Update, update_markers.in_set(Stage::Markers));
    }
}

fn spawn_markers(
    mut commands: Commands,
    fonts: Res<Fonts>,
    bodies: Query<(Entity, &BodyClass, &Ownership, &DisplayName)>,
    children: Query<&ChildOf>,
) {
    for (entity, class, ownership, name) in &bodies {
        let half = marker_half_size(*class);
        let color = colors::ownership_color(*ownership);

        let base_node = Node {
            position_type: PositionType::Absolute,
            width: px(half.x * 2.0),
            height: px(half.y * 2.0),
            display: Display::None,
            ..default()
        };
        let no_outline = Outline {
            width: px(0),
            offset: px(2),
            color: Color::NONE,
        };

        let (marker_entity, style) = match class {
            BodyClass::Star | BodyClass::Planet | BodyClass::Moon => (
                commands
                    .spawn((
                        MarkerFor(entity),
                        MarkerHalfSize(half),
                        MarkerBaseColor(color),
                        MarkerStyle::Fill,
                        Node {
                            border_radius: BorderRadius::MAX,
                            ..base_node.clone()
                        },
                        BackgroundColor(color),
                        no_outline,
                    ))
                    .id(),
                MarkerStyle::Fill,
            ),
            BodyClass::Station => (
                commands
                    .spawn((
                        MarkerFor(entity),
                        MarkerHalfSize(half),
                        MarkerBaseColor(color),
                        MarkerStyle::Border,
                        Node {
                            border: UiRect::all(px(1.5)),
                            ..base_node.clone()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(color),
                        no_outline,
                    ))
                    .id(),
                MarkerStyle::Border,
            ),
            BodyClass::Gate => (
                commands
                    .spawn((
                        MarkerFor(entity),
                        MarkerHalfSize(half),
                        MarkerBaseColor(color),
                        MarkerStyle::Border,
                        Node {
                            border: UiRect::all(px(1.5)),
                            border_radius: BorderRadius::MAX,
                            ..base_node.clone()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(color),
                        no_outline,
                    ))
                    .id(),
                MarkerStyle::Border,
            ),
            BodyClass::SalvageSite | BodyClass::Ship => {
                // Drawn as a glyph: X for a site, and per the sketches'
                // marker language, a triangle for an own ship and a
                // diamond for a foreign contact.
                let glyph = match (class, ownership) {
                    (BodyClass::SalvageSite, _) => "X",
                    (_, Ownership::Foreign) => "<>",
                    _ => "^",
                };
                let marker_entity = commands
                    .spawn((
                        MarkerFor(entity),
                        MarkerHalfSize(half),
                        MarkerBaseColor(color),
                        MarkerStyle::Glyph,
                        base_node.clone(),
                        no_outline,
                    ))
                    .id();
                commands.entity(marker_entity).with_children(|parent| {
                    parent.spawn((
                        Text::new(glyph),
                        fonts.label(half.x * 1.8),
                        TextColor(color),
                    ));
                });
                (marker_entity, MarkerStyle::Glyph)
            }
        };
        let _ = style;

        commands
            .entity(marker_entity)
            .observe(hover_inspect::on_marker_over)
            .observe(hover_inspect::on_marker_out)
            .observe(hover_inspect::on_marker_click);

        // Label, positioned just right of the marker.
        commands.entity(marker_entity).with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(half.x * 2.0 + 4.0),
                    top: px(0),
                    ..default()
                },
                Text::new(name.0.clone()),
                fonts.label(12.0),
                TextColor(color),
            ));
        });

        if matches!(class, BodyClass::Gate) {
            commands
                .spawn((
                    EdgeIndicatorFor(entity),
                    MarkerHalfSize(half),
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(half.x * 2.0),
                        height: px(half.y * 2.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        display: Display::None,
                        ..default()
                    },
                    UiTransform::IDENTITY,
                ))
                .with_children(|parent| {
                    // Points up by default; rotated per-frame to point
                    // toward the gate.
                    parent.spawn((Text::new("^"), fonts.label(half.x * 2.2), TextColor(color)));
                });

            commands.spawn((
                EdgeLabelFor(entity),
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
                Text::new(name.0.clone()),
                fonts.label(12.0),
                TextColor(color),
            ));
        }
    }

    // A count pill for every body that has satellites - which includes
    // moons with moons of their own, not just planets.
    let mut parents: Vec<Entity> = children.iter().map(ChildOf::parent).collect();
    parents.sort();
    parents.dedup();
    for parent in parents {
        commands
            .spawn((
                GroupPillFor(parent),
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    align_items: AlignItems::Center,
                    column_gap: px(4),
                    padding: UiRect::axes(px(5), px(1)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.07, 0.09, 0.11, 0.9)),
                BorderColor::all(Color::NONE),
                children![
                    (
                        PillCountText,
                        Text::new(""),
                        fonts.mono(11.0),
                        TextColor(colors::NEUTRAL)
                    ),
                    (
                        PillOwnShipDot,
                        Node {
                            width: px(5),
                            height: px(5),
                            display: Display::None,
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(colors::OWN),
                    )
                ],
            ))
            .observe(hover_inspect::on_pill_over)
            .observe(hover_inspect::on_pill_out)
            .observe(hover_inspect::on_pill_click);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_markers(
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    bodies: Query<(
        Entity,
        &GlobalTransform,
        &BodyClass,
        Option<&ChildOf>,
        Option<&SatelliteFoldState>,
        Option<&MotionModeState>,
    )>,
    folds: Res<FoldSummary>,
    // These five all write `Node`, and Bevy needs each pair proven
    // disjoint by an explicit `Without` - requiring a distinct marker
    // component isn't enough on its own.
    mut markers: Query<
        (
            &MarkerFor,
            &MarkerHalfSize,
            &MarkerBaseColor,
            &MarkerStyle,
            &mut Node,
            Option<&mut BackgroundColor>,
            Option<&mut BorderColor>,
            Option<&Children>,
        ),
        (
            Without<GroupPillFor>,
            Without<PillOwnShipDot>,
            Without<EdgeIndicatorFor>,
            Without<EdgeLabelFor>,
        ),
    >,
    mut pills: Query<
        (&GroupPillFor, &mut Node, &mut BorderColor, &Children),
        (
            Without<MarkerFor>,
            Without<PillOwnShipDot>,
            Without<EdgeIndicatorFor>,
            Without<EdgeLabelFor>,
        ),
    >,
    mut pill_texts: Query<&mut Text, With<PillCountText>>,
    mut pill_dots: Query<
        &mut Node,
        (
            With<PillOwnShipDot>,
            Without<MarkerFor>,
            Without<GroupPillFor>,
            Without<EdgeIndicatorFor>,
            Without<EdgeLabelFor>,
        ),
    >,
    mut text_colors: Query<&mut TextColor>,
    mut edge_indicators: Query<
        (
            &EdgeIndicatorFor,
            &MarkerHalfSize,
            &mut Node,
            &mut UiTransform,
        ),
        (
            Without<MarkerFor>,
            Without<GroupPillFor>,
            Without<PillOwnShipDot>,
            Without<EdgeLabelFor>,
        ),
    >,
    mut edge_labels: Query<
        (&EdgeLabelFor, &mut Node),
        (
            Without<MarkerFor>,
            Without<GroupPillFor>,
            Without<PillOwnShipDot>,
            Without<EdgeIndicatorFor>,
        ),
    >,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Vec2::new(window.width(), window.height());

    let mut screen_pos: HashMap<Entity, Vec2> = HashMap::new();
    for (entity, transform, _, _, _, _) in &bodies {
        if let Ok(pos) = camera.world_to_viewport(camera_transform, transform.translation()) {
            screen_pos.insert(entity, pos);
        }
    }

    let in_viewport =
        |pos: Vec2| pos.x >= 0.0 && pos.y >= 0.0 && pos.x <= viewport.x && pos.y <= viewport.y;

    // EDGE-01: gates that fall outside a small margin get clamped to the
    // viewport edge instead of being hidden outright.
    let mut gate_edge_targets: HashMap<Entity, Vec2> = HashMap::new();
    for (entity, _, class, _, _, _) in &bodies {
        if *class != BodyClass::Gate {
            continue;
        }
        let Some(&pos) = screen_pos.get(&entity) else {
            continue;
        };
        if let Some(edge) = clamp_to_viewport_edge(pos, viewport, EDGE_MARGIN_PX) {
            gate_edge_targets.insert(entity, edge);
        }
    }

    for (marker_for, half, base_color, style, mut node, background, border, children) in
        &mut markers
    {
        let body = marker_for.0;
        let Ok((_, _, _, child_of, fold, motion)) = bodies.get(body) else {
            node.display = Display::None;
            continue;
        };

        // A folded satellite draws nothing; a body in exposure-ring mode
        // has no dot, because the ring is carrying it instead.
        let fold_alpha = fold.map(|f| f.anim_t).unwrap_or(1.0);
        let fully_hidden = fold.map(|f| f.effective_hidden).unwrap_or(false);
        let dot_strength = motion.map(MotionModeState::dot_strength).unwrap_or(1.0);
        let alpha = if fully_hidden {
            0.0
        } else {
            fold_alpha * dot_strength
        };

        let on_screen = screen_pos.get(&body).copied();
        let visible = alpha > MIN_VISIBLE_ALPHA
            && !gate_edge_targets.contains_key(&body)
            && on_screen.is_some_and(in_viewport);
        if !visible {
            node.display = Display::None;
            continue;
        }
        let mut pos = on_screen.unwrap_or_default();

        // While folding or unfolding, slide between the parent's marker and
        // the satellite's true position.
        if let (Some(child_of), Some(fold)) = (child_of, fold)
            && fold.anim_t < 1.0
            && let Some(&parent_pos) = screen_pos.get(&child_of.parent())
        {
            pos = parent_pos.lerp(pos, fold.anim_t);
        }

        node.display = Display::Flex;
        node.left = px(pos.x - half.0.x);
        node.top = px(pos.y - half.0.y);

        let color = base_color.0.with_alpha(alpha);
        match style {
            MarkerStyle::Fill => {
                if let Some(mut background) = background {
                    background.0 = color;
                }
            }
            MarkerStyle::Border => {
                if let Some(mut border) = border {
                    border.set_all(color);
                }
            }
            MarkerStyle::Glyph => {}
        }
        // Labels (and glyph markers) carry their colour on child text.
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(mut text_color) = text_colors.get_mut(child) {
                    text_color.0 = base_color.0.with_alpha(alpha);
                }
            }
        }
    }

    for (pill_for, mut node, mut border, children) in &mut pills {
        let parent = pill_for.0;
        let Some(group) = folds.0.get(&parent) else {
            node.display = Display::None;
            continue;
        };
        let Some(&pos) = screen_pos.get(&parent) else {
            node.display = Display::None;
            continue;
        };
        if group.members.is_empty() || !in_viewport(pos) {
            node.display = Display::None;
            continue;
        }

        node.display = Display::Flex;
        node.left = px(pos.x + 10.0);
        node.top = px(pos.y + 8.0);
        // Orange outline when a foreign contact is among the folded.
        border.set_all(if group.has_foreign {
            colors::FOREIGN
        } else {
            Color::NONE
        });

        let mut children_iter = children.iter();
        if let Some(text_entity) = children_iter.next()
            && let Ok(mut text) = pill_texts.get_mut(text_entity)
        {
            *text = Text::new(format!("+{}", group.members.len()));
        }
        // Blue dot when own ships are parked there.
        if let Some(dot_entity) = children_iter.next()
            && let Ok(mut dot) = pill_dots.get_mut(dot_entity)
        {
            dot.display = if group.has_own_ship {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    for (edge_for, half, mut node, mut transform) in &mut edge_indicators {
        let Some(&pos) = gate_edge_targets.get(&edge_for.0) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x - half.0.x);
        node.top = px(pos.y - half.0.y);
        // The glyph points up by default; rotate it (clockwise) to point
        // from the viewport centre toward the gate's true direction.
        transform.rotation = Rot2::radians(edge_arrow_angle(pos, viewport));
    }

    for (label_for, mut node) in &mut edge_labels {
        let Some(&pos) = gate_edge_targets.get(&label_for.0) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x + 10.0);
        node.top = px(pos.y + 10.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEWPORT: Vec2 = Vec2::new(1600.0, 900.0);
    const MARGIN: f32 = 28.0;

    /// EDGE-01: a gate comfortably on screen is not an edge indicator.
    #[test]
    fn edge_01_on_screen_gates_are_not_clamped() {
        assert_eq!(
            clamp_to_viewport_edge(Vec2::new(800.0, 450.0), VIEWPORT, MARGIN),
            None
        );
        assert_eq!(
            clamp_to_viewport_edge(Vec2::new(100.0, 100.0), VIEWPORT, MARGIN),
            None
        );
    }

    /// EDGE-01: an off-screen gate is pinned to the edge, inside the
    /// margin, in its own direction.
    #[test]
    fn edge_01_off_screen_gates_are_pinned_inside_the_margin() {
        for target in [
            Vec2::new(4000.0, 450.0),
            Vec2::new(-4000.0, 450.0),
            Vec2::new(800.0, -3000.0),
            Vec2::new(800.0, 3000.0),
            Vec2::new(9000.0, -9000.0),
        ] {
            let clamped = clamp_to_viewport_edge(target, VIEWPORT, MARGIN)
                .unwrap_or_else(|| panic!("{target} is off screen and needs an arrow"));
            assert!(
                clamped.x >= MARGIN - 0.01
                    && clamped.x <= VIEWPORT.x - MARGIN + 0.01
                    && clamped.y >= MARGIN - 0.01
                    && clamped.y <= VIEWPORT.y - MARGIN + 0.01,
                "{target} clamped to {clamped}, outside the margin"
            );
            // Still on the ray from the centre towards the gate.
            let centre = VIEWPORT / 2.0;
            let to_gate = (target - centre).normalize();
            let to_arrow = (clamped - centre).normalize();
            assert!(
                to_gate.dot(to_arrow) > 0.999,
                "{target}: arrow at {clamped} is not in the gate's direction"
            );
        }
    }

    /// EDGE-01: the clamp touches exactly one edge - it slides along the
    /// nearer one rather than cutting the corner.
    #[test]
    fn edge_01_clamp_lands_on_the_first_edge_crossed() {
        // Far right, slightly up: the right edge binds.
        let clamped = clamp_to_viewport_edge(Vec2::new(5000.0, 400.0), VIEWPORT, MARGIN).unwrap();
        assert!((clamped.x - (VIEWPORT.x - MARGIN)).abs() < 0.01);
        // Far up, slightly right: the top edge binds.
        let clamped = clamp_to_viewport_edge(Vec2::new(850.0, -5000.0), VIEWPORT, MARGIN).unwrap();
        assert!((clamped.y - MARGIN).abs() < 0.01);
    }

    /// EDGE-01: a gate straight above or beside the centre is a degenerate
    /// direction on one axis and must not divide by zero.
    #[test]
    fn edge_01_axis_aligned_gates_clamp_cleanly() {
        let centre = VIEWPORT / 2.0;
        let above = clamp_to_viewport_edge(Vec2::new(centre.x, -2000.0), VIEWPORT, MARGIN).unwrap();
        assert!((above.x - centre.x).abs() < 0.01);
        assert!((above.y - MARGIN).abs() < 0.01);
        let beside =
            clamp_to_viewport_edge(Vec2::new(-2000.0, centre.y), VIEWPORT, MARGIN).unwrap();
        assert!((beside.y - centre.y).abs() < 0.01);
        assert!((beside.x - MARGIN).abs() < 0.01);
    }

    /// EDGE-01: the arrow points away from the centre, towards the gate -
    /// 0 is up, and the angle turns clockwise with the glyph.
    #[test]
    fn edge_01_arrow_points_towards_the_gate() {
        let centre = VIEWPORT / 2.0;
        let up = edge_arrow_angle(Vec2::new(centre.x, MARGIN), VIEWPORT);
        assert!(up.abs() < 1e-6);
        let right = edge_arrow_angle(Vec2::new(VIEWPORT.x - MARGIN, centre.y), VIEWPORT);
        assert!((right - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        let down = edge_arrow_angle(Vec2::new(centre.x, VIEWPORT.y - MARGIN), VIEWPORT);
        assert!((down.abs() - std::f32::consts::PI).abs() < 1e-6);
        let left = edge_arrow_angle(Vec2::new(MARGIN, centre.y), VIEWPORT);
        assert!((left + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }
}

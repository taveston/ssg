//! Gate edge indicators (EDGE-01): a gate outside the viewport shows as an
//! arrow pinned to the screen edge in its direction, with its label.
//!
//! `GateEdgeTargets` says which gates are pinned this frame and where; the
//! marker overlay reads it to hide those gates' ordinary markers.

use std::collections::HashMap;

use bevy::prelude::*;

use super::colors;
use super::fonts::Fonts;
use super::screen::ScreenPositions;
use super::topbar::TOP_BAR_HEIGHT_PX;
use super::world::ViewNow;
use crate::sim::body::{BodyClass, DisplayName, Ownership};
use crate::{Boot, Stage};

/// How far inside the viewport edge a pinned arrow sits.
const EDGE_MARGIN_PX: f32 = 28.0;
/// Half the arrowhead's box. The same footprint as a gate's own marker, so
/// the hand-over between the two doesn't change size.
const ARROW_HALF_PX: f32 = 5.5;
/// The arrowhead glyph is drawn a little larger than its box so it reads
/// as a solid head rather than a caret.
const ARROW_GLYPH_SCALE: f32 = 2.2;
/// Where the label sits relative to the arrow's centre.
const LABEL_OFFSET_PX: Vec2 = Vec2::new(10.0, 10.0);
const LABEL_FONT_PX: f32 = 12.0;

/// Edge-of-screen arrowhead for a gate that's off-screen, rotated each
/// frame to point toward it.
#[derive(Component)]
pub struct EdgeIndicatorFor(pub Entity);

/// The label next to an edge indicator - a sibling, not a child, so it
/// doesn't rotate along with the arrowhead.
#[derive(Component)]
pub struct EdgeLabelFor(pub Entity);

/// Gates pinned to the edge this frame, and where their arrows sit.
#[derive(Resource, Default)]
pub struct GateEdgeTargets(pub HashMap<Entity, Vec2>);

impl GateEdgeTargets {
    pub fn is_pinned(&self, gate: Entity) -> bool {
        self.0.contains_key(&gate)
    }
}

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

/// EDGE-01 (F4-08): which way on screen a point lies, from the camera's own
/// axes. Used for a gate the camera can't project because it is behind it:
/// its direction is still well defined - a gate behind a steeply tilted
/// camera reads as "down" - even though it has no screen position.
pub fn screen_direction(to_target: Vec3, camera_right: Vec3, camera_up: Vec3) -> Vec2 {
    // Screen y grows downwards.
    Vec2::new(to_target.dot(camera_right), -to_target.dot(camera_up))
}

/// EDGE-01 (F4-08): pin an arrow on the edge in a screen direction from the
/// centre. `None` for no direction at all (a point dead behind the lens).
pub fn pin_direction_to_edge(direction: Vec2, viewport: Vec2, margin_px: f32) -> Option<Vec2> {
    let direction = direction.try_normalize()?;
    // Any point well past the viewport along the direction clamps the same.
    let beyond = viewport / 2.0 + direction * viewport.length();
    clamp_to_viewport_edge(beyond, viewport, margin_px)
}

/// EDGE-01: the rotation that turns the arrowhead glyph - which points up
/// when unrotated - towards the gate.
pub fn edge_arrow_angle(edge_pos: Vec2, viewport: Vec2) -> f32 {
    let dir = edge_pos - viewport / 2.0;
    // Screen y grows downwards, so "up" is -y.
    dir.x.atan2(-dir.y)
}

/// EDGE-01 (F5-09): the part of the window the map shows in, below the
/// top bar. Arrows are pinned to its edges, so none hides under the bar;
/// the clamping functions work in its own coordinates.
struct MapArea {
    offset: Vec2,
    size: Vec2,
}

impl MapArea {
    fn of(viewport: Vec2) -> Self {
        let offset = Vec2::new(0.0, TOP_BAR_HEIGHT_PX);
        Self {
            offset,
            size: (viewport - offset).max(Vec2::ZERO),
        }
    }
}

pub struct EdgePlugin;

impl Plugin for EdgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GateEdgeTargets>()
            .add_systems(Startup, spawn_edge_indicators.in_set(Boot::Overlay))
            .add_systems(Update, update_edge_targets.in_set(Stage::Layout))
            .add_systems(
                Update,
                (place_edge_indicators, place_edge_labels).in_set(Stage::Overlay),
            );
    }
}

fn spawn_edge_indicators(
    mut commands: Commands,
    fonts: Res<Fonts>,
    gates: Query<(Entity, &BodyClass, &Ownership, &DisplayName)>,
) {
    for (entity, class, ownership, name) in &gates {
        if *class != BodyClass::Gate {
            continue;
        }
        let color = colors::ownership_color(*ownership);
        commands
            .spawn((
                EdgeIndicatorFor(entity),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(ARROW_HALF_PX * 2.0),
                    height: px(ARROW_HALF_PX * 2.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    display: Display::None,
                    ..default()
                },
                UiTransform::IDENTITY,
            ))
            .with_children(|parent| {
                // Points up by default; rotated per-frame to point toward
                // the gate.
                parent.spawn((
                    Text::new("^"),
                    fonts.label(ARROW_HALF_PX * ARROW_GLYPH_SCALE),
                    TextColor(color),
                ));
            });

        commands.spawn((
            EdgeLabelFor(entity),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            Text::new(name.0.clone()),
            fonts.label(LABEL_FONT_PX),
            TextColor(color),
        ));
    }
}

/// EDGE-01: gates that fall outside a small margin get clamped to the
/// viewport edge instead of being hidden outright. A gate behind the camera
/// has no projected position; it is pinned in its direction instead (F4-08).
fn update_edge_targets(
    screen: Res<ScreenPositions>,
    view: ViewNow,
    gates: Query<(Entity, &BodyClass)>,
    mut targets: ResMut<GateEdgeTargets>,
) {
    targets.0.clear();
    let camera = view.camera().map(|(_, transform)| transform);
    for (entity, class) in &gates {
        if *class != BodyClass::Gate {
            continue;
        }
        let area = MapArea::of(screen.viewport);
        let edge = match (screen.get(entity), camera) {
            (Some(pos), _) => clamp_to_viewport_edge(pos - area.offset, area.size, EDGE_MARGIN_PX),
            (None, Some(camera)) => {
                let Some(gate) = view.position(entity) else {
                    continue;
                };
                let to_gate = gate - camera.translation();
                let direction = screen_direction(to_gate, *camera.right(), *camera.up());
                pin_direction_to_edge(direction, area.size, EDGE_MARGIN_PX)
            }
            (None, None) => None,
        };
        if let Some(edge) = edge {
            targets.0.insert(entity, edge + area.offset);
        }
    }
}

fn place_edge_indicators(
    screen: Res<ScreenPositions>,
    targets: Res<GateEdgeTargets>,
    mut indicators: Query<(&EdgeIndicatorFor, &mut Node, &mut UiTransform)>,
) {
    for (edge_for, mut node, mut transform) in &mut indicators {
        let Some(&pos) = targets.0.get(&edge_for.0) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x - ARROW_HALF_PX);
        node.top = px(pos.y - ARROW_HALF_PX);
        // The glyph points up by default; rotate it (clockwise) to point
        // from the viewport centre toward the gate's true direction.
        let area = MapArea::of(screen.viewport);
        transform.rotation = Rot2::radians(edge_arrow_angle(pos - area.offset, area.size));
    }
}

fn place_edge_labels(
    targets: Res<GateEdgeTargets>,
    mut labels: Query<(&EdgeLabelFor, &mut Node), Without<EdgeIndicatorFor>>,
) {
    for (label_for, mut node) in &mut labels {
        let Some(&pos) = targets.0.get(&label_for.0) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x + LABEL_OFFSET_PX.x);
        node.top = px(pos.y + LABEL_OFFSET_PX.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEWPORT: Vec2 = Vec2::new(1600.0, 900.0);
    const MARGIN: f32 = 28.0;

    /// EDGE-01: a gate comfortably on screen is not an edge indicator.
    #[test]
    fn on_screen_gates_are_not_clamped() {
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
    fn off_screen_gates_are_pinned_inside_the_margin() {
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
    fn clamp_lands_on_the_first_edge_crossed() {
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
    fn axis_aligned_gates_clamp_cleanly() {
        let centre = VIEWPORT / 2.0;
        let above = clamp_to_viewport_edge(Vec2::new(centre.x, -2000.0), VIEWPORT, MARGIN).unwrap();
        assert!((above.x - centre.x).abs() < 0.01);
        assert!((above.y - MARGIN).abs() < 0.01);
        let beside =
            clamp_to_viewport_edge(Vec2::new(-2000.0, centre.y), VIEWPORT, MARGIN).unwrap();
        assert!((beside.y - centre.y).abs() < 0.01);
        assert!((beside.x - MARGIN).abs() < 0.01);
    }

    /// EDGE-01 (F4-08): a point behind a camera looking down and forward
    /// reads as "down the screen", and to the right of it as "right".
    #[test]
    fn points_behind_the_camera_still_have_a_direction() {
        let (right, up) = (Vec3::X, Vec3::new(0.0, 0.4, -0.9).normalize());
        // Behind and on the near side of a steeply tilted camera.
        let behind = screen_direction(Vec3::new(0.0, -0.2, 5.0), right, up);
        assert!(behind.y > 0.0, "down the screen");
        let behind_right = screen_direction(Vec3::new(3.0, -0.2, 5.0), right, up);
        assert!(behind_right.x > 0.0 && behind_right.y > 0.0);
    }

    /// EDGE-01 (F4-08): a direction pins to the edge, inside the margin;
    /// no direction pins nowhere.
    #[test]
    fn directions_pin_to_the_edge() {
        let down = pin_direction_to_edge(Vec2::Y, VIEWPORT, MARGIN).unwrap();
        assert!((down.y - (VIEWPORT.y - MARGIN)).abs() < 0.01);
        assert!((down.x - VIEWPORT.x / 2.0).abs() < 0.01);
        let left = pin_direction_to_edge(Vec2::new(-1.0, 0.0), VIEWPORT, MARGIN).unwrap();
        assert!((left.x - MARGIN).abs() < 0.01);
        assert_eq!(pin_direction_to_edge(Vec2::ZERO, VIEWPORT, MARGIN), None);
    }

    /// EDGE-01: the arrow points away from the centre, towards the gate -
    /// 0 is up, and the angle turns clockwise with the glyph.
    #[test]
    fn arrow_points_towards_the_gate() {
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

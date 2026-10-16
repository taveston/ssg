//! Gate edge indicators (EDGE-01 to EDGE-03): a gate outside the viewport
//! shows as an arrow pinned to the screen edge in its direction, beside a
//! tab with its destination's name (`edge_tab.rs`).
//!
//! `GateEdgeTargets` says which gates are pinned this frame and where; the
//! marker overlay reads it to hide those gates' ordinary markers.

use std::collections::HashMap;

use bevy::prelude::*;

use super::edge_tab::{TabChevron, place_tab, spawn_tab, tab_label};
use super::fonts::Fonts;
use super::screen::{MapArea, ScreenPositions, screen_angle};
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{BodyClass, GateDestination};
use crate::{Boot, Stage};

/// How far inside the viewport edge a pinned arrow sits.
pub const EDGE_MARGIN_PX: f32 = 28.0;

/// A gate's edge tab: its arrow and its destination (EDGE-03).
#[derive(Component)]
pub struct EdgeTabFor(pub Entity);

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

/// EDGE-01: which way on screen a point lies, from the camera's own
/// axes. Used for a gate the camera can't project because it is behind it:
/// its direction is still well defined - a gate behind a steeply tilted
/// camera reads as "down" - even though it has no screen position.
pub fn screen_direction(to_target: Vec3, camera_right: Vec3, camera_up: Vec3) -> Vec2 {
    // Screen y grows downwards.
    Vec2::new(to_target.dot(camera_right), -to_target.dot(camera_up))
}

/// EDGE-01: pin an arrow on the edge in a screen direction from the
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
    screen_angle(viewport / 2.0, edge_pos)
}

/// EDGE-01: where on the map area's edge, in window pixels, the arrow for
/// a body sits: pinned towards its projected position `on_screen`, or, if
/// it is behind the camera, towards its direction from the camera. `None`
/// while it is comfortably inside the map area.
pub fn edge_point(
    on_screen: Option<Vec2>,
    world: Option<Vec3>,
    camera: Option<&GlobalTransform>,
    area: &MapArea,
) -> Option<Vec2> {
    let edge = match (on_screen, camera) {
        (Some(pos), _) => clamp_to_viewport_edge(pos - area.offset, area.size, EDGE_MARGIN_PX),
        (None, Some(camera)) => {
            let to_body = world? - camera.translation();
            let direction = screen_direction(to_body, *camera.right(), *camera.up());
            pin_direction_to_edge(direction, area.size, EDGE_MARGIN_PX)
        }
        (None, None) => None,
    };
    edge.map(|edge| edge + area.offset)
}

pub struct EdgePlugin;

impl Plugin for EdgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GateEdgeTargets>()
            .add_systems(Startup, spawn_edge_tabs.in_set(Boot::Overlay))
            .add_systems(Update, update_edge_targets.in_set(Stage::Layout))
            .add_systems(Update, place_edge_tabs.in_set(Stage::Overlay));
    }
}

/// EDGE-01 / EDGE-03: a hidden tab per gate, labelled with the gate's
/// destination alone ("Ostra", not "Gate Ostra").
fn spawn_edge_tabs(
    mut commands: Commands,
    fonts: Res<Fonts>,
    tunables: Res<Tunables>,
    gates: Query<(Entity, &GateDestination)>,
) {
    for (gate, destination) in &gates {
        spawn_tab(
            &mut commands,
            &fonts,
            tunables.button_stroke_px,
            |content| {
                content.spawn(tab_label(&fonts, &destination.0));
            },
        )
        .insert(EdgeTabFor(gate));
    }
}

/// EDGE-01: gates that fall outside a small margin get clamped to the
/// viewport edge instead of being hidden outright. A gate behind the camera
/// has no projected position; it is pinned in its direction instead.
fn update_edge_targets(
    screen: Res<ScreenPositions>,
    view: ViewNow,
    gates: Query<(Entity, &BodyClass)>,
    mut targets: ResMut<GateEdgeTargets>,
) {
    targets.0.clear();
    let camera = view.camera().map(|(_, transform)| transform);
    let area = MapArea::of(screen.viewport);
    for (entity, class) in &gates {
        if *class != BodyClass::Gate {
            continue;
        }
        let at = view.position(entity);
        if let Some(edge) = edge_point(screen.get(entity), at, camera.as_ref(), &area) {
            targets.0.insert(entity, edge);
        }
    }
}

/// EDGE-01 / EDGE-02: show each pinned gate's tab with its arrow on the
/// pinned point, turned towards the gate, and the tab whole inside the map
/// area.
fn place_edge_tabs(
    screen: Res<ScreenPositions>,
    targets: Res<GateEdgeTargets>,
    mut tabs: Query<(&EdgeTabFor, &mut Node, &ComputedNode, &Children)>,
    mut chevrons: Query<&mut UiTransform, With<TabChevron>>,
) {
    let area = MapArea::of(screen.viewport);
    for (tab_for, mut node, computed, children) in &mut tabs {
        let Some(&at) = targets.0.get(&tab_for.0) else {
            node.display = Display::None;
            continue;
        };
        place_tab(&mut node, computed, at, &area);
        let angle = edge_arrow_angle(at - area.offset, area.size);
        for &child in children {
            if let Ok(mut transform) = chevrons.get_mut(child) {
                transform.rotation = Rot2::radians(angle);
            }
        }
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

    /// EDGE-01: a point behind a camera looking down and forward
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

    /// EDGE-01: a direction pins to the edge, inside the margin;
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

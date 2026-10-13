//! Where every body lands on screen this frame (MRK-03). Projected once, in
//! `Stage::Screen`, and shared by every overlay that sits on a body's
//! position: markers, pills and gate edge arrows.

use std::collections::HashMap;

use bevy::prelude::*;

use super::topbar::TOP_BAR_HEIGHT_PX;
use super::world::ViewNow;
use crate::Stage;

/// Each body's projected screen position, in logical pixels from the
/// window's top left and snapped to a whole screen pixel (SYM-16), plus
/// the viewport it was projected into. A body
/// behind the camera has no entry.
#[derive(Resource, Default)]
pub struct ScreenPositions {
    pub viewport: Vec2,
    pub bodies: HashMap<Entity, Vec2>,
}

impl ScreenPositions {
    pub fn get(&self, body: Entity) -> Option<Vec2> {
        self.bodies.get(&body).copied()
    }

    pub fn in_viewport(&self, pos: Vec2) -> bool {
        in_viewport(pos, self.viewport)
    }
}

/// Whether a screen position is inside the viewport, edges included.
pub fn in_viewport(pos: Vec2, viewport: Vec2) -> bool {
    pos.x >= 0.0 && pos.y >= 0.0 && pos.x <= viewport.x && pos.y <= viewport.y
}

/// SYM-16: `pos` moved to the nearest whole physical pixel, so a marker,
/// symbol or pill centred there draws its outlines as sharply as the
/// orbit rings. `scale` is physical pixels per logical pixel.
pub fn snap_to_pixel(pos: Vec2, scale: f32) -> Vec2 {
    (pos * scale).round() / scale
}

/// The part of the window the map shows in, below the top bar (EDGE-01,
/// INS-07, ZOOM-14, CAM-07). Gate arrows are pinned to its edges and the
/// zoom limits and the focus margin are measured in it.
pub struct MapArea {
    pub offset: Vec2,
    pub size: Vec2,
}

impl MapArea {
    pub fn of(viewport: Vec2) -> Self {
        let offset = Vec2::new(0.0, TOP_BAR_HEIGHT_PX);
        Self {
            offset,
            size: (viewport - offset).max(Vec2::ZERO),
        }
    }
}

pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenPositions>()
            .add_systems(Update, project_bodies.in_set(Stage::Screen));
    }
}

/// MRK-03 / CAM-05: project every body with this frame's camera and
/// positions, so markers and the camera never disagree by a frame.
fn project_bodies(view: ViewNow, windows: Query<&Window>, mut screen: ResMut<ScreenPositions>) {
    let Ok(window) = windows.single() else {
        return;
    };
    screen.viewport = Vec2::new(window.width(), window.height());
    screen.bodies.clear();
    let scale = window.scale_factor();
    for (&body, &world) in view.positions() {
        if let Some(pos) = view.project(world) {
            screen.bodies.insert(body, snap_to_pixel(pos, scale));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Covers MRK-03.
    #[test]
    fn viewport_test_includes_the_edges() {
        let viewport = Vec2::new(800.0, 600.0);
        assert!(in_viewport(Vec2::new(0.0, 0.0), viewport));
        assert!(in_viewport(Vec2::new(800.0, 600.0), viewport));
        assert!(in_viewport(Vec2::new(400.0, 300.0), viewport));
        assert!(!in_viewport(Vec2::new(-0.1, 300.0), viewport));
        assert!(!in_viewport(Vec2::new(400.0, 600.1), viewport));
    }

    /// SYM-16: positions land on whole physical pixels, at any UI scale.
    #[test]
    fn positions_snap_to_whole_screen_pixels() {
        assert_eq!(
            snap_to_pixel(Vec2::new(10.4, 7.6), 1.0),
            Vec2::new(10.0, 8.0)
        );
        let snapped = snap_to_pixel(Vec2::new(10.3, 7.1), 1.5);
        let physical = snapped * 1.5;
        assert!((physical - physical.round()).abs().max_element() < 1e-4);
        assert!((snapped - Vec2::new(10.3, 7.1)).abs().max_element() <= 0.5 / 1.5);
    }
}

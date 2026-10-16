//! Where every body lands on screen this frame (MRK-03), and the map area
//! they land in. Projected once, in `Stage::Screen`, and shared by every
//! overlay that sits on a body's position: markers, multi-markers and gate edge
//! arrows.
//!
//! The **viewport** is the part of the window the camera draws in: the
//! window less the sidebar (INS-07). Screen positions, the map area and
//! the fits are all measured in it, so the view narrows with the sidebar.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::sidebar::SidebarState;
use super::topbar::TOP_BAR_HEIGHT_PX;
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::Stage;

/// Each body's projected screen position, in logical pixels from the
/// window's top left and snapped to a whole screen pixel (SYM-16), plus
/// the viewport it was projected into. A body behind the camera has no
/// entry.
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

/// The clockwise angle from screen up to the direction from `from` to
/// `to` (screen y grows downwards): what a node drawn pointing up is
/// turned by to point that way (MRK-17, EDGE-01, SYM-11, MOT-14).
pub fn screen_angle(from: Vec2, to: Vec2) -> f32 {
    let d = to - from;
    d.x.atan2(-d.y)
}

/// The stretch of the ray from `origin` along the unit vector `dir` that
/// lies inside `rect`, as distances along the ray, or `None` if the ray
/// misses it.
pub fn ray_inside(origin: Vec2, dir: Vec2, rect: Rect) -> Option<(f32, f32)> {
    let (mut near, mut far) = (0.0_f32, f32::INFINITY);
    for axis in 0..2 {
        let (o, d) = (origin[axis], dir[axis]);
        let (lo, hi) = (rect.min[axis], rect.max[axis]);
        if d.abs() < 1e-9 {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let (a, b) = ((lo - o) / d, (hi - o) / d);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far).then_some((near, far))
}

/// SYM-16: `pos` moved to the nearest whole physical pixel, so a marker,
/// symbol or multi-marker centred there draws its outlines as sharply as the
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

    /// The map area as a rectangle in window pixels.
    pub fn rect(&self) -> Rect {
        Rect {
            min: self.offset,
            max: self.offset + self.size,
        }
    }
}

/// INS-07: the camera's viewport: the window less the sidebar's width at
/// its right edge, never narrower than a pixel.
pub fn camera_viewport(window: Vec2, sidebar_px: f32) -> Vec2 {
    Vec2::new((window.x - sidebar_px).max(1.0), window.y)
}

/// The window and the camera's viewport in it this frame (INS-07).
#[derive(SystemParam)]
pub struct MapView<'w, 's> {
    windows: Query<'w, 's, &'static Window>,
    sidebar: Res<'w, SidebarState>,
    tunables: Res<'w, Tunables>,
}

impl MapView<'_, '_> {
    /// The window's logical size, if there is one.
    pub fn window(&self) -> Option<Vec2> {
        let window = self.windows.single().ok()?;
        Some(Vec2::new(window.width(), window.height()))
    }

    /// Physical pixels per logical pixel.
    pub fn scale_factor(&self) -> f32 {
        self.windows.single().map_or(1.0, Window::scale_factor)
    }

    /// INS-07: the width the sidebar takes this frame.
    pub fn sidebar_px(&self) -> f32 {
        self.sidebar.width_px(&self.tunables)
    }

    /// INS-07: the camera's viewport: the window less the sidebar.
    pub fn viewport(&self) -> Option<Vec2> {
        Some(camera_viewport(self.window()?, self.sidebar_px()))
    }

    /// The map area this frame.
    pub fn area(&self) -> Option<MapArea> {
        Some(MapArea::of(self.viewport()?))
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
fn project_bodies(view: ViewNow, map: MapView, mut screen: ResMut<ScreenPositions>) {
    let Some(viewport) = map.viewport() else {
        return;
    };
    screen.viewport = viewport;
    screen.bodies.clear();
    let scale = map.scale_factor();
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

    /// INS-07: the viewport is the window less the sidebar, never empty.
    #[test]
    fn the_viewport_loses_the_sidebar() {
        let window = Vec2::new(1600.0, 900.0);
        assert_eq!(camera_viewport(window, 0.0), window);
        assert_eq!(camera_viewport(window, 320.0), Vec2::new(1280.0, 900.0));
        assert_eq!(camera_viewport(window, 5000.0).x, 1.0);
    }

    /// MRK-17: angles are clockwise from screen up.
    #[test]
    fn screen_angles_run_clockwise_from_up() {
        let o = Vec2::new(100.0, 100.0);
        let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert!(close(screen_angle(o, o + Vec2::NEG_Y), 0.0));
        assert!(close(
            screen_angle(o, o + Vec2::X),
            std::f32::consts::FRAC_PI_2
        ));
        assert!(close(
            screen_angle(o, o + Vec2::NEG_X),
            -std::f32::consts::FRAC_PI_2
        ));
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

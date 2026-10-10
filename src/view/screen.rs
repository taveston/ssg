//! Where every body lands on screen this frame (MRK-03). Projected once, in
//! `Stage::Screen`, and shared by every overlay that sits on a body's
//! position: markers, pills and gate edge arrows.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::Stage;
use crate::sim::body::BodyClass;

/// Each body's projected screen position, in logical pixels from the
/// window's top left, plus the viewport it was projected into. A body
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

pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenPositions>()
            .add_systems(Update, project_bodies.in_set(Stage::Screen));
    }
}

fn project_bodies(
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    bodies: Query<(Entity, &GlobalTransform), With<BodyClass>>,
    mut screen: ResMut<ScreenPositions>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    screen.viewport = Vec2::new(window.width(), window.height());
    screen.bodies.clear();
    for (entity, transform) in &bodies {
        if let Ok(pos) = camera.world_to_viewport(camera_transform, transform.translation()) {
            screen.bodies.insert(entity, pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mrk_03_viewport_test_includes_the_edges() {
        let viewport = Vec2::new(800.0, 600.0);
        assert!(in_viewport(Vec2::new(0.0, 0.0), viewport));
        assert!(in_viewport(Vec2::new(800.0, 600.0), viewport));
        assert!(in_viewport(Vec2::new(400.0, 300.0), viewport));
        assert!(!in_viewport(Vec2::new(-0.1, 300.0), viewport));
        assert!(!in_viewport(Vec2::new(400.0, 600.1), viewport));
    }
}

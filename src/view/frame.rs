//! The reference frame: the object whose non-rotating frame the camera sits
//! in (CAM-05 to CAM-08), as shared view state. There is always exactly
//! one; by default it is the star, which doesn't move, so the star's frame
//! behaves as a free camera. The camera tracks the frame body; the
//! breadcrumb, the brackets and the click handlers read it. Selection
//! (`selection.rs`) is separate and never moves the camera.
//!
//! Like `selection`, it is its own small module so every reader can depend
//! on it without depending on the camera (ARCH-03).

use bevy::prelude::*;

use crate::sim::load::SystemLoadApp;

use super::world::WorldPositions;
use crate::Boot;
use crate::sim::body::BodyClass;

/// The frame body and the pan offset of the pivot from it. The offset is in
/// render units, in the orbital plane, and does not turn with the body's
/// orbit (CAM-05). In the star's frame the offset is simply the pivot.
///
/// `star` is the system's star, kept here so any reader can tell the
/// star's frame apart. Before the system is spawned both are
/// `Entity::PLACEHOLDER`, which reads as the star's frame at the origin.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub star: Entity,
    pub body: Entity,
    pub offset: Vec3,
}

impl Default for Frame {
    fn default() -> Self {
        Self::star_frame(Entity::PLACEHOLDER, Vec3::ZERO)
    }
}

impl Frame {
    /// The star's frame, with the pivot at `offset` from the star.
    pub fn star_frame(star: Entity, offset: Vec3) -> Self {
        Self {
            star,
            body: star,
            offset,
        }
    }

    /// Whether the camera sits in the star's frame.
    pub fn in_star_frame(&self) -> bool {
        self.body == self.star
    }

    /// Whether `body` is the frame body (ZOOM-17 asks).
    pub fn is_on(&self, body: Entity) -> bool {
        self.body == body
    }

    /// The frame body, unless it is the star.
    pub fn non_star_body(&self) -> Option<Entity> {
        (!self.in_star_frame()).then_some(self.body)
    }

    /// The same star, with `body` as the frame and `offset` from it.
    pub fn on(&self, body: Entity, offset: Vec3) -> Self {
        Self {
            star: self.star,
            body,
            offset,
        }
    }
}

/// Where a body is this frame, in render units. A body with no entry, like
/// the star, sits at the origin (CAM-05).
pub fn body_position(body: Entity, positions: &WorldPositions) -> Vec3 {
    positions.0.get(&body).copied().unwrap_or(Vec3::ZERO)
}

/// CAM-05: the pivot is the frame body's position plus the pan offset,
/// recomputed every frame.
pub fn frame_pivot(body_position: Vec3, offset: Vec3) -> Vec3 {
    body_position + offset
}

/// CAM-19: the pivot never moves further than `bound` from the star (the
/// origin), in the plane.
pub fn clamp_to_pan_bound(pivot: Vec3, bound: f32) -> Vec3 {
    let flat = Vec3::new(pivot.x, 0.0, pivot.z);
    if flat.length() > bound {
        flat.normalize() * bound
    } else {
        flat
    }
}

/// CAM-07 / CAM-19: the offset that keeps `body + offset` inside the bound.
pub fn clamp_offset_to_pan_bound(body_position: Vec3, offset: Vec3, bound: f32) -> Vec3 {
    let body = Vec3::new(body_position.x, 0.0, body_position.z);
    clamp_to_pan_bound(body + offset, bound) - body
}

/// The ancestors of `body`, nearest first: its parent, its parent's parent,
/// and so on, stopping below the star (planets have no parent entity).
pub fn ancestors(body: Entity, parent_of: impl Fn(Entity) -> Option<Entity>) -> Vec<Entity> {
    let mut chain = Vec::new();
    let mut current = parent_of(body);
    // The hierarchy is a tree; the bound only guards against a cycle.
    while let Some(parent) = current
        && chain.len() < MAX_DEPTH
    {
        chain.push(parent);
        current = parent_of(parent);
    }
    chain
}

/// Deeper than any real hierarchy; stops a cycle looping forever.
const MAX_DEPTH: usize = 64;

pub struct FramePlugin;

impl Plugin for FramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Frame>()
            .add_load_systems(Boot::Derive, start_in_the_star_frame);
    }
}

/// CAM-05: the view starts in the star's frame, with the pivot on the star.
fn start_in_the_star_frame(stars: Query<(Entity, &BodyClass)>, mut frame: ResMut<Frame>) {
    if let Some((star, _)) = stars.iter().find(|(_, class)| **class == BodyClass::Star) {
        *frame = Frame::star_frame(star, Vec3::ZERO);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CAM-05: the pivot is the frame body's position plus the offset, and
    /// the offset is not turned by where the body is in its orbit.
    #[test]
    fn pivot_is_position_plus_offset() {
        let offset = Vec3::new(2.0, 0.0, -1.0);
        for position in [Vec3::new(180.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -180.0)] {
            assert_eq!(frame_pivot(position, offset), position + offset);
        }
        assert_eq!(frame_pivot(Vec3::ONE, Vec3::ZERO), Vec3::ONE);
    }

    /// CAM-05: the star isn't in the position table; it is the origin.
    #[test]
    fn the_star_sits_at_the_origin() {
        let mut world = World::new();
        let star = world.spawn_empty().id();
        assert_eq!(body_position(star, &WorldPositions::default()), Vec3::ZERO);
    }

    /// There is always a frame; by default it is the star's, and only the
    /// frame body counts as being "on" it.
    /// Covers CAM-06, ZOOM-17.
    #[test]
    fn the_frame_defaults_to_the_star() {
        let mut world = World::new();
        let [star, body, other] = [(); 3].map(|_| world.spawn_empty().id());
        let frame = Frame::star_frame(star, Vec3::ZERO);
        assert!(frame.in_star_frame());
        assert_eq!(frame.non_star_body(), None);
        let on_body = frame.on(body, Vec3::X);
        assert!(!on_body.in_star_frame());
        assert!(on_body.is_on(body) && !on_body.is_on(other));
        assert_eq!(on_body.non_star_body(), Some(body));
        assert!(Frame::default().in_star_frame());
    }

    /// CAM-19 in a body's frame: the offset is cut back so body plus offset
    /// stays on the bound.
    #[test]
    fn frame_offset_stays_within_the_pan_bound() {
        let body = Vec3::new(90.0, 0.0, 0.0);
        let offset = clamp_offset_to_pan_bound(body, Vec3::new(50.0, 0.0, 0.0), 100.0);
        assert!((offset - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-4);
        let small = Vec3::new(1.0, 0.0, 1.0);
        assert!((clamp_offset_to_pan_bound(body, small, 100.0) - small).length() < 1e-4);
    }

    /// CAM-19: inside the bound the pivot is untouched; outside, it stops on
    /// the bound in the same direction.
    #[test]
    fn pivot_stays_within_the_pan_bound() {
        let inside = Vec3::new(30.0, 0.0, 40.0);
        assert_eq!(clamp_to_pan_bound(inside, 100.0), inside);
        let outside = Vec3::new(300.0, 0.0, 400.0);
        let clamped = clamp_to_pan_bound(outside, 100.0);
        assert!((clamped.length() - 100.0).abs() < 1e-3);
        assert!(clamped.normalize().dot(outside.normalize()) > 0.9999);
    }

    /// CAM-25: a moon's ancestors are its planet, nearest first; a planet
    /// has none below the star; a cycle stops.
    #[test]
    fn ancestors_run_from_the_parent_upwards() {
        let mut world = World::new();
        let [planet, moon, submoon] = [(); 3].map(|_| world.spawn_empty().id());
        let parent_of = |e: Entity| {
            if e == submoon {
                Some(moon)
            } else if e == moon {
                Some(planet)
            } else {
                None
            }
        };
        assert_eq!(ancestors(submoon, parent_of), vec![moon, planet]);
        assert!(ancestors(planet, parent_of).is_empty());
        let looped = ancestors(moon, Some);
        assert_eq!(looped.len(), MAX_DEPTH);
    }
}

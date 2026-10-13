//! Where everything is *this frame*, in render space (CAM-05).
//!
//! Bevy only updates `GlobalTransform` after a frame's systems have run, so
//! anything that read it saw last frame's world - and a camera following a
//! body one frame behind its marker made the marker shake. Instead, body
//! positions come straight from the simulation's km positions up the parent
//! chain (`WorldPositions`, filled in `Stage::Projection`), and the camera is
//! read from the `Transform` the rig wrote this frame. `ViewNow` bundles the
//! two for every system that places something on screen.
//!
//! Shared state, read across the view (ARCH-03); `projection.rs` writes it.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::math::DVec3;
use bevy::prelude::*;

/// Every body's position this frame, in render units, relative to the
/// render origin (`RenderOrigin`). The star has an entry like any body.
#[derive(Resource, Default, Debug)]
pub struct WorldPositions(pub HashMap<Entity, Vec3>);

/// SCL-06: the floating origin. Render space is centred on the frame body,
/// so whatever the camera looks at is near the origin and keeps full `f32`
/// precision however far the body is from the star.
///
/// `km` is the frame body's position relative to the star this frame;
/// `star` is where the star lands in render space (minus `km`, converted);
/// `shift` is how far points held from last frame must move to stay put
/// in this frame's render space.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct RenderOrigin {
    pub km: DVec3,
    pub star: Vec3,
    pub shift: Vec3,
}

/// CAM-05 / ARCH-11: each body's position relative to the star, from its
/// position relative to its parent. `locals` maps a body to its local km
/// position and the body it orbits (if any); a body with no entry, like the
/// star, sits at the origin.
pub fn world_positions_km(
    locals: &HashMap<Entity, (DVec3, Option<Entity>)>,
) -> HashMap<Entity, DVec3> {
    fn resolve(
        body: Entity,
        locals: &HashMap<Entity, (DVec3, Option<Entity>)>,
        done: &mut HashMap<Entity, DVec3>,
        depth: usize,
    ) -> DVec3 {
        if let Some(&known) = done.get(&body) {
            return known;
        }
        let Some(&(local, parent)) = locals.get(&body) else {
            return DVec3::ZERO;
        };
        // A hierarchy deeper than the number of bodies is a cycle; stop
        // rather than recurse forever.
        let parent_position = match parent {
            Some(parent) if depth < locals.len() => resolve(parent, locals, done, depth + 1),
            _ => DVec3::ZERO,
        };
        let world = parent_position + local;
        done.insert(body, world);
        world
    }
    let mut done = HashMap::new();
    for &body in locals.keys() {
        resolve(body, locals, &mut done, 0);
    }
    done
}

/// This frame's view: body positions and the camera as placed this frame.
#[derive(SystemParam)]
pub struct ViewNow<'w, 's> {
    positions: Res<'w, WorldPositions>,
    origin: Res<'w, RenderOrigin>,
    cameras: Query<'w, 's, (&'static Camera, &'static Transform), With<Camera3d>>,
}

impl ViewNow<'_, '_> {
    /// Where a body is this frame, in render units.
    pub fn position(&self, body: Entity) -> Option<Vec3> {
        self.positions.0.get(&body).copied()
    }

    /// Where the star is this frame, in render units (SCL-06).
    pub fn star(&self) -> Vec3 {
        self.origin.star
    }

    /// Every body's position this frame.
    pub fn positions(&self) -> &HashMap<Entity, Vec3> {
        &self.positions.0
    }

    /// The camera and its transform as placed this frame.
    pub fn camera(&self) -> Option<(&Camera, GlobalTransform)> {
        let (camera, transform) = self.cameras.single().ok()?;
        Some((camera, GlobalTransform::from(*transform)))
    }

    /// Where a world point lands on screen this frame.
    pub fn project(&self, world: Vec3) -> Option<Vec2> {
        let (camera, transform) = self.camera()?;
        camera.world_to_viewport(&transform, world).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CAM-05: a moon's world position is its planet's plus its own offset,
    /// whatever order the bodies come in; the star is the origin.
    #[test]
    fn world_positions_add_up_the_parent_chain() {
        let mut world = World::new();
        let [star, planet, moon, submoon] = [(); 4].map(|_| world.spawn_empty().id());
        let mut locals = HashMap::new();
        locals.insert(submoon, (DVec3::new(0.0, 0.0, 1.0), Some(moon)));
        locals.insert(moon, (DVec3::new(0.0, 0.0, 10.0), Some(planet)));
        locals.insert(planet, (DVec3::new(100.0, 0.0, 0.0), None));
        let resolved = world_positions_km(&locals);
        assert_eq!(resolved[&planet], DVec3::new(100.0, 0.0, 0.0));
        assert_eq!(resolved[&moon], DVec3::new(100.0, 0.0, 10.0));
        assert_eq!(resolved[&submoon], DVec3::new(100.0, 0.0, 11.0));
        assert!(!resolved.contains_key(&star));
    }

    /// A parent loop can't hang the resolver.
    #[test]
    fn a_parent_cycle_terminates() {
        let mut world = World::new();
        let [a, b] = [(); 2].map(|_| world.spawn_empty().id());
        let mut locals = HashMap::new();
        locals.insert(a, (DVec3::X, Some(b)));
        locals.insert(b, (DVec3::X, Some(a)));
        assert_eq!(world_positions_km(&locals).len(), 2);
    }
}

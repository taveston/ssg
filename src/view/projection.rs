//! The view side of ARCH-11: turns each body's simulated position in km
//! into a render-space `Transform`, through `scale.rs` (ARCH-12). The one
//! place a simulation position becomes something the camera can see.
//!
//! Satellites' positions are relative to their parent, so their transforms
//! are local offsets and Bevy's propagation adds the parent's position.

use bevy::prelude::*;

use std::collections::HashMap;

use bevy::math::DVec3;

use super::scale::km_to_render_vec3;
use super::world::{WorldPositions, world_positions_km};
use crate::sim::body::BodyClass;
use crate::sim::orbit::OrbitPositionKm;
use crate::{Boot, Stage};

pub struct ProjectionPlugin;

impl Plugin for ProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldPositions>()
            .add_systems(Startup, attach_transforms.in_set(Boot::Derive))
            .add_systems(
                Update,
                (update_world_positions, project_orbit_positions).in_set(Stage::Projection),
            );
    }
}

/// Bodies that don't have a render transform yet.
type UnplacedBodies = (With<BodyClass>, Without<Transform>);

/// Every body gets a transform already matching its simulated position, so
/// the first frame draws everything in place. The star, which doesn't
/// orbit, sits at the origin.
fn attach_transforms(
    mut commands: Commands,
    bodies: Query<(Entity, Option<&OrbitPositionKm>), UnplacedBodies>,
) {
    for (entity, position) in &bodies {
        let translation = position
            .map(|p| km_to_render_vec3(p.0))
            .unwrap_or(Vec3::ZERO);
        commands
            .entity(entity)
            .insert(Transform::from_translation(translation));
    }
}

/// A body, its position relative to what it orbits, and what that is.
type BodyPlacement = (
    Entity,
    Option<&'static OrbitPositionKm>,
    Option<&'static ChildOf>,
);

/// CAM-05: every body's position this frame, relative to the star,
/// from the simulation's km positions - not last frame's `GlobalTransform`.
fn update_world_positions(
    bodies: Query<BodyPlacement, With<BodyClass>>,
    mut world: ResMut<WorldPositions>,
) {
    let locals: HashMap<Entity, (DVec3, Option<Entity>)> = bodies
        .iter()
        .filter_map(|(body, position, child_of)| {
            Some((body, (position?.0, child_of.map(ChildOf::parent))))
        })
        .collect();
    let km = world_positions_km(&locals);
    world.0.clear();
    for (body, _, _) in &bodies {
        let render = km.get(&body).map_or(Vec3::ZERO, |&p| km_to_render_vec3(p));
        world.0.insert(body, render);
    }
}

/// ARCH-11: the simulated km position, written to `Transform` each frame.
fn project_orbit_positions(mut bodies: Query<(&OrbitPositionKm, &mut Transform)>) {
    for (position, mut transform) in &mut bodies {
        transform.translation = km_to_render_vec3(position.0);
    }
}

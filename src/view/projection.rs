//! The view side of ARCH-11: turns each body's simulated position in km
//! into a render-space `Transform`, through `scale.rs` (ARCH-12). The one
//! place a simulation position becomes something the camera can see.
//!
//! Render space is centred on the frame body (SCL-06): its position is
//! subtracted from every other in `f64` km before the conversion, so the
//! view keeps full precision near it wherever it is in the system.
//! Satellites' positions are relative to their parent, so their transforms
//! are local offsets and Bevy's propagation adds the parent's position;
//! only bodies with no parent are shifted by the origin.

use bevy::prelude::*;

use crate::sim::load::SystemLoadApp;

use std::collections::HashMap;

use bevy::math::DVec3;

use super::frame::Frame;
use super::scale::km_to_render_vec3;
use super::world::{RenderOrigin, WorldPositions, world_positions_km};
use crate::sim::body::BodyClass;
use crate::sim::orbit::OrbitPositionKm;
use crate::{Boot, Stage};

pub struct ProjectionPlugin;

impl Plugin for ProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.reset_with_system::<WorldPositions>()
            .reset_with_system::<RenderOrigin>()
            .init_resource::<Frame>()
            .add_load_systems(Boot::Derive, attach_transforms)
            .add_systems(
                Update,
                (update_world_positions, project_orbit_positions)
                    .chain()
                    .in_set(Stage::Projection),
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

/// CAM-05 / SCL-06: every body's position this frame, relative to the
/// frame body, from the simulation's km positions - not last frame's
/// `GlobalTransform`.
fn update_world_positions(
    bodies: Query<BodyPlacement, With<BodyClass>>,
    frame: Res<Frame>,
    mut world: ResMut<WorldPositions>,
    mut origin: ResMut<RenderOrigin>,
) {
    let locals: HashMap<Entity, (DVec3, Option<Entity>)> = bodies
        .iter()
        .filter_map(|(body, position, child_of)| {
            Some((body, (position?.0, child_of.map(ChildOf::parent))))
        })
        .collect();
    let km = world_positions_km(&locals);
    let origin_km = km.get(&frame.body).copied().unwrap_or(DVec3::ZERO);
    *origin = next_origin(&origin, origin_km);
    world.0.clear();
    for (body, _, _) in &bodies {
        let star_relative = km.get(&body).copied().unwrap_or(DVec3::ZERO);
        world
            .0
            .insert(body, km_to_render_vec3(star_relative - origin_km));
    }
}

/// A body's local position, what it orbits, and its render transform.
type BodyTransform = (
    Option<&'static OrbitPositionKm>,
    Option<&'static ChildOf>,
    &'static mut Transform,
);

/// SCL-06: the origin at `km`, and how far it moved since `previous`.
fn next_origin(previous: &RenderOrigin, km: DVec3) -> RenderOrigin {
    RenderOrigin {
        km,
        star: km_to_render_vec3(-km),
        shift: km_to_render_vec3(previous.km - km),
    }
}

/// ARCH-11: the simulated km position, written to `Transform` each frame.
/// A body with no parent is placed from the star, less the origin; the
/// star itself sits at minus the origin (SCL-06).
fn project_orbit_positions(
    origin: Res<RenderOrigin>,
    mut bodies: Query<BodyTransform, With<BodyClass>>,
) {
    for (position, child_of, mut transform) in &mut bodies {
        let local = position.map_or(DVec3::ZERO, |p| p.0);
        let from = if child_of.is_some() {
            DVec3::ZERO
        } else {
            origin.km
        };
        transform.translation = km_to_render_vec3(local - from);
    }
}

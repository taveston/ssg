//! The belt's translucent shaded band (MRK-05), the one piece of real 3D
//! geometry in the system view. Sized from the simulation's `Belt` record
//! through `scale.rs`.

use bevy::color::palettes::css::SLATE_GRAY;
use bevy::prelude::*;

use super::scale::km_to_render;
use super::world::RenderOrigin;
use crate::sim::body::Belt;
use crate::{Boot, Stage};

/// MRK-05: the band is faint enough that orbits and markers read through it.
const BELT_ALPHA: f32 = 0.18;
/// Segments round the annulus; enough that it reads as round at any zoom.
const BELT_RESOLUTION: u32 = 128;

pub struct BeltPlugin;

impl Plugin for BeltPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_belt_mesh.in_set(Boot::Derive))
            .add_systems(Update, centre_belt_on_star.in_set(Stage::Camera));
    }
}

/// The belt circles the star, which sits wherever the floating origin
/// puts it this frame (SCL-06).
fn centre_belt_on_star(origin: Res<RenderOrigin>, mut belts: Query<&mut Transform, With<Belt>>) {
    for mut transform in &mut belts {
        transform.translation = origin.star;
    }
}

fn add_belt_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    belts: Query<(Entity, &Belt)>,
) {
    for (entity, belt) in &belts {
        let inner = km_to_render(belt.inner_radius_km);
        let outer = km_to_render(belt.outer_radius_km);
        commands.entity(entity).insert((
            Mesh3d(
                meshes.add(
                    Annulus::new(inner, outer)
                        .mesh()
                        .resolution(BELT_RESOLUTION),
                ),
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::from(SLATE_GRAY).with_alpha(BELT_ALPHA),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            })),
            // An annulus mesh lies in XY; the orbital plane is XZ.
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
    }
}

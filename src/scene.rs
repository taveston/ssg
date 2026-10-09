//! Spawns the star, planets, moon, and gates from `SystemData`.

use bevy::prelude::*;

use crate::camera::on_body_clicked;
use crate::orbits::OrbitalBody;
use crate::scale::{au_to_render, km_to_render_radius, moon_orbit_render, polar_au_to_render};
use crate::system_data::SystemData;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_system);
    }
}

fn spawn_system(
    mut commands: Commands,
    system_data: Res<SystemData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let star = &system_data.star;
    let star_radius = km_to_render_radius(star.radius_km);

    commands
        .spawn((
            Name::new(star.name),
            Mesh3d(meshes.add(Sphere::new(star_radius))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: star.color,
                // Always reads as bright regardless of its own light's falloff.
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .observe(on_body_clicked);

    commands.spawn((
        Name::new("Star Light"),
        PointLight {
            intensity: 40_000_000.0,
            range: 150.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    for planet in &system_data.planets {
        let planet_radius = km_to_render_radius(planet.radius_km);
        let orbit_radius = au_to_render(planet.orbit_radius_au);

        let planet_entity = commands
            .spawn((
                Name::new(planet.name),
                Mesh3d(meshes.add(Sphere::new(planet_radius))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: planet.color,
                    ..default()
                })),
                Transform::from_xyz(orbit_radius, 0.0, 0.0),
                OrbitalBody {
                    orbit_radius,
                    period_days: planet.period_days,
                    phase0_rad: 0.0,
                },
            ))
            .observe(on_body_clicked)
            .id();

        if let Some(moon) = &planet.moon {
            let moon_radius = km_to_render_radius(moon.radius_km);
            let moon_orbit_radius = moon_orbit_render(moon.orbit_radius_km, planet_radius);

            commands
                .spawn((
                    Name::new(moon.name),
                    Mesh3d(meshes.add(Sphere::new(moon_radius))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: moon.color,
                        ..default()
                    })),
                    Transform::from_xyz(moon_orbit_radius, 0.0, 0.0),
                    OrbitalBody {
                        orbit_radius: moon_orbit_radius,
                        period_days: moon.period_days,
                        phase0_rad: 0.0,
                    },
                    ChildOf(planet_entity),
                ))
                .observe(on_body_clicked);
        }
    }

    for gate in &system_data.gates {
        let position = polar_au_to_render(gate.distance_au, gate.angle_deg);

        commands
            .spawn((
                Name::new(gate.name),
                Mesh3d(meshes.add(Cuboid::new(0.6, 0.6, 0.6))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.9, 0.9),
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(position)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)),
            ))
            .observe(on_body_clicked);
    }
}

//! Spawns the Corvane system as invisible, positioned entities: a star, a
//! tree of planets/moons/stations, belt salvage sites, and gates. Nothing
//! here is rendered directly - `markers.rs` draws the vector-display
//! overlay from these entities' projected screen positions, and the belt's
//! translucent annulus mesh is the one piece of actual 3D geometry.

use bevy::color::palettes::css::SLATE_GRAY;
use bevy::prelude::*;

use crate::grouping::SatelliteFoldState;
use crate::orbits::{Orbit, OrbitPositionKm, orbit_position_km};
use crate::scale::{km_to_render, km_to_render_vec3};
use crate::system_data::{
    BodyBlueprint, BodyClass, Description, DisplayName, KeyFacts, Knowledge, Ownership, SystemData,
    SystemDataResource, period_days_for,
};

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_system);
    }
}

pub fn spawn_system(
    mut commands: Commands,
    mut system_data: ResMut<SystemDataResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(system_data) = system_data.0.take() else {
        return;
    };
    let SystemData {
        star_name,
        star_mass_kg,
        star_knowledge,
        planets,
        belt,
        gates,
    } = system_data;

    commands.spawn((
        Name::new(star_name),
        DisplayName(star_name.to_string()),
        Description("The system's star.".to_string()),
        KeyFacts(vec![("Class".to_string(), "K2V".to_string())]),
        BodyClass::Star,
        Ownership::Neutral,
        star_knowledge,
        Transform::IDENTITY,
    ));

    for planet in planets {
        spawn_body(&mut commands, planet, None, star_mass_kg);
    }

    for site in belt.salvage_sites {
        spawn_body(&mut commands, site, None, star_mass_kg);
    }

    let inner = km_to_render(belt.inner_radius_km);
    let outer = km_to_render(belt.outer_radius_km);
    commands.spawn((
        Name::new("Belt"),
        Mesh3d(meshes.add(Annulus::new(inner, outer).mesh().resolution(128))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::from(SLATE_GRAY).with_alpha(0.18),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    for gate in gates {
        let orbit = Orbit {
            radius_km: gate.distance_km,
            period_days: period_days_for(gate.distance_km, star_mass_kg),
            phase0_rad: gate.angle_deg.to_radians(),
        };
        commands.spawn((
            Name::new(gate.name),
            DisplayName(gate.name.to_string()),
            Description(format!(
                "A gate linking to the {} system.",
                gate.destination
            )),
            KeyFacts(vec![(
                "Destination".to_string(),
                gate.destination.to_string(),
            )]),
            BodyClass::Gate,
            Ownership::Neutral,
            Knowledge::Surveyed { imaged_day: 0.0 },
            orbit_components(orbit),
        ));
    }
}

/// An orbiting body's simulation state at the epoch, plus a `Transform`
/// already matching it so the first frame draws everything in place rather
/// than at the origin.
fn orbit_components(orbit: Orbit) -> (Orbit, OrbitPositionKm, Transform) {
    let position = orbit_position_km(&orbit, 0.0);
    (
        orbit,
        OrbitPositionKm(position),
        Transform::from_translation(km_to_render_vec3(position)),
    )
}

fn spawn_body(
    commands: &mut Commands,
    blueprint: BodyBlueprint,
    parent: Option<Entity>,
    central_mass_kg: f64,
) -> Entity {
    let BodyBlueprint {
        name,
        class,
        ownership,
        description,
        key_facts,
        knowledge,
        orbit_radius_km,
        phase0_rad,
        mass_kg,
        children,
    } = blueprint;

    let orbit = Orbit {
        radius_km: orbit_radius_km,
        period_days: period_days_for(orbit_radius_km, central_mass_kg),
        phase0_rad,
    };

    let mut entity_commands = commands.spawn((
        Name::new(name),
        DisplayName(name.to_string()),
        Description(description.to_string()),
        KeyFacts(
            key_facts
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ),
        class,
        ownership,
        knowledge,
        orbit_components(orbit),
    ));
    if let Some(parent) = parent {
        // Anything spawned under a parent is a satellite, and so subject to
        // grouping. Everything starts folded - the opening view is the
        // whole system, where no satellite is drawn individually.
        entity_commands.insert((ChildOf(parent), SatelliteFoldState::default()));
    }
    let entity = entity_commands.id();

    for child in children {
        spawn_body(commands, child, Some(entity), mass_kg);
    }

    entity
}

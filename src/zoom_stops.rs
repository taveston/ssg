//! Zoom stops, computed per system from its actual orbits rather than from
//! fixed scales (`docs/spec/system-view.md`, ZOOM-15).
//!
//! Stops are stored as *render-space radii*, not camera distances: the
//! conversion depends on the camera's current pitch, which the player can
//! change by right-dragging, so it has to happen at the moment a stop is
//! used.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::orbits::Orbit;
use crate::scale::km_to_render;
use crate::system_data::BodyClass;

#[derive(Resource, Default, Debug)]
pub struct ZoomStops {
    /// All planetary orbits fit. Gates are deliberately excluded so they
    /// stay edge-of-screen indicators instead of dragging the frame out.
    pub system_radius: f32,
    /// The inner system: every planetary orbit but the outermost.
    pub inner_system_radius: f32,
    /// Per parent body, the radius that fits all of its satellites.
    pub per_body_satellite_radius: HashMap<Entity, f32>,
}

impl ZoomStops {
    /// The satellite-frame radius for a body, if it has satellites.
    pub fn satellite_radius(&self, body: Entity) -> Option<f32> {
        self.per_body_satellite_radius.get(&body).copied()
    }
}

/// The camera distance at which a flat circle of `radius` centred on the
/// focus point just fits the viewport.
///
/// A circle in the XZ plane seen at elevation `pitch` projects to an
/// ellipse whose vertical semi-axis is `radius * sin(pitch)`; at typical
/// window aspect ratios and this project's steep default pitch, that
/// vertical extent is what binds, so it's what we fit against.
pub fn distance_for_radius(radius: f32, fov_y: f32, pitch: f32, margin: f32) -> f32 {
    // Floor the compression so a near-edge-on view can't demand a
    // zero (or absurdly small) distance.
    let vertical_extent = radius * pitch.sin().abs().max(0.2);
    vertical_extent / (margin * (fov_y * 0.5).tan()).max(f32::EPSILON)
}

/// ZOOM-15 (replaced ZOOM-04): the system frame fits every planetary
/// orbit; the region frame
/// is the inner system, i.e. every planetary orbit but the outermost.
/// Both come from the system's real orbits, never from fixed scales.
///
/// Gates are deliberately not passed in: they stay edge indicators rather
/// than dragging the system frame out to their distance.
pub fn frame_radii(planet_orbit_radii: &mut [f32]) -> (f32, f32) {
    planet_orbit_radii.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let system = planet_orbit_radii.last().copied().unwrap_or(1.0);
    let inner = planet_orbit_radii
        .get(planet_orbit_radii.len().saturating_sub(2))
        .copied()
        .unwrap_or(system);
    (system, inner)
}

/// The inverse: how much of the orbital plane fits at a given distance.
pub fn radius_for_distance(distance: f32, fov_y: f32, pitch: f32, margin: f32) -> f32 {
    distance * margin * (fov_y * 0.5).tan() / pitch.sin().abs().max(0.2)
}

pub struct ZoomStopsPlugin;

impl Plugin for ZoomStopsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoomStops>()
            .add_systems(Startup, compute_stops.after(crate::scene::spawn_system));
    }
}

pub fn compute_stops(
    mut stops: ResMut<ZoomStops>,
    bodies: Query<(Entity, &Orbit, &BodyClass, Option<&ChildOf>)>,
) {
    let mut planet_radii: Vec<f32> = Vec::new();
    let mut per_parent: HashMap<Entity, f32> = HashMap::new();

    for (entity, orbital, class, child_of) in &bodies {
        match child_of {
            Some(child_of) => {
                let entry = per_parent.entry(child_of.parent()).or_insert(0.0);
                *entry = entry.max(km_to_render(orbital.radius_km));
            }
            None => {
                if *class == BodyClass::Planet {
                    planet_radii.push(km_to_render(orbital.radius_km));
                }
            }
        }
        let _ = entity;
    }

    let (system_radius, inner_system_radius) = frame_radii(&mut planet_radii);
    stops.system_radius = system_radius;
    stops.inner_system_radius = inner_system_radius;
    stops.per_body_satellite_radius = per_parent;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_data::{KM_PER_AU, hand_made_system};

    /// The planetary orbit radii of the real test system, in render units.
    fn corvane_planet_radii() -> Vec<f32> {
        hand_made_system()
            .planets
            .iter()
            .map(|planet| km_to_render(planet.orbit_radius_km))
            .collect()
    }

    #[test]
    fn zoom_15_system_frame_fits_every_planetary_orbit() {
        let mut radii = corvane_planet_radii();
        let (system, _) = frame_radii(&mut radii);
        for radius in &radii {
            assert!(
                *radius <= system + f32::EPSILON,
                "{radius} should fit inside the system frame {system}"
            );
        }
        // Morrow, the outermost planet, defines it.
        assert!((system - km_to_render(4.3 * KM_PER_AU)).abs() < 0.01);
    }

    /// ZOOM-15: gates are excluded on purpose - they stay edge indicators
    /// rather than stretching the frame out to ~6 AU.
    #[test]
    fn zoom_15_system_frame_excludes_gates() {
        let system_data = hand_made_system();
        let mut radii = corvane_planet_radii();
        let (system, _) = frame_radii(&mut radii);
        for gate in &system_data.gates {
            assert!(
                km_to_render(gate.distance_km) > system,
                "gates should sit outside the system frame"
            );
        }
    }

    #[test]
    fn zoom_15_region_frame_is_the_inner_system() {
        let mut radii = corvane_planet_radii();
        let (system, inner) = frame_radii(&mut radii);
        assert!(
            inner < system,
            "the region frame is tighter than the system"
        );
        // Halden IV at 2 AU is the outermost of the inner planets.
        assert!((inner - km_to_render(2.0 * KM_PER_AU)).abs() < 0.01);
    }

    #[test]
    fn zoom_15_degenerate_systems_still_give_a_frame() {
        assert_eq!(frame_radii(&mut []), (1.0, 1.0));
        assert_eq!(frame_radii(&mut [7.0]), (7.0, 7.0));
    }

    /// A radius converted to a camera distance and back is the same radius,
    /// so a stop frames exactly what it says it does.
    #[test]
    fn zoom_15_distance_and_radius_round_trip() {
        let fov = std::f32::consts::FRAC_PI_4;
        let pitch = 1.15;
        let margin = 0.8;
        for radius in [0.4_f32, 12.0, 180.0, 387.0] {
            let distance = distance_for_radius(radius, fov, pitch, margin);
            let back = radius_for_distance(distance, fov, pitch, margin);
            assert!(
                (back - radius).abs() < radius * 1e-4,
                "{radius} -> {distance} -> {back}"
            );
        }
    }
}

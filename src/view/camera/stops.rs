//! Zoom stops, computed per system from its actual orbits rather than from
//! fixed scales (`docs/spec/system-view.md`, ZOOM-15).
//!
//! Stops are stored as *render-space radii*, not camera distances: the
//! conversion depends on the camera's current tilt, which the player can
//! change by right-dragging, so it has to happen at the moment a stop is
//! used.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::Boot;
use crate::sim::body::BodyClass;
use crate::sim::orbit::Orbit;
use crate::view::scale::km_to_render;

#[derive(Resource, Default, Debug)]
pub struct ZoomStops {
    /// All planetary orbits fit. Gates are deliberately excluded so they
    /// stay edge-of-screen indicators instead of dragging the frame out.
    pub system_radius: f32,
    /// Per parent body, the radius that fits all of its satellites.
    pub per_body_satellite_radius: HashMap<Entity, f32>,
}

impl ZoomStops {
    /// The satellite-frame radius for a body, if it has satellites.
    pub fn satellite_radius(&self, body: Entity) -> Option<f32> {
        self.per_body_satellite_radius.get(&body).copied()
    }

    /// ZOOM-16: the radius a focus jump frames: the body's own satellite
    /// frame, else its parent's. `None` means the body's inner limit.
    pub fn framing_radius(&self, body: Entity, parent: Option<Entity>) -> Option<f32> {
        self.satellite_radius(body)
            .or_else(|| parent.and_then(|parent| self.satellite_radius(parent)))
    }
}

/// The flattest a framed circle is allowed to look, as cos(tilt), so a
/// near-edge-on view can't demand a zero (or absurdly small) distance.
const MIN_TILT_COMPRESSION: f32 = 0.2;

/// The camera distance at which a flat circle of `radius` centred on the
/// pivot just fits the viewport.
///
/// A circle in the XZ plane seen at `tilt` from straight down projects to
/// an ellipse whose vertical semi-axis is `radius * cos(tilt)`; at typical
/// window aspect ratios that vertical extent is what binds, so it's what
/// we fit against.
pub fn distance_for_radius(radius: f32, fov_y: f32, tilt: f32, margin: f32) -> f32 {
    // Floor the compression so a near-edge-on view can't demand a
    // zero (or absurdly small) distance.
    let vertical_extent = radius * tilt.cos().abs().max(MIN_TILT_COMPRESSION);
    vertical_extent / (margin * (fov_y * 0.5).tan()).max(f32::EPSILON)
}

/// ZOOM-15: the system frame fits every planetary orbit, from the system's
/// real orbits, never from fixed scales.
///
/// Gates are deliberately not passed in: they stay edge indicators rather
/// than dragging the system frame out to their distance.
pub fn system_frame_radius(planet_orbit_radii: &[f32]) -> f32 {
    planet_orbit_radii.iter().copied().fold(1.0, f32::max)
}

pub struct StopsPlugin;

impl Plugin for StopsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoomStops>()
            .add_systems(Startup, compute_stops.in_set(Boot::Derive));
    }
}

/// ZOOM-15: the stops for this system, from its real orbits.
fn compute_stops(
    mut stops: ResMut<ZoomStops>,
    bodies: Query<(Entity, &Orbit, &BodyClass, Option<&ChildOf>)>,
) {
    let mut planet_radii: Vec<f32> = Vec::new();
    let mut per_parent: HashMap<Entity, f32> = HashMap::new();

    for (_, orbital, class, child_of) in &bodies {
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
    }

    stops.system_radius = system_frame_radius(&planet_radii);
    stops.per_body_satellite_radius = per_parent;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::corvane::hand_made_system;
    use crate::sim::units::KM_PER_AU;

    /// The planetary orbit radii of the real test system, in render units.
    fn corvane_planet_radii() -> Vec<f32> {
        hand_made_system()
            .planets
            .iter()
            .map(|planet| km_to_render(planet.orbit_radius_km))
            .collect()
    }

    /// Covers ZOOM-15.
    #[test]
    fn system_frame_fits_every_planetary_orbit() {
        let radii = corvane_planet_radii();
        let system = system_frame_radius(&radii);
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
    fn system_frame_excludes_gates() {
        let system_data = hand_made_system();
        let system = system_frame_radius(&corvane_planet_radii());
        for gate in &system_data.gates {
            assert!(
                km_to_render(gate.distance_km) > system,
                "gates should sit outside the system frame"
            );
        }
    }

    /// ZOOM-16: a body with satellites frames them; a moon frames its
    /// planet's; a bare planet has no frame of its own (its inner limit
    /// applies instead).
    #[test]
    fn framing_falls_back_to_the_parent_then_to_nothing() {
        let mut world = World::new();
        let [planet, moon, bare] = [(); 3].map(|_| world.spawn_empty().id());
        let mut stops = ZoomStops::default();
        stops.per_body_satellite_radius.insert(planet, 0.44);
        assert_eq!(stops.framing_radius(planet, None), Some(0.44));
        assert_eq!(stops.framing_radius(moon, Some(planet)), Some(0.44));
        assert_eq!(stops.framing_radius(bare, None), None);
    }

    /// Covers ZOOM-15.
    #[test]
    fn degenerate_systems_still_give_a_frame() {
        assert_eq!(system_frame_radius(&[]), 1.0);
        assert_eq!(system_frame_radius(&[7.0]), 7.0);
    }

    /// A stop frames exactly what it says: the framed radius fills
    /// `margin` of the view's half-height at that distance, at any tilt.
    /// Covers ZOOM-15.
    #[test]
    fn a_framed_radius_fills_the_margin() {
        let fov = std::f32::consts::FRAC_PI_4;
        let margin = 0.8;
        for tilt in [0.0_f32, 0.42] {
            for radius in [0.4_f32, 12.0, 180.0, 387.0] {
                let distance = distance_for_radius(radius, fov, tilt, margin);
                let half_height = distance * (fov / 2.0).tan();
                let fill = radius * tilt.cos() / half_height;
                assert!((fill - margin).abs() < 1e-4, "{radius} at {tilt}: {fill}");
            }
        }
    }
}

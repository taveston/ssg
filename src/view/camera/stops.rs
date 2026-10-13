//! Zoom stops, computed per system from its actual orbits rather than from
//! fixed scales (`docs/spec/system-view.md`, ZOOM-15).
//!
//! Stops are stored as *render-space radii*, not camera distances: the
//! conversion depends on the camera's current tilt, which the player can
//! change by right-dragging, so it has to happen at the moment a stop is
//! used.

use std::collections::HashMap;

use bevy::prelude::*;

use super::rules::camera_rotation;
use crate::Boot;
use crate::sim::body::BodyClass;
use crate::sim::orbit::Orbit;
use crate::view::scale::km_to_render;
use crate::view::screen::MapArea;

#[derive(Resource, Default, Debug)]
pub struct ZoomStops {
    /// All planetary orbits fit. Gates are deliberately excluded so they
    /// stay edge-of-screen indicators instead of dragging the fit out.
    pub system_radius: f32,
    /// Per parent body, the radius that fits all of its satellites.
    pub per_body_satellite_radius: HashMap<Entity, f32>,
}

impl ZoomStops {
    /// The satellite-fit radius for a body, if it has satellites.
    pub fn satellite_radius(&self, body: Entity) -> Option<f32> {
        self.per_body_satellite_radius.get(&body).copied()
    }

    /// ZOOM-16: the radius a frame jump fits: the body's own satellite fit,
    /// else its parent's. `None` means the body's inner limit.
    pub fn jump_radius(&self, body: Entity, parent: Option<Entity>) -> Option<f32> {
        self.satellite_radius(body)
            .or_else(|| parent.and_then(|parent| self.satellite_radius(parent)))
    }
}

/// Points sampled round a circle when fitting it to the view.
const FIT_SAMPLES: usize = 180;

/// The camera distance at which a flat circle of `radius` centred on the
/// pivot fits the map area of a `viewport`-sized window, filling at most
/// `margin` of it.
///
/// The circle lies in the orbital plane and is seen from `tilt` (from
/// straight down) through a perspective camera looking at its centre. Each
/// sampled point must project inside the map area (below the top bar) on
/// both axes; with the pivot on the camera's axis each limit is linear in
/// the distance, so the answer is the largest of them. Using both axes and
/// the perspective, not just the circle's height, keeps the near and far
/// sides of a wide, shallow view on screen.
pub fn distance_for_radius(radius: f32, fov_y: f32, tilt: f32, margin: f32, viewport: Vec2) -> f32 {
    let half_tan = (fov_y * 0.5).tan();
    let aspect = viewport.x / viewport.y.max(1.0);
    // Normalised screen limits: the bottom and sides at `margin`, the top
    // pulled down by the top bar.
    let up_limit = margin * MapArea::of(viewport).size.y / viewport.y.max(1.0);
    let into_camera = camera_rotation(0.0, tilt).inverse();
    let mut distance = f32::EPSILON;
    for i in 0..FIT_SAMPLES {
        let angle = std::f32::consts::TAU * i as f32 / FIT_SAMPLES as f32;
        let local = into_camera * Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
        let y_limit = if local.y > 0.0 { up_limit } else { margin };
        let needed = [
            local.x.abs() / (margin * half_tan * aspect),
            local.y.abs() / (y_limit * half_tan),
        ];
        for lateral in needed {
            distance = distance.max(local.z + lateral);
        }
    }
    distance
}

/// ZOOM-15: the system fit takes in every planetary orbit, from the system's
/// real orbits, never from fixed scales.
///
/// Gates are deliberately not passed in: they stay edge indicators rather
/// than dragging the system fit out to their distance.
pub fn system_fit_radius(planet_orbit_radii: &[f32]) -> f32 {
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

    stops.system_radius = system_fit_radius(&planet_radii);
    stops.per_body_satellite_radius = per_parent;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::corvane::hand_made_system;
    use crate::sim::units::KM_PER_AU;
    use crate::view::camera::rules::project_plane_point;

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
    fn system_fit_takes_in_every_planetary_orbit() {
        let radii = corvane_planet_radii();
        let system = system_fit_radius(&radii);
        for radius in &radii {
            assert!(
                *radius <= system + f32::EPSILON,
                "{radius} should fit inside the system fit {system}"
            );
        }
        // Morrow, the outermost planet, defines it.
        assert!((system - km_to_render(4.3 * KM_PER_AU)).abs() < 0.01);
    }

    /// ZOOM-15: gates are excluded on purpose - they stay edge indicators
    /// rather than stretching the fit out to ~6 AU.
    #[test]
    fn system_fit_excludes_gates() {
        let system_data = hand_made_system();
        let system = system_fit_radius(&corvane_planet_radii());
        for gate in &system_data.gates {
            assert!(
                km_to_render(gate.distance_km) > system,
                "gates should sit outside the system fit"
            );
        }
    }

    /// ZOOM-16: a jump to a body with satellites fits them; to a moon, its
    /// planet's; a bare planet has no fit of its own (its inner limit
    /// applies instead).
    #[test]
    fn jumps_fall_back_to_the_parent_then_to_nothing() {
        let mut world = World::new();
        let [planet, moon, bare] = [(); 3].map(|_| world.spawn_empty().id());
        let mut stops = ZoomStops::default();
        stops.per_body_satellite_radius.insert(planet, 0.44);
        assert_eq!(stops.jump_radius(planet, None), Some(0.44));
        assert_eq!(stops.jump_radius(moon, Some(planet)), Some(0.44));
        assert_eq!(stops.jump_radius(bare, None), None);
    }

    /// Covers ZOOM-15.
    #[test]
    fn degenerate_systems_still_give_a_fit() {
        assert_eq!(system_fit_radius(&[]), 1.0);
        assert_eq!(system_fit_radius(&[7.0]), 7.0);
    }

    /// A stop fits exactly what it says: at the computed distance every
    /// point of the fitted circle is inside the map area, and the circle
    /// reaches the limit on some side. Checked at several tilts, including
    /// shallow ones where the width binds, and in a narrow window.
    /// Covers ZOOM-15.
    #[test]
    fn a_fitted_circle_fits_the_map_area() {
        let fov = std::f32::consts::FRAC_PI_4;
        let margin = 0.8;
        for viewport in [Vec2::new(1600.0, 900.0), Vec2::new(700.0, 900.0)] {
            for tilt in [0.0_f32, 0.42, 1.05, 1.4] {
                for radius in [0.4_f32, 387.0] {
                    let distance = distance_for_radius(radius, fov, tilt, margin, viewport);
                    let mut outermost = 0.0_f32;
                    for i in 0..360 {
                        let angle = (i as f32).to_radians();
                        let point = Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
                        let screen = project_plane_point(
                            point,
                            (Vec3::ZERO, 0.0, tilt, distance),
                            fov,
                            viewport,
                        )
                        .expect("in front of the camera");
                        let area = MapArea::of(viewport);
                        let centre = viewport / 2.0;
                        let x = (screen.x - centre.x).abs() / (viewport.x / 2.0);
                        let (top, bottom) = (
                            (screen.y - area.offset.y) / area.size.y,
                            screen.y / viewport.y,
                        );
                        assert!(x <= margin + 1e-3, "x {x} at tilt {tilt}");
                        assert!(top >= -1e-3 && bottom <= 0.5 + margin / 2.0 + 1e-3);
                        outermost = outermost.max(x / margin);
                    }
                    assert!(outermost > 0.0);
                }
            }
        }
    }
}

//! Zoom stops, computed per system from its actual orbits rather than from
//! fixed scales (`docs/presentation.md` section 2, "Navigation and zoom").
//!
//! Stops are stored as *render-space radii*, not camera distances: the
//! conversion depends on the camera's current pitch, which the player can
//! change by right-dragging, so it has to happen at the moment a stop is
//! used.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::orbits::OrbitalBody;
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
    bodies: Query<(Entity, &OrbitalBody, &BodyClass, Option<&ChildOf>)>,
) {
    let mut planet_radii: Vec<f32> = Vec::new();
    let mut per_parent: HashMap<Entity, f32> = HashMap::new();

    for (entity, orbital, class, child_of) in &bodies {
        match child_of {
            Some(child_of) => {
                let entry = per_parent.entry(child_of.parent()).or_insert(0.0);
                *entry = entry.max(orbital.orbit_radius);
            }
            None => {
                if *class == BodyClass::Planet {
                    planet_radii.push(orbital.orbit_radius);
                }
            }
        }
        let _ = entity;
    }

    planet_radii.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    stops.system_radius = planet_radii.last().copied().unwrap_or(1.0);
    stops.inner_system_radius = planet_radii
        .get(planet_radii.len().saturating_sub(2))
        .copied()
        .unwrap_or(stops.system_radius);
    stops.per_body_satellite_radius = per_parent;
}

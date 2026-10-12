//! The camera distances that mean something to zoom and focus: the system
//! frame, a body's satellite frame, and the inner limit (ZOOM-14 to
//! ZOOM-16). Gathers the camera's field of view, the map area and the
//! bodies' radii so `zoom_input.rs` and `actions.rs` ask one place.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::state::{MAX_DISTANCE, MIN_DISTANCE, OrbitCamera, camera_fov};
use super::stops::{ZoomStops, distance_for_radius};
use crate::sim::body::PhysicalRadius;
use crate::view::scale::km_to_render;
use crate::view::screen::MapArea;
use crate::view::true_size::distance_for_screen_fraction;
use crate::view::tunables::Tunables;

/// Used when no window exists yet (a headless start-up frame).
const FALLBACK_VIEWPORT: Vec2 = Vec2::new(1600.0, 900.0);

#[derive(SystemParam)]
pub struct Frames<'w, 's> {
    tunables: Res<'w, Tunables>,
    stops: Res<'w, ZoomStops>,
    projections: Query<'w, 's, &'static Projection, With<OrbitCamera>>,
    windows: Query<'w, 's, &'static Window>,
    radii: Query<'w, 's, &'static PhysicalRadius>,
    parents: Query<'w, 's, &'static ChildOf>,
}

impl Frames<'_, '_> {
    fn fov(&self) -> f32 {
        self.projections
            .single()
            .map_or(std::f32::consts::FRAC_PI_4, camera_fov)
    }

    fn viewport(&self) -> Vec2 {
        self.windows.single().map_or(FALLBACK_VIEWPORT, |window| {
            Vec2::new(window.width(), window.height())
        })
    }

    /// The camera distance at which a circle of `radius` round the pivot
    /// fits the view at `tilt`.
    pub fn fit(&self, radius: f32, tilt: f32) -> f32 {
        distance_for_radius(radius, self.fov(), tilt, self.tunables.zoom_fit_margin)
    }

    /// ZOOM-15: the system frame, the outer limit.
    pub fn system(&self, tilt: f32) -> f32 {
        self.fit(self.stops.system_radius, tilt).min(MAX_DISTANCE)
    }

    /// ZOOM-15: the focused body's satellite frame, if it has satellites.
    pub fn satellite(&self, body: Entity, tilt: f32) -> Option<f32> {
        Some(self.fit(self.stops.satellite_radius(body)?, tilt))
    }

    /// ZOOM-14: the inner limit. With a body focused, where its true
    /// diameter spans `body_max_screen_fraction` of the map's height; with
    /// nothing, a view radius of `min_view_radius_au`.
    pub fn inner_limit(&self, focused: Option<Entity>, tilt: f32) -> f32 {
        let from_body = focused
            .and_then(|body| self.radii.get(body).ok())
            .map(|radius| {
                let viewport = self.viewport();
                distance_for_screen_fraction(
                    km_to_render(radius.0),
                    self.fov(),
                    viewport.y,
                    MapArea::of(viewport).size.y,
                    self.tunables.body_max_screen_fraction,
                )
            });
        let distance = from_body.unwrap_or_else(|| {
            let radius = km_to_render(
                self.tunables.min_view_radius_au as f64 * crate::sim::units::KM_PER_AU,
            );
            self.fit(radius, tilt)
        });
        distance.max(MIN_DISTANCE)
    }

    /// ZOOM-16: the distance a focus jump to `body` ends at: its satellite
    /// frame, else its parent's, else its inner limit; never beyond the
    /// limits.
    pub fn framing(&self, body: Entity, tilt: f32) -> f32 {
        let parent = self.parents.get(body).ok().map(ChildOf::parent);
        let inner = self.inner_limit(Some(body), tilt);
        match self.stops.framing_radius(body, parent) {
            Some(radius) => self.fit(radius, tilt),
            None => inner,
        }
        .clamp(inner, self.system(tilt).max(inner))
    }
}

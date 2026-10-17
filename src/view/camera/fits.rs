//! The camera distances that mean something to zoom and frame jumps: the
//! system fit, a body's satellite fit, and the inner limit (ZOOM-14 to
//! ZOOM-16). Gathers the camera's field of view, the map area and the
//! bodies' radii so `zoom_input.rs` and `actions.rs` ask one place. In
//! the sector view the outer limit is the sector fit and the inner limit
//! a view radius in light years (SECT-11).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::state::{MAX_DISTANCE, MIN_DISTANCE, OrbitCamera, camera_fov};
use super::stops::{
    FitView, SectorFit, ZoomStops, distance_for_points, distance_for_radius, outer_radius,
};
use crate::sim::body::PhysicalRadius;
use crate::view::map_layer::MapLayer;
use crate::view::scale::{km_to_render, ly_to_render};
use crate::view::screen::{MapArea, MapView};
use crate::view::true_size::distance_for_screen_fraction;
use crate::view::tunables::Tunables;

/// Used when no window exists yet (a headless start-up frame).
pub const FALLBACK_VIEWPORT: Vec2 = Vec2::new(1600.0, 900.0);

#[derive(SystemParam)]
pub struct Fits<'w, 's> {
    tunables: Res<'w, Tunables>,
    stops: Res<'w, ZoomStops>,
    layer: Res<'w, MapLayer>,
    sector: Res<'w, SectorFit>,
    projections: Query<'w, 's, &'static Projection, With<OrbitCamera>>,
    map: MapView<'w, 's>,
    radii: Query<'w, 's, &'static PhysicalRadius>,
    parents: Query<'w, 's, &'static ChildOf>,
}

impl Fits<'_, '_> {
    pub fn fov(&self) -> f32 {
        self.projections
            .single()
            .map_or(std::f32::consts::FRAC_PI_4, camera_fov)
    }

    /// INS-07: the camera's viewport, the window less the sidebar.
    pub fn viewport(&self) -> Vec2 {
        self.map.viewport().unwrap_or(FALLBACK_VIEWPORT)
    }

    /// The camera distance at which a circle of `radius` round the pivot
    /// fits the view at `tilt`.
    pub fn fit(&self, radius: f32, tilt: f32) -> f32 {
        distance_for_radius(
            radius,
            self.fov(),
            tilt,
            self.tunables.zoom_fit_margin,
            self.viewport(),
        )
    }

    /// ZOOM-15: the system fit, the outer limit; in the sector view, the
    /// sector fit, at which every node fits the map area seen from
    /// `heading` and `tilt` (SECT-11). The system fit is round, so the
    /// heading doesn't change it.
    pub fn system_fit(&self, heading: f32, tilt: f32) -> f32 {
        if self.layer.is_sector() {
            let view = FitView {
                heading,
                tilt,
                fov_y: self.fov(),
                margin: self.tunables.zoom_fit_margin,
                viewport: self.viewport(),
            };
            return distance_for_points(&self.sector.points, view).min(MAX_DISTANCE);
        }
        self.fit(self.stops.system_radius, tilt).min(MAX_DISTANCE)
    }

    /// CAM-19 / SECT-10: the radius the pan bound is a multiple of: the
    /// system fit's, or in the sector view the sector fit's.
    pub fn outer_radius(&self) -> f32 {
        outer_radius(*self.layer, &self.stops, &self.sector)
    }

    /// ZOOM-15: a body's satellite fit, if it has satellites.
    pub fn satellite_fit(&self, body: Entity, tilt: f32) -> Option<f32> {
        Some(self.fit(self.stops.satellite_radius(body)?, tilt))
    }

    /// ZOOM-14: the inner limit. In a body's frame (`frame_body`), where its
    /// true diameter spans `body_max_screen_fraction` of the map's height;
    /// in the star's frame (`None`), a view radius of `min_view_radius_au`;
    /// in the sector view, of `sector_min_view_radius_ly` (SECT-11).
    pub fn inner_limit(&self, frame_body: Option<Entity>, tilt: f32) -> f32 {
        if self.layer.is_sector() {
            let radius = ly_to_render(self.tunables.sector_min_view_radius_ly as f64);
            return self.fit(radius, tilt).max(MIN_DISTANCE);
        }
        let from_body = frame_body
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

    /// ZOOM-16: the distance a frame jump to `body` ends at: its satellite
    /// fit, else its parent's, else its inner limit; never beyond the
    /// limits.
    pub fn jump_distance(&self, body: Entity, heading: f32, tilt: f32) -> f32 {
        let parent = self.parents.get(body).ok().map(ChildOf::parent);
        let inner = self.inner_limit(Some(body), tilt);
        match self.stops.jump_radius(body, parent) {
            Some(radius) => self.fit(radius, tilt),
            None => inner,
        }
        .clamp(inner, self.system_fit(heading, tilt).max(inner))
    }
}

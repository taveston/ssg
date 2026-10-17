//! Procedural planet surfaces (`docs/spec/surfaces.md`): every natural
//! body's picture, drawn by one shader from its planet facts, its star and
//! a seed, with no art assets. A look is chosen from the facts; the
//! surface is baked once into a cubemap strip, spread over frames; each
//! frame a cheap pass draws the globe from the bake with the star's
//! light. The inset and the system view draw from the same bake and the
//! same globe pass.
//!
//! - `chooser.rs`: which look the facts select, and the rules the layers
//!   read (temperature, craters, rays, knowledge, resolution).
//! - `star.rs`: what a surface reads of its star.
//! - `seed.rs`: which seed a body's surface is drawn from.
//! - `cube.rs`: the bake's cubemap as a strip of faces.
//! - `bake.rs`: what a bake is of, the cache, the scheduler and the
//!   compute pass; `bake.wesl` is the shader.
//! - `shading.rs`: the globe's material and the per-frame rules (light,
//!   body frame, atmosphere); `globe.wesl` is the shader.
//! - `scene.rs`: what a globe reads of its body, its star and the camera
//!   each frame, shared by the inset and the map.
//! - `inset.rs`: the globe in the inspection inset.
//! - `map.rs`: the globe over a body's true-size disc in the system view.

use bevy::prelude::*;

use crate::Stage;
use crate::sim::load::SystemLoadApp;
use crate::view::inset::inspect::InsetRebuilt;

pub mod bake;
pub mod chooser;
pub mod cube;
pub mod inset;
pub mod map;
pub mod scene;
pub mod seed;
pub mod shading;
pub mod star;

#[cfg(test)]
mod gpu_tests;

/// The cache, the scheduler and the inset's globes. Runs headless; needs
/// `SurfaceRenderPlugin` (or, headless, the materials' asset stores) to
/// draw.
pub struct SurfacesPlugin;

impl Plugin for SurfacesPlugin {
    fn build(&self, app: &mut App) {
        app.reset_with_system::<bake::SurfaceBakes>()
            .reset_with_system::<scene::WarnedNoFacts>()
            .init_resource::<bake::PendingBakeJobs>()
            .init_resource::<bake::BakedFaces>()
            .add_systems(
                Update,
                inset::draw_inset_globes
                    .after(InsetRebuilt)
                    .in_set(Stage::Overlay),
            )
            .add_systems(PostUpdate, bake::advance_bakes);
    }
}

/// The bake's compute pass and the globe's material, for the renderer.
pub struct SurfaceRenderPlugin;

impl Plugin for SurfaceRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((bake::BakeRenderPlugin, shading::GlobeMaterialPlugin));
    }
}

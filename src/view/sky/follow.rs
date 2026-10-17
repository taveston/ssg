//! Keeping the sky in step with the game (SKY-02, SKY-23): its seed
//! follows the loaded sector, and its opacities follow the layer shown.
//!
//! - The sky is made once, when the game starts, from the sector's seed
//!   or the default. Should the loaded sector change later (a sector
//!   loaded at run time), the sky is remade from the new seed: the old
//!   meshes go and new ones take their place. Entering and leaving a
//!   system of the loaded sector changes no seed, so the sky stays.
//! - The sector view dims every star and the glow by `sector_sky_factor`;
//!   the system view shows them whole. The factor is one number in each
//!   material, written only when it changes, so changing layer changes
//!   nothing else in the sky.

use bevy::prelude::*;

use super::draw::{
    SkyAssets, SkyGlow, SkyGlowMaterial, SkySource, SkyStars, SkyStarsMaterial, spawn_sky_from,
};
use super::look::layer_factor;
use super::seed::sky_seed;
use crate::content::start::LoadedSector;
use crate::view::map_layer::MapLayer;
use crate::view::tunables::Tunables;

/// The sky's two mesh entities.
type SkyEntities<'w, 's> = Query<'w, 's, Entity, Or<(With<SkyStars>, With<SkyGlow>)>>;

/// SKY-02: remake the sky when the seed it should be made from is no
/// longer the one it was made from, because the loaded sector changed.
pub fn reseed_sky(
    mut commands: Commands,
    tunables: Res<Tunables>,
    sector: Option<Res<LoadedSector>>,
    source: Option<Res<SkySource>>,
    sky: SkyEntities,
    mut assets: SkyAssets,
) {
    let seed = sky_seed(sector.as_deref().map(|loaded| &loaded.sector), &tunables);
    if source.is_some_and(|source| source.seed == seed) {
        return;
    }
    for entity in &sky {
        commands.entity(entity).despawn();
    }
    spawn_sky_from(&mut commands, seed, &tunables, &mut assets);
}

/// SKY-23: keep both materials' factor at the shown layer's: dimmed in
/// the sector view, whole in the system view. A material is only written
/// when its factor changes.
pub fn dim_for_layer(
    layer: Res<MapLayer>,
    tunables: Res<Tunables>,
    stars: Query<&MeshMaterial3d<SkyStarsMaterial>>,
    glows: Query<&MeshMaterial3d<SkyGlowMaterial>>,
    mut star_materials: ResMut<Assets<SkyStarsMaterial>>,
    mut glow_materials: ResMut<Assets<SkyGlowMaterial>>,
) {
    let factor = layer_factor(*layer, &tunables);
    for handle in &stars {
        let stale = star_materials
            .get(&handle.0)
            .is_some_and(|m| m.stars.factor != factor);
        if stale && let Some(mut material) = star_materials.get_mut(&handle.0) {
            material.stars.factor = factor;
        }
    }
    for handle in &glows {
        let stale = glow_materials
            .get(&handle.0)
            .is_some_and(|m| m.glow.factor != factor);
        if stale && let Some(mut material) = glow_materials.get_mut(&handle.0) {
            material.glow.factor = factor;
        }
    }
}

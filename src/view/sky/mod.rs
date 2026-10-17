//! The sky behind the map (`docs/spec/sky.md`: SKY-01 to SKY-24): a field
//! of stars of many brightnesses and a Milky Way, a denser band of faint
//! stars with a very faint glow and dark lanes, tilted off the orbital
//! plane so it crosses the opening view. It is one sky for the whole
//! game, seeded by the loaded sector (or a default for a hand-made
//! system), drawn at infinity, turning with the camera's heading and tilt
//! and never moving when the view pans or zooms, as the bottom layer of
//! the map in both the system and the sector view, inside the map area
//! only. It is always on and never a target. In the sector view it is
//! dimmed, so it stays quieter than the sector's own stars.
//!
//! - `seed.rs`: which seed the sky is made from.
//! - `generate.rs`: the sky as a plain value from a seed and the tunables.
//! - `band.rs`: the band's orientation and coordinates.
//! - `look.rs`: a star's opacity, size, light and colour.
//! - `glow.rs`: the band's glow, its lanes, mottle and dither.
//! - `projection.rs`: where a direction lands on screen.
//! - `draw.rs`: the meshes, materials and systems; `stars.wesl` and
//!   `glow.wesl` draw them.
//! - `follow.rs`: the sky follows the loaded sector's seed and the layer
//!   shown.

use bevy::asset::embedded_asset;
use bevy::prelude::*;

use crate::view::map_layer::MapLayer;
use crate::{Boot, Stage};

pub mod band;
pub mod draw;
pub mod follow;
pub mod generate;
pub mod glow;
pub mod look;
pub mod projection;
pub mod seed;

/// Makes the sky when the game starts and keeps it clipped to the map
/// area, seeded by the loaded sector and dimmed for the layer shown. Needs
/// `SkyMaterialPlugin` (or, headless, the materials' asset stores) to
/// draw.
pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapLayer>()
            .add_systems(Startup, draw::spawn_sky.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    follow::reseed_sky,
                    draw::clip_sky_to_map,
                    follow::dim_for_layer,
                )
                    .chain()
                    .in_set(Stage::Layout),
            );
    }
}

/// The sky's shaders and materials, for the renderer.
pub struct SkyMaterialPlugin;

impl Plugin for SkyMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "stars.wesl");
        embedded_asset!(app, "glow.wesl");
        app.add_plugins((
            MaterialPlugin::<draw::SkyStarsMaterial>::default(),
            MaterialPlugin::<draw::SkyGlowMaterial>::default(),
        ));
    }
}

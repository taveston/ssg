//! Procedural placeholder imagery for the inspection inset's fidelity
//! states (INS-04). There are no art assets yet, so these are generated in
//! code: a shaded gradient globe for surveyed and seeded grain for
//! unsurveyed. One image
//! per *state*, shared by every body in that state.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::view::colors;

const IMAGE_WIDTH: u32 = 160;
const IMAGE_HEIGHT: u32 = 64;

#[derive(Resource, Clone)]
pub struct FidelityImages {
    pub surveyed: Handle<Image>,
    pub unsurveyed: Handle<Image>,
}

pub struct FidelityPlugin;

impl Plugin for FidelityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_images);
    }
}

fn blank_image() -> Image {
    Image::new_fill(
        Extent3d {
            width: IMAGE_WIDTH,
            height: IMAGE_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// A deterministic pseudo-random value in [0, 1) - no `rand` dependency
/// needed for a bit of static grain.
fn hash01(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(374_761_393)
        ^ y.wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h ^= h >> 13;
    h = h.wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h % 1000) as f32 / 1000.0
}

/// A lit sphere silhouette: `background` outside the circle, a gradient
/// from `shadow` to `highlight` (biased toward the upper-left) inside it.
fn draw_globe(image: &mut Image, shadow: Color, highlight: Color, background: Color) {
    let cx = IMAGE_WIDTH as f32 / 2.0;
    let cy = IMAGE_HEIGHT as f32 / 2.0;
    let radius = IMAGE_HEIGHT as f32 * 0.46;
    let light_center = (cx - radius * 0.4, cy - radius * 0.4);

    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let color = if dist > radius {
                background
            } else {
                let hdx = x as f32 - light_center.0;
                let hdy = y as f32 - light_center.1;
                let hdist = (hdx * hdx + hdy * hdy).sqrt();
                let t = (1.0 - hdist / (radius * 1.7)).clamp(0.0, 1.0);
                shadow.mix(&highlight, t)
            };
            let _ = image.set_color_at(x, y, color);
        }
    }
}

/// A grainy telescope-frame look: seeded static tinted toward `tint`.
fn draw_noise(image: &mut Image, tint: Color) {
    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let n = hash01(x, y, 7);
            let gray = 0.12 + n * 0.22;
            let color = Color::srgb(gray, gray, gray).mix(&tint, 0.4);
            let _ = image.set_color_at(x, y, color);
        }
    }
}

fn generate_images(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut surveyed_image = blank_image();
    draw_globe(
        &mut surveyed_image,
        Color::srgb(0.06, 0.16, 0.24),
        Color::srgb(0.55, 0.72, 0.85),
        colors::PANEL,
    );

    let mut unsurveyed_image = blank_image();
    draw_noise(&mut unsurveyed_image, colors::WARNING);

    commands.insert_resource(FidelityImages {
        surveyed: images.add(surveyed_image),
        unsurveyed: images.add(unsurveyed_image),
    });
}

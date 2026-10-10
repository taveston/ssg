//! The filled marker shapes UI nodes can't draw themselves (MRK-04, F4-07):
//! a triangle for an own ship and a diamond for a foreign contact. Each is a
//! small white image with antialiased edges, generated once at startup and
//! tinted per marker through `ImageNode::color`, so it fades and colours
//! exactly like the other markers.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::Boot;

/// Side of a generated shape image, in pixels. Larger than any marker, so it
/// is only ever scaled down.
const IMAGE_PX: u32 = 32;
/// Samples per pixel side for antialiasing the edges.
const SUPERSAMPLE: u32 = 4;

#[derive(Resource, Clone)]
pub struct MarkerImages {
    pub triangle: Handle<Image>,
    pub diamond: Handle<Image>,
}

/// MRK-04: a triangle pointing up, apex at the top centre and base along
/// the bottom, in a unit square with y down.
pub fn in_triangle(u: f32, v: f32) -> bool {
    (0.0..=1.0).contains(&v) && 2.0 * (u - 0.5).abs() <= v
}

/// MRK-04: a diamond touching the middle of each side of a unit square.
pub fn in_diamond(u: f32, v: f32) -> bool {
    (u - 0.5).abs() + (v - 0.5).abs() <= 0.5
}

/// How much of pixel (x, y) of a `size` square a shape covers, 0 to 1.
pub fn pixel_coverage(shape: fn(f32, f32) -> bool, x: u32, y: u32, size: u32) -> f32 {
    let samples = SUPERSAMPLE * SUPERSAMPLE;
    let inside = (0..samples)
        .filter(|i| {
            let sx = (i % SUPERSAMPLE) as f32 + 0.5;
            let sy = (i / SUPERSAMPLE) as f32 + 0.5;
            let u = (x as f32 + sx / SUPERSAMPLE as f32) / size as f32;
            let v = (y as f32 + sy / SUPERSAMPLE as f32) / size as f32;
            shape(u, v)
        })
        .count();
    inside as f32 / samples as f32
}

fn shape_image(shape: fn(f32, f32) -> bool) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: IMAGE_PX,
            height: IMAGE_PX,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..IMAGE_PX {
        for x in 0..IMAGE_PX {
            let alpha = pixel_coverage(shape, x, y, IMAGE_PX);
            let _ = image.set_color_at(x, y, Color::WHITE.with_alpha(alpha));
        }
    }
    image
}

pub struct MarkerImagesPlugin;

impl Plugin for MarkerImagesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_images.in_set(Boot::Derive));
    }
}

fn generate_images(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(MarkerImages {
        triangle: images.add(shape_image(in_triangle)),
        diamond: images.add(shape_image(in_diamond)),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MRK-04: the triangle points up: wide at the base, a point at the top.
    #[test]
    fn mrk_04_triangle_points_up() {
        assert!(in_triangle(0.5, 0.5));
        assert!(in_triangle(0.05, 0.99), "bottom corners are inside");
        assert!(!in_triangle(0.05, 0.05), "top corners are outside");
        assert!(!in_triangle(0.95, 0.05));
        assert!(in_triangle(0.5, 0.02), "the apex is at the top");
    }

    /// MRK-04: the diamond touches each side's middle and leaves the
    /// corners empty.
    #[test]
    fn mrk_04_diamond_touches_the_sides_and_not_the_corners() {
        for (u, v) in [
            (0.5, 0.01),
            (0.99, 0.5),
            (0.5, 0.99),
            (0.01, 0.5),
            (0.5, 0.5),
        ] {
            assert!(in_diamond(u, v), "({u}, {v}) should be inside");
        }
        for (u, v) in [(0.05, 0.05), (0.95, 0.05), (0.05, 0.95), (0.95, 0.95)] {
            assert!(!in_diamond(u, v), "({u}, {v}) should be outside");
        }
    }

    /// The shapes are *filled*: a pixel well inside is fully covered, one
    /// well outside not at all, and an edge pixel is in between.
    #[test]
    fn mrk_04_shapes_are_filled_with_soft_edges() {
        assert_eq!(pixel_coverage(in_diamond, 16, 16, 32), 1.0);
        assert_eq!(pixel_coverage(in_diamond, 0, 0, 32), 0.0);
        let edge = pixel_coverage(in_diamond, 7, 8, 32);
        assert!(edge > 0.0 && edge < 1.0, "edge pixel coverage {edge}");
    }
}

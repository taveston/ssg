//! The images ship and station symbols are built from (SYM-02 to SYM-07):
//! white masks with antialiased edges, generated once at startup at
//! exactly the sizes they are drawn at (`ship_symbol_px` on the map,
//! `list_symbol_px` in lists), so the outline is `symbol_stroke_px` wide on
//! screen. Each part is tinted per symbol through `ImageNode::color`, so it
//! fades and colours like any other marker.
//!
//! Shapes are signed distance functions in pixels from the image centre (y
//! down): negative inside, positive outside. An outline covers the pixels
//! within half the stroke of the edge; a fill covers the inside.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::symbols::FrameShape;
use super::tunables::Tunables;
use crate::Boot;

/// Samples per pixel side for antialiasing the edges.
const SUPERSAMPLE: u32 = 4;
/// Dashes round a dashed frame (SYM-08).
const DASHES: f32 = 12.0;
/// The part of each dash period that is drawn.
const DASH_ON: f32 = 0.55;
/// The square frame is drawn a little inside the symbol square, so it
/// doesn't look bigger than a circle of the same box (NATO practice).
const SQUARE_SCALE: f32 = 0.85;
/// A quatrefoil is four circles of this radius (as a fraction of the
/// frame's half-size), their centres the rest of the way out.
const QUATREFOIL_LOBE: f32 = 0.6;
/// The icon's half-size, as a fraction of the frame's.
const ICON_SCALE: f32 = 0.45;

/// One mask a symbol is built from.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SymbolMask {
    /// The frame's outline, solid or dashed (SYM-02, SYM-08).
    Frame { shape: FrameShape, dashed: bool },
    /// The frame's inside, tinted for command (SYM-03).
    Fill(FrameShape),
    /// Filled triangle: warship (SYM-05).
    Triangle,
    /// Outline drop: fleet supply (SYM-05).
    Drop,
    /// Outline crate: cargo (SYM-05).
    Crate,
    /// Outline unit box, with both diagonals when `crossed` (SYM-05,
    /// SYM-06).
    UnitBox { crossed: bool },
}

/// Every mask at every size it is drawn at.
#[derive(Resource, Default, Clone)]
pub struct SymbolImages(pub HashMap<(SymbolMask, u32), Handle<Image>>);

impl SymbolImages {
    /// The mask for a symbol `size_px` across, if generated.
    pub fn get(&self, mask: SymbolMask, size_px: f32) -> Option<Handle<Image>> {
        self.0.get(&(mask, size_px.round() as u32)).cloned()
    }
}

/// Distance from `p` to a box of half-size `half` round the origin.
fn box_sdf(p: Vec2, half: Vec2) -> f32 {
    let d = p.abs() - half;
    d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.0)
}

/// Distance from `p` to the segment `a`-`b`.
fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Signed distance to a convex or simple polygon (points in order).
fn polygon_sdf(p: Vec2, points: &[Vec2]) -> f32 {
    let n = points.len();
    let mut distance = f32::INFINITY;
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        distance = distance.min(segment_distance(p, a, b));
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    if inside { -distance } else { distance }
}

/// SYM-02: the frame of `shape` with half-size `half` (to the middle of
/// its outline).
pub fn frame_sdf(shape: FrameShape, p: Vec2, half: f32) -> f32 {
    match shape {
        FrameShape::Circle => p.length() - half,
        FrameShape::Square => box_sdf(p, Vec2::splat(half * SQUARE_SCALE)),
        FrameShape::Diamond => (p.x.abs() + p.y.abs() - half) / std::f32::consts::SQRT_2,
        FrameShape::Quatrefoil => {
            let lobe = half * QUATREFOIL_LOBE;
            let out = half - lobe;
            [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
                .into_iter()
                .map(|d| p.distance(d * out) - lobe)
                .fold(f32::INFINITY, f32::min)
        }
    }
}

/// SYM-08: whether a point of a dashed outline is on a dash, by its angle
/// round the centre.
fn on_dash(p: Vec2) -> bool {
    let turn = (p.y.atan2(p.x) / std::f32::consts::TAU).rem_euclid(1.0);
    (turn * DASHES).fract() < DASH_ON
}

/// Whether point `p` of a symbol with frame half-size `half` and stroke
/// `stroke` is covered by `mask`.
pub fn covers(mask: SymbolMask, p: Vec2, half: f32, stroke: f32) -> bool {
    let line = |d: f32| d.abs() <= stroke / 2.0;
    let icon = half * ICON_SCALE;
    match mask {
        SymbolMask::Frame { shape, dashed } => {
            line(frame_sdf(shape, p, half)) && (!dashed || on_dash(p))
        }
        SymbolMask::Fill(shape) => frame_sdf(shape, p, half) <= 0.0,
        SymbolMask::Triangle => {
            let points = [
                Vec2::new(0.0, -icon),
                Vec2::new(icon, icon * 0.8),
                Vec2::new(-icon, icon * 0.8),
            ];
            polygon_sdf(p, &points) <= 0.0
        }
        SymbolMask::Drop => {
            let bulb = p.distance(Vec2::new(0.0, icon * 0.3)) - icon * 0.6;
            let tip = [
                Vec2::new(0.0, -icon),
                Vec2::new(icon * 0.52, icon * 0.0),
                Vec2::new(-icon * 0.52, icon * 0.0),
            ];
            line(bulb.min(polygon_sdf(p, &tip)))
        }
        SymbolMask::Crate => {
            let half_box = Vec2::new(icon, icon * 0.75);
            line(box_sdf(p, half_box)) || (p.y.abs() <= stroke / 2.0 && p.x.abs() <= icon)
        }
        SymbolMask::UnitBox { crossed } => {
            let half_box = Vec2::new(icon * 1.1, icon * 0.65);
            let cross = crossed
                && [half_box, Vec2::new(half_box.x, -half_box.y)]
                    .into_iter()
                    .any(|corner| line(segment_distance(p, -corner, corner)));
            line(box_sdf(p, half_box)) || cross
        }
    }
}

/// How much of pixel (x, y) of a `size` image `mask` covers, 0 to 1.
pub fn pixel_coverage(mask: SymbolMask, (x, y): (u32, u32), size: u32, stroke: f32) -> f32 {
    let centre = size as f32 / 2.0;
    // To the middle of the outline, so the stroke stays inside the image.
    let half = centre - stroke / 2.0;
    let samples = SUPERSAMPLE * SUPERSAMPLE;
    let inside = (0..samples)
        .filter(|i| {
            let sx = (i % SUPERSAMPLE) as f32 + 0.5;
            let sy = (i / SUPERSAMPLE) as f32 + 0.5;
            let p = Vec2::new(
                x as f32 + sx / SUPERSAMPLE as f32 - centre,
                y as f32 + sy / SUPERSAMPLE as f32 - centre,
            );
            covers(mask, p, half, stroke)
        })
        .count();
    inside as f32 / samples as f32
}

fn mask_image(mask: SymbolMask, size: u32, stroke: f32) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..size {
        for x in 0..size {
            let alpha = pixel_coverage(mask, (x, y), size, stroke);
            let _ = image.set_color_at(x, y, Color::WHITE.with_alpha(alpha));
        }
    }
    image
}

/// Every mask a symbol can need.
fn all_masks() -> Vec<SymbolMask> {
    let shapes = [
        FrameShape::Circle,
        FrameShape::Square,
        FrameShape::Diamond,
        FrameShape::Quatrefoil,
    ];
    let frames = shapes.into_iter().flat_map(|shape| {
        [
            SymbolMask::Frame {
                shape,
                dashed: false,
            },
            SymbolMask::Frame {
                shape,
                dashed: true,
            },
            SymbolMask::Fill(shape),
        ]
    });
    frames
        .chain([
            SymbolMask::Triangle,
            SymbolMask::Drop,
            SymbolMask::Crate,
            SymbolMask::UnitBox { crossed: false },
            SymbolMask::UnitBox { crossed: true },
        ])
        .collect()
}

pub struct SymbolImagesPlugin;

impl Plugin for SymbolImagesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_images.in_set(Boot::Derive));
    }
}

/// SYM-04: masks at the map size and the list size.
fn generate_images(
    mut commands: Commands,
    tunables: Res<Tunables>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut generated = SymbolImages::default();
    for size in [tunables.ship_symbol_px, tunables.list_symbol_px] {
        let px = size.round() as u32;
        for mask in all_masks() {
            let image = images.add(mask_image(mask, px, tunables.symbol_stroke_px));
            generated.0.insert((mask, px), image);
        }
    }
    commands.insert_resource(generated);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SYM-02: each frame's outline passes through its edge and leaves the
    /// centre empty; the circle reaches the image's edge midpoints.
    #[test]
    fn frames_are_outlines_of_their_shape() {
        let (half, stroke) = (10.0, 1.6);
        for shape in [
            FrameShape::Circle,
            FrameShape::Square,
            FrameShape::Diamond,
            FrameShape::Quatrefoil,
        ] {
            let frame = SymbolMask::Frame {
                shape,
                dashed: false,
            };
            assert!(!covers(frame, Vec2::ZERO, half, stroke), "{shape:?}");
            assert!(covers(SymbolMask::Fill(shape), Vec2::ZERO, half, stroke));
        }
        let circle = SymbolMask::Frame {
            shape: FrameShape::Circle,
            dashed: false,
        };
        assert!(covers(circle, Vec2::new(half, 0.0), half, stroke));
        // A diamond's corners are on the axes, its sides' middles inside.
        let diamond = SymbolMask::Frame {
            shape: FrameShape::Diamond,
            dashed: false,
        };
        assert!(covers(diamond, Vec2::new(0.0, -half), half, stroke));
        assert!(!covers(diamond, Vec2::new(half, half) * 0.95, half, stroke));
        // A square's corner is drawn; a circle's isn't.
        let corner = Vec2::splat(half * SQUARE_SCALE);
        let square = SymbolMask::Frame {
            shape: FrameShape::Square,
            dashed: false,
        };
        assert!(covers(square, corner, half, stroke));
        assert!(!covers(circle, corner, half, stroke));
    }

    /// SYM-08: a dashed frame leaves gaps round its outline.
    #[test]
    fn dashed_frames_have_gaps() {
        let dashed = SymbolMask::Frame {
            shape: FrameShape::Circle,
            dashed: true,
        };
        let drawn = (0..360)
            .filter(|deg| {
                let a = (*deg as f32).to_radians();
                covers(dashed, Vec2::new(a.cos(), a.sin()) * 10.0, 10.0, 1.6)
            })
            .count();
        assert!(drawn > 150 && drawn < 250, "{drawn} of 360 degrees drawn");
    }

    /// SYM-05: the warship triangle is filled; the drop, crate and unit box
    /// are outlines; the crate has its centre line and the crossed box its
    /// diagonals.
    #[test]
    fn icons_are_filled_or_outlined_as_specced() {
        let (half, stroke) = (10.0, 1.6);
        assert!(covers(
            SymbolMask::Triangle,
            Vec2::new(0.0, 1.0),
            half,
            stroke
        ));
        assert!(!covers(SymbolMask::Drop, Vec2::new(0.0, 1.5), half, stroke));
        assert!(covers(SymbolMask::Crate, Vec2::ZERO, half, stroke));
        let open_box = SymbolMask::UnitBox { crossed: false };
        let crossed = SymbolMask::UnitBox { crossed: true };
        assert!(!covers(open_box, Vec2::ZERO, half, stroke));
        assert!(covers(crossed, Vec2::ZERO, half, stroke));
    }

    /// Edges are antialiased: a pixel straddling the outline is partly
    /// covered.
    #[test]
    fn edges_are_soft() {
        let frame = SymbolMask::Frame {
            shape: FrameShape::Circle,
            dashed: false,
        };
        let coverages: Vec<f32> = (0..22)
            .map(|x| pixel_coverage(frame, (x, 11), 22, 1.6))
            .collect();
        assert!(coverages.iter().any(|c| *c > 0.0 && *c < 1.0));
        assert_eq!(pixel_coverage(frame, (11, 11), 22, 1.6), 0.0);
    }
}

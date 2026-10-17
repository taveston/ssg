//! Close orbits in front of their body (MRK-12): rings are world gizmos
//! drawn under every UI node, so a body drawn as a true-size disc (MRK-07)
//! would hide its satellites' rings on both sides. For each ring centred
//! on such a body, the part of its drawn stroke nearer the camera than the
//! body's centre is drawn again in a UI node over the disc, one layer
//! above it, clipped to the disc: outside the disc the gizmo already
//! shows. The node takes the gizmo's lines and colours from `rings/mod.rs`, so
//! the two passes match, and it is never a pick target (MRK-08).

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::{DrawnRings, RingLine, RingsDrawn};
use crate::Stage;
use crate::view::markers::{MarkerHalfSize, MarkerShape, MarkersPlaced};
use crate::view::targets::MarkerFor;
use crate::view::world::ViewNow;

/// The most segments one near arc carries to the shader; longer arcs are
/// thinned by joining neighbours.
pub const MAX_SEGMENTS: usize = 128;

/// One piece of a near arc on screen, in logical pixels from the disc's
/// centre, with the colour at each end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenSegment {
    pub from: Vec2,
    pub to: Vec2,
    pub from_color: Color,
    pub to_color: Color,
}

/// A body drawn as a true-size disc, on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Disc {
    pub centre: Vec2,
    pub half: f32,
    pub layer: i32,
}

/// The values the shader reads; field order matches `NearArc` in
/// `near_arcs.wesl`. Each segment is its two ends (xy, zw) and their
/// colours.
#[derive(ShaderType, Clone, Copy, Debug)]
pub struct NearArcUniform {
    pub ends: [Vec4; MAX_SEGMENTS],
    pub from_colors: [Vec4; MAX_SEGMENTS],
    pub to_colors: [Vec4; MAX_SEGMENTS],
    pub count: u32,
    pub half_px: f32,
    /// The line width in physical pixels, as the gizmo's.
    pub width_px: f32,
}

impl Default for NearArcUniform {
    fn default() -> Self {
        Self {
            ends: [Vec4::ZERO; MAX_SEGMENTS],
            from_colors: [Vec4::ZERO; MAX_SEGMENTS],
            to_colors: [Vec4::ZERO; MAX_SEGMENTS],
            count: 0,
            half_px: 1.0,
            width_px: 1.0,
        }
    }
}

/// The material that draws one near arc.
#[derive(AsBindGroup, Asset, TypePath, Clone, Debug, Default)]
pub struct NearArcMaterial {
    #[uniform(0)]
    pub arc: NearArcUniform,
}

impl UiMaterial for NearArcMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://orphan_worlds/view/rings/near_arcs.wesl".into()
    }
}

/// The node that draws one ring's near arc, for the ring's owner.
#[derive(Component)]
pub struct NearArcFor(pub Entity);

pub struct NearArcsPlugin;

impl Plugin for NearArcsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "near_arcs.wesl");
        app.add_plugins(UiMaterialPlugin::<NearArcMaterial>::default())
            .add_systems(
                Update,
                (drop_orphaned_arcs, draw_near_arcs)
                    .chain()
                    .after(RingsDrawn)
                    .after(MarkersPlaced)
                    .in_set(Stage::Overlay),
            );
    }
}

/// MRK-12: the part of a ring's lines nearer the camera than the centre
/// body, as screen segments from the disc's centre, keeping only those
/// that reach the disc. A line crossing the body's depth is cut there.
/// `depth` gives a world point's depth along the camera's view and
/// `project` its screen point.
pub fn near_segments(
    lines: &[RingLine],
    centre_depth: f32,
    disc: Disc,
    width: f32,
    depth: impl Fn(Vec3) -> f32,
    project: impl Fn(Vec3) -> Option<Vec2>,
) -> Vec<ScreenSegment> {
    let mut out = Vec::new();
    for line in lines {
        let (da, db) = (
            depth(line.from) - centre_depth,
            depth(line.to) - centre_depth,
        );
        if da >= 0.0 && db >= 0.0 {
            continue;
        }
        let (mut from, mut to) = (line.from, line.to);
        let (mut from_color, mut to_color) = (line.from_color, line.to_color);
        if da >= 0.0 || db >= 0.0 {
            let t = da / (da - db);
            let cut = from.lerp(to, t);
            let cut_color = mix(from_color, to_color, t);
            if da >= 0.0 {
                (from, from_color) = (cut, cut_color);
            } else {
                (to, to_color) = (cut, cut_color);
            }
        }
        let (Some(a), Some(b)) = (project(from), project(to)) else {
            continue;
        };
        let (a, b) = (a - disc.centre, b - disc.centre);
        if distance_to_segment(Vec2::ZERO, a, b) > disc.half + width {
            continue;
        }
        out.push(ScreenSegment {
            from: a,
            to: b,
            from_color,
            to_color,
        });
    }
    thin(out, MAX_SEGMENTS)
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let (a, b) = (a.to_linear(), b.to_linear());
    Color::LinearRgba(a * (1.0 - t) + b * t)
}

fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// Join neighbouring segments that meet end to end until at most `max`
/// remain; whatever still doesn't fit is dropped.
fn thin(mut segments: Vec<ScreenSegment>, max: usize) -> Vec<ScreenSegment> {
    while segments.len() > max {
        let before = segments.len();
        let mut joined = Vec::with_capacity(before / 2 + 1);
        let mut iter = segments.into_iter().peekable();
        while let Some(first) = iter.next() {
            match iter.peek() {
                Some(next) if next.from.distance(first.to) < 0.01 => {
                    let next = iter.next().unwrap();
                    joined.push(ScreenSegment {
                        to: next.to,
                        to_color: next.to_color,
                        ..first
                    });
                }
                _ => joined.push(first),
            }
        }
        segments = joined;
        if segments.len() == before {
            segments.truncate(max);
        }
    }
    segments
}

/// The bodies drawn as true-size discs this frame, from their markers.
fn true_size_discs(
    markers: &Query<(
        &MarkerFor,
        &MarkerShape,
        &MarkerHalfSize,
        &Node,
        &GlobalZIndex,
    )>,
) -> Vec<(Entity, Disc)> {
    markers
        .iter()
        .filter(|(_, shape, _, node, _)| {
            **shape == MarkerShape::Disc && node.display != Display::None
        })
        .filter_map(|(marker_for, _, class_half, node, z)| {
            let (Val::Px(left), Val::Px(top), Val::Px(width)) = (node.left, node.top, node.width)
            else {
                return None;
            };
            let half = width / 2.0;
            (half > class_half.0.x).then_some((
                marker_for.0,
                Disc {
                    centre: Vec2::new(left + half, top + half),
                    half,
                    layer: z.0,
                },
            ))
        })
        .collect()
}

/// An arc whose ring is gone, with the system it was in (SECT-32), goes
/// too; an arc whose ring only isn't drawn this frame is kept, hidden.
fn drop_orphaned_arcs(
    mut commands: Commands,
    arcs: Query<(Entity, &NearArcFor)>,
    everything: Query<()>,
) {
    for (arc, arc_for) in &arcs {
        if !everything.contains(arc_for.0) {
            commands.entity(arc).despawn();
        }
    }
}

/// MRK-12: redraw each satellite ring's near arc over its true-size
/// parent, one layer above the parent's disc.
fn draw_near_arcs(
    mut commands: Commands,
    view: ViewNow,
    windows: Query<&Window>,
    drawn: Res<DrawnRings>,
    markers: Query<(
        &MarkerFor,
        &MarkerShape,
        &MarkerHalfSize,
        &Node,
        &GlobalZIndex,
    )>,
    mut arcs: Query<
        (
            &NearArcFor,
            &mut Node,
            &mut GlobalZIndex,
            &MaterialNode<NearArcMaterial>,
        ),
        Without<MarkerShape>,
    >,
    mut materials: ResMut<Assets<NearArcMaterial>>,
) {
    let scale = windows.single().map_or(1.0, Window::scale_factor);
    let discs = true_size_discs(&markers);
    let camera = view.camera().map(|(_, t)| t);
    let mut wanted: Vec<(Entity, Disc, Vec<ScreenSegment>, f32)> = Vec::new();
    if let Some(camera) = camera {
        let forward = camera.forward().as_vec3();
        let eye = camera.translation();
        let depth = |p: Vec3| (p - eye).dot(forward);
        for (&owner, ring) in &drawn.0 {
            let Some(centre) = ring.centre else { continue };
            let Some(&(_, disc)) = discs.iter().find(|(body, _)| *body == centre) else {
                continue;
            };
            let Some(centre_pos) = view.position(centre) else {
                continue;
            };
            let width = ring.width / scale;
            let segments = near_segments(&ring.lines, depth(centre_pos), disc, width, depth, |p| {
                view.project(p)
            });
            if !segments.is_empty() {
                wanted.push((owner, disc, segments, ring.width));
            }
        }
    }
    for (arc_for, mut node, mut z, material) in &mut arcs {
        match wanted.iter().position(|w| w.0 == arc_for.0) {
            Some(i) => {
                let (_, disc, segments, width) = wanted.swap_remove(i);
                place(&mut node, &mut z, disc);
                if let Some(mut material) = materials.get_mut(&material.0) {
                    material.arc = uniform(&segments, disc.half, width);
                }
            }
            None => node.display = Display::None,
        }
    }
    for (owner, disc, segments, width) in wanted {
        let mut node = Node {
            position_type: PositionType::Absolute,
            ..default()
        };
        let mut z = GlobalZIndex(0);
        place(&mut node, &mut z, disc);
        let handle = materials.add(NearArcMaterial {
            arc: uniform(&segments, disc.half, width),
        });
        commands.spawn((
            NearArcFor(owner),
            node,
            z,
            MaterialNode(handle),
            Pickable::IGNORE,
        ));
    }
}

/// The node covers the disc, one layer above it.
fn place(node: &mut Node, z: &mut GlobalZIndex, disc: Disc) {
    node.display = Display::Flex;
    node.left = px(disc.centre.x - disc.half);
    node.top = px(disc.centre.y - disc.half);
    node.width = px(disc.half * 2.0);
    node.height = px(disc.half * 2.0);
    z.0 = disc.layer + 1;
}

fn uniform(segments: &[ScreenSegment], half: f32, width_px: f32) -> NearArcUniform {
    let mut arc = NearArcUniform {
        count: segments.len() as u32,
        half_px: half,
        width_px,
        ..default()
    };
    for (i, s) in segments.iter().enumerate() {
        arc.ends[i] = Vec4::new(s.from.x, s.from.y, s.to.x, s.to.y);
        arc.from_colors[i] = s.from_color.to_linear().to_vec4();
        arc.to_colors[i] = s.to_color.to_linear().to_vec4();
    }
    arc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(from: Vec3, to: Vec3) -> RingLine {
        RingLine {
            from,
            to,
            from_color: Color::WHITE,
            to_color: Color::BLACK,
        }
    }

    /// Looking down -Z from z = 10, x and y straight to screen.
    fn depth(p: Vec3) -> f32 {
        10.0 - p.z
    }
    fn project(p: Vec3) -> Option<Vec2> {
        Some(p.truncate())
    }
    const DISC: Disc = Disc {
        centre: Vec2::ZERO,
        half: 5.0,
        layer: 3,
    };

    /// MRK-12: only the part nearer the camera than the body's centre is
    /// redrawn, cut where a line crosses the body's depth.
    #[test]
    fn only_the_near_part_of_a_ring_is_redrawn() {
        let lines = [
            line(Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 0.0, -1.0)),
            line(Vec3::new(-1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),
            line(Vec3::new(0.0, 2.0, -1.0), Vec3::new(0.0, 4.0, 1.0)),
        ];
        let near = near_segments(&lines, 10.0, DISC, 1.0, depth, project);
        assert_eq!(near.len(), 2);
        assert_eq!(near[0].from, Vec2::new(-1.0, 1.0));
        assert_eq!(near[1].from, Vec2::new(0.0, 3.0));
        assert_eq!(near[1].to, Vec2::new(0.0, 4.0));
        let grey = near[1].from_color.to_linear();
        assert!((grey.red - 0.5).abs() < 1e-5);
    }

    /// MRK-12: a near piece that doesn't reach the disc is left to the
    /// gizmo.
    #[test]
    fn near_pieces_off_the_disc_are_skipped() {
        let lines = [line(Vec3::new(20.0, 0.0, 1.0), Vec3::new(30.0, 0.0, 1.0))];
        assert!(near_segments(&lines, 10.0, DISC, 1.0, depth, project).is_empty());
    }

    /// MRK-12: a long arc is thinned by joining neighbours, keeping its
    /// ends.
    #[test]
    fn long_arcs_are_thinned_to_fit_the_shader() {
        let segments: Vec<ScreenSegment> = (0..300)
            .map(|i| ScreenSegment {
                from: Vec2::new(i as f32, 0.0),
                to: Vec2::new(i as f32 + 1.0, 0.0),
                from_color: Color::WHITE,
                to_color: Color::WHITE,
            })
            .collect();
        let thinned = thin(segments, MAX_SEGMENTS);
        assert!(thinned.len() <= MAX_SEGMENTS);
        assert_eq!(thinned[0].from, Vec2::ZERO);
        assert_eq!(thinned.last().unwrap().to, Vec2::new(300.0, 0.0));
    }

    /// MRK-12: the redrawn arc sits one layer above its body's disc.
    #[test]
    fn the_near_arc_sits_just_above_its_disc() {
        let mut node = Node::default();
        let mut z = GlobalZIndex(0);
        place(&mut node, &mut z, DISC);
        assert_eq!(z.0, DISC.layer + 1);
        assert_eq!((node.left, node.width), (px(-5.0), px(10.0)));
    }
}

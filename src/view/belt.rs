//! The belts (MRK-18): each a soft translucent band round the star with a
//! fixed scatter of faint points across it, the one piece of real 3D
//! geometry in the system view. Each is sized from its simulation `Belt`
//! record through `scale.rs`; a system with no belt draws none.
//!
//! The band is a flat ring mesh split into narrow concentric strips, its
//! alpha set per vertex: full along the centre line, falling as 1 − u² to
//! nothing at both edges, so it has no hard edge. The grain is one
//! point-list mesh: each point is drawn by the GPU as a single pixel. The
//! points are scattered once with the same falloff from a fixed seed of
//! the belt's own, so they look the same every run and every frame and no
//! two belts share a pattern. Both meshes lie in the
//! orbital plane (XZ) and only move with the star; they don't orbit, and
//! nothing in them is an entity of its own or a hover or click target.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::colors;
use super::scale::km_to_render;
use super::tunables::Tunables;
use super::world::RenderOrigin;
use crate::sim::body::Belt;
use crate::{Boot, Stage};

/// Segments round the band; enough that it reads as round at any zoom.
const BAND_SEGMENTS: u32 = 256;
/// Concentric strips across the band. Alpha is linear within a strip, so
/// this many steps follow the 1 − u² curve closely enough that no step
/// shows.
const BAND_STRIPS: u32 = 16;
/// The base of every belt's grain seed, fixed so the points never move
/// between runs.
const SPECK_SEED: u64 = 0x0B5E_7A11_D05E_ED01;

/// A belt's own grain seed (MRK-18): the base seed mixed (FNV-1a) with the
/// belt's name and edges, so it is the same every run and differs between
/// the belts of a system, whose edges never coincide.
pub fn belt_speck_seed(belt: &Belt) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01B3;
    let edges = [belt.inner_radius_km, belt.outer_radius_km];
    let bytes = belt
        .name
        .bytes()
        .chain(edges.iter().flat_map(|km| km.to_bits().to_le_bytes()));
    bytes.fold(SPECK_SEED, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

/// The point-list mesh that draws the belt's grain.
#[derive(Component)]
pub struct BeltSpecks;

pub struct BeltPlugin;

impl Plugin for BeltPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_belt_mesh.in_set(Boot::Derive))
            .add_systems(Update, centre_belt_on_star.in_set(Stage::Camera));
    }
}

/// The band's alpha at `u`, the distance from its centre line as a
/// fraction of its half-width: `peak` on the line, falling as 1 − u² to
/// nothing at either edge and beyond (MRK-18).
pub fn band_alpha(u: f32, peak: f32) -> f32 {
    peak * (1.0 - u * u).max(0.0)
}

/// A small deterministic generator (SplitMix64), so the scatter needs no
/// crate and comes out the same on every machine.
struct SplitMix(u64);

impl SplitMix {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// `count` points scattered across the band from `inner` to `outer`
/// (render units), in the orbital plane round the origin. Each point's
/// distance from the centre line is drawn with density following the
/// band's 1 − u² falloff, its angle evenly; the same seed always gives the
/// same points (MRK-18).
pub fn speck_positions(inner: f32, outer: f32, count: usize, seed: u64) -> Vec<Vec3> {
    let mut rng = SplitMix(seed);
    let mid = (inner + outer) / 2.0;
    let half = (outer - inner) / 2.0;
    let mut points = Vec::with_capacity(count);
    while points.len() < count {
        let u = rng.unit() * 2.0 - 1.0;
        if rng.unit() >= band_alpha(u, 1.0) {
            continue;
        }
        let angle = rng.unit() * TAU;
        let r = mid + u * half;
        points.push(Vec3::new(r * angle.cos(), 0.0, r * angle.sin()));
    }
    points
}

/// The band as a flat ring in the orbital plane: concentric strips whose
/// vertices carry `color` at the band's alpha for their distance from the
/// centre line (MRK-18).
fn band_mesh(inner: f32, outer: f32, peak: f32, color: Color) -> Mesh {
    let rings = BAND_STRIPS + 1;
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    for ring in 0..rings {
        let u = ring as f32 / BAND_STRIPS as f32 * 2.0 - 1.0;
        let r = inner + (outer - inner) * (u + 1.0) / 2.0;
        let rgba = color
            .to_linear()
            .with_alpha(band_alpha(u, peak))
            .to_f32_array();
        for seg in 0..BAND_SEGMENTS {
            let angle = seg as f32 / BAND_SEGMENTS as f32 * TAU;
            positions.push([r * angle.cos(), 0.0, r * angle.sin()]);
            colors.push(rgba);
        }
    }
    let mut indices = Vec::new();
    for ring in 0..BAND_STRIPS {
        for seg in 0..BAND_SEGMENTS {
            let next = (seg + 1) % BAND_SEGMENTS;
            let a = ring * BAND_SEGMENTS + seg;
            let b = ring * BAND_SEGMENTS + next;
            let c = a + BAND_SEGMENTS;
            let d = b + BAND_SEGMENTS;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

/// The grain as one point-list mesh, every point in `color` at `alpha`.
fn speck_mesh(points: &[Vec3], color: Color, alpha: f32) -> Mesh {
    let rgba = color.to_linear().with_alpha(alpha).to_f32_array();
    Mesh::new(PrimitiveTopology::PointList, RenderAssetUsages::default())
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            points.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![rgba; points.len()])
}

/// An unlit, see-through material that takes its colour from the mesh's
/// vertices.
fn vertex_color_material() -> StandardMaterial {
    StandardMaterial {
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

/// The belts circle the star, which sits wherever the floating origin
/// puts it this frame (SCL-06).
fn centre_belt_on_star(origin: Res<RenderOrigin>, mut belts: Query<&mut Transform, With<Belt>>) {
    for mut transform in &mut belts {
        transform.translation = origin.star;
    }
}

/// Gives each belt its band mesh and, as a child, its grain (MRK-18).
fn add_belt_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tunables: Res<Tunables>,
    belts: Query<(Entity, &Belt)>,
) {
    for (entity, belt) in &belts {
        let inner = km_to_render(belt.inner_radius_km);
        let outer = km_to_render(belt.outer_radius_km);
        let band = band_mesh(inner, outer, tunables.belt_band_alpha, colors::ORBIT_LINE);
        let seed = belt_speck_seed(belt);
        let points = speck_positions(inner, outer, tunables.belt_speck_count, seed);
        let specks = speck_mesh(&points, colors::ORBIT_LINE, tunables.belt_speck_alpha);
        let material = materials.add(vertex_color_material());
        commands
            .entity(entity)
            .insert((
                Mesh3d(meshes.add(band)),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                NotShadowCaster,
                Pickable::IGNORE,
            ))
            .with_child((
                BeltSpecks,
                Mesh3d(meshes.add(specks)),
                MeshMaterial3d(material),
                Transform::default(),
                NotShadowCaster,
                Pickable::IGNORE,
            ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    /// MRK-18: the band is at its peak on the centre line and falls as
    /// 1 − u² to nothing at both edges, with no hard step anywhere.
    #[test]
    fn band_alpha_peaks_on_the_centre_line_and_fades_to_nothing_at_the_edges() {
        assert_eq!(band_alpha(0.0, 0.1), 0.1);
        assert_eq!(band_alpha(1.0, 0.1), 0.0);
        assert_eq!(band_alpha(-1.0, 0.1), 0.0);
        assert!((band_alpha(0.5, 0.1) - 0.075).abs() < 1e-6);
        assert_eq!(band_alpha(1.5, 0.1), 0.0);
        let mut last = band_alpha(0.0, 1.0);
        for step in 1..=100 {
            let a = band_alpha(step as f32 / 100.0, 1.0);
            assert!(a <= last && last - a < 0.03, "step at u = {step}%");
            last = a;
        }
    }

    /// MRK-18: the band mesh's vertex alphas follow the profile, from
    /// nothing at the inner and outer edge to the peak at the middle.
    #[test]
    fn band_mesh_alphas_follow_the_profile_across_the_band() {
        let mesh = band_mesh(10.0, 20.0, 0.1, colors::ORBIT_LINE);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colours");
        };
        for (p, c) in positions.iter().zip(colors) {
            let r = Vec2::new(p[0], p[2]).length();
            assert_eq!(p[1], 0.0, "in the orbital plane");
            let u = (r - 15.0) / 5.0;
            assert!((c[3] - band_alpha(u, 0.1)).abs() < 1e-4, "r = {r}");
        }
        let edge = colors.iter().map(|c| c[3]).fold(f32::MAX, f32::min);
        let middle = colors.iter().map(|c| c[3]).fold(0.0, f32::max);
        assert_eq!(edge, 0.0);
        assert!((middle - 0.1).abs() < 1e-6);
    }

    /// MRK-18: the same seed gives the same scatter every time; another
    /// seed gives another.
    #[test]
    fn speck_scatter_is_the_same_for_the_same_seed() {
        let a = speck_positions(10.0, 20.0, 500, SPECK_SEED);
        let b = speck_positions(10.0, 20.0, 500, SPECK_SEED);
        let c = speck_positions(10.0, 20.0, 500, SPECK_SEED + 1);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// MRK-18: every point lies in the orbital plane inside the band, the
    /// count is exact, and the points crowd the centre line and thin out
    /// towards the edges as the band's falloff does.
    #[test]
    fn specks_lie_in_the_band_and_thin_towards_its_edges() {
        let points = speck_positions(10.0, 20.0, 1200, SPECK_SEED);
        assert_eq!(points.len(), 1200);
        let mut middle = 0;
        let mut outer_quarters = 0;
        for p in &points {
            assert_eq!(p.y, 0.0);
            let r = Vec2::new(p.x, p.z).length();
            assert!((10.0..=20.0).contains(&r), "r = {r}");
            let u = ((r - 15.0) / 5.0).abs();
            if u < 0.25 {
                middle += 1;
            } else if u > 0.75 {
                outer_quarters += 1;
            }
        }
        // With density 1 − u², the inner half of the half-width holds about
        // 34% of the points and the outer quarter about 11%.
        assert!(middle > 3 * outer_quarters, "{middle} vs {outer_quarters}");
        // Evenly round: each quadrant holds about a quarter.
        let quadrant = points.iter().filter(|p| p.x > 0.0 && p.z > 0.0).count();
        assert!((240..360).contains(&quadrant), "{quadrant}");
    }

    /// A belt record with no mass or composition.
    fn belt(name: &str, inner_radius_km: f64, outer_radius_km: f64) -> Belt {
        Belt {
            name: name.to_string(),
            inner_radius_km,
            outer_radius_km,
            mass_kg: None,
            composition: None,
        }
    }

    /// An app that gives belts their meshes, with nothing in it yet.
    fn belt_app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Tunables>()
            .add_systems(Update, add_belt_mesh);
        app
    }

    /// The points of a grain mesh, in render units.
    fn mesh_points(mesh: &Mesh) -> Vec<Vec3> {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        positions.iter().map(|p| Vec3::from_array(*p)).collect()
    }

    /// MRK-18: the belt is one band mesh with its grain as one point-list
    /// child of `belt_speck_count` points in the orbit colour at
    /// `belt_speck_alpha`; neither is a pick target and the points are not
    /// entities of their own.
    #[test]
    fn belt_spawns_one_band_and_one_point_mesh_of_faint_specks() {
        let mut app = belt_app();
        let belt = app.world_mut().spawn(belt("Belt", 3.0e8, 4.5e8)).id();
        app.update();
        let world = app.world_mut();
        let tunables = world.resource::<Tunables>().clone();
        assert_eq!(world.get::<Pickable>(belt).copied(), Some(Pickable::IGNORE));
        let mut specks = world.query_filtered::<(&Mesh3d, &ChildOf, &Pickable), With<BeltSpecks>>();
        let (mesh, parent, pickable) = specks.single(world).unwrap();
        assert_eq!(parent.parent(), belt);
        assert_eq!(*pickable, Pickable::IGNORE);
        let mesh = world.resource::<Assets<Mesh>>().get(&mesh.0).unwrap();
        assert_eq!(mesh.primitive_topology(), PrimitiveTopology::PointList);
        assert_eq!(mesh.count_vertices(), tunables.belt_speck_count);
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colours");
        };
        let expected = colors::ORBIT_LINE
            .to_linear()
            .with_alpha(tunables.belt_speck_alpha)
            .to_f32_array();
        assert!(colors.iter().all(|c| *c == expected));
    }

    /// MRK-18: two belts draw two bands, each with its grain scattered
    /// across its own annulus from a seed of its own.
    #[test]
    fn two_belts_draw_two_bands_each_with_its_own_grain() {
        let mut app = belt_app();
        let inner = app.world_mut().spawn(belt("Inner belt", 1.5e8, 2.0e8)).id();
        let outer = app.world_mut().spawn(belt("Outer belt", 6.0e8, 7.5e8)).id();
        app.update();
        let count = app.world().resource::<Tunables>().belt_speck_count;
        let world = app.world_mut();
        let mut bands = world.query_filtered::<Entity, (With<Belt>, With<Mesh3d>)>();
        assert_eq!(bands.iter(world).count(), 2);
        let mut specks = world.query_filtered::<(&Mesh3d, &ChildOf), With<BeltSpecks>>();
        let specks: Vec<(Handle<Mesh>, Entity)> = specks
            .iter(world)
            .map(|(mesh, parent)| (mesh.0.clone(), parent.parent()))
            .collect();
        assert_eq!(specks.len(), 2);
        let mut seeds = Vec::new();
        for entity in [inner, outer] {
            let record = world.get::<Belt>(entity).unwrap().clone();
            let seed = belt_speck_seed(&record);
            seeds.push(seed);
            let (mesh, _) = specks.iter().find(|(_, p)| *p == entity).unwrap();
            let mesh = world.resource::<Assets<Mesh>>().get(mesh).unwrap();
            let expected = speck_positions(
                km_to_render(record.inner_radius_km),
                km_to_render(record.outer_radius_km),
                count,
                seed,
            );
            assert_eq!(mesh_points(mesh), expected, "{}", record.name);
        }
        assert_ne!(seeds[0], seeds[1]);
        assert!(seeds.iter().all(|s| *s != SPECK_SEED));
    }

    /// MRK-18: a belt's grain seed is fixed by its record, so it is the
    /// same every run, and belts with different names or edges get
    /// different seeds.
    #[test]
    fn each_belt_has_a_fixed_seed_of_its_own() {
        let a = belt("Main belt", 3.0e8, 4.5e8);
        assert_eq!(belt_speck_seed(&a), belt_speck_seed(&a.clone()));
        assert_ne!(
            belt_speck_seed(&a),
            belt_speck_seed(&belt("Kuiper belt", 3.0e8, 4.5e8))
        );
        assert_ne!(
            belt_speck_seed(&a),
            belt_speck_seed(&belt("Main belt", 3.1e8, 4.5e8))
        );
    }

    /// MRK-18: a system with no belt draws no band and no grain.
    #[test]
    fn a_system_with_no_belt_draws_none() {
        let mut app = belt_app();
        app.update();
        let world = app.world_mut();
        let mut meshes = world.query::<&Mesh3d>();
        assert_eq!(meshes.iter(world).count(), 0);
        assert!(world.resource::<Assets<Mesh>>().is_empty());
    }
}

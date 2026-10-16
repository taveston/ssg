//! Stage 1, the volume: sizes the sector's slab from N and places a site
//! for every system and brown dwarf (SGEN-10 to SGEN-12).
//!
//! The slab is a disc in plan, slightly elliptical, and thin in z: its
//! thickness is a fixed share of its width, so the map reads well from a
//! tilted top-down view. Sites are spread uniformly through it, never
//! closer than a minimum separation.

use std::collections::HashMap;

use bevy::math::DVec3;

use super::rng::Rng;
use super::tunables::GeneratorTunables;
use crate::sim::units::KM_PER_PARSEC;

/// A site that lands too close to an earlier one is redrawn up to this many
/// times in all; then the best of the tries is kept (SGEN-12).
const PLACEMENT_TRIES: usize = 50;

/// How many things the sector holds (SGEN-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// T: star systems, giants included.
    pub systems: usize,
    /// P: sites that hold a wide pair, two systems each.
    pub wide_pairs: usize,
    /// S = T − P: sites for systems.
    pub system_sites: usize,
    /// B: brown dwarfs, each on a site of its own.
    pub brown_dwarfs: usize,
}

/// The counts for N gated systems: T = N × `stars_per_gated` systems, of
/// which P = round(T × `wide_pair_fraction` ÷ 2) sites hold wide pairs, so
/// T − P sites hold systems; and B = round(T × `brown_dwarf_per_system`)
/// brown dwarfs (SGEN-10).
pub fn counts(gated_count: u32, tunables: &GeneratorTunables) -> Counts {
    let systems = (f64::from(gated_count) * tunables.stars_per_gated).round() as usize;
    let wide_pairs = (systems as f64 * tunables.wide_pair_fraction / 2.0).round() as usize;
    let brown_dwarfs = (systems as f64 * tunables.brown_dwarf_per_system).round() as usize;
    Counts {
        systems,
        wide_pairs,
        system_sites: systems - wide_pairs,
        brown_dwarfs,
    }
}

/// The slab's shape and size (SGEN-11, SGEN-12). Part of the sector
/// (SGEN-90).
#[derive(Debug, Clone, PartialEq)]
pub struct Slab {
    pub counts: Counts,
    /// V = T ÷ density, in cubic parsecs.
    pub volume_pc3: f64,
    /// W: the diameter of the circle whose area is the plan's area, in
    /// parsecs.
    pub width_pc: f64,
    /// `slab_thickness_ratio` × W, in parsecs. The slab runs from minus
    /// half of it to plus half in z.
    pub thickness_pc: f64,
    /// E, the plan's stretch: the ellipse's semi-axes are (W ÷ 2) × √E
    /// along x and (W ÷ 2) ÷ √E along y.
    pub elongation: f64,
}

impl Slab {
    /// The ellipse's semi-axis along x, in parsecs.
    pub fn semi_axis_x_pc(&self) -> f64 {
        self.width_pc / 2.0 * self.elongation.sqrt()
    }

    /// The ellipse's semi-axis along y, in parsecs.
    pub fn semi_axis_y_pc(&self) -> f64 {
        self.width_pc / 2.0 / self.elongation.sqrt()
    }

    /// Whether a point, in parsecs from the centre, lies inside the slab.
    pub fn contains_pc(&self, point: DVec3) -> bool {
        let (a, b) = (self.semi_axis_x_pc(), self.semi_axis_y_pc());
        (point.x / a).powi(2) + (point.y / b).powi(2) <= 1.0
            && point.z.abs() <= self.thickness_pc / 2.0
    }
}

/// V = T ÷ density, and W and the thickness from V = π (W ÷ 2)² ×
/// thickness with thickness = ratio × W, so W = (4 V ÷ (π × ratio))^(1/3)
/// (SGEN-11). Returns (V, W, thickness) in parsecs.
pub fn slab_size(systems: usize, tunables: &GeneratorTunables) -> (f64, f64, f64) {
    let volume = systems as f64 / tunables.system_density_per_pc3;
    let ratio = tunables.slab_thickness_ratio;
    let width = (4.0 * volume / (std::f64::consts::PI * ratio)).cbrt();
    (volume, width, ratio * width)
}

/// Stage 1's output: the slab and its sites, in km from its centre.
#[derive(Debug, Clone, PartialEq)]
pub struct Volume {
    pub slab: Slab,
    /// One site per system site (S of them), in placement order.
    pub system_sites_km: Vec<DVec3>,
    /// One site per brown dwarf (B of them), placed after the systems'.
    pub brown_dwarf_sites_km: Vec<DVec3>,
}

/// Runs stage 1: sizes the slab for N gated systems, draws its stretch and
/// places every site (SGEN-10 to SGEN-12).
pub fn run(gated_count: u32, rng: &mut Rng, tunables: &GeneratorTunables) -> Volume {
    let counts = counts(gated_count, tunables);
    let (volume_pc3, width_pc, thickness_pc) = slab_size(counts.systems, tunables);
    let slab = Slab {
        counts,
        volume_pc3,
        width_pc,
        thickness_pc,
        elongation: rng.uniform(1.0, tunables.plan_elongation_max),
    };
    let total = counts.system_sites + counts.brown_dwarfs;
    let mut sites = place_sites(&slab, total, tunables.min_system_separation_pc, rng);
    let brown_dwarf_sites_km = sites
        .split_off(counts.system_sites)
        .into_iter()
        .map(|site| site * KM_PER_PARSEC)
        .collect();
    Volume {
        system_sites_km: sites.into_iter().map(|site| site * KM_PER_PARSEC).collect(),
        brown_dwarf_sites_km,
        slab,
    }
}

/// A point drawn uniformly in the slab, in parsecs: uniform in the
/// ellipse by rejection from its bounding box, uniform in z.
fn point_in_slab(slab: &Slab, rng: &mut Rng) -> DVec3 {
    let (a, b) = (slab.semi_axis_x_pc(), slab.semi_axis_y_pc());
    loop {
        let x = rng.uniform(-a, a);
        let y = rng.uniform(-b, b);
        if (x / a).powi(2) + (y / b).powi(2) <= 1.0 {
            let half = slab.thickness_pc / 2.0;
            return DVec3::new(x, y, rng.uniform(-half, half));
        }
    }
}

/// Places `count` sites uniformly in the slab, each at least `min_pc`
/// from every earlier one. A site that cannot be placed in
/// `PLACEMENT_TRIES` draws goes where the best draw was: the one farthest
/// from its nearest neighbour (SGEN-12). Positions are in parsecs.
pub fn place_sites(slab: &Slab, count: usize, min_pc: f64, rng: &mut Rng) -> Vec<DVec3> {
    let mut grid = SiteGrid::new(min_pc);
    let mut sites = Vec::with_capacity(count);
    for _ in 0..count {
        let mut best = (f64::NEG_INFINITY, DVec3::ZERO);
        for _ in 0..PLACEMENT_TRIES {
            let candidate = point_in_slab(slab, rng);
            let gap = grid
                .nearest_within(candidate, &sites)
                .unwrap_or(f64::INFINITY);
            if gap > best.0 {
                best = (gap, candidate);
            }
            if gap >= min_pc {
                break;
            }
        }
        grid.insert(best.1, sites.len());
        sites.push(best.1);
    }
    sites
}

/// Sites bucketed into cubes as wide as the minimum separation, so the
/// nearest site within that separation is found by looking in the 27
/// cubes round a point, not at every site.
struct SiteGrid {
    cell: f64,
    cells: HashMap<(i64, i64, i64), Vec<usize>>,
}

impl SiteGrid {
    fn new(cell: f64) -> Self {
        Self {
            cell,
            cells: HashMap::new(),
        }
    }

    fn key(&self, point: DVec3) -> (i64, i64, i64) {
        let k = |v: f64| (v / self.cell).floor() as i64;
        (k(point.x), k(point.y), k(point.z))
    }

    fn insert(&mut self, point: DVec3, index: usize) {
        self.cells.entry(self.key(point)).or_default().push(index);
    }

    /// The distance to the nearest site within one cell width of `point`,
    /// or `None` if there is none that close.
    fn nearest_within(&self, point: DVec3, sites: &[DVec3]) -> Option<f64> {
        let (x, y, z) = self.key(point);
        let mut nearest: Option<f64> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let Some(indices) = self.cells.get(&(x + dx, y + dy, z + dz)) else {
                        continue;
                    };
                    for &index in indices {
                        let gap = point.distance(sites[index]);
                        if gap < self.cell && nearest.is_none_or(|n| gap < n) {
                            nearest = Some(gap);
                        }
                    }
                }
            }
        }
        nearest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::testing::{check_seeds, check_sizes};

    /// SGEN-10: T = 3N systems, P wide-pair sites, S = T − P system sites
    /// and B brown dwarfs.
    #[test]
    fn the_counts_follow_from_n() {
        let t = GeneratorTunables::default();
        let small = counts(30, &t);
        assert_eq!(small.systems, 90);
        // round(90 × 0.035 ÷ 2) = round(1.575) = 2.
        assert_eq!(small.wide_pairs, 2);
        assert_eq!(small.system_sites, 88);
        assert_eq!(small.brown_dwarfs, 18);
        let large = counts(200, &t);
        assert_eq!(
            (large.systems, large.wide_pairs, large.brown_dwarfs),
            (600, 11, 120)
        );
        assert_eq!(large.system_sites + large.wide_pairs, large.systems);
    }

    /// SGEN-11: the slab's sizes for the presets, and gated systems about
    /// 3.1 pc apart at every N.
    #[test]
    fn the_slab_is_sized_for_its_density() {
        let t = GeneratorTunables::default();
        for (n, width, thickness) in [(30, 17.0, 4.0), (80, 23.0, 6.0), (200, 31.0, 8.0)] {
            let systems = counts(n, &t).systems;
            let (volume, w, th) = slab_size(systems, &t);
            assert!((w - width).abs() < 0.6, "N = {n}: width {w}");
            assert!((th - thickness).abs() < 0.3, "N = {n}: thickness {th}");
            assert!((th / w - 0.25).abs() < 1e-12);
            let disc = std::f64::consts::PI * (w / 2.0).powi(2) * th;
            assert!((disc - volume).abs() < 1e-9 * volume);
            assert!((volume - systems as f64 / 0.1).abs() < 1e-9);
            let spacing = (volume / f64::from(n)).cbrt();
            assert!((spacing - 3.1).abs() < 0.05, "N = {n}: spacing {spacing}");
        }
    }

    /// SGEN-12: every site lies in the slab, at least the minimum
    /// separation from every other, and the plan's stretch is in range,
    /// over many seeds and sizes.
    #[test]
    fn sites_fill_the_slab_and_keep_apart() {
        let t = GeneratorTunables::default();
        for n in check_sizes(&t) {
            for seed in check_seeds(10) {
                let volume = run(n, &mut Rng::new(seed), &t);
                let slab = &volume.slab;
                assert!((1.0..=t.plan_elongation_max).contains(&slab.elongation));
                let plan = std::f64::consts::PI * slab.semi_axis_x_pc() * slab.semi_axis_y_pc();
                let circle = std::f64::consts::PI * (slab.width_pc / 2.0).powi(2);
                assert!((plan - circle).abs() < 1e-9 * circle);
                let sites: Vec<DVec3> = volume
                    .system_sites_km
                    .iter()
                    .chain(&volume.brown_dwarf_sites_km)
                    .map(|km| *km / KM_PER_PARSEC)
                    .collect();
                assert_eq!(volume.system_sites_km.len(), slab.counts.system_sites);
                assert_eq!(volume.brown_dwarf_sites_km.len(), slab.counts.brown_dwarfs);
                for (i, site) in sites.iter().enumerate() {
                    assert!(slab.contains_pc(*site), "N {n} seed {seed}: {site}");
                    for other in &sites[..i] {
                        let gap = site.distance(*other);
                        assert!(
                            gap >= t.min_system_separation_pc,
                            "N {n} seed {seed}: {gap}"
                        );
                    }
                }
            }
        }
    }

    /// SGEN-12: a site with no room is placed at the best of its tries,
    /// not dropped.
    #[test]
    fn a_crowded_site_is_placed_at_its_best_try() {
        let t = GeneratorTunables::default();
        let slab = Slab {
            counts: counts(8, &t),
            volume_pc3: 1.0,
            width_pc: 1.0,
            thickness_pc: 0.2,
            elongation: 1.0,
        };
        // Far more sites than fit 0.4 pc apart in a 1 pc disc.
        let sites = place_sites(&slab, 40, 0.4, &mut Rng::new(3));
        assert_eq!(sites.len(), 40);
        assert!(sites.iter().all(|site| slab.contains_pc(*site)));
    }

    /// SGEN-12: positions are in km.
    #[test]
    fn sites_are_stored_in_km() {
        let t = GeneratorTunables::default();
        let volume = run(30, &mut Rng::new(1), &t);
        let half_width_km = volume.slab.semi_axis_x_pc() * KM_PER_PARSEC;
        let widest = volume
            .system_sites_km
            .iter()
            .map(|site| site.x.abs())
            .fold(0.0, f64::max);
        assert!(widest > 0.5 * half_width_km && widest <= half_width_km);
    }
}

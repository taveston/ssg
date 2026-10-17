//! The band's glow (SKY-13 to SKY-15): a very faint fill in the sky tone
//! along the band, falling off across it as a normal curve and along it
//! towards the core, broken by dark lanes and a slight mottle, and
//! dithered so its gradient shows no steps. `glow.wesl` draws it per
//! pixel; the functions here are the same sums as plain Rust, built for
//! the tests only, since the game itself runs the shader's copy.
//!
//! Lanes and mottle come from value noise of the direction: random values
//! on a grid of whole-number points, blended smoothly in between, and
//! summed over a few octaves (each twice as fine and half as strong). The
//! seed moves the grid, so each sky has its own pattern.

#[cfg(test)]
use bevy::math::DVec3;

#[cfg(test)]
use super::band::{Band, core_weight};
#[cfg(test)]
use crate::view::tunables::Tunables;

/// How fine the lanes are: the noise grid has this many cells per unit of
/// direction at the coarsest octave, so a lane is a few degrees wide...
pub const LANE_SCALE: f32 = 5.5;
/// ...over this many octaves.
pub const LANE_OCTAVES: u32 = 4;
/// The lanes are where that noise climbs from the first value to the
/// second: below it there is no lane, above it the lane is at full depth.
pub const LANE_FROM: f32 = 0.5;
pub const LANE_TO: f32 = 0.72;
/// Lanes lie along the band's centre line: their depth falls off across
/// the band as a normal curve of this spread, in degrees.
pub const LANE_SIGMA_DEG: f32 = 5.5;
/// How fine the mottle is: cells per unit of direction...
pub const MOTTLE_SCALE: f32 = 22.0;
/// ...over this many octaves.
pub const MOTTLE_OCTAVES: u32 = 3;
/// Each octave is this much finer than the last; not exactly 2, so the
/// octaves' grids never line up.
#[cfg(test)]
const OCTAVE_STEP: f32 = 2.03;
/// Past this many of the glow's standard deviations from the centre line
/// the glow is under a hundredth of a display level, and is not drawn.
pub const GLOW_CUTOFF_SIGMAS: f32 = 4.0;
/// Past this many of the lanes' spreads from the centre line a lane is
/// under a thousandth of its depth, and its noise is not worked out.
pub const LANE_CUTOFF_SIGMAS: f32 = 4.0;

/// The seeded offsets of the glow's noise (SKY-14) and dither (SKY-15).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseOffsets {
    /// Where the lanes' noise grid sits.
    pub lanes: [f32; 3],
    /// Where the mottle's noise grid sits.
    pub mottle: [f32; 3],
    /// Mixed into the dither's per-pixel hash.
    pub dither: u32,
}

/// A whole-number grid point hashed to a number in [0, 1). The same
/// integer sums run in `glow.wesl`.
#[cfg(test)]
pub fn lattice_hash(i: i32, j: i32, k: i32) -> f32 {
    let h = (i as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((j as u32).wrapping_mul(668_265_263))
        .wrapping_add((k as u32).wrapping_mul(2_147_483_647));
    let h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    let h = h ^ (h >> 16);
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Value noise at a point: the hashes of the eight grid points round it,
/// blended with a smooth step so there are no creases. In [0, 1).
#[cfg(test)]
pub fn value_noise(p: [f32; 3]) -> f32 {
    let cell = p.map(f32::floor);
    let f: [f32; 3] = std::array::from_fn(|k| p[k] - cell[k]);
    let s = f.map(|t| t * t * (3.0 - 2.0 * t));
    let [x, y, z] = cell.map(|c| c as i32);
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let corner = |dx: i32, dy: i32, dz: i32| lattice_hash(x + dx, y + dy, z + dz);
    let face = |dz: i32| {
        mix(
            mix(corner(0, 0, dz), corner(1, 0, dz), s[0]),
            mix(corner(0, 1, dz), corner(1, 1, dz), s[0]),
            s[1],
        )
    };
    mix(face(0), face(1), s[2])
}

/// Octaves of value noise, normalised to [0, 1): each octave finer by
/// `OCTAVE_STEP` and half as strong.
#[cfg(test)]
pub fn fractal_noise(p: [f32; 3], octaves: u32) -> f32 {
    let (mut sum, mut weight, mut total, mut scale) = (0.0, 0.5, 0.0, 1.0);
    for _ in 0..octaves {
        sum += weight * value_noise(p.map(|c| c * scale));
        total += weight;
        weight *= 0.5;
        scale *= OCTAVE_STEP;
    }
    sum / total
}

/// A smooth step from 0 below `from` to 1 above `to`.
#[cfg(test)]
fn smooth_step(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The noise grid's point for a direction, at a scale and offset.
#[cfg(test)]
fn grid_point(direction: [f32; 3], scale: f32, offset: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|k| direction[k] * scale + offset[k])
}

/// SKY-13: the glow's shape before its lanes and mottle, as a fraction of
/// its peak: a normal curve of spread `sky_glow_sigma_deg` across the
/// band, times the core weight along it. Angles in radians.
#[cfg(test)]
pub fn glow_profile(longitude: f64, latitude: f64, tunables: &Tunables) -> f32 {
    let sigma = f64::from(tunables.sky_glow_sigma_deg).to_radians();
    let across = (-latitude * latitude / (2.0 * sigma * sigma)).exp();
    let along = core_weight(
        longitude,
        f64::from(tunables.sky_core_sigma_deg).to_radians(),
        f64::from(tunables.sky_core_floor),
    );
    (across * along) as f32
}

/// SKY-14: how deep a lane is at a direction, 0 (none) to 1 (full): the
/// lanes' noise past its threshold, strongest on the centre line.
#[cfg(test)]
pub fn lane_strength(direction: [f32; 3], latitude: f64, offsets: &NoiseOffsets) -> f32 {
    let spread = f64::from(LANE_SIGMA_DEG).to_radians();
    if latitude.abs() > f64::from(LANE_CUTOFF_SIGMAS) * spread {
        return 0.0;
    }
    let noise = fractal_noise(
        grid_point(direction, LANE_SCALE, offsets.lanes),
        LANE_OCTAVES,
    );
    let across = (-latitude * latitude / (2.0 * spread * spread)).exp() as f32;
    smooth_step(LANE_FROM, LANE_TO, noise) * across
}

/// SKY-14: the factor the lanes and mottle put on the glow at a
/// direction: a lane lowers it by up to `sky_lane_depth`, and the mottle
/// moves it by up to `sky_glow_mottle` either way. So it stays between
/// (1 − lane depth) × (1 − mottle) and 1 + mottle.
#[cfg(test)]
pub fn glow_factor(
    direction: [f32; 3],
    latitude: f64,
    offsets: &NoiseOffsets,
    tunables: &Tunables,
) -> f32 {
    let lane = lane_strength(direction, latitude, offsets);
    let mottle = fractal_noise(
        grid_point(direction, MOTTLE_SCALE, offsets.mottle),
        MOTTLE_OCTAVES,
    );
    (1.0 - tunables.sky_lane_depth * lane) * (1.0 + tunables.sky_glow_mottle * (2.0 * mottle - 1.0))
}

/// SKY-13 / SKY-14: the glow's opacity at a sky-frame direction:
/// `sky_glow_alpha` × its profile × the lanes' and mottle's factor.
#[cfg(test)]
pub fn glow_opacity(
    direction: DVec3,
    band: &Band,
    offsets: &NoiseOffsets,
    tunables: &Tunables,
) -> f32 {
    let (longitude, latitude) = band.coordinates(direction);
    let cutoff = f64::from(GLOW_CUTOFF_SIGMAS * tunables.sky_glow_sigma_deg).to_radians();
    if latitude.abs() > cutoff {
        return 0.0;
    }
    let d = direction.as_vec3().to_array();
    tunables.sky_glow_alpha
        * glow_profile(longitude, latitude, tunables)
        * glow_factor(d, latitude, offsets, tunables)
}

/// SKY-15: the dither at a pixel, in display levels: a triangular noise
/// between −`levels` and `levels`, averaging zero, the same for the same
/// pixel and seed. It is added to the glow before the display rounds it,
/// so the gradient shows no steps.
#[cfg(test)]
pub fn dither(x: u32, y: u32, seed: u32, levels: f32) -> f32 {
    let a = lattice_hash(x as i32, y as i32, seed as i32);
    let b = lattice_hash(y as i32 ^ 0x5bd1, x as i32, seed.wrapping_add(1) as i32);
    (a + b - 1.0) * levels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::rng::Rng;
    use crate::view::sky::band::make_band;

    fn offsets(seed: u64) -> NoiseOffsets {
        let mut rng = Rng::new(seed);
        let mut point = || std::array::from_fn(|_| rng.uniform(0.0, 200.0) as f32);
        NoiseOffsets {
            lanes: point(),
            mottle: point(),
            dither: seed as u32,
        }
    }

    /// SKY-13: across the band the glow falls as a normal curve: at three
    /// standard deviations it is under 5% of its value on the centre line,
    /// with no hard edge, and it is brightest at the core.
    #[test]
    fn the_glow_fades_across_the_band_with_no_hard_edge() {
        let t = Tunables::default();
        let sigma = f64::from(t.sky_glow_sigma_deg).to_radians();
        for longitude in [0.0, 1.0, 3.0] {
            let centre = glow_profile(longitude, 0.0, &t);
            assert!(glow_profile(longitude, 3.0 * sigma, &t) < 0.05 * centre);
            let mut last = centre;
            for step in 1..120 {
                let here = glow_profile(longitude, f64::from(step) * sigma / 20.0, &t);
                assert!(
                    here < last && last - here < 0.05 * centre,
                    "a step at {step}"
                );
                last = here;
            }
        }
        assert!(glow_profile(0.0, 0.0, &t) > glow_profile(2.0, 0.0, &t));
        assert!((glow_profile(0.0, 0.0, &t) - 1.0).abs() < 1e-6);
    }

    /// SKY-13 / SKY-14: over many directions the glow never exceeds its
    /// peak opacity times the mottle's most, and is nothing far off the
    /// band.
    #[test]
    fn the_glow_stays_faint_and_on_the_band() {
        let t = Tunables::default();
        let band = make_band(&mut Rng::new(4), &t);
        let noise = offsets(4);
        let mut rng = Rng::new(9);
        for _ in 0..20_000 {
            let d = rng.direction();
            let glow = glow_opacity(d, &band, &noise, &t);
            assert!(glow >= 0.0 && glow <= t.sky_glow_alpha * (1.0 + t.sky_glow_mottle));
            let cutoff = f64::from(GLOW_CUTOFF_SIGMAS * t.sky_glow_sigma_deg).to_radians();
            if band.coordinates(d).1.abs() > cutoff {
                assert_eq!(glow, 0.0);
            }
        }
    }

    /// SKY-14: the lanes and mottle keep the glow's factor between
    /// (1 − lane depth) × (1 − mottle) and 1 + mottle, use most of that
    /// range, and change with the seed.
    #[test]
    fn lanes_and_mottle_stay_within_their_bounds_and_follow_the_seed() {
        let t = Tunables::default();
        let (lo, hi) = (
            (1.0 - t.sky_lane_depth) * (1.0 - t.sky_glow_mottle),
            1.0 + t.sky_glow_mottle,
        );
        let (a, b) = (offsets(1), offsets(2));
        let mut rng = Rng::new(5);
        let (mut least, mut most, mut differ) = (f32::MAX, f32::MIN, 0);
        for _ in 0..20_000 {
            let d = rng.direction().as_vec3().to_array();
            let latitude = rng.normal(0.0, 0.05);
            let factor = glow_factor(d, latitude, &a, &t);
            assert!((lo - 1e-5..=hi + 1e-5).contains(&factor), "{factor}");
            least = least.min(factor);
            most = most.max(factor);
            if (factor - glow_factor(d, latitude, &b, &t)).abs() > 0.01 {
                differ += 1;
            }
        }
        assert!(least < 0.6 && most > 1.1, "range {least} to {most}");
        assert!(differ > 15_000, "only {differ} directions differ by seed");
    }

    /// SKY-14: a lane is at full depth only on the centre line and fades
    /// to nothing well off it.
    #[test]
    fn lanes_lie_along_the_centre_line() {
        let noise = offsets(3);
        let mut rng = Rng::new(6);
        for _ in 0..2_000 {
            let d = rng.direction().as_vec3().to_array();
            assert!(lane_strength(d, 25_f64.to_radians(), &noise) < 1e-3);
        }
    }

    /// SKY-15: the dither is at most `sky_dither_levels` either way,
    /// averages zero over the screen, and is fixed per pixel and seed.
    #[test]
    fn the_dither_is_small_and_averages_zero() {
        let t = Tunables::default();
        let levels = t.sky_dither_levels;
        let (mut sum, mut n, mut spread) = (0.0_f64, 0, 0.0_f32);
        for y in 0..400 {
            for x in 0..400 {
                let d = dither(x, y, 77, levels);
                assert!(d.abs() <= levels);
                spread = spread.max(d.abs());
                sum += f64::from(d);
                n += 1;
            }
        }
        assert!(
            (sum / f64::from(n)).abs() < 0.01,
            "mean {}",
            sum / f64::from(n)
        );
        assert!(spread > 0.8 * levels);
        assert_eq!(dither(12, 34, 77, levels), dither(12, 34, 77, levels));
        assert_ne!(dither(12, 34, 77, levels), dither(12, 34, 78, levels));
    }

    /// The noise stays in [0, 1) and is smooth: nearby points differ
    /// little.
    #[test]
    fn value_noise_is_bounded_and_smooth() {
        let mut rng = Rng::new(8);
        for _ in 0..5_000 {
            let p: [f32; 3] = std::array::from_fn(|_| rng.uniform(-50.0, 50.0) as f32);
            let v = fractal_noise(p, 4);
            assert!((0.0..1.0).contains(&v));
            let q = p.map(|c| c + 1e-3);
            assert!((fractal_noise(q, 4) - v).abs() < 0.02);
        }
    }
}

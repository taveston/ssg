//! The sky as a plain value (SKY-01, SKY-03 to SKY-06, SKY-12): made by
//! `make_sky` from a seed and the tunables, with no clock, no window and
//! no unseeded randomness, so the same seed gives the same sky on every
//! run and machine. The view makes it when a game starts; nothing stores
//! it in the sector or a save.
//!
//! It holds field stars, scattered over the whole sphere, and band stars,
//! crowded along the band and towards its core; the band's orientation;
//! and the offsets of the glow's noise. Each part draws from its own
//! stream, derived from the seed and the part's name, so changing how
//! many stars there are leaves the band where it was.

use std::f64::consts::PI;

use bevy::math::DVec3;

use super::band::{Band, core_weight, make_band};
use super::glow::NoiseOffsets;
use super::look::{Rgb, TYPE_WEIGHTS, star_colour, type_colours};
use crate::content::generator::rng::{Rng, derive_seed};
use crate::view::tunables::Tunables;

/// The noise grids are moved by up to this much along each axis: far more
/// than the grid's size, so two seeds' patterns are unrelated, and small
/// enough that the GPU's 32-bit sums stay exact to a fraction of a cell.
const NOISE_OFFSET_RANGE: f64 = 256.0;

/// One star on the sky: its direction in the sky frame (east, north, up;
/// a unit vector), its magnitude (smaller is brighter) and its pale
/// colour (sRGB).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyStar {
    pub direction: DVec3,
    pub magnitude: f64,
    pub colour: Rgb,
}

/// SKY-01: the whole sky.
#[derive(Debug, Clone, PartialEq)]
pub struct Sky {
    /// Stars uniform over the sphere (SKY-05).
    pub field: Vec<SkyStar>,
    /// Stars along the band (SKY-06).
    pub band_stars: Vec<SkyStar>,
    /// The band's orientation (SKY-07, SKY-08).
    pub band: Band,
    /// Where the glow's noise sits (SKY-14, SKY-15).
    pub noise: NoiseOffsets,
}

impl Sky {
    /// Every star, field then band.
    pub fn stars(&self) -> impl Iterator<Item = &SkyStar> {
        self.field.iter().chain(&self.band_stars)
    }
}

/// SKY-05: a magnitude between `lo` and `hi` from `u` in [0, 1), drawn so
/// the number of stars at magnitude m goes as 10 to the power
/// (`slope` × m): a few bright stars and a faint majority. This inverts
/// that law's cumulative count.
pub fn draw_magnitude(u: f64, lo: f64, hi: f64, slope: f64) -> f64 {
    if slope.abs() < 1e-12 || hi <= lo {
        return lo + u * (hi - lo);
    }
    let span = 10_f64.powf(slope * (hi - lo)) - 1.0;
    (lo + (1.0 + u * span).log10() / slope).min(hi)
}

/// SKY-12: a star's colour: a type drawn by its weight, paled towards the
/// sky tone.
fn draw_colour(rng: &mut Rng, colours: &[Rgb; 4], tunables: &Tunables) -> Rgb {
    star_colour(colours[rng.weighted_index(&TYPE_WEIGHTS)], tunables)
}

/// SKY-05: a field star: a direction uniform over the sphere and a
/// magnitude from the field's law.
fn field_star(rng: &mut Rng, colours: &[Rgb; 4], tunables: &Tunables) -> SkyStar {
    SkyStar {
        direction: rng.direction(),
        magnitude: draw_magnitude(
            rng.unit(),
            f64::from(tunables.sky_mag_min),
            f64::from(tunables.sky_mag_max),
            f64::from(tunables.sky_mag_slope),
        ),
        colour: draw_colour(rng, colours, tunables),
    }
}

/// SKY-06: a band star: its latitude from a normal curve about the centre
/// line, and its longitude uniform but kept only with the core weight's
/// chance, so the band is denser towards the core; its magnitude from the
/// band's own, fainter law.
fn band_star(rng: &mut Rng, band: &Band, colours: &[Rgb; 4], tunables: &Tunables) -> SkyStar {
    let sigma = f64::from(tunables.sky_band_sigma_deg).to_radians();
    let core_sigma = f64::from(tunables.sky_core_sigma_deg).to_radians();
    let floor = f64::from(tunables.sky_core_floor);
    let direction = loop {
        let latitude = rng.normal(0.0, sigma);
        let longitude = rng.uniform(-PI, PI);
        if rng.chance(core_weight(longitude, core_sigma, floor)) {
            break band.direction_at(longitude, latitude);
        }
    };
    SkyStar {
        direction,
        magnitude: draw_magnitude(
            rng.unit(),
            f64::from(tunables.sky_band_mag_min),
            f64::from(tunables.sky_band_mag_max),
            f64::from(tunables.sky_band_mag_slope),
        ),
        colour: draw_colour(rng, colours, tunables),
    }
}

/// SKY-14 / SKY-15: the noise's offsets from the seed's stream.
fn noise_offsets(rng: &mut Rng) -> NoiseOffsets {
    let mut point = || std::array::from_fn(|_| rng.uniform(0.0, NOISE_OFFSET_RANGE) as f32);
    let lanes = point();
    let mottle = point();
    NoiseOffsets {
        lanes,
        mottle,
        dither: (rng.next_u64() >> 40) as u32,
    }
}

/// SKY-01: the sky for a seed and the tunables. Each part draws from its
/// own stream, derived from the seed and the part's name.
pub fn make_sky(seed: u64, tunables: &Tunables) -> Sky {
    let colours = type_colours(tunables);
    let band = make_band(&mut Rng::new(derive_seed(seed, "band")), tunables);
    let mut rng = Rng::new(derive_seed(seed, "field"));
    let field = (0..tunables.sky_field_stars)
        .map(|_| field_star(&mut rng, &colours, tunables))
        .collect();
    let mut rng = Rng::new(derive_seed(seed, "band stars"));
    let band_stars = (0..tunables.sky_band_stars)
        .map(|_| band_star(&mut rng, &band, &colours, tunables))
        .collect();
    let noise = noise_offsets(&mut Rng::new(derive_seed(seed, "noise")));
    Sky {
        field,
        band_stars,
        band,
        noise,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tunables with fewer stars, for tests that make many skies.
    fn small(field: usize, band: usize) -> Tunables {
        Tunables {
            sky_field_stars: field,
            sky_band_stars: band,
            ..Tunables::default()
        }
    }

    /// SKY-01: the same seed and tunables give the same sky, field for
    /// field.
    #[test]
    fn the_same_seed_gives_the_same_sky() {
        let t = small(3_000, 5_000);
        assert_eq!(make_sky(42, &t), make_sky(42, &t));
    }

    /// SKY-03: different seeds give different star lists and a different
    /// band.
    #[test]
    fn different_seeds_give_different_skies() {
        let t = small(3_000, 5_000);
        let (a, b) = (make_sky(1, &t), make_sky(2, &t));
        assert_ne!(a.field, b.field);
        assert_ne!(a.band_stars, b.band_stars);
        assert_ne!(a.band.normal, b.band.normal);
        assert_ne!(a.noise, b.noise);
    }

    /// SKY-04: a sky holds `sky_field_stars` field stars and
    /// `sky_band_stars` band stars.
    #[test]
    fn a_sky_holds_the_set_number_of_stars() {
        let t = Tunables::default();
        let sky = make_sky(t.sky_default_seed, &t);
        assert_eq!(sky.field.len(), t.sky_field_stars);
        assert_eq!(sky.band_stars.len(), t.sky_band_stars);
        assert_eq!(sky.stars().count(), t.sky_field_stars + t.sky_band_stars);
    }

    /// Counts of magnitudes in whole-magnitude bins from `lo`.
    fn bins(magnitudes: impl Iterator<Item = f64>, lo: f64, n: usize) -> Vec<f64> {
        let mut counts = vec![0.0; n];
        for m in magnitudes {
            let bin = (m - lo).floor() as usize;
            if bin < n {
                counts[bin] += 1.0;
            }
        }
        counts
    }

    /// Each full bin holds 10^slope times the one before, within the
    /// counts' own noise.
    fn assert_ratio_per_step(counts: &[f64], slope: f64) {
        let ratio = 10_f64.powf(slope);
        for pair in counts.windows(2) {
            let expected = pair[0] * ratio;
            let allowed = 5.0 * (pair[1].sqrt() + ratio * pair[0].sqrt());
            assert!(
                (pair[1] - expected).abs() <= allowed,
                "{} after {}: expected about {expected}",
                pair[1],
                pair[0]
            );
        }
    }

    /// SKY-05: over many seeds, field-star counts per magnitude step grow
    /// by 10 to the power `sky_mag_slope`, inside the range; and their
    /// directions are uniform over the sphere.
    #[test]
    fn field_star_counts_climb_by_the_slope_per_magnitude() {
        let t = Tunables::default();
        let skies: Vec<Sky> = (0..8)
            .map(|seed| make_sky(seed, &small(40_000, 0)))
            .collect();
        let field = || skies.iter().flat_map(|s| &s.field);
        let (lo, hi) = (f64::from(t.sky_mag_min), f64::from(t.sky_mag_max));
        assert!(field().all(|s| (lo..=hi).contains(&s.magnitude)));
        let counts = bins(field().map(|s| s.magnitude), lo, (hi - lo) as usize);
        assert_ratio_per_step(&counts, f64::from(t.sky_mag_slope));
        // Uniform on the sphere: each eighth of it holds an eighth.
        let total = field().count() as f64;
        for octant in 0..8 {
            let inside = field()
                .filter(|s| {
                    let d = s.direction;
                    (d.x > 0.0) == (octant & 1 == 0)
                        && (d.y > 0.0) == (octant & 2 == 0)
                        && (d.z > 0.0) == (octant & 4 == 0)
                })
                .count() as f64;
            assert!((inside / total - 0.125).abs() < 0.005, "octant {octant}");
        }
    }

    /// SKY-06: band stars spread across the band with the set standard
    /// deviation, crowd towards the core by the core weight, and follow
    /// their own fainter magnitude law.
    #[test]
    fn band_stars_spread_across_the_band_and_crowd_to_the_core() {
        let t = Tunables::default();
        let skies: Vec<Sky> = (0..4)
            .map(|seed| make_sky(seed, &small(0, 110_000)))
            .collect();
        let coordinates: Vec<(f64, f64)> = skies
            .iter()
            .flat_map(|s| {
                s.band_stars
                    .iter()
                    .map(|star| s.band.coordinates(star.direction))
            })
            .collect();
        let n = coordinates.len() as f64;
        let sd = (coordinates.iter().map(|(_, b)| b * b).sum::<f64>() / n).sqrt();
        let expected = f64::from(t.sky_band_sigma_deg).to_radians();
        assert!(
            (sd / expected - 1.0).abs() < 0.02,
            "spread {}°",
            sd.to_degrees()
        );
        // Near the core against the far side, by the core weight's ratio.
        let width = 15_f64.to_radians();
        let near = coordinates.iter().filter(|(l, _)| l.abs() < width).count() as f64;
        let far = coordinates
            .iter()
            .filter(|(l, _)| PI - l.abs() < width)
            .count() as f64;
        let core_sigma = f64::from(t.sky_core_sigma_deg).to_radians();
        let floor = f64::from(t.sky_core_floor);
        let mean_weight = |from: f64| {
            (0..100)
                .map(|i| core_weight(from + width * f64::from(i) / 100.0, core_sigma, floor))
                .sum::<f64>()
                / 100.0
        };
        let ratio = mean_weight(0.0) / mean_weight(PI - width);
        assert!((near / far / ratio - 1.0).abs() < 0.08, "{near} vs {far}");
        let (lo, hi) = (f64::from(t.sky_band_mag_min), f64::from(t.sky_band_mag_max));
        let magnitudes = || {
            skies
                .iter()
                .flat_map(|s| s.band_stars.iter().map(|x| x.magnitude))
        };
        assert!(magnitudes().all(|m| (lo..=hi).contains(&m)));
        let counts = bins(magnitudes(), lo, (hi - lo) as usize);
        assert_ratio_per_step(&counts, f64::from(t.sky_band_mag_slope));
    }

    /// SKY-05: the magnitude draw spans its range end to end.
    #[test]
    fn the_magnitude_draw_spans_its_range() {
        assert_eq!(draw_magnitude(0.0, 1.0, 8.0, 0.4), 1.0);
        assert!((draw_magnitude(1.0, 1.0, 8.0, 0.4) - 8.0).abs() < 1e-9);
        assert!((draw_magnitude(0.5, 1.0, 8.0, 0.0) - 4.5).abs() < 1e-9);
    }
}

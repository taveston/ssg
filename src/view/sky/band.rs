//! The band, the Milky Way: a great circle on the sky, tilted off the
//! orbital plane, with a dense core somewhere along it (SKY-06, SKY-07,
//! SKY-08). Its orientation is drawn from the seed, but always so that it
//! crosses the standard view's line of sight, so a new game opens with
//! the band on screen.
//!
//! Directions are in the sky frame (east, north, up; `projection.rs`).
//! The band's **normal** is the pole of its great circle. A direction's
//! **latitude** is its angle from the centre line, and its **longitude**
//! is its angle along the band, measured from the core.

use std::f64::consts::{PI, TAU};

use bevy::math::DVec3;

use super::projection::line_of_sight;
use crate::content::generator::rng::Rng;
use crate::view::tunables::Tunables;

/// The band's orientation: its normal, the direction of its core on the
/// centre line, and the tilt the normal was drawn with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    /// The pole of the band's great circle, a unit vector.
    pub normal: DVec3,
    /// The core: a unit vector on the centre line, longitude 0.
    pub core: DVec3,
    /// The angle between the normal and the plane's normal, in radians.
    pub tilt: f64,
}

impl Band {
    /// The unit vector on the centre line a quarter turn on from the
    /// core, so longitude grows from the core towards it.
    pub fn along(&self) -> DVec3 {
        self.normal.cross(self.core)
    }

    /// The direction at a longitude from the core and a latitude from the
    /// centre line, both in radians.
    pub fn direction_at(&self, longitude: f64, latitude: f64) -> DVec3 {
        let on_line = self.core * longitude.cos() + self.along() * longitude.sin();
        on_line * latitude.cos() + self.normal * latitude.sin()
    }

    /// A direction's longitude from the core and latitude from the centre
    /// line, in radians: the inverse of `direction_at`.
    #[cfg(test)]
    pub fn coordinates(&self, direction: DVec3) -> (f64, f64) {
        let latitude = direction.dot(self.normal).clamp(-1.0, 1.0).asin();
        let longitude = direction.dot(self.along()).atan2(direction.dot(self.core));
        (longitude, latitude)
    }
}

/// The band's normal for a tilt from the plane's normal and an azimuth
/// from north towards east, both in radians.
pub fn band_normal(tilt: f64, azimuth: f64) -> DVec3 {
    DVec3::new(
        tilt.sin() * azimuth.sin(),
        tilt.sin() * azimuth.cos(),
        tilt.cos(),
    )
}

/// The angle between a direction and the band's great circle, in
/// radians: 0 on the centre line, a quarter turn at the pole.
#[cfg(test)]
pub fn distance_from_band(direction: DVec3, normal: DVec3) -> f64 {
    direction.dot(normal).abs().min(1.0).asin()
}

/// SKY-06 / SKY-13: how dense the band is at a longitude from the core,
/// as a fraction of its density there: 1 at the core, falling as a normal
/// curve of spread `core_sigma` (radians) to `floor` far from it.
pub fn core_weight(longitude: f64, core_sigma: f64, floor: f64) -> f64 {
    floor + (1.0 - floor) * (-longitude * longitude / (2.0 * core_sigma * core_sigma)).exp()
}

/// SKY-08: the azimuth of the band's normal, for a tilt, so the band's
/// great circle passes within `margin` (radians) of the line of sight
/// `los`. `u` in [0, 1) picks it uniformly among all azimuths that do. If
/// none does, it is the azimuth that brings the band nearest.
pub fn band_azimuth(tilt: f64, los: DVec3, margin: f64, u: f64) -> f64 {
    // los · normal = r cos(azimuth − lean) + c: a wave in the azimuth.
    let r = tilt.sin() * los.x.hypot(los.y);
    let c = tilt.cos() * los.z;
    let lean = los.x.atan2(los.y);
    let reach = margin.sin();
    if r < 1e-12 {
        // Seen straight down the normal, every azimuth is as near as any.
        return u * TAU;
    }
    let lo = ((-reach - c) / r).max(-1.0);
    let hi = ((reach - c) / r).min(1.0);
    if lo > hi {
        // Out of reach: the wave's nearest point to zero.
        return if c > 0.0 { lean + PI } else { lean };
    }
    // cos(offset) in [lo, hi]: two arcs, mirror images about the lean.
    let (start, end) = (hi.acos(), lo.acos());
    let width = end - start;
    let pick = u * 2.0 * width;
    let offset = if pick < width {
        start + pick
    } else {
        -(start + pick - width)
    };
    (lean + offset).rem_euclid(TAU)
}

/// A unit vector on the band's centre line, at right angles to up where
/// it can be, so longitudes have a fixed origin.
fn reference_on_line(normal: DVec3) -> DVec3 {
    let across = DVec3::Z.cross(normal);
    if across.length() < 1e-9 {
        DVec3::X
    } else {
        across.normalize()
    }
}

/// SKY-07 / SKY-08: the band's orientation from the seed's stream: a tilt
/// uniform between the tunables' bounds, an azimuth that puts the band
/// across the standard view (SKY-08), and the core uniform along the
/// centre line.
pub fn make_band(rng: &mut Rng, tunables: &Tunables) -> Band {
    let tilt = rng.uniform(
        f64::from(tunables.sky_tilt_min_deg).to_radians(),
        f64::from(tunables.sky_tilt_max_deg).to_radians(),
    );
    let los = line_of_sight(0.0, tunables.standard_tilt_deg.to_radians());
    let margin = f64::from(tunables.sky_band_view_margin_deg).to_radians();
    let azimuth = band_azimuth(tilt, los, margin, rng.unit());
    let normal = band_normal(tilt, azimuth);
    let reference = reference_on_line(normal);
    let core_longitude = rng.uniform(0.0, TAU);
    let core = reference * core_longitude.cos() + normal.cross(reference) * core_longitude.sin();
    Band { normal, core, tilt }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::rng::derive_seed;

    fn standard_los(tunables: &Tunables) -> DVec3 {
        line_of_sight(0.0, tunables.standard_tilt_deg.to_radians())
    }

    /// SKY-07 / SKY-08: over 1,000 seeds the band's tilt is inside its
    /// range and its centre line passes within the margin of the standard
    /// view's line of sight.
    #[test]
    fn every_seed_tilts_the_band_in_range_and_across_the_opening_view() {
        let tunables = Tunables::default();
        let (lo, hi) = (
            f64::from(tunables.sky_tilt_min_deg).to_radians(),
            f64::from(tunables.sky_tilt_max_deg).to_radians(),
        );
        let margin = f64::from(tunables.sky_band_view_margin_deg).to_radians();
        let los = standard_los(&tunables);
        for seed in 0..1_000 {
            let band = make_band(&mut Rng::new(derive_seed(seed, "band")), &tunables);
            assert!((lo..=hi).contains(&band.tilt), "seed {seed}");
            let tilt_from_up = band.normal.dot(DVec3::Z).acos();
            assert!((tilt_from_up - band.tilt).abs() < 1e-9);
            assert!(
                distance_from_band(los, band.normal) <= margin + 1e-9,
                "seed {seed} misses the view"
            );
            assert!(band.core.dot(band.normal).abs() < 1e-9);
            assert!((band.core.length() - 1.0).abs() < 1e-9);
        }
    }

    /// SKY-08: the azimuth is spread over every value that works, not
    /// pinned to one: both arcs either side of the lean are used.
    #[test]
    fn the_azimuth_is_spread_over_both_arcs_that_work() {
        let tunables = Tunables::default();
        let los = standard_los(&tunables);
        let margin = 10_f64.to_radians();
        let tilt = 60_f64.to_radians();
        let azimuths: Vec<f64> = (0..200)
            .map(|i| band_azimuth(tilt, los, margin, f64::from(i) / 200.0))
            .collect();
        for &azimuth in &azimuths {
            assert!(distance_from_band(los, band_normal(tilt, azimuth)) <= margin + 1e-9);
        }
        let east = azimuths.iter().filter(|a| a.sin() > 0.0).count();
        assert!((80..=120).contains(&east), "{east} of 200 lean east");
    }

    /// SKY-08: when the tunables put the margin out of reach, the azimuth
    /// is the one that brings the band nearest the line of sight.
    #[test]
    fn out_of_reach_the_band_comes_as_near_as_it_can() {
        let los = line_of_sight(0.0, 60_f32.to_radians());
        let margin = 2_f64.to_radians();
        for tilt_deg in [5.0_f64, 20.0] {
            let tilt = tilt_deg.to_radians();
            let azimuth = band_azimuth(tilt, los, margin, 0.37);
            let got = distance_from_band(los, band_normal(tilt, azimuth));
            let best = (0..3600)
                .map(|i| distance_from_band(los, band_normal(tilt, f64::from(i) / 3600.0 * TAU)))
                .fold(f64::INFINITY, f64::min);
            assert!(got > margin, "tilt {tilt_deg} was reachable");
            assert!(got <= best + 1e-6, "tilt {tilt_deg}: {got} > {best}");
        }
    }

    /// The band's coordinates invert `direction_at`.
    #[test]
    fn band_coordinates_round_trip() {
        let mut rng = Rng::new(3);
        let band = make_band(&mut rng, &Tunables::default());
        for (lon, lat) in [(0.0, 0.0), (1.2, -0.3), (-2.9, 0.6)] {
            let (l, b) = band.coordinates(band.direction_at(lon, lat));
            assert!((l - lon).abs() < 1e-9 && (b - lat).abs() < 1e-9);
        }
    }

    /// SKY-06 / SKY-13: the core weight is 1 at the core and falls to
    /// the floor far from it.
    #[test]
    fn the_core_weight_falls_from_one_to_the_floor() {
        let sigma = 40_f64.to_radians();
        assert!((core_weight(0.0, sigma, 0.4) - 1.0).abs() < 1e-12);
        assert!((core_weight(PI, sigma, 0.4) - 0.4).abs() < 1e-3);
        assert!(core_weight(0.5, sigma, 0.4) > core_weight(1.0, sigma, 0.4));
    }
}

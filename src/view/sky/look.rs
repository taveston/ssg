//! How a sky star looks (SKY-09 to SKY-12, SKY-21): its opacity from its
//! magnitude, its size as a soft point, the light it leaves on the pixels
//! round it, and its pale colour. `stars.wesl` draws each star with the
//! same sums; the ones only the shader needs at run time are built here
//! for the tests.
//!
//! A star is a filled Gaussian, brightest at its centre, never an outline.
//! It is drawn at its exact screen position, so it spreads over the
//! pixels round it; the shader shades every pixel whose centre lies
//! within `star_reach_px` of it on both axes.

use bevy::color::Srgba;

use crate::view::tunables::Tunables;

/// A star is shaded out to this many standard deviations of its
/// Gaussian, where it has fallen to about 1% of its peak...
pub const REACH_SIGMAS: f32 = 3.0;
/// ...plus half a pixel, so the pixel a star's edge falls in is shaded
/// whole. Every pixel shaded costs time, so the square is kept no larger.
pub const REACH_EXTRA_PX: f32 = 0.5;

/// An sRGB colour as three numbers from 0 to 1.
pub type Rgb = [f32; 3];

/// SKY-09: the opacity a star of magnitude `m` is drawn at: `sky_peak` ×
/// 10 to the power (−0.4 × `sky_gamma` × m), so each magnitude step dims
/// it by the same factor. Never above `sky_peak` (the brightest possible
/// star) and never below `sky_faint_floor`, so the faintest stay visible.
pub fn star_opacity(m: f32, tunables: &Tunables) -> f32 {
    let falling = tunables.sky_peak * 10_f32.powf(-0.4 * tunables.sky_gamma * m);
    falling.min(tunables.sky_peak).max(tunables.sky_faint_floor)
}

/// SKY-10: a star's softness: the standard deviation of its Gaussian in
/// screen pixels, `sky_sigma_px` for faint stars and growing by
/// `sky_sigma_bright_px` per magnitude brighter than `sky_bright_mag`.
/// It never depends on the zoom or the field of view. It is never below
/// `steady_sigma_px`, so the star cannot flicker (SKY-11).
pub fn star_sigma_px(m: f32, tunables: &Tunables) -> f32 {
    let sigma = tunables.sky_sigma_px
        + tunables.sky_sigma_bright_px * (tunables.sky_bright_mag - m).max(0.0);
    sigma.max(steady_sigma_px(tunables.sky_flicker_tolerance))
}

/// SKY-11: the least softness that keeps a star's total light within
/// `tolerance` of its mean wherever it sits between pixels. A Gaussian
/// sampled at pixel centres sums to its mean times (1 + 2q cos(2πx)) on
/// each axis, q = exp(−2π²σ²), so the light varies by about 4q at most;
/// this is the σ at which 5q equals the tolerance, leaving room for the
/// finer terms and the cut at the star's reach.
pub fn steady_sigma_px(tolerance: f32) -> f32 {
    let q = (tolerance / 5.0).clamp(1e-9, 1.0);
    (-q.ln() / (2.0 * std::f32::consts::PI * std::f32::consts::PI)).sqrt()
}

/// How far from its centre a star of softness `sigma` is shaded, on each
/// axis, in the same pixels as `sigma`.
#[cfg(test)]
pub fn star_reach_px(sigma: f32) -> f32 {
    REACH_SIGMAS * sigma + REACH_EXTRA_PX
}

/// SKY-10 / SKY-21: the opacity a star leaves on a pixel whose centre is
/// `offset` pixels from the star's centre: `opacity` at the centre,
/// falling smoothly as a Gaussian of spread `sigma`, and nothing beyond
/// its reach. Brightest in the middle, so it is never a ring.
#[cfg(test)]
pub fn star_opacity_at(offset: [f32; 2], opacity: f32, sigma: f32) -> f32 {
    let reach = star_reach_px(sigma);
    if offset[0].abs() > reach || offset[1].abs() > reach {
        return 0.0;
    }
    let r2 = offset[0] * offset[0] + offset[1] * offset[1];
    opacity * (-r2 / (2.0 * sigma * sigma)).exp()
}

/// SKY-11: the total opacity one star leaves on the screen when its
/// centre sits at `position` in pixels: the sum over every pixel it
/// shades. Pixel (i, j) covers [i, i + 1) × [j, j + 1), with its centre
/// half a pixel in.
#[cfg(test)]
pub fn deposited_light(position: [f32; 2], opacity: f32, sigma: f32) -> f32 {
    let reach = star_reach_px(sigma).ceil() as i32 + 1;
    let (cx, cy) = (position[0].floor() as i32, position[1].floor() as i32);
    let mut total = 0.0;
    for j in cy - reach..=cy + reach {
        for i in cx - reach..=cx + reach {
            let offset = [i as f32 + 0.5 - position[0], j as f32 + 0.5 - position[1]];
            total += star_opacity_at(offset, opacity, sigma);
        }
    }
    total
}

/// The four star types and how often each is drawn: bluish-white, white,
/// yellow-white and orange (SKY-12).
pub const TYPE_WEIGHTS: [f64; 4] = [0.25, 0.30, 0.25, 0.20];

/// The type colours in the order of `TYPE_WEIGHTS`.
pub fn type_colours(tunables: &Tunables) -> [Rgb; 4] {
    [
        rgb(tunables.sky_type_blue),
        rgb(tunables.sky_type_white),
        rgb(tunables.sky_type_yellow),
        rgb(tunables.sky_type_orange),
    ]
}

/// A colour's three sRGB channels, 0 to 1.
pub fn rgb(colour: Srgba) -> Rgb {
    [colour.red, colour.green, colour.blue]
}

/// A colour's HSV saturation: how far its strongest channel stands above
/// its weakest, as a fraction of the strongest. 0 for grey.
pub fn saturation(colour: Rgb) -> f32 {
    let max = colour[0].max(colour[1]).max(colour[2]);
    let min = colour[0].min(colour[1]).min(colour[2]);
    if max <= 0.0 { 0.0 } else { (max - min) / max }
}

/// SKY-12: a star's colour: the sky tone moved `sky_colour_keep` of the
/// way towards its type's colour, so it stays pale. If that is still more
/// saturated than `sky_max_saturation`, it is moved towards grey of the
/// same value until it is not, so no star reads as an affiliation colour
/// or the warning orange.
pub fn star_colour(type_colour: Rgb, tunables: &Tunables) -> Rgb {
    let tone = rgb(tunables.sky_tone);
    let keep = tunables.sky_colour_keep;
    let pale: Rgb = std::array::from_fn(|k| tone[k] + keep * (type_colour[k] - tone[k]));
    let current = saturation(pale);
    let cap = tunables.sky_max_saturation;
    if current <= cap {
        return pale;
    }
    let max = pale[0].max(pale[1]).max(pale[2]);
    let scale = cap / current;
    std::array::from_fn(|k| max - (max - pale[k]) * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SKY-09: the brightest possible star is drawn at `sky_peak`, each
    /// magnitude step dims by the same factor, and the faintest star in
    /// the sky stays at or above `sky_faint_floor`.
    #[test]
    fn opacity_runs_from_the_peak_down_to_the_faint_floor() {
        let t = Tunables::default();
        assert!((star_opacity(t.sky_mag_min, &t) - t.sky_peak).abs() < 1e-6);
        assert!((star_opacity(-3.0, &t) - t.sky_peak).abs() < 1e-6);
        let step = star_opacity(4.0, &t) / star_opacity(3.0, &t);
        assert!((step - 10_f32.powf(-0.4 * t.sky_gamma)).abs() < 1e-5);
        let faintest = t.sky_mag_max.max(t.sky_band_mag_max);
        assert!(star_opacity(faintest, &t) >= t.sky_faint_floor);
        let steep = Tunables {
            sky_gamma: 2.0,
            ..Tunables::default()
        };
        assert_eq!(star_opacity(faintest, &steep), steep.sky_faint_floor);
    }

    /// SKY-10: brighter stars are larger, faint ones share the smallest
    /// size.
    #[test]
    fn brighter_stars_are_drawn_larger() {
        let t = Tunables::default();
        assert_eq!(star_sigma_px(8.0, &t), t.sky_sigma_px);
        assert_eq!(star_sigma_px(t.sky_bright_mag, &t), t.sky_sigma_px);
        assert!(star_sigma_px(0.0, &t) > star_sigma_px(2.0, &t));
        let expected = t.sky_sigma_px + t.sky_sigma_bright_px * t.sky_bright_mag;
        assert!((star_sigma_px(0.0, &t) - expected).abs() < 1e-6);
    }

    /// SKY-11: the light a star leaves stays within the flicker tolerance
    /// of its mean at every sub-pixel offset, for the faintest (smallest)
    /// and the brightest (largest) stars alike.
    #[test]
    fn a_star_leaves_the_same_light_wherever_it_sits_between_pixels() {
        let t = Tunables::default();
        for m in [t.sky_mag_max, 4.0, t.sky_mag_min] {
            let (opacity, sigma) = (star_opacity(m, &t), star_sigma_px(m, &t));
            let steps = 16;
            let deposits: Vec<f32> = (0..steps * steps)
                .map(|n| {
                    let (i, j) = (n % steps, n / steps);
                    let at = [
                        100.0 + i as f32 / steps as f32,
                        50.0 + j as f32 / steps as f32,
                    ];
                    deposited_light(at, opacity, sigma)
                })
                .collect();
            let mean = deposits.iter().sum::<f32>() / deposits.len() as f32;
            for deposit in deposits {
                let off = (deposit - mean).abs() / mean;
                assert!(off <= t.sky_flicker_tolerance, "m {m}: {off}");
            }
        }
    }

    /// SKY-11: a softness set below the steady size is lifted to it, and
    /// the light then still stays within the tolerance.
    #[test]
    fn a_star_too_sharp_to_stay_steady_is_softened() {
        let t = Tunables {
            sky_sigma_px: 0.2,
            ..Tunables::default()
        };
        let sigma = star_sigma_px(8.0, &t);
        assert_eq!(sigma, steady_sigma_px(t.sky_flicker_tolerance));
        let deposits: Vec<f32> = (0..64)
            .map(|n| {
                deposited_light(
                    [10.0 + (n % 8) as f32 / 8.0, 10.0 + (n / 8) as f32 / 8.0],
                    0.5,
                    sigma,
                )
            })
            .collect();
        let mean = deposits.iter().sum::<f32>() / deposits.len() as f32;
        assert!(
            deposits
                .iter()
                .all(|d| (d - mean).abs() / mean <= t.sky_flicker_tolerance)
        );
        assert!(steady_sigma_px(t.sky_flicker_tolerance) < Tunables::default().sky_sigma_px);
    }

    /// SKY-21 / SKY-10: a star is a filled soft point: brightest at its
    /// centre and falling steadily outwards, with no ring anywhere.
    #[test]
    fn a_star_is_a_filled_point_never_an_outline() {
        let t = Tunables::default();
        for m in [0.0, 3.0, 9.0] {
            let (opacity, sigma) = (star_opacity(m, &t), star_sigma_px(m, &t));
            assert_eq!(star_opacity_at([0.0, 0.0], opacity, sigma), opacity);
            let mut last = opacity;
            let mut r = 0.05;
            while r < star_reach_px(sigma) {
                let here = star_opacity_at([r * 0.6, r * 0.8], opacity, sigma);
                assert!(here < last, "m {m}: brighter at {r} px than inside");
                last = here;
                r += 0.05;
            }
        }
    }

    /// SKY-12: with the default colours and with every type colour kept
    /// whole, no star colour is more saturated than the cap.
    #[test]
    fn no_star_colour_is_above_the_saturation_cap() {
        let mut t = Tunables::default();
        for keep in [t.sky_colour_keep, 1.0] {
            t.sky_colour_keep = keep;
            for colour in type_colours(&t) {
                let star = star_colour(colour, &t);
                assert!(
                    saturation(star) <= t.sky_max_saturation + 1e-6,
                    "keep {keep}"
                );
            }
        }
    }

    /// SKY-12: a star's colour is the tone moved part of the way towards
    /// its type, so the types still differ, warmer and cooler.
    #[test]
    fn star_colours_are_pale_versions_of_their_types() {
        let t = Tunables::default();
        let [blue, _, _, orange] = type_colours(&t).map(|c| star_colour(c, &t));
        assert!(blue[2] > blue[0], "blue is cool");
        assert!(orange[0] > orange[2], "orange is warm");
        let tone = rgb(t.sky_tone);
        let expected = tone[0] + t.sky_colour_keep * (rgb(t.sky_type_orange)[0] - tone[0]);
        assert!((orange[0] - expected).abs() < 1e-6);
    }
}

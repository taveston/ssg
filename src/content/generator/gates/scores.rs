//! How much the old network's builders wanted each system, for four
//! reasons: settlement, extraction, industry and science (SGEN-50 to
//! SGEN-54). Each score runs from 0 to 1 and depends only on the entry and
//! its summary.

use crate::content::generator::bodies::{
    Atmosphere, BeltSummary, PlanetSummary, Summary, Template, Water,
};
use crate::content::generator::catalogue::{Entry, StarClass};
use crate::content::generator::network::Scores;
use crate::content::generator::tunables::{FourPointFit, GeneratorTunables};

/// How much each kind of water helps a planet's habitability (SGEN-50).
const WATER_FIT_SURFACE: f64 = 1.0;
const WATER_FIT_ICE: f64 = 0.8;
const WATER_FIT_NONE: f64 = 0.2;
/// How much each kind of atmosphere helps a planet's habitability: a thin
/// or dense one fully, none by half, and an envelope (no surface) not at
/// all (SGEN-50).
const ATMOSPHERE_FIT_THIN_OR_DENSE: f64 = 1.0;
const ATMOSPHERE_FIT_AIRLESS: f64 = 0.5;
const ATMOSPHERE_FIT_ENVELOPE: f64 = 0.0;
/// A belt is a water source for industry when at least this share of it
/// is ice: the belts beyond the snow line (SGEN-52).
const ICY_BELT_MIN_FRACTION: f64 = 0.5;

/// A fit that takes four values: 0 below the first and above the last, 1
/// between the middle two, and linear on the slopes (SGEN-50).
pub fn four_point_fit(value: f64, fit: &FourPointFit) -> f64 {
    let [low, good_low, good_high, high] = *fit;
    if value <= low || value >= high {
        0.0
    } else if value < good_low {
        (value - low) / (good_low - low)
    } else if value <= good_high {
        1.0
    } else {
        (high - value) / (high - good_high)
    }
}

/// How calm the star is, from 0 to 1: young M dwarfs flare, so they score
/// `young_m_calm`; every other star is calm (SGEN-50).
pub fn calm_factor(entry: &Entry, t: &GeneratorTunables) -> f64 {
    let young_m =
        entry.primary().is_some_and(|s| s.class == StarClass::M) && entry.age_gyr < t.calm_age_gyr;
    if young_m { t.young_m_calm } else { 1.0 }
}

/// A planet's habitability (SGEN-50): the product of its fits for
/// gravity, equilibrium temperature, water and atmosphere.
pub fn habitability(planet: &PlanetSummary, t: &GeneratorTunables) -> f64 {
    let water = match planet.water {
        Water::Surface => WATER_FIT_SURFACE,
        Water::Ice => WATER_FIT_ICE,
        Water::None => WATER_FIT_NONE,
    };
    let atmosphere = match planet.atmosphere {
        Atmosphere::Thin | Atmosphere::Dense => ATMOSPHERE_FIT_THIN_OR_DENSE,
        Atmosphere::Airless => ATMOSPHERE_FIT_AIRLESS,
        Atmosphere::Envelope => ATMOSPHERE_FIT_ENVELOPE,
    };
    four_point_fit(planet.gravity_g, &t.settle_gravity_g)
        * four_point_fit(planet.equilibrium_temperature_k, &t.settle_t_eq_k)
        * water
        * atmosphere
}

/// The settlement score: the best planet's habitability times the star's
/// calm factor; 0 for a system with no planets (SGEN-50).
pub fn settlement(summary: &Summary, calm: f64, t: &GeneratorTunables) -> f64 {
    let best = summary
        .planets
        .iter()
        .map(|planet| habitability(planet, t))
        .fold(0.0, f64::max);
    best * calm
}

/// A belt's value (SGEN-51): its mass in units of `belt_value_scale_kg`
/// times the value of what it is made of, metal counting most.
pub fn belt_value(belt: &BeltSummary, t: &GeneratorTunables) -> f64 {
    let c = belt.composition;
    belt.mass_kg / t.belt_value_scale_kg
        * (c.metallic * t.value_metal + c.rocky * t.value_rock + c.icy * t.value_ice)
}

/// The extraction score (SGEN-51): 1 − exp(−(B + G)), B the belts' summed
/// value and G `giant_extraction_bonus` per giant planet, both scaled by
/// 10^(`metallicity_value_slope` × [Fe/H]). Heavy belts dominate.
pub fn extraction(summary: &Summary, metallicity: f64, t: &GeneratorTunables) -> f64 {
    let belts: f64 = summary.belts.iter().map(|belt| belt_value(belt, t)).sum();
    let giants = summary.planets.iter().filter(|p| p.kind.is_giant()).count() as f64;
    let richness = 10f64.powf(t.metallicity_value_slope * metallicity);
    1.0 - (-(belts + giants * t.giant_extraction_bonus) * richness).exp()
}

/// A body that can supply industry: where it orbits, how good a source it
/// is, and which body it is, so a body is never paired with itself.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Source {
    orbit_au: f64,
    score: f64,
    body: Body,
}

/// A planet or a belt of a summary, by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Body {
    Planet(usize),
    Belt(usize),
}

/// A source's score from its mass index: 1 − exp(−index) (SGEN-52).
fn source_score(index: f64) -> f64 {
    1.0 - (-index).exp()
}

/// The water and metal sources of a summary (SGEN-52). Water: icy belts,
/// and planets with ice or surface water. Metal: belts with any rock or
/// metal, and rocky planets and super-Earths. A belt's index is its value
/// (`belt_value`), a planet's `planet_source_index`; a belt orbits at its
/// centre, the geometric mean of its edges.
fn sources(summary: &Summary, t: &GeneratorTunables) -> (Vec<Source>, Vec<Source>) {
    let (mut water, mut metal) = (Vec::new(), Vec::new());
    for (index, belt) in summary.belts.iter().enumerate() {
        let source = Source {
            orbit_au: (belt.inner_au * belt.outer_au).sqrt(),
            score: source_score(belt_value(belt, t)),
            body: Body::Belt(index),
        };
        if belt.composition.icy >= ICY_BELT_MIN_FRACTION {
            water.push(source);
        }
        if belt.composition.metallic > 0.0 || belt.composition.rocky > 0.0 {
            metal.push(source);
        }
    }
    for (index, planet) in summary.planets.iter().enumerate() {
        let source = Source {
            orbit_au: planet.orbit_au,
            score: source_score(t.planet_source_index),
            body: Body::Planet(index),
        };
        if matches!(planet.water, Water::Ice | Water::Surface) {
            water.push(source);
        }
        if planet.kind.is_rocky() {
            metal.push(source);
        }
    }
    (water, metal)
}

/// The industry score (SGEN-52): the calm factor times the best pair of a
/// water source and a different metal source whose orbits differ by at
/// most the factor `industry_radius_ratio`, a pair scoring the poorer of
/// its two sources. 0 if there is no such pair.
pub fn industry(summary: &Summary, calm: f64, t: &GeneratorTunables) -> f64 {
    let (water, metal) = sources(summary, t);
    let mut best: f64 = 0.0;
    for w in &water {
        for m in metal.iter().filter(|m| m.body != w.body) {
            let ratio = w.orbit_au.max(m.orbit_au) / w.orbit_au.min(m.orbit_au);
            if ratio <= t.industry_radius_ratio {
                best = best.max(w.score.min(m.score));
            }
        }
    }
    best * calm
}

/// The science score (SGEN-53): the largest value in `science_values` of
/// the unusual things the system has: a white dwarf, youth, a flare star
/// (a young M dwarf), a debris disc, a hot giant, or a metallicity far
/// from the Sun's either way. 0 for an ordinary system.
pub fn science(entry: &Entry, summary: &Summary, t: &GeneratorTunables) -> f64 {
    let values = &t.science_values;
    let class = entry.primary().map(|star| star.class);
    let applies = [
        (class == Some(StarClass::WhiteDwarf), values.white_dwarf),
        (entry.age_gyr < t.young_system_gyr, values.young_system),
        (
            class == Some(StarClass::M) && entry.age_gyr < t.flare_age_gyr,
            values.flare_star,
        ),
        (summary.template == Template::DebrisDisc, values.debris_disc),
        (summary.template == Template::HotGiant, values.hot_giant),
        (
            entry.metallicity.abs() > t.extreme_metallicity,
            values.extreme_metallicity,
        ),
    ];
    applies
        .iter()
        .filter(|(applies, _)| *applies)
        .map(|(_, value)| *value)
        .fold(0.0, f64::max)
}

/// All four scores of a system (SGEN-50 to SGEN-53).
pub fn scores(entry: &Entry, summary: &Summary, t: &GeneratorTunables) -> Scores {
    let calm = calm_factor(entry, t);
    Scores {
        settlement: settlement(summary, calm, t),
        extraction: extraction(summary, entry.metallicity, t),
        industry: industry(summary, calm, t),
        science: science(entry, summary, t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::bodies::{BeltComposition, PlanetKind};
    use crate::content::generator::network::Reason;
    use crate::content::generator::testing::summaries_for;

    fn earth_like() -> PlanetSummary {
        PlanetSummary {
            kind: PlanetKind::Rocky,
            mass_earth: 1.0,
            radius_earth: 1.0,
            orbit_au: 1.0,
            gravity_g: 1.0,
            equilibrium_temperature_k: 255.0,
            atmosphere: Atmosphere::Dense,
            water: Water::Surface,
            volatile_endowment: Some(1.0),
        }
    }

    fn belt(centre_au: f64, mass_kg: f64, rocky: f64, metallic: f64, icy: f64) -> BeltSummary {
        BeltSummary {
            inner_au: centre_au / 1.2,
            outer_au: centre_au * 1.2,
            mass_kg,
            composition: BeltComposition {
                rocky,
                metallic,
                icy,
            },
        }
    }

    fn summary(planets: Vec<PlanetSummary>, belts: Vec<BeltSummary>) -> Summary {
        Summary {
            template: Template::SolarLike,
            snow_line_au: 2.7,
            habitable_zone_inner_au: 0.99,
            habitable_zone_outer_au: 1.7,
            planets,
            belts,
        }
    }

    /// SGEN-50: a four-point fit is 0 outside, 1 in the middle and linear
    /// between.
    #[test]
    fn four_point_fits_ramp_up_hold_and_ramp_down() {
        let fit = [0.2, 0.4, 1.2, 1.6];
        assert_eq!(four_point_fit(0.1, &fit), 0.0);
        assert!((four_point_fit(0.3, &fit) - 0.5).abs() < 1e-12);
        assert_eq!(four_point_fit(1.0, &fit), 1.0);
        assert!((four_point_fit(1.5, &fit) - 0.25).abs() < 1e-12);
        assert_eq!(four_point_fit(1.7, &fit), 0.0);
    }

    /// SGEN-50: an Earth-like planet is fully habitable; airless and dry
    /// costs it, an envelope rules it out, and a young M dwarf halves it.
    #[test]
    fn settlement_scores_the_best_planet_round_a_calm_star() {
        let t = GeneratorTunables::default();
        let earth = earth_like();
        assert_eq!(habitability(&earth, &t), 1.0);
        let barren = PlanetSummary {
            atmosphere: Atmosphere::Airless,
            water: Water::None,
            ..earth.clone()
        };
        assert!((habitability(&barren, &t) - 0.5 * 0.2).abs() < 1e-12);
        let icy = PlanetSummary {
            water: Water::Ice,
            ..earth.clone()
        };
        assert!((habitability(&icy, &t) - 0.8).abs() < 1e-12);
        let envelope = PlanetSummary {
            atmosphere: Atmosphere::Envelope,
            ..earth.clone()
        };
        assert_eq!(habitability(&envelope, &t), 0.0);
        let both = summary(vec![barren, earth], vec![]);
        assert_eq!(settlement(&both, 1.0, &t), 1.0);
        assert_eq!(settlement(&both, t.young_m_calm, &t), 0.5);
        assert_eq!(settlement(&summary(vec![], vec![]), 1.0, &t), 0.0);
        // The calm factor: only young M dwarfs flare.
        let (entries, _) = summaries_for(80, 3);
        for entry in entries.iter().filter(|e| e.has_summary()) {
            let young_m = entry.primary().unwrap().class == StarClass::M && entry.age_gyr < 2.0;
            let expected = if young_m { 0.5 } else { 1.0 };
            assert_eq!(calm_factor(entry, &t), expected);
        }
    }

    /// SGEN-51: extraction is 1 − exp(−(B + G)) scaled by metallicity,
    /// with metal worth the most.
    #[test]
    fn extraction_grows_with_belt_value_giants_and_metallicity() {
        let t = GeneratorTunables::default();
        let rich = belt(2.0, 1e22, 0.7, 0.3, 0.0);
        assert!((belt_value(&rich, &t) - (0.7 + 0.3 * 3.0)).abs() < 1e-12);
        let giant = PlanetSummary {
            kind: PlanetKind::GasGiant,
            atmosphere: Atmosphere::Envelope,
            water: Water::None,
            volatile_endowment: None,
            orbit_au: 5.0,
            ..earth_like()
        };
        let system = summary(vec![giant], vec![rich]);
        let b_plus_g: f64 = 1.6 + 0.1;
        assert!((extraction(&system, 0.0, &t) - (1.0 - (-b_plus_g).exp())).abs() < 1e-12);
        let scaled = b_plus_g * 10f64.powf(0.5 * 0.4);
        assert!((extraction(&system, 0.4, &t) - (1.0 - (-scaled).exp())).abs() < 1e-12);
        assert_eq!(extraction(&summary(vec![], vec![]), 0.0, &t), 0.0);
    }

    /// SGEN-52: industry pairs a water source with a different metal
    /// source within the radius factor, scoring the poorer of the two.
    #[test]
    fn industry_pairs_water_with_metal_nearby() {
        let t = GeneratorTunables::default();
        let icy_belt = belt(3.0, 2e22, 0.25, 0.05, 0.7);
        let rocky_planet = PlanetSummary {
            water: Water::None,
            ..earth_like()
        };
        let planet_score = 1.0 - (-1.0f64).exp();
        let belt_score = 1.0 - (-belt_value(&icy_belt, &t)).exp();
        // The planet at 1 AU and the belt at 3 AU are within a factor 4.
        let near = summary(vec![rocky_planet.clone()], vec![icy_belt.clone()]);
        assert!((industry(&near, 1.0, &t) - planet_score.min(belt_score)).abs() < 1e-12);
        assert!((industry(&near, 0.5, &t) - 0.5 * planet_score.min(belt_score)).abs() < 1e-12);
        // At 0.5 AU the factor is 6: too far.
        let far_planet = PlanetSummary {
            orbit_au: 0.5,
            ..rocky_planet
        };
        assert_eq!(
            industry(&summary(vec![far_planet], vec![icy_belt.clone()]), 1.0, &t),
            0.0
        );
        // One icy belt alone is not a pair with itself.
        assert_eq!(industry(&summary(vec![], vec![icy_belt]), 1.0, &t), 0.0);
        // A wet rocky planet is a water source for a dry one.
        let dry = PlanetSummary {
            water: Water::None,
            orbit_au: 1.5,
            ..earth_like()
        };
        let pair = summary(vec![earth_like(), dry], vec![]);
        assert!((industry(&pair, 1.0, &t) - planet_score).abs() < 1e-12);
    }

    /// SGEN-53: science is the largest value of what applies.
    #[test]
    fn science_takes_the_most_unusual_feature() {
        let t = GeneratorTunables::default();
        let (entries, summaries) = summaries_for(200, 5);
        let mut seen = 0;
        for (entry, summary) in entries.iter().zip(&summaries) {
            let Some(summary) = summary else { continue };
            let score = science(entry, summary, &t);
            let class = entry.primary().unwrap().class;
            if class == StarClass::WhiteDwarf {
                assert_eq!(score, 1.0);
            } else if entry.age_gyr < 1.0 {
                assert_eq!(score, 0.6);
            } else if (class == StarClass::M && entry.age_gyr < 2.0)
                || summary.template == Template::DebrisDisc
            {
                assert_eq!(score, 0.5);
            } else if summary.template == Template::HotGiant {
                assert_eq!(score, 0.4);
            } else if entry.metallicity.abs() > 0.4 {
                assert_eq!(score, 0.3);
            } else {
                assert_eq!(score, 0.0);
                seen += 1;
            }
        }
        assert!(seen > 0);
    }

    /// SGEN-54: the overall score is the best of the four, and the best
    /// reason breaks ties in the order settlement, extraction, industry,
    /// science.
    #[test]
    fn the_overall_score_is_the_best_reason() {
        let scores = Scores {
            settlement: 0.2,
            extraction: 0.7,
            industry: 0.7,
            science: 0.5,
        };
        assert_eq!(scores.overall(), 0.7);
        assert_eq!(scores.best_reason(), Reason::Extraction);
        let level = Scores {
            settlement: 0.0,
            extraction: 0.0,
            industry: 0.0,
            science: 0.0,
        };
        assert_eq!(level.best_reason(), Reason::Settlement);
        let t = GeneratorTunables::default();
        let (entries, summaries) = summaries_for(80, 8);
        for (entry, summary) in entries.iter().zip(&summaries) {
            let Some(summary) = summary else { continue };
            let s = super::scores(entry, summary, &t);
            for reason in Reason::ALL {
                assert!((0.0..=1.0).contains(&s.get(reason)));
                assert!(s.get(reason) <= s.overall());
            }
        }
    }
}

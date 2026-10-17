//! Stage 3, the summaries: gives every system that is not a giant a
//! coarse description: a template, planets, belts and per-planet facts
//! (SGEN-30 to SGEN-43).
//!
//! The order within a system is: the template, its planets, cold giants,
//! belts (which must clear the planets), then each planet's facts. All
//! orbits are circular, coplanar and prograde round the primary.

mod belts;
mod facts;
pub mod planets;

pub use facts::rework_moved_planet;

use super::bodies::Summary;
use super::catalogue::{Entry, Star, StarClass};
use super::rng::Rng;
use super::tunables::GeneratorTunables;
use crate::content::generator::bodies::Template;

/// What the summary of one system is drawn against.
pub struct Context<'a> {
    /// The primary star, which every planet and belt orbits.
    pub star: &'a Star,
    /// The system's [Fe/H].
    pub metallicity: f64,
    pub template: Template,
    /// The snow line, in AU (SGEN-42).
    pub snow_line_au: f64,
    /// Nothing is placed beyond this radius, in AU: the companion limit,
    /// or infinity for a single star (SGEN-30).
    pub limit_au: f64,
}

/// Runs stage 3: a summary for every entry that has one, by id; `None`
/// for brown dwarfs and giants (SGEN-30).
pub fn run(entries: &[Entry], rng: &mut Rng, t: &GeneratorTunables) -> Vec<Option<Summary>> {
    entries
        .iter()
        .map(|entry| entry.has_summary().then(|| summarise(entry, rng, t)))
        .collect()
}

/// The snow line, in AU (SGEN-42): `snow_line_au` × √L, or for a white
/// dwarf `snow_line_au` itself, the line its progenitor had.
pub fn snow_line_au(star: &Star, t: &GeneratorTunables) -> f64 {
    if star.class == StarClass::WhiteDwarf {
        t.snow_line_au
    } else {
        t.snow_line_au * star.luminosity_lsun.sqrt()
    }
}

/// The habitable zone's inner and outer edges, in AU: the Sun's edges
/// times √L (SGEN-42).
pub fn habitable_zone_au(star: &Star, t: &GeneratorTunables) -> (f64, f64) {
    let scale = star.luminosity_lsun.sqrt();
    (t.hz_inner_au * scale, t.hz_outer_au * scale)
}

/// The summary of one system (SGEN-30 to SGEN-41). Planets lie inside
/// `companion_truncation_fraction` × the nearest companion's separation;
/// planets round a pair or of a companion are not made.
fn summarise(entry: &Entry, rng: &mut Rng, t: &GeneratorTunables) -> Summary {
    let system = entry.system().expect("only systems are summarised");
    let star = &system.primary;
    let template = rng.weighted(t.template_weights.get(star.class));
    let ctx = Context {
        star,
        metallicity: entry.metallicity,
        template,
        snow_line_au: snow_line_au(star, t),
        limit_au: system
            .nearest_companion_au()
            .map_or(f64::INFINITY, |separation| {
                separation * t.companion_truncation_fraction
            }),
    };
    let rule = t.template_rules.get(template);
    let mut placed = planets::template_planets(&ctx, rule, rng, t);
    if rule.cold_giants {
        planets::add_cold_giants(&ctx, &mut placed, rng, t);
    }
    placed.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    let belts = belts::draw_belts(&ctx, &placed, rule, rng, t);
    let planets = placed
        .iter()
        .map(|planet| facts::planet_summary(planet, &ctx, rng, t))
        .collect();
    let (habitable_zone_inner_au, habitable_zone_outer_au) = habitable_zone_au(star, t);
    Summary {
        template,
        snow_line_au: ctx.snow_line_au,
        habitable_zone_inner_au,
        habitable_zone_outer_au,
        planets,
        belts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::bodies::PlanetKind;
    use crate::content::generator::testing::{about, check_seeds, sample_star, summaries_for};
    use std::collections::HashMap;

    /// SGEN-30: every system but a giant has a summary, and with
    /// companions every planet lies inside the truncation limit.
    #[test]
    fn systems_get_summaries_truncated_by_companions() {
        let t = GeneratorTunables::default();
        let mut truncated = 0;
        for seed in check_seeds(10) {
            let (entries, summaries) = summaries_for(80, seed);
            assert_eq!(entries.len(), summaries.len());
            for (entry, summary) in entries.iter().zip(&summaries) {
                assert_eq!(summary.is_some(), entry.has_summary());
                let (Some(summary), Some(system)) = (summary, entry.system()) else {
                    continue;
                };
                if let Some(separation) = system.nearest_companion_au() {
                    truncated += 1;
                    let limit = separation * t.companion_truncation_fraction;
                    assert!(summary.planets.iter().all(|p| p.orbit_au <= limit));
                    assert!(summary.belts.iter().all(|b| b.outer_au <= limit));
                }
            }
        }
        assert!(truncated > 0);
    }

    /// SGEN-31: templates follow the class's weights; white dwarfs are
    /// always remnants and nothing else is.
    #[test]
    fn templates_follow_their_class_weights() {
        let t = GeneratorTunables::default();
        let mut tally: HashMap<(StarClass, Template), usize> = HashMap::new();
        let mut per_class: HashMap<StarClass, usize> = HashMap::new();
        for seed in check_seeds(30) {
            let (entries, summaries) = summaries_for(200, seed);
            for (entry, summary) in entries.iter().zip(&summaries) {
                let Some(summary) = summary else { continue };
                let class = entry.primary().unwrap().class;
                *tally.entry((class, summary.template)).or_default() += 1;
                *per_class.entry(class).or_default() += 1;
                assert_eq!(
                    class == StarClass::WhiteDwarf,
                    summary.template == Template::WhiteDwarfRemnant
                );
            }
        }
        let classes = [
            StarClass::M,
            StarClass::K,
            StarClass::G,
            StarClass::F,
            StarClass::A,
        ];
        for class in classes {
            let systems = per_class[&class] as f64;
            for (template, weight) in t.template_weights.get(class) {
                let share = *tally.get(&(class, *template)).unwrap_or(&0) as f64 / systems;
                // Four standard errors of a share from this many systems,
                // so rare classes (few A stars) get a fair margin.
                let margin = 4.0 * (weight * (1.0 - weight) / systems).sqrt();
                assert!(
                    (share - weight).abs() <= margin.max(0.02),
                    "{class:?} {template:?}: {share} vs {weight} over {systems}"
                );
            }
        }
    }

    /// SGEN-33: each template's planets have its kinds, its counts and lie
    /// in its range.
    #[test]
    fn templates_shape_their_planets() {
        let t = GeneratorTunables::default();
        for seed in check_seeds(10) {
            let (entries, summaries) = summaries_for(80, seed);
            for (entry, summary) in entries.iter().zip(&summaries) {
                let Some(summary) = summary else { continue };
                let star = entry.primary().unwrap();
                let snow = summary.snow_line_au;
                let rule = t.template_rules.get(summary.template);
                let own: Vec<_> = summary
                    .planets
                    .iter()
                    .filter(|p| rule.kinds.iter().any(|(kind, _)| *kind == p.kind))
                    .collect();
                assert!(own.len() <= rule.planet_count.max as usize);
                match summary.template {
                    Template::CompactMultiPlanet => {
                        let inner = own.first().map(|p| p.orbit_au);
                        if let Some(inner) = inner {
                            let first = planets::orbit_for_period_au(0.8, star.mass_msun);
                            let last = planets::orbit_for_period_au(6.0, star.mass_msun);
                            assert!(inner >= first * (1.0 - 1e-9));
                            assert!(inner <= last * (1.0 + 1e-9));
                        }
                        assert!(own.iter().all(|p| p.orbit_au <= 0.5 * snow * (1.0 + 1e-9)));
                    }
                    Template::SolarLike => {
                        assert!(own.iter().all(|p| p.orbit_au >= 0.25 * snow * (1.0 - 1e-9)
                            && p.orbit_au <= 1.2 * snow * (1.0 + 1e-9)));
                    }
                    Template::HotGiant => {
                        let giant = summary
                            .planets
                            .iter()
                            .find(|p| p.kind == PlanetKind::GasGiant);
                        if let Some(giant) = giant {
                            let longest = planets::orbit_for_period_au(5.0, star.mass_msun);
                            assert!(giant.orbit_au <= longest * (1.0 + 1e-9));
                        }
                        assert!(own.iter().all(|p| p.orbit_au >= 0.5 * snow * (1.0 - 1e-9)));
                    }
                    Template::DebrisDisc => {
                        assert!(own.iter().all(|p| p.orbit_au <= 2.0 * snow * (1.0 + 1e-9)));
                    }
                    Template::WhiteDwarfRemnant => {
                        assert!(
                            summary
                                .planets
                                .iter()
                                .all(|p| (2.0..=30.0).contains(&p.orbit_au))
                        );
                    }
                }
            }
        }
    }

    /// SGEN-33: compact systems step outward by period ratios of 1.3 to
    /// 2.0, each planet from the last one placed.
    #[test]
    fn compact_systems_are_period_chains() {
        let mut chains = 0;
        for seed in check_seeds(5) {
            let (entries, summaries) = summaries_for(80, seed);
            for (entry, summary) in entries.iter().zip(&summaries) {
                let Some(summary) = summary else { continue };
                if summary.template != Template::CompactMultiPlanet {
                    continue;
                }
                let star = entry.primary().unwrap().mass_msun;
                let small: Vec<f64> = summary
                    .planets
                    .iter()
                    .filter(|p| !p.kind.is_giant())
                    .map(|p| p.orbit_au)
                    .collect();
                for pair in small.windows(2) {
                    // Kepler: the period ratio is the orbit ratio to the
                    // power 3/2, whatever the star's mass.
                    let ratio = (pair[1] / pair[0]).powf(1.5);
                    assert!(about(1.3, 2.0).contains(&ratio), "{ratio} round {star}");
                    chains += 1;
                }
            }
        }
        assert!(chains > 0);
    }

    /// SGEN-42: the habitable zone and snow line scale with √L; a white
    /// dwarf keeps its progenitor's snow line.
    #[test]
    fn zones_scale_with_the_root_of_luminosity() {
        let t = GeneratorTunables::default();
        let mut star = sample_star();
        star.luminosity_lsun = 4.0;
        assert_eq!(habitable_zone_au(&star, &t), (1.98, 3.40));
        assert_eq!(snow_line_au(&star, &t), 5.4);
        star.class = StarClass::WhiteDwarf;
        star.luminosity_lsun = 0.001;
        assert_eq!(snow_line_au(&star, &t), 2.7);
        for seed in check_seeds(3) {
            let (entries, summaries) = summaries_for(30, seed);
            for (entry, summary) in entries.iter().zip(&summaries) {
                let Some(summary) = summary else { continue };
                let star = entry.primary().unwrap();
                assert_eq!(summary.snow_line_au, snow_line_au(star, &t));
                let (inner, outer) = habitable_zone_au(star, &t);
                assert_eq!(
                    (
                        summary.habitable_zone_inner_au,
                        summary.habitable_zone_outer_au
                    ),
                    (inner, outer)
                );
            }
        }
    }

    /// SGEN-43: a summary is planets of the five kinds and belts with
    /// composition fractions only; at most three belts.
    #[test]
    fn summaries_hold_classes_not_deposits() {
        for seed in check_seeds(5) {
            for summary in summaries_for(80, seed).1.into_iter().flatten() {
                assert!(summary.belts.len() <= 3);
                for belt in &summary.belts {
                    let c = belt.composition;
                    assert!(
                        [c.rocky, c.metallic, c.icy]
                            .iter()
                            .all(|f| (0.0..=1.0).contains(f))
                    );
                }
            }
        }
    }
}

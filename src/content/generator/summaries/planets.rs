//! A summary's planet list: the template's planets and any cold giants,
//! each with a kind, a mass and a circular orbit, spaced so neighbours
//! stay stable (SGEN-32 to SGEN-35).

use super::Context;
use crate::content::generator::bodies::PlanetKind;
use crate::content::generator::rng::Rng;
use crate::content::generator::tunables::{GeneratorTunables, OrbitRule, Span, TemplateRule};
use crate::sim::units::{
    EARTH_MASS_KG, KM_PER_AU, SOLAR_MASS_KG, gravitational_parameter_km3_per_day2,
};

/// A planet whose orbit crowds a neighbour is drawn again up to this many
/// times, then dropped (SGEN-34).
pub const HILL_REDRAWS: usize = 20;
/// Each cold giant after the first lies this many times farther out than
/// the one before, the factor drawn uniformly (SGEN-35).
const COLD_GIANT_STEP: (f64, f64) = (1.6, 3.0);

/// A planet before its facts are worked out: its kind, its mass in Earth
/// masses and its orbit radius in AU.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub kind: PlanetKind,
    pub mass_earth: f64,
    pub orbit_au: f64,
}

/// A planet's mass in Earth masses, log-uniform in its kind's range
/// (SGEN-32).
pub fn draw_mass(kind: PlanetKind, rng: &mut Rng, t: &GeneratorTunables) -> f64 {
    let range = t.planet_kind_masses.get(kind);
    rng.log_uniform(range.min, range.max)
}

/// The radius, in AU, of a circular orbit of `period_days` round a star of
/// `star_mass_msun`, by Kepler's third law.
pub fn orbit_for_period_au(period_days: f64, star_mass_msun: f64) -> f64 {
    let mu = gravitational_parameter_km3_per_day2(star_mass_msun * SOLAR_MASS_KG);
    orbits::circular_radius_for_period(mu, period_days) / KM_PER_AU
}

/// The mutual Hill radius of two planets, in AU: ((m₁ + m₂) ÷ (3 M★))^(1/3)
/// × (a₁ + a₂) ÷ 2, masses in Earth masses (SGEN-34).
pub fn mutual_hill_radius_au(a: &Placed, b: &Placed, star_mass_earth: f64) -> f64 {
    ((a.mass_earth + b.mass_earth) / (3.0 * star_mass_earth)).cbrt() * (a.orbit_au + b.orbit_au)
        / 2.0
}

/// A star's mass in Earth masses, the unit planet masses are in.
pub fn star_mass_earth(star_mass_msun: f64) -> f64 {
    star_mass_msun * SOLAR_MASS_KG / EARTH_MASS_KG
}

/// Whether `candidate` lies at least `min_mutual_hill` mutual Hill radii
/// from its nearest neighbour on each side among `placed` (SGEN-34).
pub fn keeps_spacing(
    placed: &[Placed],
    candidate: &Placed,
    ctx: &Context,
    t: &GeneratorTunables,
) -> bool {
    let star = star_mass_earth(ctx.star.mass_msun);
    let inner = placed
        .iter()
        .filter(|p| p.orbit_au <= candidate.orbit_au)
        .max_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    let outer = placed
        .iter()
        .filter(|p| p.orbit_au > candidate.orbit_au)
        .min_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    [inner, outer].into_iter().flatten().all(|neighbour| {
        let gap = (neighbour.orbit_au - candidate.orbit_au).abs();
        gap >= t.min_mutual_hill * mutual_hill_radius_au(neighbour, candidate, star)
    })
}

/// The template's own planets (SGEN-32 to SGEN-34): a hot giant first if
/// the template has one, then a drawn number of planets of the template's
/// kinds placed by its orbit rule. A planet that would lie outside the
/// template's range or beyond the companion limit (SGEN-30) is dropped;
/// one that crowds a neighbour is drawn again, then dropped.
pub fn template_planets(
    ctx: &Context,
    rule: &TemplateRule,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> Vec<Placed> {
    let mut placed = Vec::new();
    if let Some(period) = rule.hot_giant_period_days {
        let mass_earth = draw_mass(PlanetKind::GasGiant, rng, t);
        let orbit_au =
            orbit_for_period_au(rng.log_uniform(period.min, period.max), ctx.star.mass_msun);
        if orbit_au <= ctx.limit_au {
            placed.push(Placed {
                kind: PlanetKind::GasGiant,
                mass_earth,
                orbit_au,
            });
        }
    }
    let count = rng.int_between(rule.planet_count.min, rule.planet_count.max);
    match rule.orbits {
        OrbitRule::PeriodChain {
            first_period_days,
            period_ratio,
            outer_snow_lines,
        } => {
            let chain = PeriodChain {
                first_period_days,
                period_ratio,
                outer_au: outer_snow_lines * ctx.snow_line_au,
            };
            chain.place(count, &mut placed, ctx, rule, rng, t);
        }
        OrbitRule::SnowLines(range) => {
            let range_au = Span::new(range.min * ctx.snow_line_au, range.max * ctx.snow_line_au);
            place_in_range(count, range_au, &mut placed, ctx, rule, rng, t);
        }
        OrbitRule::Au(range_au) => place_in_range(count, range_au, &mut placed, ctx, rule, rng, t),
    }
    placed
}

/// The compact template's rule: the innermost period drawn log-uniformly,
/// each next one a drawn factor longer than the last planet placed, none
/// beyond `outer_au` (SGEN-33).
struct PeriodChain {
    first_period_days: Span,
    period_ratio: Span,
    outer_au: f64,
}

impl PeriodChain {
    fn place(
        &self,
        count: u32,
        placed: &mut Vec<Placed>,
        ctx: &Context,
        rule: &TemplateRule,
        rng: &mut Rng,
        t: &GeneratorTunables,
    ) {
        let mut last_period: Option<f64> = None;
        for _ in 0..count {
            let kind = rng.weighted(&rule.kinds);
            let mass_earth = draw_mass(kind, rng, t);
            for _ in 0..=HILL_REDRAWS {
                let period = match last_period {
                    None => rng.log_uniform(self.first_period_days.min, self.first_period_days.max),
                    Some(last) => last * rng.uniform(self.period_ratio.min, self.period_ratio.max),
                };
                let orbit_au = orbit_for_period_au(period, ctx.star.mass_msun);
                if orbit_au > self.outer_au || orbit_au > ctx.limit_au {
                    // Out of range: dropped, not redrawn.
                    break;
                }
                let candidate = Placed {
                    kind,
                    mass_earth,
                    orbit_au,
                };
                if keeps_spacing(placed, &candidate, ctx, t) {
                    placed.push(candidate);
                    last_period = Some(period);
                    break;
                }
            }
        }
    }
}

/// Places `count` planets of the template's kinds at orbits drawn
/// log-uniformly in `range_au` (SGEN-33, SGEN-34).
fn place_in_range(
    count: u32,
    range_au: Span,
    placed: &mut Vec<Placed>,
    ctx: &Context,
    rule: &TemplateRule,
    rng: &mut Rng,
    t: &GeneratorTunables,
) {
    for _ in 0..count {
        let kind = rng.weighted(&rule.kinds);
        let mass_earth = draw_mass(kind, rng, t);
        place_with_redraws(placed, ctx, rng, t, |rng| Placed {
            kind,
            mass_earth,
            orbit_au: rng.log_uniform(range_au.min, range_au.max),
        });
    }
}

/// Draws a planet with `draw` until it keeps its spacing, up to
/// `HILL_REDRAWS` redraws, and places it; a draw beyond the companion
/// limit is dropped at once (SGEN-30, SGEN-34). Returns the planet if
/// placed.
fn place_with_redraws(
    placed: &mut Vec<Placed>,
    ctx: &Context,
    rng: &mut Rng,
    t: &GeneratorTunables,
    mut draw: impl FnMut(&mut Rng) -> Placed,
) -> Option<Placed> {
    for _ in 0..=HILL_REDRAWS {
        let candidate = draw(rng);
        if candidate.orbit_au > ctx.limit_au {
            return None;
        }
        if keeps_spacing(placed, &candidate, ctx, t) {
            placed.push(candidate);
            return Some(candidate);
        }
    }
    None
}

/// The chance of cold giants for this system: its class group's base
/// chance times 10^(`giant_metallicity_exponent` × [Fe/H]), capped
/// (SGEN-35).
pub fn cold_giant_chance(ctx: &Context, t: &GeneratorTunables) -> f64 {
    use crate::content::generator::catalogue::StarClass;
    let base = if ctx.star.class == StarClass::M {
        t.cold_giant_chance.m
    } else {
        t.cold_giant_chance.fgka
    };
    (base * 10f64.powf(t.giant_metallicity_exponent * ctx.metallicity)).min(t.cold_giant_chance_cap)
}

/// Cold giants, added to the template's planets (SGEN-35): with the
/// system's chance, 1 to 3 giants, each a gas giant with
/// `cold_giant_gas_share` and otherwise an ice giant. The first lies
/// log-uniformly in `cold_giant_orbit_snow_lines` snow lines, each next a
/// drawn factor farther out than the last one placed.
pub fn add_cold_giants(
    ctx: &Context,
    placed: &mut Vec<Placed>,
    rng: &mut Rng,
    t: &GeneratorTunables,
) {
    if !rng.chance(cold_giant_chance(ctx, t)) {
        return;
    }
    let count = rng.weighted(&t.cold_giant_count_weights);
    let first = t.cold_giant_orbit_snow_lines;
    let mut last_au: Option<f64> = None;
    for _ in 0..count {
        let kind = if rng.chance(t.cold_giant_gas_share) {
            PlanetKind::GasGiant
        } else {
            PlanetKind::IceGiant
        };
        let mass_earth = draw_mass(kind, rng, t);
        let giant = place_with_redraws(placed, ctx, rng, t, |rng| {
            let orbit_au = match last_au {
                None => rng.log_uniform(first.min, first.max) * ctx.snow_line_au,
                Some(last) => last * rng.uniform(COLD_GIANT_STEP.0, COLD_GIANT_STEP.1),
            };
            Placed {
                kind,
                mass_earth,
                orbit_au,
            }
        });
        if let Some(giant) = giant {
            last_au = Some(giant.orbit_au);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::bodies::Template;
    use crate::content::generator::testing::{all_summaries, sample_star};

    fn context(star: &crate::content::generator::catalogue::Star, metallicity: f64) -> Context<'_> {
        Context {
            star,
            metallicity,
            template: Template::SolarLike,
            snow_line_au: 2.7,
            limit_au: f64::INFINITY,
        }
    }

    /// SGEN-32: masses lie in their kind's range, and giants are ice and
    /// gas giants.
    #[test]
    fn planet_masses_lie_in_their_kinds_range() {
        let t = GeneratorTunables::default();
        for summary in all_summaries(10) {
            for planet in &summary.planets {
                let range = t.planet_kind_masses.get(planet.kind);
                assert!(range.contains(planet.mass_earth), "{planet:?}");
            }
        }
        assert!(PlanetKind::IceGiant.is_giant() && PlanetKind::GasGiant.is_giant());
        assert!(!PlanetKind::SubNeptune.is_giant());
    }

    /// SGEN-33: a one-year period round one solar mass is 1 AU.
    #[test]
    fn keplers_law_puts_a_year_at_one_au() {
        let a = orbit_for_period_au(365.256_36, 1.0);
        assert!((a - 1.0).abs() < 1e-3, "{a}");
    }

    /// SGEN-34: neighbouring planets keep at least `min_mutual_hill`
    /// mutual Hill radii apart in every summary.
    #[test]
    fn neighbours_keep_their_hill_spacing() {
        let t = GeneratorTunables::default();
        for (star_mass, summary) in crate::content::generator::testing::summaries_with_star_mass(10)
        {
            let star = star_mass_earth(star_mass);
            for pair in summary.planets.windows(2) {
                let [a, b] = pair else { unreachable!() };
                let (a, b) = (
                    Placed {
                        kind: a.kind,
                        mass_earth: a.mass_earth,
                        orbit_au: a.orbit_au,
                    },
                    Placed {
                        kind: b.kind,
                        mass_earth: b.mass_earth,
                        orbit_au: b.orbit_au,
                    },
                );
                let gap = b.orbit_au - a.orbit_au;
                assert!(
                    gap >= t.min_mutual_hill * mutual_hill_radius_au(&a, &b, star) * (1.0 - 1e-12)
                );
            }
        }
    }

    /// SGEN-34: a planet that crowds its neighbour is refused.
    #[test]
    fn a_crowding_planet_is_refused() {
        let t = GeneratorTunables::default();
        let star = sample_star();
        let ctx = context(&star, 0.0);
        let jupiter = Placed {
            kind: PlanetKind::GasGiant,
            mass_earth: 317.8,
            orbit_au: 5.2,
        };
        let close = Placed {
            kind: PlanetKind::Rocky,
            mass_earth: 1.0,
            orbit_au: 5.6,
        };
        let far = Placed {
            kind: PlanetKind::Rocky,
            mass_earth: 1.0,
            orbit_au: 1.0,
        };
        assert!(!keeps_spacing(&[jupiter], &close, &ctx, &t));
        assert!(keeps_spacing(&[jupiter], &far, &ctx, &t));
    }

    /// SGEN-35: the chance of cold giants rises with metallicity, up to
    /// its cap, and is lower round M dwarfs.
    #[test]
    fn cold_giants_grow_likelier_with_metallicity() {
        let t = GeneratorTunables::default();
        let sun = sample_star();
        assert!((cold_giant_chance(&context(&sun, 0.0), &t) - 0.17).abs() < 1e-12);
        let rich = cold_giant_chance(&context(&sun, 0.3), &t);
        assert!((rich - 0.6f64.min(0.17 * 10f64.powf(0.6))).abs() < 1e-12);
        assert_eq!(cold_giant_chance(&context(&sun, 0.6), &t), 0.6);
        let mut dwarf = sun.clone();
        dwarf.class = crate::content::generator::catalogue::StarClass::M;
        assert!((cold_giant_chance(&context(&dwarf, 0.0), &t) - 0.04).abs() < 1e-12);
    }

    /// SGEN-35: cold giants come one to three at a time, the first
    /// between their snow-line limits and each next 1.6 to 3 times farther
    /// out, when nothing crowds them.
    #[test]
    fn cold_giants_step_outward() {
        let t = GeneratorTunables {
            cold_giant_chance_cap: 1.0,
            cold_giant_chance: crate::content::generator::tunables::ColdGiantChance {
                fgka: 1.0,
                m: 1.0,
            },
            ..GeneratorTunables::default()
        };
        let star = sample_star();
        let ctx = context(&star, 0.0);
        let mut seen = [false; 4];
        for seed in 0..200 {
            let mut placed = Vec::new();
            add_cold_giants(&ctx, &mut placed, &mut Rng::new(seed), &t);
            assert!((1..=3).contains(&placed.len()), "{placed:?}");
            seen[placed.len()] = true;
            assert!(placed.iter().all(|p| p.kind.is_giant()));
            let first = placed[0].orbit_au / ctx.snow_line_au;
            assert!(t.cold_giant_orbit_snow_lines.contains(first));
            for pair in placed.windows(2) {
                let step = pair[1].orbit_au / pair[0].orbit_au;
                assert!((1.6..=3.0).contains(&step), "{step}");
            }
        }
        assert!(seen[1] && seen[2]);
    }

    /// SGEN-35: hot-giant and remnant systems get no cold giants.
    #[test]
    fn hot_giants_and_remnants_have_no_cold_giants() {
        for summary in all_summaries(20) {
            let giants = summary.planets.iter().filter(|p| p.kind.is_giant()).count();
            match summary.template {
                // Only the hot giant itself, which a close companion can
                // drop (SGEN-30).
                Template::HotGiant => assert!(giants <= 1),
                Template::CompactMultiPlanet | Template::SolarLike | Template::DebrisDisc => {
                    assert!(giants <= 3)
                }
                Template::WhiteDwarfRemnant => assert!(giants <= 2),
            }
        }
    }
}

//! A summary's belts: up to three annuli round the primary, each with a
//! mass and a rocky, metallic and icy composition set by its side of the
//! snow line (SGEN-37).

use super::Context;
use super::planets::Placed;
use crate::content::generator::bodies::{BeltComposition, BeltSummary, Template};
use crate::content::generator::catalogue::StarClass;
use crate::content::generator::rng::Rng;
use crate::content::generator::tunables::{GeneratorTunables, TemplateRule};

/// A belt that cannot find a clear place is drawn again up to this many
/// times in all, then dropped (SGEN-37).
const BELT_TRIES: usize = 20;

/// The belts of a system, innermost first (SGEN-37). The count is drawn
/// from the template's range. Each belt's centre is log-uniform in
/// `belt_centre_snow_lines` snow lines and its outer ÷ inner radius
/// log-uniform in `belt_width_ratio`, the centre being their geometric
/// mean. A belt is placed only where no planet orbits inside it and it
/// overlaps no other belt; it also keeps inside the companion limit, as
/// planets do (SGEN-30).
pub fn draw_belts(
    ctx: &Context,
    planets: &[Placed],
    rule: &TemplateRule,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> Vec<BeltSummary> {
    let count = rng.int_between(rule.belt_count.min, rule.belt_count.max);
    let mut belts: Vec<BeltSummary> = Vec::new();
    for _ in 0..count {
        for _ in 0..BELT_TRIES {
            let centre = t.belt_centre_snow_lines;
            let centre_au = rng.log_uniform(centre.min, centre.max) * ctx.snow_line_au;
            let width = t.belt_width_ratio;
            let half = rng.log_uniform(width.min, width.max).sqrt();
            let (inner_au, outer_au) = (centre_au / half, centre_au * half);
            let clear = outer_au <= ctx.limit_au
                && !planets
                    .iter()
                    .any(|p| (inner_au..=outer_au).contains(&p.orbit_au))
                && !belts
                    .iter()
                    .any(|b| b.inner_au <= outer_au && inner_au <= b.outer_au);
            if clear {
                let inside = centre_au < ctx.snow_line_au;
                belts.push(BeltSummary {
                    inner_au,
                    outer_au,
                    mass_kg: belt_mass_kg(ctx, inside, rng, t),
                    composition: belt_composition(inside, rng, t),
                });
                break;
            }
        }
    }
    belts.sort_by(|a, b| a.inner_au.total_cmp(&b.inner_au));
    belts
}

/// A belt's composition (SGEN-37). Inside the snow line it is mostly rock
/// with some metal and little ice; beyond it mostly ice with little metal.
/// The rocky fraction is what remains, so the three sum to 1.
pub fn belt_composition(
    inside_snow_line: bool,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> BeltComposition {
    let rule = &t.belt_composition;
    let (icy, metallic) = if inside_snow_line {
        (
            rng.uniform(0.0, rule.inside_icy_max),
            rng.uniform(rule.inside_metallic.min, rule.inside_metallic.max),
        )
    } else {
        (
            rng.uniform(rule.beyond_icy.min, rule.beyond_icy.max),
            rng.uniform(0.0, rule.beyond_metallic_max),
        )
    };
    BeltComposition {
        rocky: 1.0 - icy - metallic,
        metallic,
        icy,
    }
}

/// A belt's mass in kg (SGEN-37): log-uniform in the range for its side of
/// the snow line (round a white dwarf, the white-dwarf range), times
/// `debris_mass_factor` for a debris disc, times 10^[Fe/H].
pub fn belt_mass_kg(
    ctx: &Context,
    inside_snow_line: bool,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> f64 {
    let masses = &t.belt_mass_kg;
    let range = if ctx.star.class == StarClass::WhiteDwarf {
        masses.white_dwarf
    } else if inside_snow_line {
        masses.inside
    } else {
        masses.beyond
    };
    let debris = if ctx.template == Template::DebrisDisc {
        t.debris_mass_factor
    } else {
        1.0
    };
    rng.log_uniform(range.min, range.max) * debris * 10f64.powf(ctx.metallicity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::testing::{about, all_summaries, summaries_for};

    /// SGEN-37: belts come in the template's numbers, sit in their range
    /// of snow lines with their width ratio, hold no planet's orbit and
    /// overlap no other belt.
    #[test]
    fn belts_are_clear_annuli_in_range() {
        let t = GeneratorTunables::default();
        let mut counts = [0usize; 4];
        for summary in all_summaries(10) {
            let rule = t.template_rules.get(summary.template);
            assert!(summary.belts.len() <= rule.belt_count.max as usize);
            counts[summary.belts.len()] += 1;
            for (index, belt) in summary.belts.iter().enumerate() {
                let ratio = belt.outer_au / belt.inner_au;
                assert!(about(1.2, 2.0).contains(&ratio));
                let centre = (belt.inner_au * belt.outer_au).sqrt() / summary.snow_line_au;
                assert!(about(0.15, 6.0).contains(&centre));
                assert!(summary.planets.iter().all(|p| !belt.contains(p.orbit_au)));
                for other in &summary.belts[..index] {
                    assert!(!belt.overlaps(other));
                }
            }
        }
        assert!(counts[3] > 0, "{counts:?}");
    }

    /// SGEN-37: compositions sum to 1 and follow the snow line.
    #[test]
    fn belt_composition_follows_the_snow_line() {
        for summary in all_summaries(10) {
            for belt in &summary.belts {
                let c = belt.composition;
                assert!((c.rocky + c.metallic + c.icy - 1.0).abs() < 1e-12);
                assert!(c.rocky >= 0.0);
                let centre = (belt.inner_au * belt.outer_au).sqrt();
                if centre < summary.snow_line_au {
                    assert!(c.icy <= 0.10 && (0.10..=0.30).contains(&c.metallic));
                } else {
                    assert!((0.60..=0.90).contains(&c.icy) && c.metallic <= 0.05);
                }
            }
        }
    }

    /// SGEN-37: masses lie in their range, scaled for debris discs and by
    /// metallicity.
    #[test]
    fn belt_masses_follow_side_template_and_metallicity() {
        let t = GeneratorTunables::default();
        for seed in crate::content::generator::testing::check_seeds(10) {
            let (entries, summaries) = summaries_for(80, seed);
            for (entry, summary) in entries.iter().zip(&summaries) {
                let Some(summary) = summary else { continue };
                let star = entry.primary().unwrap();
                for belt in &summary.belts {
                    let inside = (belt.inner_au * belt.outer_au).sqrt() < summary.snow_line_au;
                    let range = if star.class == StarClass::WhiteDwarf {
                        t.belt_mass_kg.white_dwarf
                    } else if inside {
                        t.belt_mass_kg.inside
                    } else {
                        t.belt_mass_kg.beyond
                    };
                    let debris = if summary.template == Template::DebrisDisc {
                        30.0
                    } else {
                        1.0
                    };
                    let base = belt.mass_kg / debris / 10f64.powf(entry.metallicity);
                    assert!(about(range.min, range.max).contains(&base));
                }
            }
        }
    }
}

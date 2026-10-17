//! A planet's moons (SYSG-30 to SYSG-35). Regular moons start well clear
//! of the planet, are spaced out geometrically and stay inside part of its
//! Hill sphere, so a giant gets a family of moons from a few to a few
//! dozen planet radii out, and a small planet a moon or none.

use super::worlds::{natural_body, radius_km_for_density, solid_facts};
use super::{Detail, starting_phase};
use crate::content::generator::bodies::{PlanetKind, PlanetSummary};
use crate::content::generator::gates::ring::hill_radius_au;
use crate::content::generator::rng::{Rng, derive_indexed_seed, derive_seed};
use crate::content::generator::summaries::planets::star_mass_earth;
use crate::content::generator::summaries::{Sunlight, solid_body_facts};
use crate::content::generator::tunables::{GeneratorTunables, MoonCount};
use crate::sim::body::BodyClass;
use crate::sim::planet_facts::WorldKind;
use crate::sim::system::BodyBlueprint;
use crate::sim::units::{EARTH_MASS_KG, EARTH_RADIUS_KM, KM_PER_AU};

/// Moons are lettered a to z (SYSG-34), so no planet can have more than
/// this many whatever `moon_max_count` says.
const LETTERS: u32 = 26;

/// SYSG-30: how many moons a planet of `kind` draws, by its kind's range
/// or weights, and never more than `moon_max_count` or the alphabet.
pub fn moon_count(kind: PlanetKind, rng: &mut Rng, t: &GeneratorTunables) -> u32 {
    let count = match t.moon_counts.get(kind) {
        MoonCount::Range(span) => rng.int_between(span.min, span.max),
        MoonCount::Weights(weights) => rng.weighted(weights),
    };
    count.min(t.moon_max_count).min(LETTERS)
}

/// SYSG-31: the outer limit of a planet's moons, in planet radii: the
/// smaller of `moon_outer_limit_radii` and `moon_hill_fraction` of its
/// Hill radius.
pub fn outer_limit_radii(planet_radius_km: f64, hill_radius_km: f64, t: &GeneratorTunables) -> f64 {
    t.moon_outer_limit_radii
        .min(t.moon_hill_fraction * hill_radius_km / planet_radius_km)
}

/// SYSG-31: up to `count` moon orbit radii, in planet radii, innermost
/// first. The first lies between `moon_first_orbit_radii` (and never
/// inside `moon_inner_limit_radii`), each next one `moon_orbit_ratio`
/// times farther out; the first beyond `outer_limit` ends the list, so it
/// and every moon beyond it are dropped.
pub fn orbit_radii(count: u32, outer_limit: f64, rng: &mut Rng, t: &GeneratorTunables) -> Vec<f64> {
    let first = t.moon_first_orbit_radii;
    let ratio = t.moon_orbit_ratio;
    let mut radii = Vec::new();
    for _ in 0..count {
        let radius = match radii.last() {
            None => rng.uniform(first.min, first.max),
            Some(inner) => inner * rng.uniform(ratio.min, ratio.max),
        }
        .max(t.moon_inner_limit_radii);
        if radius > outer_limit {
            break;
        }
        radii.push(radius);
    }
    radii
}

/// SYSG-32: a moon's mass in kg: its planet's times a ratio drawn
/// log-uniformly from the giant or the other range.
pub fn moon_mass_kg(planet: &PlanetSummary, rng: &mut Rng, t: &GeneratorTunables) -> f64 {
    let range = if planet.kind.is_giant() {
        t.moon_mass_ratio_giant
    } else {
        t.moon_mass_ratio_other
    };
    planet.mass_earth * EARTH_MASS_KG * rng.log_uniform(range.min, range.max)
}

/// SYSG-34: the name of a planet's moon at `index` by distance: the
/// planet's name, a space and a lowercase letter, the nearest "a".
pub fn moon_name(planet_name: &str, index: usize) -> String {
    let letter = char::from(b'a' + index as u8);
    format!("{planet_name} {letter}")
}

/// SYSG-30 to SYSG-35: a planet's moons as records round it, nearest
/// first. Each has its orbit radius, a mass from its planet's, a radius
/// from an icy density (the planet at or beyond the snow line) or a rocky
/// one, facts by the rules for a rocky planet at its planet's distance
/// from the star, a name from its planet's and a starting phase of its
/// own. Their periods follow from the planet's mass at spawn (SYSG-35).
/// The draws come from the planet's own seed: the count, the orbits, then
/// each moon's mass and facts.
pub fn moons(
    planet: &PlanetSummary,
    planet_name: &str,
    planet_seed: u64,
    detail: &Detail,
    t: &GeneratorTunables,
) -> Vec<BodyBlueprint> {
    let mut rng = Rng::new(derive_seed(planet_seed, "moons"));
    let planet_radius_km = planet.radius_earth * EARTH_RADIUS_KM;
    let hill_km = hill_radius_au(planet, star_mass_earth(detail.star.mass_msun)) * KM_PER_AU;
    let count = moon_count(planet.kind, &mut rng, t);
    let limit = outer_limit_radii(planet_radius_km, hill_km, t);
    let beyond_snow_line = planet.orbit_au >= detail.summary.snow_line_au;
    let light = Sunlight {
        luminosity_lsun: detail.star.luminosity_lsun,
        distance_au: planet.orbit_au,
        beyond_snow_line,
    };
    orbit_radii(count, limit, &mut rng, t)
        .into_iter()
        .enumerate()
        .map(|(index, out_radii)| {
            let mass_kg = moon_mass_kg(planet, &mut rng, t);
            let radius_km = radius_km_for_density(mass_kg, t.moon_density.get(beyond_snow_line));
            let mass_earth = mass_kg / EARTH_MASS_KG;
            let facts =
                solid_body_facts(mass_earth, radius_km / EARTH_RADIUS_KM, light, &mut rng, t);
            let phase = starting_phase(derive_indexed_seed(planet_seed, "moon", index as u64));
            let mut moon = natural_body(
                moon_name(planet_name, index),
                BodyClass::Moon,
                solid_facts(WorldKind::Moon, facts),
                out_radii * planet_radius_km,
                phase,
            );
            moon.radius_km = radius_km;
            moon.mass_kg = mass_kg;
            moon
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::tunables::CountSpan;

    /// SYSG-30: each kind's moon counts stay in its range, a rocky planet
    /// has none 80% of the time, and none passes `moon_max_count`.
    #[test]
    fn moon_counts_follow_the_planet_kind() {
        let t = GeneratorTunables::default();
        let mut rng = Rng::new(30);
        let draws = |kind, rng: &mut Rng| -> Vec<u32> {
            (0..4000).map(|_| moon_count(kind, rng, &t)).collect()
        };
        let ranges = [
            (PlanetKind::GasGiant, 3, 12),
            (PlanetKind::IceGiant, 1, 6),
            (PlanetKind::SubNeptune, 0, 2),
            (PlanetKind::SuperEarth, 0, 1),
            (PlanetKind::Rocky, 0, 2),
        ];
        for (kind, min, max) in ranges {
            let counts = draws(kind, &mut rng);
            assert!(counts.iter().all(|c| (min..=max).contains(c)), "{kind:?}");
            assert!(counts.contains(&min) && counts.contains(&max), "{kind:?}");
        }
        let rocky = draws(PlanetKind::Rocky, &mut rng);
        let share = |n| rocky.iter().filter(|&&c| c == n).count() as f64 / 4000.0;
        assert!((share(0) - 0.80).abs() < 0.03);
        assert!((share(1) - 0.15).abs() < 0.03);
        assert!((share(2) - 0.05).abs() < 0.02);
        let capped = GeneratorTunables {
            moon_max_count: 4,
            ..t.clone()
        };
        assert!((0..200).all(|_| moon_count(PlanetKind::GasGiant, &mut rng, &capped) <= 4));
        // Past the alphabet the letters run out, so the count stops at 26.
        let mut many = GeneratorTunables {
            moon_max_count: 40,
            ..t.clone()
        };
        many.moon_counts.gas_giant = MoonCount::Range(CountSpan::new(30, 40));
        assert!((0..200).all(|_| moon_count(PlanetKind::GasGiant, &mut rng, &many) == 26));
    }

    /// SYSG-31: orbits start in the first range, grow by the ratio, and
    /// end at the outer limit, dropping every moon beyond.
    #[test]
    fn moon_orbits_grow_geometrically_up_to_the_limit() {
        let t = GeneratorTunables::default();
        let mut rng = Rng::new(31);
        for _ in 0..500 {
            let radii = orbit_radii(12, 30.0, &mut rng, &t);
            let first = radii[0];
            assert!(t.moon_first_orbit_radii.contains(first));
            for pair in radii.windows(2) {
                let ratio = pair[1] / pair[0];
                assert!((1.4 - 1e-9..=2.2 + 1e-9).contains(&ratio), "{ratio}");
            }
            assert!(radii.iter().all(|&r| (2.0..=30.0).contains(&r)));
        }
        // A tight limit leaves only the moons inside it.
        let radii = orbit_radii(12, 9.0, &mut rng, &t);
        assert!(radii.iter().all(|&r| r <= 9.0));
        assert!(orbit_radii(12, 2.0, &mut rng, &t).is_empty());
    }

    /// SYSG-31: the outer limit is the nearer of 30 planet radii and half
    /// the Hill radius.
    #[test]
    fn the_outer_limit_is_the_nearer_of_radii_and_hill_sphere() {
        let t = GeneratorTunables::default();
        assert_eq!(outer_limit_radii(1000.0, 100_000.0, &t), 30.0);
        assert_eq!(outer_limit_radii(1000.0, 20_000.0, &t), 10.0);
    }

    /// SYSG-34: moons are lettered by distance, the nearest "a".
    #[test]
    fn moons_are_lettered_by_distance() {
        assert_eq!(moon_name("Morrow", 0), "Morrow a");
        assert_eq!(moon_name("Morrow", 1), "Morrow b");
        assert_eq!(moon_name("Morrow", 25), "Morrow z");
    }
}

//! The belts and their large bodies (SYSG-40, SYSG-41). Each belt of the
//! summary becomes a belt record named from the system, and may hold a few
//! large bodies: ordinary small worlds with orbits of their own inside the
//! belt, named from the sector's spare names for it.

use super::worlds::{natural_body, radius_km_for_density, solid_facts};
use super::{Detail, starting_phase};
use crate::content::generator::bodies::BeltSummary;
use crate::content::generator::rng::{Rng, derive_indexed_seed, derive_seed};
use crate::content::generator::summaries::{Sunlight, solid_body_facts};
use crate::content::generator::tunables::GeneratorTunables;
use crate::sim::body::{BeltComposition, BodyClass};
use crate::sim::planet_facts::WorldKind;
use crate::sim::system::{BeltBlueprint, BodyBlueprint};
use crate::sim::units::{EARTH_MASS_KG, EARTH_RADIUS_KM, KM_PER_AU};

/// Belt bodies' orbits are at least this ratio apart: 2% (SYSG-41).
const MIN_ORBIT_RATIO: f64 = 1.02;
/// A belt body's orbit that is too near another's or the ring is drawn
/// again up to this many times; then the body is dropped (as planets are,
/// SGEN-34).
const ORBIT_TRIES: usize = 20;
/// A belt whose icy share is above this is mostly icy (SYSG-41).
const MOSTLY: f64 = 0.5;

/// SYSG-40: the belts' names, inside out: "*System* belt" for one belt;
/// "inner" and "outer" for two; "inner", "middle" and "outer" for three.
pub fn belt_names(system: &str, count: usize) -> Vec<String> {
    let places: &[&str] = match count {
        0 => &[],
        1 => &[""],
        2 => &["inner ", "outer "],
        _ => &["inner ", "middle ", "outer "],
    };
    places
        .iter()
        .take(count)
        .map(|place| format!("{system} {place}belt"))
        .collect()
}

/// SYSG-03 / SYSG-40: the summary's belts as records, inside out, with
/// their radii, mass and composition unchanged. Their large bodies are
/// records of their own round the star (`bodies`).
pub fn belts(detail: &Detail) -> Vec<BeltBlueprint> {
    let belts = &detail.summary.belts;
    belts
        .iter()
        .zip(belt_names(&detail.gated.names.system, belts.len()))
        .map(|(belt, name)| BeltBlueprint {
            name,
            inner_radius_km: belt.inner_au * KM_PER_AU,
            outer_radius_km: belt.outer_au * KM_PER_AU,
            mass_kg: Some(belt.mass_kg),
            composition: Some(BeltComposition {
                rocky: belt.composition.rocky,
                metallic: belt.composition.metallic,
                icy: belt.composition.icy,
            }),
            salvage_sites: vec![],
        })
        .collect()
}

/// A large belt body as drawn: its orbit in AU and its mass in Earth
/// masses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawnBody {
    pub orbit_au: f64,
    pub mass_earth: f64,
}

/// SYSG-41: whether an orbit of `radius_au` keeps its distance: at least
/// 2% from every orbit in `others`, and `ring_planet_gap` × the ring's
/// radius from the ring's.
pub fn keeps_clear(radius_au: f64, others: &[f64], ring_au: f64, t: &GeneratorTunables) -> bool {
    let apart = others
        .iter()
        .all(|&other| radius_au.max(other) / radius_au.min(other) >= MIN_ORBIT_RATIO);
    apart && (radius_au - ring_au).abs() >= t.ring_planet_gap * ring_au
}

/// SYSG-41: keeps the heaviest bodies whose masses sum to at most
/// `cap_earth`, dropping the smallest until they do.
pub fn within_mass_cap(mut bodies: Vec<DrawnBody>, cap_earth: f64) -> Vec<DrawnBody> {
    bodies.sort_by(|a, b| b.mass_earth.total_cmp(&a.mass_earth));
    while bodies.iter().map(|b| b.mass_earth).sum::<f64>() > cap_earth {
        bodies.pop();
    }
    bodies
}

/// SYSG-41: a belt's large bodies, inside out. Their number is drawn by
/// `belt_body_count_weights`, at most `belt_body_max`; each has a mass
/// log-uniform in `belt_body_mass_me` and an orbit drawn uniformly across
/// the annulus, drawn again while it is too near another or the ring.
/// Then the smallest are dropped until the masses fit the cap.
pub fn draw_bodies(
    belt: &BeltSummary,
    ring_au: f64,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> Vec<DrawnBody> {
    let count = rng
        .weighted(&t.belt_body_count_weights)
        .min(t.belt_body_max);
    let mut bodies: Vec<DrawnBody> = Vec::new();
    for _ in 0..count {
        let mass_earth = rng.log_uniform(t.belt_body_mass_me.min, t.belt_body_mass_me.max);
        let taken: Vec<f64> = bodies.iter().map(|b| b.orbit_au).collect();
        let orbit = (0..ORBIT_TRIES)
            .map(|_| rng.uniform(belt.inner_au, belt.outer_au))
            .find(|&orbit| keeps_clear(orbit, &taken, ring_au, t));
        if let Some(orbit_au) = orbit {
            bodies.push(DrawnBody {
                orbit_au,
                mass_earth,
            });
        }
    }
    let cap_earth = t.belt_body_mass_cap * belt.mass_kg / EARTH_MASS_KG;
    let mut bodies = within_mass_cap(bodies, cap_earth);
    bodies.sort_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
    bodies
}

/// SYSG-41: every belt's large bodies as records round the star, belt by
/// belt and inside out, each named from the sector's spare names for its
/// belt in that order. A body's radius follows from an icy density in a
/// mostly icy belt and a rocky one otherwise; its facts follow the rules
/// for a rocky planet; its starting phase comes from a seed of its own.
pub fn bodies(detail: &Detail, t: &GeneratorTunables) -> Vec<BodyBlueprint> {
    let mut records = Vec::new();
    for (index, belt) in detail.summary.belts.iter().enumerate() {
        let belt_seed = detail.item_seed("belt", index);
        let mut rng = Rng::new(derive_seed(belt_seed, "bodies"));
        let names = &detail.gated.names.belt_bodies[index];
        let drawn = draw_bodies(belt, detail.gated.ring_radius_au, &mut rng, t);
        let density = t.belt_body_density.get(belt.composition.icy > MOSTLY);
        for (k, (body, name)) in drawn.iter().zip(names).enumerate() {
            let mass_kg = body.mass_earth * EARTH_MASS_KG;
            let radius_km = radius_km_for_density(mass_kg, density);
            let light = Sunlight {
                luminosity_lsun: detail.star.luminosity_lsun,
                distance_au: body.orbit_au,
                beyond_snow_line: body.orbit_au >= detail.summary.snow_line_au,
            };
            let radius_earth = radius_km / EARTH_RADIUS_KM;
            let facts = solid_body_facts(body.mass_earth, radius_earth, light, &mut rng, t);
            let phase = starting_phase(derive_indexed_seed(belt_seed, "body", k as u64));
            let mut record = natural_body(
                name.clone(),
                BodyClass::Planet,
                solid_facts(WorldKind::Rocky, facts),
                body.orbit_au * KM_PER_AU,
                phase,
            );
            record.radius_km = radius_km;
            record.mass_kg = mass_kg;
            records.push(record);
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SYSG-40: belts are named from the system, by place when there are
    /// two or three.
    #[test]
    fn belts_are_named_from_the_system_by_place() {
        assert_eq!(belt_names("Avel", 1), ["Avel belt"]);
        assert_eq!(
            belt_names("Avel", 2),
            ["Avel inner belt", "Avel outer belt"]
        );
        assert_eq!(
            belt_names("Avel", 3),
            ["Avel inner belt", "Avel middle belt", "Avel outer belt"]
        );
        assert!(belt_names("Avel", 0).is_empty());
    }

    /// SYSG-41: the smallest bodies are dropped until the rest fit the
    /// cap.
    #[test]
    fn the_smallest_bodies_go_until_the_rest_fit_the_cap() {
        let body = |mass_earth| DrawnBody {
            orbit_au: 1.0,
            mass_earth,
        };
        let kept = within_mass_cap(vec![body(1.0), body(3.0), body(2.0)], 4.5);
        let masses: Vec<f64> = kept.iter().map(|b| b.mass_earth).collect();
        assert_eq!(masses, [3.0]);
        assert_eq!(within_mass_cap(vec![body(1.0), body(2.0)], 3.0).len(), 2);
        assert!(within_mass_cap(vec![body(1.0)], 0.5).is_empty());
    }

    /// SYSG-41: an orbit keeps 2% from the others and the ring's gap from
    /// the ring.
    #[test]
    fn belt_body_orbits_keep_apart_and_off_the_ring() {
        let t = GeneratorTunables::default();
        assert!(keeps_clear(2.0, &[2.05], 4.0, &t));
        assert!(!keeps_clear(2.0, &[2.03], 4.0, &t));
        assert!(!keeps_clear(3.9, &[], 4.0, &t));
        assert!(keeps_clear(3.7, &[], 4.0, &t));
    }

    /// SYSG-41: the count follows its weights, never past
    /// `belt_body_max`, and every body is inside the annulus.
    #[test]
    fn belt_bodies_follow_their_weights_inside_the_belt() {
        let t = GeneratorTunables::default();
        let belt = BeltSummary {
            inner_au: 2.0,
            outer_au: 3.5,
            mass_kg: 1e23,
            composition: crate::content::generator::bodies::BeltComposition {
                rocky: 0.7,
                metallic: 0.2,
                icy: 0.1,
            },
        };
        let mut rng = Rng::new(41);
        let counts: Vec<usize> = (0..4000)
            .map(|_| {
                let bodies = draw_bodies(&belt, 10.0, &mut rng, &t);
                assert!(bodies.iter().all(|b| belt.contains(b.orbit_au)));
                bodies.len()
            })
            .collect();
        let share = |n| counts.iter().filter(|&&c| c == n).count() as f64 / 4000.0;
        for (n, weight) in [(0, 0.4), (1, 0.3), (2, 0.2), (3, 0.1)] {
            assert!((share(n) - weight).abs() < 0.03, "{n}: {}", share(n));
        }
        let fewer = GeneratorTunables {
            belt_body_max: 1,
            ..t.clone()
        };
        assert!((0..200).all(|_| draw_bodies(&belt, 10.0, &mut rng, &fewer).len() <= 1));
    }
}

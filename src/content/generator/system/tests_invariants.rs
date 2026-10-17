//! SYSG-70: the system generator's invariants, checked over every gated
//! system of the sector sweep (`invariant_seeds` sectors of each size,
//! SGEN-91).

use std::collections::HashSet;
use std::f64::consts::TAU;

use super::moons::outer_limit_radii;
use super::worlds::{planet_facts, radius_km_for_density};
use super::*;
use crate::content::generator::bodies::PlanetSummary;
use crate::content::generator::gates::ring::hill_radius_au;
use crate::content::generator::names::list::NameList;
use crate::content::generator::summaries::planets::star_mass_earth;
use crate::content::generator::summaries::tides::tides;
use crate::content::generator::summaries::{TidalSetting, TideClass};
use crate::content::generator::testing::{about, sweep};
use crate::sim::body::BodyClass;
use crate::sim::planet_facts::Atmosphere;
use crate::sim::system::{BodyBlueprint, OrbitPlacement};
use crate::sim::units::{EARTH_MASS_KG, EARTH_RADIUS_KM, KM_PER_AU};

/// Every gated system of every sector of the sweep, generated once and
/// shared by the tests that read them: the sector's index in the sweep,
/// the system's id and its records.
fn generated() -> &'static [(usize, usize, SystemData)] {
    static GENERATED: std::sync::OnceLock<Vec<(usize, usize, SystemData)>> =
        std::sync::OnceLock::new();
    GENERATED.get_or_init(|| {
        let t = &GeneratorTunables::default();
        sweep()
            .iter()
            .enumerate()
            .flat_map(|(index, sector)| {
                sector.gated.iter().map(move |g| {
                    let system = generate_system(sector, g.id, t).expect("gated");
                    (index, g.id, system)
                })
            })
            .collect()
    })
}

/// What one system's checks read: its sector, records and summary.
struct Case<'a> {
    sector: &'a Sector,
    gated: &'a GatedSystem,
    summary: &'a Summary,
    system: &'a SystemData,
}

fn cases() -> impl Iterator<Item = Case<'static>> {
    generated().iter().map(|(index, id, system)| {
        let sector = &sweep()[*index];
        Case {
            sector,
            gated: sector.gated_system(*id).unwrap(),
            summary: sector.summary(*id).unwrap(),
            system,
        }
    })
}

fn own_radius(body: &BodyBlueprint) -> f64 {
    match body.orbit {
        OrbitPlacement::Own { radius_km } => radius_km,
        OrbitPlacement::Named(name) => panic!("{} sits in {name}", body.name),
    }
}

/// The records of a system's planets: the first of its records round the
/// star, one per planet of the summary.
fn planets<'a>(case: &Case<'a>) -> &'a [BodyBlueprint] {
    &case.system.planets[..case.summary.planets.len()]
}

/// A system's belt bodies: the planets round the star that the summary
/// doesn't list.
fn belt_bodies<'a>(case: &Case<'a>) -> impl Iterator<Item = &'a BodyBlueprint> {
    case.system.planets[case.summary.planets.len()..]
        .iter()
        .filter(|b| b.class == BodyClass::Planet)
}

/// SYSG-02, SYSG-70: generating every system of a sector in reverse
/// order, or again, gives equal records.
#[test]
fn systems_come_out_the_same_in_any_order() {
    let t = GeneratorTunables::default();
    let all = generated();
    for (index, sector) in sweep().iter().enumerate() {
        for g in sector.gated.iter().rev() {
            let again = generate_system(sector, g.id, &t).unwrap();
            let first = all
                .iter()
                .find(|(i, id, _)| *i == index && *id == g.id)
                .map(|(.., system)| system)
                .unwrap();
            assert_eq!(&again, first);
        }
    }
}

/// SYSG-03, SYSG-70: the star, every planet and every belt are their
/// summary's, unchanged.
#[test]
fn detail_never_rerolls_the_summary() {
    for case in cases() {
        let star = case.sector.entries[case.gated.id].primary().unwrap();
        assert_eq!(case.system.star_mass_kg, star.mass_msun * SOLAR_MASS_KG);
        assert_eq!(
            case.system.star_radius_km,
            star.radius_rsun * SOLAR_RADIUS_KM
        );
        assert_eq!(case.system.star_key_facts[0].1, star.spectral_type);
        for (planet, record) in case.summary.planets.iter().zip(planets(&case)) {
            assert_eq!(own_radius(record), planet.orbit_au * KM_PER_AU);
            assert_eq!(record.mass_kg, planet.mass_earth * EARTH_MASS_KG);
            assert_eq!(record.radius_km, planet.radius_earth * EARTH_RADIUS_KM);
            assert_eq!(record.planet_facts, Some(planet_facts(planet)));
        }
        assert_eq!(case.system.belts.len(), case.summary.belts.len());
        for (belt, record) in case.summary.belts.iter().zip(&case.system.belts) {
            assert_eq!(record.inner_radius_km, belt.inner_au * KM_PER_AU);
            assert_eq!(record.outer_radius_km, belt.outer_au * KM_PER_AU);
            assert_eq!(record.mass_kg, Some(belt.mass_kg));
            let mix = record.composition.unwrap();
            let summary_mix = belt.composition;
            assert_eq!(
                [mix.rocky, mix.metallic, mix.icy],
                [summary_mix.rocky, summary_mix.metallic, summary_mix.icy]
            );
        }
    }
}

/// SYSG-31, SYSG-70: moons orbit from the first range outward by the
/// ratio, between the inner and outer limits, and no planet has more
/// than `moon_max_count`.
#[test]
fn moons_keep_their_spacing_and_limits() {
    let t = GeneratorTunables::default();
    let mut moons_seen = 0;
    for case in cases() {
        let star = case.sector.entries[case.gated.id].primary().unwrap();
        for (planet, record) in case.summary.planets.iter().zip(planets(&case)) {
            let radius_km = record.radius_km;
            let hill_km = hill_radius_au(planet, star_mass_earth(star.mass_msun)) * KM_PER_AU;
            let limit = outer_limit_radii(radius_km, hill_km, &t);
            let radii: Vec<f64> = record
                .children
                .iter()
                .map(|moon| own_radius(moon) / radius_km)
                .collect();
            assert!(radii.len() <= t.moon_max_count as usize);
            moons_seen += radii.len();
            if let Some(&first) = radii.first() {
                let lowest = t.moon_first_orbit_radii.min.max(t.moon_inner_limit_radii);
                assert!(about(lowest, t.moon_first_orbit_radii.max).contains(&first));
            }
            for pair in radii.windows(2) {
                let ratio = t.moon_orbit_ratio;
                assert!(about(ratio.min, ratio.max).contains(&(pair[1] / pair[0])));
            }
            let inner = t.moon_inner_limit_radii;
            assert!(radii.iter().all(|r| about(inner, limit).contains(r)));
        }
    }
    assert!(moons_seen > 1000, "{moons_seen} moons");
}

/// SYSG-32, SYSG-70: a moon's mass is its planet's times a ratio in its
/// range, and its radius follows from the icy or rocky density.
#[test]
fn moon_masses_and_radii_follow_their_planet() {
    let t = GeneratorTunables::default();
    for case in cases() {
        for (planet, record) in case.summary.planets.iter().zip(planets(&case)) {
            let range = if planet.kind.is_giant() {
                t.moon_mass_ratio_giant
            } else {
                t.moon_mass_ratio_other
            };
            let icy = planet.orbit_au >= case.summary.snow_line_au;
            for moon in &record.children {
                let ratio = moon.mass_kg / record.mass_kg;
                assert!(about(range.min, range.max).contains(&ratio), "{ratio}");
                let radius = radius_km_for_density(moon.mass_kg, t.moon_density.get(icy));
                assert!((moon.radius_km - radius).abs() <= 1e-9 * radius);
            }
        }
    }
}

/// SYSG-41, SYSG-70: belt bodies lie inside their belt, 2% apart and off
/// the ring, with masses in range summing within the cap, radii from the
/// belt's density, and names from the belt's spare names.
#[test]
fn belt_bodies_fit_their_belt() {
    let t = GeneratorTunables::default();
    let mut bodies_seen = 0;
    for case in cases() {
        let ring_au = case.gated.ring_radius_au;
        let mut orbits: Vec<f64> = Vec::new();
        for (index, belt) in case.summary.belts.iter().enumerate() {
            let inside: Vec<&BodyBlueprint> = belt_bodies(&case)
                .filter(|b| belt.contains(own_radius(b) / KM_PER_AU))
                .collect();
            assert!(inside.len() <= t.belt_body_max as usize);
            bodies_seen += inside.len();
            let mass_me = |b: &BodyBlueprint| b.mass_kg / EARTH_MASS_KG;
            let total: f64 = inside.iter().map(|b| b.mass_kg).sum();
            assert!(total <= t.belt_body_mass_cap * belt.mass_kg * (1.0 + 1e-9));
            let density = t.belt_body_density.get(belt.composition.icy > 0.5);
            for (body, name) in inside.iter().zip(&case.gated.names.belt_bodies[index]) {
                assert_eq!(&body.name, name);
                let span = t.belt_body_mass_me;
                assert!(about(span.min, span.max).contains(&mass_me(body)));
                let radius = radius_km_for_density(body.mass_kg, density);
                assert!((body.radius_km - radius).abs() <= 1e-9 * radius);
                orbits.push(own_radius(body) / KM_PER_AU);
            }
        }
        assert_eq!(orbits.len(), belt_bodies(&case).count(), "all in a belt");
        for (i, a) in orbits.iter().enumerate() {
            for b in &orbits[i + 1..] {
                assert!(a.max(*b) / a.min(*b) >= 1.02 - 1e-12);
            }
            assert!((a - ring_au).abs() >= t.ring_planet_gap * ring_au * (1.0 - 1e-9));
        }
    }
    assert!(bodies_seen > 100, "{bodies_seen} belt bodies");
}

/// The phase of each ring record, in the order of the system's links.
fn ring_phases_in_link_order(case: &Case) -> Vec<f64> {
    let id = case.gated.id;
    case.sector
        .links_of(id)
        .map(|link| {
            let destination = &case
                .sector
                .gated_system(link.other_end(id))
                .unwrap()
                .names
                .system;
            let gate = case
                .system
                .gates
                .iter()
                .find(|g| &g.destination == destination)
                .map(|g| g.angle_deg.to_radians());
            let wreck_name = format!("{destination} gate wreck");
            gate.or_else(|| {
                case.system
                    .planets
                    .iter()
                    .find(|b| b.name == wreck_name)
                    .map(|b| b.phase0_rad)
            })
            .expect("a gate or a wreck for every link")
        })
        .collect()
}

/// SYSG-54, SYSG-70: the ring's records are spread in phase, in link
/// order: each within the jitter of its even place, so no two are closer
/// than (1 − 2 × `gate_jitter_fraction`) of the spacing.
#[test]
fn ring_records_are_spread_in_link_order() {
    let t = GeneratorTunables::default();
    for case in cases() {
        let phases = ring_phases_in_link_order(&case);
        let n = phases.len();
        let spacing = TAU / n as f64;
        let jitter = t.gate_jitter_fraction * spacing;
        for (i, phase) in phases.iter().enumerate() {
            // Offset from the first record's even place, folded into ±π.
            let offset =
                (phase - phases[0] - i as f64 * spacing + TAU * 1.5).rem_euclid(TAU) - TAU / 2.0;
            assert!(offset.abs() <= 2.0 * jitter + 1e-9, "record {i} of {n}");
        }
        let mut sorted = phases.clone();
        sorted.sort_by(f64::total_cmp);
        for (i, phase) in sorted.iter().enumerate() {
            let next = sorted.get(i + 1).copied().unwrap_or(sorted[0] + TAU);
            assert!(next - phase >= (1.0 - 2.0 * t.gate_jitter_fraction) * spacing - 1e-9);
        }
    }
}

/// SYSG-50, SYSG-51, SYSG-70: every system has one gate or wreck per
/// link, all at the ring's radius, and the ring clears every planet the
/// sector did not move: `ring_planet_gap` × its radius, and for a giant
/// `ring_giant_clearance_hill` Hill radii.
#[test]
fn the_ring_holds_every_link_end_clear_of_the_planets() {
    let t = GeneratorTunables::default();
    for case in cases() {
        let ring_au = case.gated.ring_radius_au;
        let ring_km = ring_au * KM_PER_AU;
        let wrecks: Vec<&BodyBlueprint> = case
            .system
            .planets
            .iter()
            .filter(|b| b.class == BodyClass::SalvageSite)
            .collect();
        let links = case.sector.links_of(case.gated.id).count();
        assert_eq!(case.system.gates.len() + wrecks.len(), links);
        assert!(case.system.gates.iter().all(|g| g.distance_km == ring_km));
        assert!(wrecks.iter().all(|w| own_radius(w) == ring_km));
        let star = case.sector.entries[case.gated.id].primary().unwrap();
        let star_earth = star_mass_earth(star.mass_msun);
        for (index, planet) in case.summary.planets.iter().enumerate() {
            if case.gated.moved_planets.contains(&index) {
                continue;
            }
            assert!(clears(planet, ring_au, star_earth, &t), "planet {index}");
        }
    }
}

/// Whether a ring of `ring_au` clears `planet`'s orbit (SGEN-58).
fn clears(planet: &PlanetSummary, ring_au: f64, star_earth: f64, t: &GeneratorTunables) -> bool {
    let gap = (planet.orbit_au - ring_au).abs();
    let slack = 1.0 - 1e-9;
    let off_planet = gap >= t.ring_planet_gap * ring_au * slack;
    let off_giant = !planet.kind.is_giant()
        || gap >= t.ring_giant_clearance_hill * hill_radius_au(planet, star_earth) * slack;
    off_planet && off_giant
}

/// SYSG-60, SYSG-70: over a whole sector no two names of systems, planets
/// and belt bodies are equal, ignoring case, and none is reserved.
#[test]
fn names_are_unique_across_the_sector_and_never_reserved() {
    let reserved: HashSet<String> = NameList::bundled()
        .reserved
        .iter()
        .map(|name| name.to_lowercase())
        .collect();
    let mut seen: Vec<HashSet<String>> = vec![HashSet::new(); sweep().len()];
    for (index, _, system) in generated() {
        let names = std::iter::once(&system.star_name).chain(
            system
                .planets
                .iter()
                .filter(|b| b.class == BodyClass::Planet)
                .map(|b| &b.name),
        );
        for name in names {
            let key = name.to_lowercase();
            assert!(!reserved.contains(&key), "{name} is reserved");
            assert!(seen[*index].insert(key), "{name} twice in sector {index}");
        }
    }
}

/// SGEN-44, SYSG-33: every generated moon of a giant is tidally locked to
/// its planet.
#[test]
fn every_moon_of_a_giant_is_locked() {
    let mut moons_seen = 0;
    for case in cases() {
        for (planet, record) in case.summary.planets.iter().zip(planets(&case)) {
            if !planet.kind.is_giant() {
                continue;
            }
            for moon in &record.children {
                moons_seen += 1;
                assert!(moon.planet_facts.unwrap().locked, "{} is free", moon.name);
            }
        }
    }
    assert!(moons_seen > 0);
}

/// SGEN-45, SYSG-22: a body with an envelope has no activity; every
/// other planet, moon and belt body has one.
#[test]
fn only_bodies_without_an_envelope_have_an_activity() {
    let mut envelopes = 0;
    for case in cases() {
        let bodies = case
            .system
            .planets
            .iter()
            .filter(|b| b.class == BodyClass::Planet)
            .flat_map(|b| std::iter::once(b).chain(&b.children));
        for body in bodies {
            let facts = body.planet_facts.unwrap();
            let envelope = facts.atmosphere == Atmosphere::Envelope;
            envelopes += usize::from(envelope);
            assert_eq!(facts.activity.is_none(), envelope, "{}", body.name);
        }
    }
    assert!(envelopes > 0);
}

/// SYSG-33: a moon's locking and activity take its planet as the parent,
/// with the moon tunables; a belt body's take the star, with the rocky
/// planet ones.
#[test]
fn moons_take_their_planet_and_belt_bodies_the_star_as_tidal_parent() {
    let t = GeneratorTunables::default();
    let mut checked = (0, 0);
    for case in cases() {
        let age_gyr = case.sector.entries[case.gated.id].age_gyr;
        let check = |body: &BodyBlueprint, parent_mass_kg: f64, class: TideClass| {
            let setting = TidalSetting {
                parent_mass_kg,
                orbit_km: own_radius(body),
                class,
                age_gyr,
            };
            let expected = tides(body.mass_kg, body.radius_km, &setting, &t);
            let facts = body.planet_facts.unwrap();
            assert_eq!(facts.locked, expected.locked, "{}", body.name);
            assert_eq!(facts.activity, expected.activity, "{}", body.name);
        };
        for planet in planets(&case) {
            for moon in &planet.children {
                check(moon, planet.mass_kg, TideClass::Moon);
                checked.0 += 1;
            }
        }
        for body in belt_bodies(&case) {
            check(body, case.system.star_mass_kg, TideClass::RockyPlanet);
            checked.1 += 1;
        }
    }
    assert!(checked.0 > 0 && checked.1 > 0, "{checked:?}");
}

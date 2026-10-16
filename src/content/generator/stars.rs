//! Stage 2, the stars: turns the volume's sites into catalogue entries,
//! each with its stars, age and metallicity, and marks which can carry a
//! gate (SGEN-20 to SGEN-29).
//!
//! Entries are numbered in a fixed order: the systems in site order, the
//! second member of a wide pair straight after the first, then the brown
//! dwarfs. Every system draws its primary from the star mix, may have
//! bound companions, and a few sites hold a wide pair of two systems.

use bevy::math::DVec3;

use super::catalogue::{Companion, Entry, EntryKind, Star, StarClass, StarSystem};
use super::rng::Rng;
use super::stellar::{
    effective_temperature_k, main_sequence_lifetime_gyr, main_sequence_luminosity,
    main_sequence_radius, spectral_type,
};
use super::tunables::GeneratorTunables;
use super::volume::Volume;
use crate::sim::units::KM_PER_AU;

/// A giant's age is its progenitor's main-sequence lifetime times a factor
/// drawn from this range: it left the main sequence a little while ago
/// (SGEN-23).
const GIANT_AGE_FACTOR: (f64, f64) = (1.0, 1.1);
/// A companion's separation that falls outside its limits is drawn again
/// up to this many times, then pulled to the nearer limit (SGEN-26).
const SEPARATION_TRIES: usize = 20;
/// The letters companions add to a designation, nearest first (SGEN-20).
const COMPANION_LETTERS: [char; 3] = ['B', 'C', 'D'];

/// Runs stage 2: every system site becomes a system (two for a wide
/// pair), every brown-dwarf site a brown dwarf (SGEN-20 to SGEN-29).
pub fn run(volume: &Volume, rng: &mut Rng, tunables: &GeneratorTunables) -> Vec<Entry> {
    let sites = &volume.system_sites_km;
    let mut holds_pair = vec![false; sites.len()];
    for index in rng.sample_indices(sites.len(), volume.slab.counts.wide_pairs) {
        holds_pair[index] = true;
    }
    let mut entries = Vec::new();
    for (site, pair) in sites.iter().zip(holds_pair) {
        let first = entries.len();
        entries.push(draw_system(first, *site, rng, tunables));
        if pair {
            let partner = first + 1;
            let position = *site + wide_pair_offset_km(rng, tunables);
            entries.push(draw_system(partner, position, rng, tunables));
            link_wide_pair(&mut entries, first, partner);
        }
    }
    for site in &volume.brown_dwarf_sites_km {
        let id = entries.len();
        entries.push(draw_brown_dwarf(id, *site, rng, tunables));
    }
    entries
}

/// "OW-0173": the prefix and the id as four digits (SGEN-20).
pub fn designation(id: usize, tunables: &GeneratorTunables) -> String {
    format!("{}-{id:04}", tunables.designation_prefix)
}

/// Where a wide pair's second system sits relative to the first: a
/// separation drawn log-uniformly between `wide_pair_min_au` and
/// `wide_pair_max_au`, in a random direction (SGEN-27).
fn wide_pair_offset_km(rng: &mut Rng, tunables: &GeneratorTunables) -> DVec3 {
    let separation_au = rng.log_uniform(tunables.wide_pair_min_au, tunables.wide_pair_max_au);
    rng.direction() * separation_au * KM_PER_AU
}

/// Each member of a wide pair records the other (SGEN-27).
fn link_wide_pair(entries: &mut [Entry], first: usize, second: usize) {
    for (this, other) in [(first, second), (second, first)] {
        if let EntryKind::System(system) = &mut entries[this].kind {
            system.wide_partner = Some(other);
        }
    }
}

/// One star system: a primary from the star mix, its age and metallicity,
/// and any companions (SGEN-21 to SGEN-26, SGEN-29).
fn draw_system(id: usize, position_km: DVec3, rng: &mut Rng, t: &GeneratorTunables) -> Entry {
    let mix: Vec<(StarClass, f64)> = StarClass::ALL
        .iter()
        .map(|&class| (class, *t.star_mix.get(class)))
        .collect();
    let class = rng.weighted(&mix);
    let mass = rng.log_uniform(
        t.star_mass_ranges.get(class).min,
        t.star_mass_ranges.get(class).max,
    );
    let primary = draw_star(class, mass, rng, t);
    let age_gyr = draw_age(&primary, rng, t);
    let metallicity = draw_metallicity(rng, t);
    let designation = designation(id, t);
    let companions = draw_companions(&primary, &designation, rng, t);
    let eligible = is_eligible(&primary, &companions);
    Entry {
        id,
        designation,
        position_km,
        age_gyr,
        metallicity,
        kind: EntryKind::System(StarSystem {
            primary,
            companions,
            wide_partner: None,
        }),
        eligible,
    }
}

/// A star of `class` and `mass`, with its luminosity, radius, temperature
/// and spectral type (SGEN-22). A main-sequence star's follow from its
/// mass. A white dwarf has a fixed radius and a luminosity drawn
/// log-uniformly. A giant's radius and luminosity are drawn from the same
/// place in their ranges, so the larger giants are the brighter ones.
pub fn draw_star(class: StarClass, mass_msun: f64, rng: &mut Rng, t: &GeneratorTunables) -> Star {
    let (luminosity_lsun, radius_rsun) = match class {
        StarClass::WhiteDwarf => {
            let range = t.white_dwarf_luminosity_lsun;
            (
                rng.log_uniform(range.min, range.max),
                t.white_dwarf_radius_rsun,
            )
        }
        StarClass::Giant => {
            let giant = &t.giant_properties;
            let place = rng.unit();
            let along = |min: f64, max: f64| (min.ln() + (max.ln() - min.ln()) * place).exp();
            (
                along(giant.luminosity_lsun.min, giant.luminosity_lsun.max),
                along(giant.radius_rsun.min, giant.radius_rsun.max),
            )
        }
        _ => (
            main_sequence_luminosity(mass_msun, &t.star_mass_luminosity),
            main_sequence_radius(mass_msun, &t.star_mass_radius),
        ),
    };
    let temperature_k = effective_temperature_k(luminosity_lsun, radius_rsun);
    let range = *t.star_mass_ranges.get(class);
    Star {
        class,
        mass_msun,
        luminosity_lsun,
        radius_rsun,
        temperature_k,
        spectral_type: spectral_type(class, mass_msun, range, temperature_k),
    }
}

/// A system's age in Gyr (SGEN-23). A main-sequence star's is uniform from
/// `age_min_gyr` to `age_max_gyr`, or to its lifetime if that is shorter,
/// so no star outlives its main sequence. A white dwarf's is uniform from
/// `white_dwarf_age_min_gyr` to `age_max_gyr`. A giant's is its
/// progenitor's lifetime, a little over.
pub fn draw_age(primary: &Star, rng: &mut Rng, t: &GeneratorTunables) -> f64 {
    match primary.class {
        StarClass::WhiteDwarf => rng.uniform(t.white_dwarf_age_min_gyr, t.age_max_gyr),
        StarClass::Giant => {
            let factor = rng.uniform(GIANT_AGE_FACTOR.0, GIANT_AGE_FACTOR.1);
            main_sequence_lifetime_gyr(primary.mass_msun) * factor
        }
        _ => {
            let oldest = t
                .age_max_gyr
                .min(main_sequence_lifetime_gyr(primary.mass_msun));
            if oldest <= t.age_min_gyr {
                oldest
            } else {
                rng.uniform(t.age_min_gyr, oldest)
            }
        }
    }
}

/// [Fe/H], normal round `metallicity_mean` with spread
/// `metallicity_sigma`, clamped to ± `metallicity_clamp` (SGEN-24).
pub fn draw_metallicity(rng: &mut Rng, t: &GeneratorTunables) -> f64 {
    rng.normal(t.metallicity_mean, t.metallicity_sigma)
        .clamp(-t.metallicity_clamp, t.metallicity_clamp)
}

/// A system's bound companions, nearest first (SGEN-25, SGEN-26). The
/// system is multiple with its class's chance; a multiple has 2, 3 or 4
/// stars by `star_count_weights`. Each companion's class is drawn from the
/// star mix, limited to main-sequence classes no heavier than the
/// primary's (M only round a white dwarf), and its mass is no more than
/// the primary's, so the primary stays the heaviest star.
fn draw_companions(
    primary: &Star,
    designation: &str,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> Vec<Companion> {
    if !rng.chance(*t.multiplicity_by_class.get(primary.class)) {
        return Vec::new();
    }
    let stars = rng.weighted(&t.star_count_weights);
    let classes = companion_classes(primary.class, t);
    if classes.is_empty() {
        return Vec::new();
    }
    let mut companions: Vec<(Star, f64)> = (1..stars)
        .map(|_| {
            let class = rng.weighted(&classes);
            let range = t.star_mass_ranges.get(class);
            let mass = rng.log_uniform(range.min, range.max.min(primary.mass_msun));
            let star = draw_star(class, mass, rng, t);
            (star, draw_separation_au(primary.class, rng, t))
        })
        .collect();
    companions.sort_by(|a, b| a.1.total_cmp(&b.1));
    companions
        .into_iter()
        .zip(COMPANION_LETTERS)
        .map(|((star, separation_au), letter)| Companion {
            star,
            separation_au,
            designation: format!("{designation} {letter}"),
        })
        .collect()
}

/// The classes a companion of a `primary` star can take, with their
/// weights from the star mix: main-sequence classes no heavier than the
/// primary's; M round a white dwarf; none round a giant (SGEN-25).
fn companion_classes(primary: StarClass, t: &GeneratorTunables) -> Vec<(StarClass, f64)> {
    let heaviest = match primary {
        StarClass::WhiteDwarf => StarClass::M,
        StarClass::Giant => return Vec::new(),
        class => class,
    };
    StarClass::MAIN_SEQUENCE
        .iter()
        .filter(|&&class| class <= heaviest)
        .map(|&class| (class, *t.star_mix.get(class)))
        .collect()
}

/// A companion's separation in AU (SGEN-26): log-normal, with M dwarfs'
/// own mean and spread, kept between `companion_min_au` and
/// `wide_pair_min_au` by drawing again, and pulled inside if every draw
/// falls out.
fn draw_separation_au(primary: StarClass, rng: &mut Rng, t: &GeneratorTunables) -> f64 {
    let spread = if primary == StarClass::M {
        t.separation_log10_au.m
    } else {
        t.separation_log10_au.other
    };
    let (min, max) = (t.companion_min_au, t.wide_pair_min_au);
    let mut separation = min;
    for _ in 0..SEPARATION_TRIES {
        separation = 10f64.powf(rng.normal(spread.mean, spread.spread));
        if separation >= min && separation < max {
            return separation;
        }
    }
    separation.clamp(min, max)
}

/// A brown dwarf: catalogue-only, with a mass, an age and a metallicity
/// and nothing else (SGEN-28).
fn draw_brown_dwarf(id: usize, position_km: DVec3, rng: &mut Rng, t: &GeneratorTunables) -> Entry {
    let mass = t.brown_dwarf_mass_msun;
    let mass_msun = rng.log_uniform(mass.min, mass.max);
    Entry {
        id,
        designation: designation(id, t),
        position_km,
        age_gyr: rng.uniform(t.age_min_gyr, t.age_max_gyr),
        metallicity: draw_metallicity(rng, t),
        kind: EntryKind::BrownDwarf { mass_msun },
        eligible: false,
    }
}

/// A system can carry a gate if it is a single star of class M, K, G, F,
/// A or white dwarf with no companion of any kind (SGEN-29). The members of
/// a wide pair are separate entries, so each can be eligible.
pub fn is_eligible(primary: &Star, companions: &[Companion]) -> bool {
    primary.class != StarClass::Giant && companions.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::testing::{check_seeds, check_sizes, stars_for};

    /// SGEN-20: ids run 0 to T + B − 1 and designations are the prefix
    /// and four digits; companions add a letter.
    #[test]
    fn entries_are_numbered_and_designated() {
        let t = GeneratorTunables::default();
        let (volume, entries) = stars_for(30, 11);
        let counts = volume.slab.counts;
        assert_eq!(entries.len(), counts.systems + counts.brown_dwarfs);
        for (index, entry) in entries.iter().enumerate() {
            assert_eq!(entry.id, index);
            assert_eq!(entry.designation, format!("OW-{index:04}"));
            for (companion, letter) in entry
                .system()
                .into_iter()
                .flat_map(|s| &s.companions)
                .zip("BCD".chars())
            {
                assert_eq!(companion.designation, format!("OW-{index:04} {letter}"));
            }
        }
        assert_eq!(designation(173, &t), "OW-0173");
        let systems = entries.iter().filter(|e| e.system().is_some()).count();
        assert_eq!(systems, counts.systems);
    }

    /// SGEN-21: over many sectors the primaries' classes match the mix
    /// within 0.02.
    #[test]
    fn primaries_follow_the_star_mix() {
        let t = GeneratorTunables::default();
        let mut tally = std::collections::HashMap::new();
        let mut total = 0usize;
        for seed in check_seeds(40) {
            for entry in stars_for(200, seed).1 {
                if let Some(star) = entry.primary() {
                    *tally.entry(star.class).or_insert(0usize) += 1;
                    total += 1;
                }
            }
        }
        for class in StarClass::ALL {
            let share = *tally.get(&class).unwrap_or(&0) as f64 / total as f64;
            let target = *t.star_mix.get(class);
            assert!(
                (share - target).abs() < 0.02,
                "{class:?}: {share} vs {target}"
            );
        }
    }

    /// SGEN-22: masses lie in the class's range and luminosity, radius,
    /// temperature and spectral type follow from them.
    #[test]
    fn stars_follow_their_class() {
        let t = GeneratorTunables::default();
        for seed in check_seeds(10) {
            for entry in stars_for(80, seed).1 {
                let Some(system) = entry.system() else {
                    continue;
                };
                let stars = std::iter::once(&system.primary)
                    .chain(system.companions.iter().map(|c| &c.star));
                for star in stars {
                    let range = t.star_mass_ranges.get(star.class);
                    assert!(range.contains(star.mass_msun), "{star:?}");
                    let expected = effective_temperature_k(star.luminosity_lsun, star.radius_rsun);
                    assert_eq!(star.temperature_k, expected);
                    match star.class {
                        StarClass::WhiteDwarf => {
                            assert_eq!(star.radius_rsun, t.white_dwarf_radius_rsun);
                            assert!(t.white_dwarf_luminosity_lsun.contains(star.luminosity_lsun));
                            assert!(star.spectral_type.starts_with('D'));
                        }
                        StarClass::Giant => {
                            assert!(t.giant_properties.radius_rsun.contains(star.radius_rsun));
                            assert!(
                                t.giant_properties
                                    .luminosity_lsun
                                    .contains(star.luminosity_lsun)
                            );
                            assert!(star.spectral_type.ends_with("III"));
                        }
                        _ => {
                            let l =
                                main_sequence_luminosity(star.mass_msun, &t.star_mass_luminosity);
                            assert_eq!(star.luminosity_lsun, l);
                            assert!(star.spectral_type.ends_with('V'));
                        }
                    }
                }
            }
        }
    }

    /// SGEN-23: ages lie in range and never past a main-sequence star's
    /// lifetime; white dwarfs are at least their minimum age; a giant is
    /// just past its progenitor's lifetime.
    #[test]
    fn ages_respect_lifetimes() {
        let t = GeneratorTunables::default();
        for seed in check_seeds(10) {
            for entry in stars_for(200, seed).1 {
                let age = entry.age_gyr;
                match entry.primary() {
                    None => assert!((t.age_min_gyr..=t.age_max_gyr).contains(&age)),
                    Some(star) => match star.class {
                        StarClass::WhiteDwarf => {
                            assert!((t.white_dwarf_age_min_gyr..=t.age_max_gyr).contains(&age))
                        }
                        StarClass::Giant => {
                            let life = main_sequence_lifetime_gyr(star.mass_msun);
                            assert!(age >= life && age <= 1.1 * life, "{age} vs {life}");
                        }
                        _ => {
                            let life = main_sequence_lifetime_gyr(star.mass_msun);
                            assert!(age <= t.age_max_gyr && age <= life);
                            assert!(age >= t.age_min_gyr.min(life));
                        }
                    },
                }
            }
        }
    }

    /// SGEN-24: metallicity is spread round its mean and clamped.
    #[test]
    fn metallicity_is_clamped_round_its_mean() {
        let t = GeneratorTunables::default();
        let mut values = Vec::new();
        for seed in check_seeds(20) {
            values.extend(stars_for(200, seed).1.iter().map(|e| e.metallicity));
        }
        assert!(values.iter().all(|m| m.abs() <= t.metallicity_clamp));
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let spread =
            (values.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt();
        assert!((mean - t.metallicity_mean).abs() < 0.01, "{mean}");
        assert!((spread - t.metallicity_sigma).abs() < 0.01, "{spread}");
    }

    /// SGEN-25: multiples follow their class's chance and the star-count
    /// weights; companions are main-sequence stars no heavier than the
    /// primary, M round white dwarfs, and share its age and metallicity.
    #[test]
    fn companions_follow_multiplicity_and_stay_lighter() {
        let t = GeneratorTunables::default();
        let (mut m_systems, mut m_multiples) = (0usize, 0usize);
        let mut star_counts = [0usize; 5];
        for seed in check_seeds(30) {
            for entry in stars_for(200, seed).1 {
                let Some(system) = entry.system() else {
                    continue;
                };
                let companions = &system.companions;
                if system.primary.class == StarClass::M {
                    m_systems += 1;
                    m_multiples += usize::from(!companions.is_empty());
                }
                if !companions.is_empty() {
                    star_counts[companions.len() + 1] += 1;
                }
                for companion in companions {
                    let class = companion.star.class;
                    assert!(StarClass::MAIN_SEQUENCE.contains(&class));
                    assert!(companion.star.mass_msun <= system.primary.mass_msun);
                    match system.primary.class {
                        StarClass::WhiteDwarf => assert_eq!(class, StarClass::M),
                        StarClass::Giant => panic!("a giant has no companions"),
                        primary => assert!(class <= primary),
                    }
                }
            }
        }
        let m_share = m_multiples as f64 / m_systems as f64;
        assert!(
            (m_share - t.multiplicity_by_class.m).abs() < 0.02,
            "{m_share}"
        );
        let multiples: usize = star_counts.iter().sum();
        let doubles = star_counts[2] as f64 / multiples as f64;
        assert!((doubles - 0.75).abs() < 0.03, "{doubles}");
    }

    /// SGEN-26: companions lie between their limits, nearest first.
    #[test]
    fn companion_separations_lie_within_their_limits() {
        let t = GeneratorTunables::default();
        for seed in check_seeds(20) {
            for entry in stars_for(200, seed).1 {
                let Some(system) = entry.system() else {
                    continue;
                };
                let separations: Vec<f64> =
                    system.companions.iter().map(|c| c.separation_au).collect();
                assert!(separations.windows(2).all(|w| w[0] <= w[1]));
                for separation in separations {
                    assert!(separation >= t.companion_min_au);
                    assert!(separation <= t.wide_pair_min_au);
                }
            }
        }
    }

    /// SGEN-27: P sites hold a wide pair, two entries that record each
    /// other, between the wide-pair limits apart.
    #[test]
    fn wide_pairs_are_two_entries_that_know_each_other() {
        let t = GeneratorTunables::default();
        for n in check_sizes(&t) {
            for seed in check_seeds(5) {
                let (volume, entries) = stars_for(n, seed);
                let mut paired = 0;
                for entry in &entries {
                    let Some(partner) = entry.system().and_then(|s| s.wide_partner) else {
                        continue;
                    };
                    paired += 1;
                    let other = &entries[partner];
                    assert_eq!(other.system().unwrap().wide_partner, Some(entry.id));
                    let au = entry.position_km.distance(other.position_km) / KM_PER_AU;
                    assert!(
                        au >= t.wide_pair_min_au * 0.999999 && au <= t.wide_pair_max_au * 1.000001
                    );
                }
                assert_eq!(paired, 2 * volume.slab.counts.wide_pairs);
            }
        }
    }

    /// SGEN-28: brown dwarfs and giants are catalogue-only: no companions,
    /// no gate, and every entry has a star or is a brown dwarf.
    #[test]
    fn brown_dwarfs_and_giants_are_catalogue_only() {
        let t = GeneratorTunables::default();
        let mut giants = 0;
        for seed in check_seeds(40) {
            let (volume, entries) = stars_for(200, seed);
            let dwarfs: Vec<&Entry> = entries
                .iter()
                .filter(|e| matches!(e.kind, EntryKind::BrownDwarf { .. }))
                .collect();
            assert_eq!(dwarfs.len(), volume.slab.counts.brown_dwarfs);
            for dwarf in dwarfs {
                let EntryKind::BrownDwarf { mass_msun } = dwarf.kind else {
                    unreachable!()
                };
                assert!(t.brown_dwarf_mass_msun.contains(mass_msun));
                assert!(!dwarf.eligible && !dwarf.has_summary());
            }
            for entry in entries.iter().filter(|e| e.is_giant()) {
                giants += 1;
                assert!(entry.system().unwrap().companions.is_empty());
                assert!(!entry.eligible && !entry.has_summary());
            }
        }
        assert!(giants > 0, "no giant in 40 large sectors");
    }

    /// SGEN-29: exactly the single stars of class M to A or white dwarf
    /// are eligible, and wide-pair members count separately.
    #[test]
    fn only_single_non_giant_stars_are_eligible() {
        let mut eligible_partners = 0;
        for seed in check_seeds(20) {
            for entry in stars_for(200, seed).1 {
                let expected = entry.system().is_some_and(|s| {
                    s.companions.is_empty() && s.primary.class != StarClass::Giant
                });
                assert_eq!(entry.eligible, expected, "{entry:?}");
                if entry.eligible && entry.system().unwrap().wide_partner.is_some() {
                    eligible_partners += 1;
                }
            }
        }
        assert!(eligible_partners > 0);
    }
}

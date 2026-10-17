//! The invariants of the sector generator, checked over every sector of
//! the sweep: `invariant_seeds` seeds for each preset and for N =
//! `min_gated_count` (SGEN-91). Each test checks one requirement's rule on
//! every sector; the per-stage tests next to each stage check the rules
//! on hand-made cases.

use std::collections::HashSet;

use bevy::math::DVec3;

use super::Sector;
use super::catalogue::StarClass;
use super::failure::rules::Failed;
use super::gates::ring::ring_is_clear;
use super::graph::Groups;
use super::links::crossing::{segments_cross, top_down};
use super::links::strategic;
use super::names::list::NameList;
use super::network::{LinkOrigin, LinkState, Pick, Reason};
use super::summaries::planets::{Placed, mutual_hill_radius_au, star_mass_earth};
use super::testing::{check_sizes, sweep};
use super::tunables::GeneratorTunables;
use super::volume::{counts, slab_size};
use crate::sim::units::KM_PER_PARSEC;

fn tunables() -> GeneratorTunables {
    GeneratorTunables::default()
}

/// A label for failure messages.
fn which(sector: &Sector) -> String {
    format!(
        "N {} seed {}",
        sector.settings.gated_count, sector.settings.seed
    )
}

/// The gated ids of a sector, ascending.
fn gated_ids(sector: &Sector) -> Vec<usize> {
    sector.gated.iter().map(|g| g.id).collect()
}

/// Each gated system's index in `sector.gated`, from its entry id.
fn index_of(sector: &Sector, id: usize) -> usize {
    sector
        .gated
        .binary_search_by_key(&id, |g| g.id)
        .expect("a gated id")
}

/// SGEN-10, SGEN-11: the counts and the slab's size follow from N.
#[test]
fn every_sector_has_its_counts_and_size() {
    let t = tunables();
    for sector in sweep() {
        let n = sector.settings.gated_count;
        let expected = counts(n, &t);
        assert_eq!(sector.slab.counts, expected, "{}", which(sector));
        let systems = sector
            .entries
            .iter()
            .filter(|e| e.system().is_some())
            .count();
        assert_eq!(systems, expected.systems);
        assert_eq!(
            sector.entries.len(),
            expected.systems + expected.brown_dwarfs
        );
        let partnered = sector
            .entries
            .iter()
            .filter(|e| e.system().is_some_and(|s| s.wide_partner.is_some()))
            .count();
        assert_eq!(partnered, 2 * expected.wide_pairs);
        let (volume, width, thickness) = slab_size(expected.systems, &t);
        let slab = &sector.slab;
        assert_eq!(
            (slab.volume_pc3, slab.width_pc, slab.thickness_pc),
            (volume, width, thickness)
        );
    }
}

/// SGEN-12: every site lies in the slab and keeps the minimum separation
/// from every other.
#[test]
fn every_site_lies_in_the_slab_apart_from_the_rest() {
    let t = tunables();
    for sector in sweep() {
        let sites: Vec<DVec3> = sector
            .entries
            .iter()
            .filter(|e| {
                e.system()
                    .and_then(|s| s.wide_partner)
                    .is_none_or(|partner| partner > e.id)
            })
            .map(|e| e.position_km / KM_PER_PARSEC)
            .collect();
        for (i, site) in sites.iter().enumerate() {
            assert!(sector.slab.contains_pc(*site), "{}", which(sector));
            for other in &sites[..i] {
                assert!(site.distance(*other) >= t.min_system_separation_pc);
            }
        }
    }
}

/// SGEN-29: exactly the single stars of class M to A or white dwarf are
/// eligible.
#[test]
fn every_eligible_entry_is_a_single_star() {
    for sector in sweep() {
        for entry in &sector.entries {
            let single = entry
                .system()
                .is_some_and(|s| s.companions.is_empty() && s.primary.class != StarClass::Giant);
            assert_eq!(entry.eligible, single, "{}", which(sector));
        }
    }
}

/// SGEN-34: in every summary, moved planets included, neighbours keep
/// `min_mutual_hill` mutual Hill radii apart.
#[test]
fn every_planet_keeps_its_hill_spacing() {
    let t = tunables();
    for sector in sweep() {
        for (entry, summary) in sector.entries.iter().zip(&sector.summaries) {
            let Some(summary) = summary else { continue };
            let star = star_mass_earth(entry.primary().unwrap().mass_msun);
            for pair in summary.planets.windows(2) {
                let [a, b] = [&pair[0], &pair[1]].map(|p| Placed {
                    kind: p.kind,
                    mass_earth: p.mass_earth,
                    orbit_au: p.orbit_au,
                });
                let least = t.min_mutual_hill * mutual_hill_radius_au(&a, &b, star);
                assert!(
                    b.orbit_au - a.orbit_au >= least * (1.0 - 1e-9),
                    "{} entry {}",
                    which(sector),
                    entry.id
                );
            }
        }
    }
}

/// SGEN-37: no planet orbits inside a belt, no two belts overlap, and no
/// system has more belts than its template allows.
#[test]
fn every_belt_is_clear_of_planets_and_belts() {
    let t = tunables();
    for sector in sweep() {
        for summary in sector.summaries.iter().flatten() {
            let rule = t.template_rules.get(summary.template);
            assert!(summary.belts.len() <= rule.belt_count.max as usize);
            for (i, belt) in summary.belts.iter().enumerate() {
                assert!(
                    summary.planets.iter().all(|p| !belt.contains(p.orbit_au)),
                    "{}",
                    which(sector)
                );
                assert!(summary.belts[..i].iter().all(|b| !belt.overlaps(b)));
            }
        }
    }
}

/// SGEN-55: each reason has the same number of quota picks, no more than
/// its quota, and a quota pick is never beaten for its reason by a
/// system that was not itself a quota pick.
#[test]
fn every_quota_takes_the_best_systems_for_its_reason() {
    let t = tunables();
    for sector in sweep() {
        let n = sector.settings.gated_count as usize;
        let quota = (t.reason_quota * n as f64).ceil() as usize;
        let quota_ids: HashSet<usize> = sector
            .gated
            .iter()
            .filter(|g| matches!(g.pick, Pick::Quota(_)))
            .map(|g| g.id)
            .collect();
        let field = super::gates::scores::scores;
        let counts = Reason::ALL.map(|reason| {
            sector
                .gated
                .iter()
                .filter(|g| g.pick == Pick::Quota(reason))
                .count()
        });
        assert!(counts.iter().all(|&c| c == counts[0]), "{}", which(sector));
        assert!(counts[0] > 0, "{}", which(sector));
        for reason in Reason::ALL {
            let picks: Vec<_> = sector
                .gated
                .iter()
                .filter(|g| g.pick == Pick::Quota(reason))
                .collect();
            assert!(picks.len() <= quota, "{}", which(sector));
            let Some(weakest) = picks
                .iter()
                .map(|g| g.scores.get(reason))
                .min_by(f64::total_cmp)
            else {
                continue;
            };
            for entry in sector.entries.iter().filter(|e| e.eligible) {
                // Gated systems keep the scores they were chosen by, from
                // before any planet moved for the ring.
                let summary = sector.summary(entry.id).unwrap();
                let scores = sector
                    .gated_system(entry.id)
                    .map_or_else(|| field(entry, summary, &t), |g| g.scores);
                if scores.get(reason) > weakest {
                    assert!(quota_ids.contains(&entry.id), "{}", which(sector));
                }
            }
        }
    }
}

/// SGEN-57: exactly N systems are gated, all eligible, joined into one
/// network within the link range.
#[test]
fn every_sector_gates_exactly_n_joined_systems() {
    let (mut junctions, mut relays) = (0, 0);
    for sector in sweep() {
        junctions += sector
            .gated
            .iter()
            .filter(|g| g.pick == Pick::Junction)
            .count();
        relays += sector
            .gated
            .iter()
            .filter(|g| g.pick == Pick::Relay)
            .count();
        let ids = gated_ids(sector);
        assert_eq!(
            ids.len(),
            sector.settings.gated_count as usize,
            "{}",
            which(sector)
        );
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
        assert!(ids.iter().all(|&id| sector.entries[id].eligible));
        let positions: Vec<DVec3> = ids
            .iter()
            .map(|&id| sector.entries[id].position_km)
            .collect();
        let mut groups = super::graph::groups_within(&positions, sector.link_range_km);
        assert_eq!(groups.count(), 1, "{}", which(sector));
    }
    eprintln!("junctions {junctions}, relays {relays}");
}

/// SGEN-58, SGEN-59: every ring clears every planet's orbit; it slides at
/// most `ring_slide_limit`, and stays at its start when planets moved.
#[test]
fn every_ring_is_clear_and_near_its_start() {
    let t = tunables();
    let mut moved = 0;
    for sector in sweep() {
        for gated in &sector.gated {
            let summary = sector.summary(gated.id).unwrap();
            let star = sector.entries[gated.id].primary().unwrap();
            let start = t.ring_snow_line_factor * summary.snow_line_au;
            let ring = gated.ring_radius_au;
            // Within a hair, for the rounding at a range's edge.
            let nudged = [ring, ring * (1.0 + 1e-12), ring * (1.0 - 1e-12)];
            assert!(
                nudged
                    .iter()
                    .any(|&r| ring_is_clear(r, &summary.planets, star, &t)),
                "{} entry {}",
                which(sector),
                gated.id
            );
            assert!((ring - start).abs() <= t.ring_slide_limit * start * (1.0 + 1e-12));
            if !gated.moved_planets.is_empty() {
                moved += 1;
                assert_eq!(ring, start);
            }
        }
    }
    eprintln!("systems with moved planets: {moved}");
}

/// SGEN-62, SGEN-65: every gated system has `links_min` to `links_max`
/// links, the old network is joined, no link is longer than the final
/// range, and for the presets the mean is at least `mean_links_min`.
/// Each link joins two gated systems at their distance.
#[test]
fn every_network_is_a_joined_dense_mesh() {
    let t = tunables();
    let presets = [t.preset_small, t.preset_medium, t.preset_large];
    let mut lowest_mean = f64::INFINITY;
    for sector in sweep() {
        let ids = gated_ids(sector);
        let mut groups = Groups::new(ids.len());
        for link in &sector.links {
            let [a, b] = link.ends;
            assert!(a < b);
            let distance = sector.entries[a]
                .position_km
                .distance(sector.entries[b].position_km);
            assert_eq!(link.length_km, distance);
            assert!(link.length_km <= sector.link_range_km, "{}", which(sector));
            groups.join(index_of(sector, a), index_of(sector, b));
        }
        assert_eq!(groups.count(), 1, "{}", which(sector));
        for &id in &ids {
            let degree = sector.links_of(id).count() as u32;
            assert!(
                (t.links_min..=t.links_max).contains(&degree),
                "{} entry {id}: {degree} links",
                which(sector)
            );
        }
        let pairs: HashSet<[usize; 2]> = sector.links.iter().map(|l| l.ends).collect();
        assert_eq!(pairs.len(), sector.links.len(), "no pair is linked twice");
        if presets.contains(&sector.settings.gated_count) {
            let mean = 2.0 * sector.links.len() as f64 / ids.len() as f64;
            lowest_mean = lowest_mean.min(mean);
            assert!(mean >= t.mean_links_min, "{}: mean {mean}", which(sector));
        }
    }
    eprintln!("lowest mean links per system: {lowest_mean}");
}

/// SGEN-63: no step-2 link crosses a link of steps 1 or 2 seen from
/// above. A step-3 link may cross one when nothing else is in reach
/// (SGEN-61); that is rare and counted.
#[test]
fn no_extra_link_crosses_another() {
    let (mut unavoidable, mut links) = (0, 0);
    for sector in sweep() {
        let flat = |id: usize| top_down(sector.entries[id].position_km);
        links += sector.links.len();
        for (i, link) in sector.links.iter().enumerate() {
            if link.origin != LinkOrigin::Extra {
                continue;
            }
            let (a, b) = (flat(link.ends[0]), flat(link.ends[1]));
            for (j, other) in sector.links.iter().enumerate() {
                if i == j || !segments_cross(a, b, flat(other.ends[0]), flat(other.ends[1])) {
                    continue;
                }
                assert_eq!(
                    other.origin,
                    LinkOrigin::Second,
                    "{}: links {i} and {j}",
                    which(sector)
                );
                unavoidable += 1;
            }
        }
    }
    eprintln!("step-3 links crossing a step-2 link: {unavoidable} of {links}");
    assert!(unavoidable * 100 < links, "crossings are rare");
}

/// SGEN-64: a system is strategic exactly when it is not valuable and
/// links to at least `strategic_min_valuable` valuable systems.
#[test]
fn strategic_systems_lead_to_valuable_ones() {
    let t = tunables();
    let mut seen = 0;
    for sector in sweep() {
        let ids = gated_ids(sector);
        let overall: Vec<f64> = sector.gated.iter().map(|g| g.scores.overall()).collect();
        let valuable = strategic::valuable(&ids, &overall, &t);
        for (index, gated) in sector.gated.iter().enumerate() {
            let valuable_links = sector
                .links_of(gated.id)
                .filter(|l| valuable[index_of(sector, l.other_end(gated.id))])
                .count() as u32;
            let expected = !valuable[index] && valuable_links >= t.strategic_min_valuable;
            assert_eq!(gated.strategic, expected, "{}", which(sector));
            seen += usize::from(gated.strategic);
        }
    }
    assert!(seen > 0);
}

/// Each sector's shares of working, dormant and collapsed links.
fn state_shares(sector: &Sector) -> [f64; 3] {
    let total = sector.links.len() as f64;
    let share = |wanted: fn(&LinkState) -> bool| {
        sector.links.iter().filter(|l| wanted(&l.state)).count() as f64 / total
    };
    [
        share(|s| *s == LinkState::Working),
        share(|s| matches!(s, LinkState::Dormant { .. })),
        share(|s| *s == LinkState::Collapsed),
    ]
}

/// SGEN-70: over the sectors of each preset the mean share of each state
/// is within 0.06 of its target, and no sector is more than 0.15 away.
#[test]
fn link_states_land_near_their_targets() {
    let t = tunables();
    let targets = [t.working_share, t.dormant_share, t.collapsed_share];
    for preset in [t.preset_small, t.preset_medium, t.preset_large] {
        let sectors: Vec<&Sector> = sweep()
            .iter()
            .filter(|s| s.settings.gated_count == preset)
            .collect();
        let mut sums = [0.0; 3];
        for sector in &sectors {
            let shares = state_shares(sector);
            for state in 0..3 {
                sums[state] += shares[state];
                let off = (shares[state] - targets[state]).abs();
                assert!(off <= 0.15, "{} state {state}: {shares:?}", which(sector));
            }
        }
        let means = sums.map(|sum| sum / sectors.len() as f64);
        eprintln!("N {preset}: mean shares {means:?}");
        for state in 0..3 {
            assert!(
                (means[state] - targets[state]).abs() <= 0.06,
                "N {preset}: {means:?}"
            );
        }
    }
}

/// The failure rules' view of a sector's links, for the region checks.
fn failed_view(sector: &Sector) -> (Vec<[usize; 2]>, Vec<LinkState>) {
    let ends = sector
        .links
        .iter()
        .map(|l| l.ends.map(|id| index_of(sector, id)))
        .collect();
    (ends, sector.links.iter().map(|l| l.state).collect())
}

/// SGEN-72: the main region holds at least `main_region_min_share` × N
/// systems.
#[test]
fn every_main_region_is_large_enough() {
    let t = tunables();
    for sector in sweep() {
        let (ends, states) = failed_view(sector);
        let failed = Failed {
            systems: sector.gated.len(),
            ends: &ends,
            weights: &[],
            lengths_km: &[],
            states,
        };
        let labels = failed.working_groups();
        let main = Failed::main_region(&labels);
        let held = labels.iter().filter(|&&l| l == main).count();
        let least = t.main_region_min_share * sector.gated.len() as f64;
        assert!(held as f64 >= least, "{}: {held}", which(sector));
    }
}

/// SGEN-73: every gated system keeps a link that has not collapsed.
#[test]
fn every_system_keeps_a_link_open() {
    for sector in sweep() {
        for gated in &sector.gated {
            let open = sector
                .links_of(gated.id)
                .any(|l| l.state != LinkState::Collapsed);
            assert!(open, "{} entry {}", which(sector), gated.id);
        }
    }
}

/// SGEN-74, SGEN-76: starting from the main region, every group joined
/// by working links can be brought in through a dormant link that is
/// faulted only at the end already held; every dormant link has a fault.
#[test]
fn every_group_can_be_repaired_into_the_main_region() {
    for sector in sweep() {
        let (ends, states) = failed_view(sector);
        let failed = Failed {
            systems: sector.gated.len(),
            ends: &ends,
            weights: &[],
            lengths_km: &[],
            states: states.clone(),
        };
        let labels = failed.working_groups();
        let main = Failed::main_region(&labels);
        let mut held: Vec<bool> = labels.iter().map(|&l| l == main).collect();
        loop {
            let reach = ends.iter().zip(&states).find_map(|(&[a, b], state)| {
                let LinkState::Dormant { faulted } = state else {
                    return None;
                };
                let restartable =
                    (held[a] && !held[b] && !faulted[1]) || (held[b] && !held[a] && !faulted[0]);
                restartable.then(|| if held[a] { labels[b] } else { labels[a] })
            });
            let Some(group) = reach else { break };
            for (system, label) in labels.iter().enumerate() {
                held[system] |= *label == group;
            }
        }
        assert!(held.iter().all(|&h| h), "{}", which(sector));
        for state in &states {
            if let LinkState::Dormant { faulted } = state {
                assert!(faulted[0] || faulted[1], "{}", which(sector));
            }
        }
    }
}

/// SGEN-81, SGEN-82: every name in a sector is unique ignoring case and
/// not reserved; each gated system has a name, one per planet and
/// `belt_body_max` per belt; ungated entries have none.
#[test]
fn every_name_is_unique_and_complete() {
    let t = tunables();
    let reserved: HashSet<String> = NameList::bundled().reserved.iter().cloned().collect();
    for sector in sweep() {
        let mut seen = HashSet::new();
        for gated in &sector.gated {
            let summary = sector.summary(gated.id).unwrap();
            let names = &gated.names;
            assert_eq!(names.planets.len(), summary.planets.len());
            assert_eq!(names.belt_bodies.len(), summary.belts.len());
            assert!(
                names
                    .belt_bodies
                    .iter()
                    .all(|b| b.len() == t.belt_body_max as usize)
            );
            let all = std::iter::once(&names.system)
                .chain(&names.planets)
                .chain(names.belt_bodies.iter().flatten());
            for name in all {
                let lower = name.to_lowercase();
                assert!(!reserved.contains(&lower), "{name}");
                assert!(seen.insert(lower), "{}: {name} twice", which(sector));
            }
        }
        for entry in &sector.entries {
            assert_eq!(
                sector.gated_system(entry.id).is_some(),
                gated_ids(sector).contains(&entry.id)
            );
        }
    }
}

/// SGEN-91: the sweep covers `invariant_seeds` seeds at each size.
#[test]
fn the_sweep_covers_every_size_and_seed() {
    let t = tunables();
    for n in check_sizes(&t) {
        let count = sweep()
            .iter()
            .filter(|s| s.settings.gated_count == n)
            .count();
        assert_eq!(count, t.invariant_seeds as usize);
    }
}

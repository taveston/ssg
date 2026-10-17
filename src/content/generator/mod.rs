//! The sector generator: builds the star map of one game from N, the
//! number of gated systems, and a seed (`docs/spec/sector-generator.md`).
//! It is plain Rust that returns plain data, with no Bevy rendering, UI or
//! ECS types and its own seeded random numbers (SGEN-05, SGEN-06).
//!
//! The work is a pipeline of stages, one module each, run in order by
//! `generate_sector`. Each stage draws from its own seed, derived from the
//! sector seed and the stage's name (`stages.rs`), and reads only the
//! tunables and the outputs of earlier stages, so tuning one stage leaves
//! every earlier one as it was (SGEN-02, SGEN-03):
//!
//! 1. `volume`: the slab and its sites (SGEN-10 to SGEN-12).
//! 2. `stars`: the catalogue's entries and stars (SGEN-20 to SGEN-29).
//! 3. `summaries`: each system's planets and belts (SGEN-30 to SGEN-43).
//! 4. `gates`: which systems carry gates, and their rings (SGEN-50 to
//!    SGEN-59).
//! 5. `links`: the old network between them (SGEN-60 to SGEN-65).
//! 6. `failure`: which links failed, and how (SGEN-70 to SGEN-76).
//! 7. `names`: names for gated systems and their planets (SGEN-80 to
//!    SGEN-82).
//!
//! Gate choice and links follow from earlier outputs alone and draw no
//! random numbers; their seeds are kept for any later draw.

pub mod bodies;
pub mod catalogue;
pub mod failure;
pub mod gates;
pub mod graph;
pub mod links;
pub mod names;
pub mod network;
pub mod rng;
pub mod settings;
pub mod stages;
pub mod stars;
pub mod stellar;
pub mod summaries;
pub mod tunables;
pub mod volume;

#[cfg(test)]
pub mod testing;
#[cfg(test)]
mod tests_invariants;

use bodies::Summary;
use catalogue::Entry;
use gates::GateRecord;
use links::Network;
use names::list::NameList;
use network::{GatedSystem, Link, LinkState, SystemNames};
use rng::Rng;
use settings::{Settings, SettingsError};
use stages::{Stage, StageSeeds};
use tunables::GeneratorTunables;
use volume::Slab;

/// A generated sector (SGEN-90): its settings and stage seeds, the slab,
/// every catalogue entry and every system's summary, the gated systems
/// with their names, rings and labels, the final link range and every
/// link with its state.
#[derive(Debug, Clone, PartialEq)]
pub struct Sector {
    pub settings: Settings,
    /// The seed each stage drew from (SGEN-03).
    pub seeds: StageSeeds,
    /// The slab's counts and dimensions (stage 1).
    pub slab: Slab,
    /// Every entry, by id (stage 2).
    pub entries: Vec<Entry>,
    /// Every entry's summary, by id: `None` for brown dwarfs and giants
    /// (stage 3). A gated system's planets are where its ring left them
    /// (SGEN-59).
    pub summaries: Vec<Option<Summary>>,
    /// The gated systems, by id (stages 4 to 7).
    pub gated: Vec<GatedSystem>,
    /// The final link range, in km (SGEN-60).
    pub link_range_km: f64,
    /// Every link of the old network, with its state after the failure
    /// (stages 5 and 6).
    pub links: Vec<Link>,
}

impl Sector {
    /// The summary of the entry with this id, if it has one.
    pub fn summary(&self, id: usize) -> Option<&Summary> {
        self.summaries.get(id).and_then(Option::as_ref)
    }

    /// The gated system with this entry id, if it is gated.
    pub fn gated_system(&self, id: usize) -> Option<&GatedSystem> {
        self.gated
            .binary_search_by_key(&id, |g| g.id)
            .ok()
            .map(|index| &self.gated[index])
    }

    /// The links with an end at the entry with this id.
    pub fn links_of(&self, id: usize) -> impl Iterator<Item = &Link> {
        self.links.iter().filter(move |link| link.touches(id))
    }
}

/// A fresh random stream for `stage`, from its own seed (SGEN-03).
pub fn stage_rng(seeds: &StageSeeds, stage: Stage) -> Rng {
    Rng::new(seeds.get(stage))
}

/// Generates the sector for these settings (SGEN-01 to SGEN-03). Settings
/// out of range are refused with an error before any stage runs. The
/// result depends on the settings and tunables only.
pub fn generate_sector(
    settings: Settings,
    tunables: &GeneratorTunables,
) -> Result<Sector, SettingsError> {
    settings.check(tunables)?;
    let seeds = StageSeeds::derive(settings.seed);
    let volume = volume::run(
        settings.gated_count,
        &mut stage_rng(&seeds, Stage::Volume),
        tunables,
    );
    let entries = stars::run(&volume, &mut stage_rng(&seeds, Stage::Stars), tunables);
    let summaries = summaries::run(&entries, &mut stage_rng(&seeds, Stage::Summaries), tunables);
    let gates = gates::run(
        settings.gated_count,
        &volume.slab,
        &entries,
        &summaries,
        tunables,
    );
    let network = links::run(&gates.systems, &entries, gates.link_range_km, tunables);
    let states = failure::run(
        &network.links,
        &gates.systems,
        &mut stage_rng(&seeds, Stage::Failure),
        tunables,
    );
    let names = names::run(
        &gates.systems,
        &gates.summaries,
        NameList::bundled(),
        &mut stage_rng(&seeds, Stage::Names),
        tunables,
    );
    let link_range_km = network.link_range_km;
    let (gated, links) = gather_network(gates.systems, network, states, names);
    Ok(Sector {
        settings,
        seeds,
        slab: volume.slab,
        entries,
        summaries: gates.summaries,
        gated,
        link_range_km,
        links,
    })
}

/// The gated systems and links from the outputs of stages 4 to 7, which
/// all run in the gate choice's order (SGEN-90).
fn gather_network(
    records: Vec<GateRecord>,
    network: Network,
    states: Vec<LinkState>,
    names: Vec<SystemNames>,
) -> (Vec<GatedSystem>, Vec<Link>) {
    let gated = records
        .into_iter()
        .zip(network.strategic)
        .zip(names)
        .map(|((record, strategic), names)| GatedSystem {
            id: record.id,
            scores: record.scores,
            pick: record.pick,
            main_reason: record.main_reason,
            ring_radius_au: record.ring_radius_au,
            moved_planets: record.moved_planets,
            strategic,
            names,
        })
        .collect();
    let links = network
        .links
        .into_iter()
        .zip(states)
        .map(|(link, state)| Link { state, ..link })
        .collect();
    (gated, links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use network::{LinkOrigin, Pick};
    use testing::{check_seeds, check_sizes};

    /// SGEN-01: a count out of range is an error from the generator, not a
    /// panic.
    #[test]
    fn the_generator_refuses_a_count_out_of_range() {
        let t = GeneratorTunables::default();
        let settings = Settings {
            gated_count: 7,
            seed: 1,
        };
        let error = generate_sector(settings, &t).unwrap_err();
        assert!(error.to_string().contains("8 to 300"));
    }

    /// SGEN-02: the same settings and tunables give the same sector, field
    /// for field.
    #[test]
    fn the_same_settings_give_the_same_sector() {
        let t = GeneratorTunables::default();
        for n in check_sizes(&t) {
            for seed in check_seeds(3) {
                let settings = Settings {
                    gated_count: n,
                    seed,
                };
                let first = generate_sector(settings, &t).unwrap();
                let second = generate_sector(settings, &t).unwrap();
                assert_eq!(first, second);
                assert_eq!(first.settings, settings);
                assert_eq!(first.seeds, StageSeeds::derive(seed));
            }
        }
        let other = Settings {
            gated_count: 30,
            seed: 2,
        };
        let one = Settings {
            gated_count: 30,
            seed: 1,
        };
        assert_ne!(
            generate_sector(one, &t).unwrap().entries,
            generate_sector(other, &t).unwrap().entries
        );
    }

    /// The positions the volume stage placed: every entry's but the second
    /// member of a wide pair, which the stars stage places beside its
    /// partner.
    fn site_positions(sector: &Sector) -> Vec<bevy::math::DVec3> {
        sector
            .entries
            .iter()
            .filter(|e| {
                e.system()
                    .and_then(|s| s.wide_partner)
                    .is_none_or(|partner| partner > e.id)
            })
            .map(|e| e.position_km)
            .collect()
    }

    /// SGEN-03: changing a tunable of one stage leaves every earlier
    /// stage's output equal.
    #[test]
    fn tuning_a_stage_leaves_earlier_stages_alone() {
        let t = GeneratorTunables::default();
        let settings = Settings {
            gated_count: 80,
            seed: 99,
        };
        let base = generate_sector(settings, &t).unwrap();

        // A summaries tunable: volume and stars hold still.
        let summaries_tuned = GeneratorTunables {
            min_mutual_hill: 12.0,
            belt_width_ratio: tunables::Span::new(1.1, 1.5),
            ..t.clone()
        };
        let tuned = generate_sector(settings, &summaries_tuned).unwrap();
        assert_eq!(tuned.slab, base.slab);
        assert_eq!(tuned.entries, base.entries);
        assert_ne!(tuned.summaries, base.summaries);

        // A stars tunable: the volume holds still.
        let stars_tuned = GeneratorTunables {
            metallicity_sigma: 0.3,
            ..t.clone()
        };
        let tuned = generate_sector(settings, &stars_tuned).unwrap();
        assert_eq!(tuned.slab, base.slab);
        assert_eq!(site_positions(&tuned), site_positions(&base));
        assert_ne!(tuned.entries, base.entries);
    }

    /// What stage 4 decided for each gated system: everything but the
    /// later stages' labels and names.
    fn gate_records(sector: &Sector) -> Vec<(usize, Pick, f64, Vec<usize>)> {
        sector
            .gated
            .iter()
            .map(|g| (g.id, g.pick, g.ring_radius_au, g.moved_planets.clone()))
            .collect()
    }

    /// Each link's ends and origin, the final range and the strategic
    /// labels.
    type NetworkShape = (Vec<([usize; 2], LinkOrigin)>, f64, Vec<bool>);

    /// What stage 5 decided: each link without its state, the range and
    /// the strategic labels.
    fn network_shape(sector: &Sector) -> NetworkShape {
        (
            sector.links.iter().map(|l| (l.ends, l.origin)).collect(),
            sector.link_range_km,
            sector.gated.iter().map(|g| g.strategic).collect(),
        )
    }

    /// Stage 3's summaries, as far as stage 4's ring moves leave them to
    /// compare: whole where no planet moved in either sector, else all
    /// but the planets.
    fn assert_same_summaries(a: &Sector, b: &Sector) {
        let moved = |s: &Sector, id| {
            s.gated_system(id)
                .is_some_and(|g| !g.moved_planets.is_empty())
        };
        for (id, (x, y)) in a.summaries.iter().zip(&b.summaries).enumerate() {
            if moved(a, id) || moved(b, id) {
                let (x, y) = (x.as_ref().unwrap(), y.as_ref().unwrap());
                assert_eq!((x.template, &x.belts), (y.template, &y.belts));
            } else {
                assert_eq!(x, y, "entry {id}");
            }
        }
    }

    /// SGEN-03: tuning gate choice, links, failure or names leaves every
    /// earlier stage's output equal.
    #[test]
    fn tuning_a_later_stage_leaves_earlier_stages_alone() {
        let t = GeneratorTunables::default();
        let settings = Settings {
            gated_count: 80,
            seed: 99,
        };
        let base = generate_sector(settings, &t).unwrap();
        let tuned = |tunables: GeneratorTunables| generate_sector(settings, &tunables).unwrap();

        let gates = tuned(GeneratorTunables {
            reason_quota: 0.2,
            ..t.clone()
        });
        assert_eq!((&gates.slab, &gates.entries), (&base.slab, &base.entries));
        assert_same_summaries(&gates, &base);
        assert_ne!(gate_records(&gates), gate_records(&base));

        let links = tuned(GeneratorTunables {
            links_max: 4,
            ..t.clone()
        });
        assert_eq!(links.summaries, base.summaries);
        assert_eq!(gate_records(&links), gate_records(&base));
        assert_ne!(network_shape(&links), network_shape(&base));

        let failure = tuned(GeneratorTunables {
            failure_traffic_weight: 0.2,
            ..t.clone()
        });
        assert_eq!(gate_records(&failure), gate_records(&base));
        assert_eq!(network_shape(&failure), network_shape(&base));
        let states = |s: &Sector| s.links.iter().map(|l| l.state).collect::<Vec<_>>();
        assert_ne!(states(&failure), states(&base));

        let names = tuned(GeneratorTunables {
            belt_body_max: 2,
            ..t.clone()
        });
        assert_eq!(names.links, base.links);
        assert_eq!(gate_records(&names), gate_records(&base));
        assert_ne!(names.gated, base.gated);
    }

    /// SGEN-90: the sector holds every stage's output: the gated systems
    /// with their reasons, labels, rings and names, the final range and
    /// every link with its state.
    #[test]
    fn the_sector_holds_every_stage_output() {
        let sector = testing::sector(30, 90);
        assert_eq!(sector.gated.len(), 30);
        for gated in &sector.gated {
            assert_eq!(sector.gated_system(gated.id), Some(gated));
            assert!(gated.ring_radius_au > 0.0);
            assert!(!gated.names.system.is_empty());
            assert!(sector.links_of(gated.id).count() >= 2);
        }
        let ungated = sector.entries.iter().find(|e| !e.eligible).unwrap();
        assert!(sector.gated_system(ungated.id).is_none());
        let start = gates::starting_link_range_km(&sector.slab, 30, &GeneratorTunables::default());
        assert!(sector.link_range_km >= start);
        assert!(sector.links.iter().any(|l| l.state == LinkState::Working));
        assert!(sector.links.iter().any(|l| l.state == LinkState::Collapsed));
    }

    /// SGEN-02, SGEN-91: every sector of the sweep comes out the same when
    /// made again.
    #[test]
    fn every_seed_gives_the_same_sector_twice() {
        for sector in testing::sweep() {
            assert_eq!(
                &testing::sector(sector.settings.gated_count, sector.settings.seed),
                sector
            );
        }
    }

    /// SGEN-92: the largest sector generates within the time cap in a test
    /// build.
    #[test]
    fn the_largest_sector_generates_in_time() {
        let t = GeneratorTunables::default();
        let settings = Settings {
            gated_count: t.max_gated_count,
            seed: 92,
        };
        let started = std::time::Instant::now();
        let sector = generate_sector(settings, &t).unwrap();
        let seconds = started.elapsed().as_secs_f64();
        assert_eq!(sector.gated.len(), t.max_gated_count as usize);
        assert!(seconds < t.generation_test_cap_s, "{seconds} s");
    }

    /// SGEN-04: the seed is part of the sector, so the view can show it.
    #[test]
    fn the_sector_keeps_its_seed() {
        let t = GeneratorTunables::default();
        let settings = Settings::with_random_seed(30);
        let sector = generate_sector(settings, &t).unwrap();
        assert_eq!(sector.settings.seed, settings.seed);
        assert_eq!(sector.seeds, StageSeeds::derive(settings.seed));
    }

    /// SGEN-30: the summary accessor finds an entry's summary by id.
    #[test]
    fn summaries_are_found_by_entry_id() {
        let sector = testing::sector(30, 4);
        for entry in &sector.entries {
            assert_eq!(sector.summary(entry.id).is_some(), entry.has_summary());
        }
        assert!(sector.summary(sector.entries.len()).is_none());
    }
}

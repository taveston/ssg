//! Stage 4, gate choice: scores every eligible system for the four
//! reasons the old network's builders had (`scores`), picks exactly N of
//! them to carry gates, a core by score with a quota per reason joined by
//! junctions and relays (`choice`, `connectors`), tags each with its main
//! reason, and sizes its gate ring, moving planets out of the ring's way
//! if it must (`ring`) (SGEN-50 to SGEN-59).
//!
//! Nothing here is random: the stage's output follows from the earlier
//! stages' and the tunables.

mod choice;
mod connectors;
pub mod ring;
pub mod scores;

use bevy::math::DVec3;

use super::bodies::Summary;
use super::catalogue::Entry;
use super::network::{Pick, Reason, Scores};
use super::tunables::GeneratorTunables;
use super::volume::Slab;
use crate::sim::units::KM_PER_PARSEC;

/// The eligible systems the choice works over: every entry's position and,
/// for the eligible ones, its scores.
pub struct Field<'a> {
    /// Every entry's position, by id, in km.
    pub positions: &'a [DVec3],
    /// Every entry's scores, by id: `Some` for eligible systems only.
    pub scores: &'a [Option<Scores>],
    /// The eligible ids, ascending.
    pub eligible: Vec<usize>,
}

impl<'a> Field<'a> {
    pub fn new(positions: &'a [DVec3], scores: &'a [Option<Scores>]) -> Self {
        let eligible = (0..scores.len())
            .filter(|&id| scores[id].is_some())
            .collect();
        Self {
            positions,
            scores,
            eligible,
        }
    }

    /// An eligible system's overall score (SGEN-54).
    pub fn overall(&self, id: usize) -> f64 {
        self.scores[id].map_or(0.0, |s| s.overall())
    }

    /// The distance between two entries, in km.
    pub fn distance(&self, a: usize, b: usize) -> f64 {
        self.positions[a].distance(self.positions[b])
    }

    /// The eligible ids ranked by a score, best first, ties to the lower
    /// id.
    fn ranked(&self, score: impl Fn(usize) -> f64) -> Vec<usize> {
        let mut ids = self.eligible.clone();
        ids.sort_by(|&a, &b| score(b).total_cmp(&score(a)).then(a.cmp(&b)));
        ids
    }

    /// The eligible ids by overall score, best first.
    pub fn by_overall(&self) -> Vec<usize> {
        self.ranked(|id| self.overall(id))
    }

    /// The eligible ids by their score for `reason`, best first.
    pub fn by_reason(&self, reason: Reason) -> Vec<usize> {
        self.ranked(|id| self.scores[id].map_or(0.0, |s| s.get(reason)))
    }
}

/// One gated system as stage 4 leaves it.
#[derive(Debug, Clone, PartialEq)]
pub struct GateRecord {
    pub id: usize,
    pub scores: Scores,
    pub pick: Pick,
    pub main_reason: Reason,
    /// In AU (SGEN-58).
    pub ring_radius_au: f64,
    /// Indices into its summary's planets of those moved to clear the
    /// ring (SGEN-59).
    pub moved_planets: Vec<usize>,
}

/// Stage 4's output.
#[derive(Debug, Clone, PartialEq)]
pub struct GateChoice {
    /// The gated systems, by id.
    pub systems: Vec<GateRecord>,
    /// The link range after any growth the connectors needed, in km
    /// (SGEN-60).
    pub link_range_km: f64,
    /// Every entry's summary, with the planets of gated systems moved
    /// where the ring needed it (SGEN-59).
    pub summaries: Vec<Option<Summary>>,
}

/// The starting link range, in km (SGEN-60): `link_range_factor` mean
/// spacings of gated systems, a spacing being (V ÷ N)^(1/3) for the
/// slab's volume V.
pub fn starting_link_range_km(slab: &Slab, gated_count: u32, t: &GeneratorTunables) -> f64 {
    let spacing_pc = (slab.volume_pc3 / f64::from(gated_count)).cbrt();
    t.link_range_factor * spacing_pc * KM_PER_PARSEC
}

/// A system's main reason (SGEN-54): the reason whose quota it filled,
/// or else its highest-scoring reason.
pub fn main_reason(pick: Pick, scores: &Scores) -> Reason {
    match pick {
        Pick::Quota(reason) => reason,
        Pick::Score | Pick::Junction | Pick::Relay => scores.best_reason(),
    }
}

/// Runs stage 4 (SGEN-50 to SGEN-59) for `gated_count` gated systems.
pub fn run(
    gated_count: u32,
    slab: &Slab,
    entries: &[Entry],
    summaries: &[Option<Summary>],
    t: &GeneratorTunables,
) -> GateChoice {
    let positions: Vec<DVec3> = entries.iter().map(|e| e.position_km).collect();
    let all_scores: Vec<Option<Scores>> = entries
        .iter()
        .zip(summaries)
        .map(|(entry, summary)| match (entry.eligible, summary) {
            (true, Some(summary)) => Some(scores::scores(entry, summary, t)),
            _ => None,
        })
        .collect();
    let field = Field::new(&positions, &all_scores);
    let mut link_range_km = starting_link_range_km(slab, gated_count, t);
    let picked = choice::choose(gated_count as usize, &field, &mut link_range_km, t);
    let mut summaries = summaries.to_vec();
    let systems = picked
        .into_iter()
        .map(|(id, pick)| {
            let scores = all_scores[id].expect("only eligible systems are gated");
            let summary = summaries[id]
                .as_mut()
                .expect("eligible systems have summaries");
            let star = entries[id].primary().expect("eligible systems have a star");
            let ring = ring::place_ring(summary, star, t);
            summary.planets = ring.planets;
            GateRecord {
                id,
                scores,
                pick,
                main_reason: main_reason(pick, &scores),
                ring_radius_au: ring.radius_au,
                moved_planets: ring.moved,
            }
        })
        .collect();
    GateChoice {
        systems,
        link_range_km,
        summaries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::testing::{check_seeds, gates_for};

    /// SGEN-54: a quota pick's main reason is its quota's; any other's is
    /// its highest-scoring reason.
    #[test]
    fn main_reasons_follow_the_pick() {
        let scores = Scores {
            settlement: 0.1,
            extraction: 0.9,
            industry: 0.2,
            science: 0.3,
        };
        assert_eq!(
            main_reason(Pick::Quota(Reason::Science), &scores),
            Reason::Science
        );
        assert_eq!(main_reason(Pick::Junction, &scores), Reason::Extraction);
        assert_eq!(main_reason(Pick::Score, &scores), Reason::Extraction);
        for seed in check_seeds(5) {
            let (_, gates) = gates_for(80, seed);
            for system in &gates.systems {
                assert_eq!(system.main_reason, main_reason(system.pick, &system.scores));
            }
        }
    }

    /// SGEN-60: the starting range is two mean spacings, about 6 pc at
    /// every preset.
    #[test]
    fn the_link_range_starts_at_two_mean_spacings() {
        let t = GeneratorTunables::default();
        for n in [30, 80, 200] {
            let (entries, gates) = gates_for(n, 1);
            let (volume, _) = crate::content::generator::testing::stars_for(n, 1);
            let start = starting_link_range_km(&volume.slab, n, &t) / KM_PER_PARSEC;
            assert!((5.8..=6.6).contains(&start), "{start}");
            assert!(gates.link_range_km >= start * KM_PER_PARSEC);
            assert!(!entries.is_empty());
        }
    }
}

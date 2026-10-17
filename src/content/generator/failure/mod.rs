//! The failure of links (SGEN-70 to SGEN-76): most of the old network
//! has failed. A draw fails the links with the highest failure weights
//! (`draw`), then three rules move some back towards less failure so that
//! every system and every pocket of the network stays useful (`rules`).
//! The result is each link working, dormant (with which ends are
//! faulted) or collapsed. Every link joins two gated systems, so none
//! leads out of the sector (SGEN-76).

pub mod draw;
pub mod rules;

use super::gates::GateRecord;
use super::network::{Link, LinkState};
use super::rng::Rng;
use super::tunables::GeneratorTunables;
use rules::Failed;

/// Runs the failure stage (SGEN-70 to SGEN-75): each link's state, in the
/// links' order. Draws, in order: the map's shares, each link's chance
/// part of its weight, which failed links are dormant, then the faults of
/// dormant links.
pub fn run(
    links: &[Link],
    gated: &[GateRecord],
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> Vec<LinkState> {
    let ids: Vec<usize> = gated.iter().map(|g| g.id).collect();
    let index_of = |id: usize| ids.binary_search(&id).expect("links join gated systems");
    let ends: Vec<[usize; 2]> = links.iter().map(|link| link.ends.map(index_of)).collect();
    let traffic: Vec<f64> = ends
        .iter()
        .map(|&[a, b]| (gated[a].scores.overall() + gated[b].scores.overall()) / 2.0)
        .collect();
    let shares = draw::draw_shares(rng, t);
    let weights = draw::failure_weights(&traffic, rng, t);
    let lengths: Vec<f64> = links.iter().map(|link| link.length_km).collect();
    let mut failed = Failed {
        systems: ids.len(),
        ends: &ends,
        weights: &weights,
        lengths_km: &lengths,
        states: draw::draw_states(&weights, shares, rng),
    };
    failed.grow_main_region(t);
    failed.keep_one_link_each();
    failed.plan_repairs(rng);
    failed.states
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::stages::{Stage, StageSeeds};
    use crate::content::generator::testing::{check_seeds, gates_for};
    use crate::content::generator::{links, stage_rng};

    /// How far a state has failed: working 0, dormant 1, collapsed 2.
    fn severity(state: LinkState) -> u8 {
        match state {
            LinkState::Working => 0,
            LinkState::Dormant { .. } => 1,
            LinkState::Collapsed => 2,
        }
    }

    /// SGEN-75: the rules run in order after the draw, and each only
    /// moves links towards less failure, so none undoes another.
    #[test]
    fn each_rule_only_lessens_failure() {
        let t = GeneratorTunables::default();
        for seed in check_seeds(10) {
            let (entries, gates) = gates_for(30, seed);
            let network = links::run(&gates.systems, &entries, gates.link_range_km, &t);
            let seeds = StageSeeds::derive(seed);
            let result = run(
                &network.links,
                &gates.systems,
                &mut stage_rng(&seeds, Stage::Failure),
                &t,
            );
            // The same steps by hand, watching each.
            let rng = &mut stage_rng(&seeds, Stage::Failure);
            let ids: Vec<usize> = gates.systems.iter().map(|g| g.id).collect();
            let ends: Vec<[usize; 2]> = network
                .links
                .iter()
                .map(|l| l.ends.map(|id| ids.binary_search(&id).unwrap()))
                .collect();
            let overall = |i: usize| gates.systems[i].scores.overall();
            let traffic: Vec<f64> = ends
                .iter()
                .map(|&[a, b]| (overall(a) + overall(b)) / 2.0)
                .collect();
            let shares = draw::draw_shares(rng, &t);
            let weights = draw::failure_weights(&traffic, rng, &t);
            let lengths: Vec<f64> = network.links.iter().map(|l| l.length_km).collect();
            let mut failed = Failed {
                systems: ids.len(),
                ends: &ends,
                weights: &weights,
                lengths_km: &lengths,
                states: draw::draw_states(&weights, shares, rng),
            };
            let mut before = failed.states.clone();
            let mut lessens = |after: &[LinkState]| {
                for (b, a) in before.iter().zip(after) {
                    assert!(severity(*a) <= severity(*b));
                }
                before = after.to_vec();
            };
            failed.grow_main_region(&t);
            lessens(&failed.states);
            failed.keep_one_link_each();
            lessens(&failed.states);
            failed.plan_repairs(rng);
            lessens(&failed.states);
            assert_eq!(failed.states, result);
        }
    }
}

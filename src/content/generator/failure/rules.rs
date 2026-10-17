//! The rules that keep a failed network useful (SGEN-72 to SGEN-75),
//! applied in order after the draw. Each only moves a link towards less
//! failure (collapsed to dormant, failed to working), so a later rule
//! never undoes an earlier one.
//!
//! 1. The **main region**, the largest group joined by working links,
//!    holds at least `main_region_min_share` of the systems (SGEN-72).
//! 2. Every system keeps a link that has not collapsed (SGEN-73).
//! 3. Every other group can be brought into the main region by repairing
//!    dormant links in turn, each faulted only at the end already held
//!    (SGEN-74).

use crate::content::generator::graph::Groups;
use crate::content::generator::network::LinkState;
use crate::content::generator::rng::Rng;
use crate::content::generator::tunables::GeneratorTunables;

/// The failed network the rules work on: links between systems by index,
/// each with its state, failure weight and length.
pub struct Failed<'a> {
    /// How many systems there are.
    pub systems: usize,
    /// Each link's two systems, by index.
    pub ends: &'a [[usize; 2]],
    pub weights: &'a [f64],
    pub lengths_km: &'a [f64],
    pub states: Vec<LinkState>,
}

impl Failed<'_> {
    /// Each system's group joined by working links, labelled by the
    /// group's lowest index.
    pub fn working_groups(&self) -> Vec<usize> {
        let mut groups = Groups::new(self.systems);
        for (link, &[a, b]) in self.ends.iter().enumerate() {
            if self.states[link] == LinkState::Working {
                groups.join(a, b);
            }
        }
        groups.labels()
    }

    /// The label of the main region: the largest working group, of equal
    /// ones the one with the lowest index.
    pub fn main_region(labels: &[usize]) -> usize {
        let mut sizes = vec![0usize; labels.len()];
        for &label in labels {
            sizes[label] += 1;
        }
        (0..sizes.len())
            .max_by(|&a, &b| sizes[a].cmp(&sizes[b]).then(b.cmp(&a)))
            .unwrap_or(0)
    }

    /// Of the links that pass `wanted` and join a system inside `inside`
    /// to one outside, the one with the lowest `key`, ties to the lower
    /// index.
    fn best_crossing(
        &self,
        inside: &[bool],
        wanted: impl Fn(LinkState) -> bool,
        key: impl Fn(usize) -> f64,
    ) -> Option<usize> {
        (0..self.ends.len())
            .filter(|&l| {
                let [a, b] = self.ends[l];
                inside[a] != inside[b] && wanted(self.states[l])
            })
            .min_by(|&p, &q| key(p).total_cmp(&key(q)).then(p.cmp(&q)))
    }

    /// SGEN-72: while the main region holds fewer than
    /// `main_region_min_share` × N systems, the failed link of lowest
    /// weight joining it to another group is made working.
    pub fn grow_main_region(&mut self, t: &GeneratorTunables) {
        loop {
            let labels = self.working_groups();
            let main = Self::main_region(&labels);
            let inside: Vec<bool> = labels.iter().map(|&l| l == main).collect();
            let held = inside.iter().filter(|&&h| h).count();
            if held as f64 >= t.main_region_min_share * self.systems as f64 {
                return;
            }
            let failed = |s: LinkState| s != LinkState::Working;
            match self.best_crossing(&inside, failed, |l| self.weights[l]) {
                Some(link) => self.states[link] = LinkState::Working,
                None => return,
            }
        }
    }

    /// SGEN-73: a system whose links have all collapsed has the collapsed
    /// one of lowest weight made dormant.
    pub fn keep_one_link_each(&mut self) {
        for system in 0..self.systems {
            let own: Vec<usize> = (0..self.ends.len())
                .filter(|&l| self.ends[l].contains(&system))
                .collect();
            if own.is_empty() || own.iter().any(|&l| self.states[l] != LinkState::Collapsed) {
                continue;
            }
            let lightest = own
                .iter()
                .copied()
                .min_by(|&p, &q| self.weights[p].total_cmp(&self.weights[q]).then(p.cmp(&q)))
                .expect("the system has links");
            self.states[lightest] = LinkState::Dormant {
                faulted: [false, false],
            };
        }
    }

    /// SGEN-74: the repair order. Starting from the main region, each
    /// step reaches a group by the shortest dormant link between it and
    /// the held region, faulted only at the held end; if no group has a
    /// dormant link to the held region, the collapsed link of lowest
    /// weight to it is made dormant first. Every other dormant link is
    /// faulted at one end or both, at random. Returns the repair links in
    /// order.
    pub fn plan_repairs(&mut self, rng: &mut Rng) -> Vec<usize> {
        let labels = self.working_groups();
        let main = Self::main_region(&labels);
        let mut held: Vec<bool> = labels.iter().map(|&l| l == main).collect();
        let mut repairs = Vec::new();
        while held.iter().any(|&h| !h) {
            let dormant = |s: LinkState| matches!(s, LinkState::Dormant { .. });
            let link = match self.best_crossing(&held, dormant, |l| self.lengths_km[l]) {
                Some(link) => link,
                None => {
                    let collapsed = |s: LinkState| s == LinkState::Collapsed;
                    let Some(link) = self.best_crossing(&held, collapsed, |l| self.weights[l])
                    else {
                        // The network is not joined: nothing reaches the rest.
                        break;
                    };
                    link
                }
            };
            let [a, b] = self.ends[link];
            let reached = if held[a] { labels[b] } else { labels[a] };
            self.states[link] = LinkState::Dormant {
                faulted: [held[a], held[b]],
            };
            for system in 0..self.systems {
                held[system] |= labels[system] == reached;
            }
            repairs.push(link);
        }
        self.fault_the_rest(&repairs, rng);
        repairs
    }

    /// Faults every dormant link that is not a repair link at one end or
    /// both, each of the three chosen with equal chance (SGEN-74).
    fn fault_the_rest(&mut self, repairs: &[usize], rng: &mut Rng) {
        for link in 0..self.states.len() {
            if repairs.contains(&link) || !matches!(self.states[link], LinkState::Dormant { .. }) {
                continue;
            }
            let faulted = match rng.below(3) {
                0 => [true, false],
                1 => [false, true],
                _ => [true, true],
            };
            self.states[link] = LinkState::Dormant { faulted };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DORMANT: LinkState = LinkState::Dormant {
        faulted: [false, false],
    };

    /// A line of five systems, 0–1–2–3–4, with these states.
    fn line(states: [LinkState; 4]) -> (Vec<[usize; 2]>, Vec<LinkState>) {
        (vec![[0, 1], [1, 2], [2, 3], [3, 4]], states.to_vec())
    }

    /// SGEN-72: a main region that is too small grows by the failed links
    /// of lowest weight at its edge.
    #[test]
    fn a_small_main_region_grows_by_its_lightest_links() {
        let t = GeneratorTunables {
            main_region_min_share: 0.6,
            ..GeneratorTunables::default()
        };
        let c = LinkState::Collapsed;
        let (ends, states) = line([LinkState::Working, c, c, c]);
        let weights = [0.1, 0.9, 0.2, 0.3];
        let lengths = [1.0; 4];
        let mut failed = Failed {
            systems: 5,
            ends: &ends,
            weights: &weights,
            lengths_km: &lengths,
            states,
        };
        failed.grow_main_region(&t);
        // The main region {0, 1} needs a third system: only link 1 joins it
        // to the rest.
        assert_eq!(failed.states[1], LinkState::Working);
        assert_eq!(failed.states[2], c);
    }

    /// SGEN-73: a system cut off by collapsed links keeps its lightest one
    /// as dormant.
    #[test]
    fn every_system_keeps_a_link_that_has_not_collapsed() {
        let c = LinkState::Collapsed;
        let (ends, states) = line([LinkState::Working, c, LinkState::Working, c]);
        let weights = [0.5, 0.4, 0.5, 0.6];
        let lengths = [1.0; 4];
        let mut failed = Failed {
            systems: 5,
            ends: &ends,
            weights: &weights,
            lengths_km: &lengths,
            states,
        };
        failed.keep_one_link_each();
        assert_eq!(failed.states[3], DORMANT);
        assert_eq!(failed.states[1], c);
    }

    /// SGEN-74: groups are reached one by one by their shortest dormant
    /// link, faulted only at the held end; a group with only collapsed
    /// links gets its lightest made dormant first.
    #[test]
    fn every_group_can_be_reached_by_repairs() {
        let c = LinkState::Collapsed;
        // Main region {0, 1, 2}; 3 by a dormant link, 4 only collapsed.
        let ends = vec![[0, 1], [1, 2], [2, 3], [0, 3], [3, 4], [2, 4]];
        let states = vec![
            LinkState::Working,
            LinkState::Working,
            DORMANT,
            DORMANT,
            c,
            c,
        ];
        let weights = [0.1, 0.1, 0.5, 0.5, 0.7, 0.3];
        let lengths = [1.0, 1.0, 2.0, 1.5, 1.0, 1.0];
        let mut failed = Failed {
            systems: 5,
            ends: &ends,
            weights: &weights,
            lengths_km: &lengths,
            states,
        };
        let repairs = failed.plan_repairs(&mut Rng::new(4));
        // 3 first, by the shorter of its dormant links (0–3). Then 4,
        // which has no dormant link: its lightest collapsed link to the
        // held region (2–4, weight 0.3) is made dormant and repaired.
        assert_eq!(repairs, vec![3, 5]);
        assert_eq!(
            failed.states[5],
            LinkState::Dormant {
                faulted: [true, false]
            }
        );
        assert_eq!(
            failed.states[3],
            LinkState::Dormant {
                faulted: [true, false]
            }
        );
        assert_eq!(failed.states[4], c);
        // The other dormant link is faulted at one end at least.
        let LinkState::Dormant { faulted } = failed.states[2] else {
            panic!("still dormant")
        };
        assert!(faulted[0] || faulted[1]);
    }
}

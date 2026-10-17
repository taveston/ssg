//! Strategic systems: approaches to valuable ones (SGEN-64). The valuable
//! systems are the `strategic_top_share` of gated systems with the highest
//! overall scores. A system that is not valuable itself but links to at
//! least `strategic_min_valuable` valuable ones in the old network is
//! strategic. The label is separate from the main reason.

use crate::content::generator::network::Link;
use crate::content::generator::tunables::GeneratorTunables;

/// Which systems are valuable: the round(`strategic_top_share` × N) with
/// the highest overall scores, ties to the lower id. `ids` and `overall`
/// run in the same order.
pub fn valuable(ids: &[usize], overall: &[f64], t: &GeneratorTunables) -> Vec<bool> {
    let count = (t.strategic_top_share * ids.len() as f64).round() as usize;
    let mut order: Vec<usize> = (0..ids.len()).collect();
    order.sort_by(|&a, &b| overall[b].total_cmp(&overall[a]).then(ids[a].cmp(&ids[b])));
    let mut valuable = vec![false; ids.len()];
    for &index in order.iter().take(count) {
        valuable[index] = true;
    }
    valuable
}

/// Each system's strategic label, in the order of `ids` (SGEN-64).
pub fn label(ids: &[usize], overall: &[f64], links: &[Link], t: &GeneratorTunables) -> Vec<bool> {
    let valuable = valuable(ids, overall, t);
    let index_of = |id: usize| ids.binary_search(&id).expect("links join gated systems");
    let mut valuable_neighbours = vec![0u32; ids.len()];
    for link in links {
        let (a, b) = (index_of(link.ends[0]), index_of(link.ends[1]));
        valuable_neighbours[a] += u32::from(valuable[b]);
        valuable_neighbours[b] += u32::from(valuable[a]);
    }
    (0..ids.len())
        .map(|i| !valuable[i] && valuable_neighbours[i] >= t.strategic_min_valuable)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::network::{LinkOrigin, LinkState};

    fn link(a: usize, b: usize) -> Link {
        Link {
            ends: [a, b],
            length_km: 1.0,
            origin: LinkOrigin::Spanning,
            state: LinkState::Working,
        }
    }

    /// SGEN-64: a system linked to two valuable ones is strategic unless
    /// it is valuable itself; one valuable neighbour is not enough.
    #[test]
    fn approaches_to_valuable_systems_are_strategic() {
        let t = GeneratorTunables {
            strategic_top_share: 0.4,
            ..GeneratorTunables::default()
        };
        // Five systems; the top two by score (ids 10 and 11) are valuable.
        let ids = [10, 11, 12, 13, 14];
        let overall = [0.9, 0.8, 0.3, 0.2, 0.1];
        assert_eq!(
            valuable(&ids, &overall, &t),
            vec![true, true, false, false, false]
        );
        let links = [
            link(10, 12),
            link(11, 12),
            link(10, 11),
            link(11, 13),
            link(13, 14),
        ];
        assert_eq!(
            label(&ids, &overall, &links, &t),
            vec![false, false, true, false, false]
        );
    }
}

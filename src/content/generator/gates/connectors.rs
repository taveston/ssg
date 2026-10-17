//! Connectors: the systems added to join the chosen ones into one network
//! whose every hop is within link range (SGEN-56). A **junction** touches
//! two or more groups of chosen systems at once; where none does, a chain
//! of **relays** bridges the two closest groups; where no chain can, the
//! link range grows.

use bevy::math::DVec3;

use super::Field;
use crate::content::generator::graph::groups_within;
use crate::content::generator::network::Pick;
use crate::content::generator::tunables::GeneratorTunables;

/// The connectors that join `chosen` into one group within link range
/// (SGEN-56), in the order they were added, each marked junction or
/// relay. `range_km` grows by `link_range_growth` whenever no junction or
/// chain can be found, and keeps its growth for later steps (SGEN-60).
pub fn connect(
    chosen: &[usize],
    field: &Field,
    range_km: &mut f64,
    t: &GeneratorTunables,
) -> Vec<(usize, Pick)> {
    let mut members = chosen.to_vec();
    let mut connectors = Vec::new();
    loop {
        let labels = group_labels(&members, field.positions, *range_km);
        let groups = distinct(&labels).len();
        if groups <= 1 {
            return connectors;
        }
        let outside = unchosen(&members, field);
        if let Some(junction) = best_junction(&members, &labels, &outside, field, *range_km) {
            members.push(junction);
            connectors.push((junction, Pick::Junction));
        } else if let Some(chain) = shortest_chain(&members, &labels, &outside, field, *range_km) {
            for relay in chain {
                members.push(relay);
                connectors.push((relay, Pick::Relay));
            }
        } else {
            *range_km *= 1.0 + t.link_range_growth;
        }
    }
}

/// Each member's group within range, as the lowest index of its group.
fn group_labels(members: &[usize], positions: &[DVec3], range_km: f64) -> Vec<usize> {
    let points: Vec<DVec3> = members.iter().map(|&id| positions[id]).collect();
    groups_within(&points, range_km).labels()
}

/// The distinct values of `labels`, in ascending order.
fn distinct(labels: &[usize]) -> Vec<usize> {
    let mut values = labels.to_vec();
    values.sort_unstable();
    values.dedup();
    values
}

/// The eligible systems not among `members`, by id.
fn unchosen(members: &[usize], field: &Field) -> Vec<usize> {
    field
        .eligible
        .iter()
        .copied()
        .filter(|id| !members.contains(id))
        .collect()
}

/// The best junction (SGEN-56): an unchosen eligible system within range
/// of members of more than one group. The one that touches most groups
/// wins, then the one with most members in range, then the one nearest
/// its nearest member, then the lowest id.
fn best_junction(
    members: &[usize],
    labels: &[usize],
    outside: &[usize],
    field: &Field,
    range_km: f64,
) -> Option<usize> {
    // (groups touched, members in range, nearest distance, id).
    let mut best: Option<(usize, usize, f64, usize)> = None;
    for &candidate in outside {
        let here = field.positions[candidate];
        let mut touched = Vec::new();
        let (mut in_range, mut nearest) = (0, f64::INFINITY);
        for (index, &member) in members.iter().enumerate() {
            let distance = here.distance(field.positions[member]);
            if distance <= range_km {
                in_range += 1;
                nearest = nearest.min(distance);
                if !touched.contains(&labels[index]) {
                    touched.push(labels[index]);
                }
            }
        }
        if touched.len() < 2 {
            continue;
        }
        let key = (touched.len(), in_range, nearest, candidate);
        let better = best.is_none_or(|b| {
            (key.0, key.1) > (b.0, b.1) || ((key.0, key.1) == (b.0, b.1) && key.2 < b.2)
        });
        if better {
            best = Some(key);
        }
    }
    best.map(|(_, _, _, id)| id)
}

/// The two groups whose nearest members are closest, by their labels.
fn closest_groups(members: &[usize], labels: &[usize], positions: &[DVec3]) -> (usize, usize) {
    let mut best = (f64::INFINITY, 0, 0);
    for a in 0..members.len() {
        for b in a + 1..members.len() {
            if labels[a] == labels[b] {
                continue;
            }
            let distance = positions[members[a]].distance(positions[members[b]]);
            if distance < best.0 {
                best = (distance, labels[a], labels[b]);
            }
        }
    }
    (best.1, best.2)
}

/// The shortest chain of unchosen eligible systems, each hop within range,
/// from the closest group to the one nearest it (SGEN-56): fewest relays
/// first, then least total length. `None` if no chain exists in range.
fn shortest_chain(
    members: &[usize],
    labels: &[usize],
    outside: &[usize],
    field: &Field,
    range_km: f64,
) -> Option<Vec<usize>> {
    let (from, to) = closest_groups(members, labels, field.positions);
    let in_group = |label| {
        members
            .iter()
            .zip(labels)
            .filter(move |(_, l)| **l == label)
            .map(|(&id, _)| id)
    };
    let starts: Vec<usize> = in_group(from).collect();
    let ends: Vec<usize> = in_group(to).collect();
    // Nodes: the starts, the relays that may be used, then the ends.
    let nodes: Vec<usize> = starts.iter().chain(outside).chain(&ends).copied().collect();
    let path = Path {
        nodes: &nodes,
        starts: starts.len(),
        first_end: starts.len() + outside.len(),
    };
    let (end, previous) = path.cheapest(field, range_km)?;
    Some(relays_on_path(end, &previous, &nodes, starts.len()))
}

/// The nodes a chain may use: the start group's members first, then the
/// systems that may become relays, then the end group's members from
/// `first_end` on. Entries are entry ids.
struct Path<'a> {
    nodes: &'a [usize],
    starts: usize,
    first_end: usize,
}

impl Path<'_> {
    /// The cheapest path from any start to any end with every hop within
    /// range, fewest hops first, then least length (Dijkstra's search).
    /// Returns the end reached and each node's predecessor on the way.
    fn cheapest(&self, field: &Field, range_km: f64) -> Option<(usize, Vec<Option<usize>>)> {
        let count = self.nodes.len();
        let mut cost: Vec<(usize, f64)> = vec![(usize::MAX, f64::INFINITY); count];
        let mut previous: Vec<Option<usize>> = vec![None; count];
        let mut done = vec![false; count];
        for start in cost.iter_mut().take(self.starts) {
            *start = (0, 0.0);
        }
        loop {
            // The cheapest node not yet settled: fewest hops, then shortest.
            let next = (0..count)
                .filter(|&i| !done[i] && cost[i].0 != usize::MAX)
                .min_by(|&a, &b| {
                    cost[a]
                        .0
                        .cmp(&cost[b].0)
                        .then(cost[a].1.total_cmp(&cost[b].1))
                })?;
            if next >= self.first_end {
                return Some((next, previous));
            }
            done[next] = true;
            let here = field.positions[self.nodes[next]];
            for other in self.starts..count {
                let hop = here.distance(field.positions[self.nodes[other]]);
                if done[other] || hop > range_km {
                    continue;
                }
                let through = (cost[next].0 + 1, cost[next].1 + hop);
                let cheaper = through.0 < cost[other].0
                    || (through.0 == cost[other].0 && through.1 < cost[other].1);
                if cheaper {
                    cost[other] = through;
                    previous[other] = Some(next);
                }
            }
        }
    }
}

/// The relays on the path that ends at node `end`: every node between a
/// start and the end, from the start's side.
fn relays_on_path(
    end: usize,
    previous: &[Option<usize>],
    nodes: &[usize],
    start_count: usize,
) -> Vec<usize> {
    let mut relays = Vec::new();
    let mut at = previous[end];
    while let Some(node) = at {
        if node < start_count {
            break;
        }
        relays.push(nodes[node]);
        at = previous[node];
    }
    relays.reverse();
    relays
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::network::Scores;

    fn field_of(points: &[(f64, f64)]) -> (Vec<DVec3>, Vec<Option<Scores>>) {
        let positions = points.iter().map(|&(x, y)| DVec3::new(x, y, 0.0)).collect();
        let scores = vec![
            Some(Scores {
                settlement: 0.5,
                extraction: 0.0,
                industry: 0.0,
                science: 0.0,
            });
            points.len()
        ];
        (positions, scores)
    }

    /// SGEN-56: a system in range of two groups joins them as a junction.
    #[test]
    fn a_junction_joins_two_groups() {
        let t = GeneratorTunables::default();
        let (positions, scores) = field_of(&[(0.0, 0.0), (2.0, 0.0), (1.0, 0.5), (1.0, 3.0)]);
        let field = Field::new(&positions, &scores);
        let mut range = 1.5;
        let connectors = connect(&[0, 1], &field, &mut range, &t);
        assert_eq!(connectors, vec![(2, Pick::Junction)]);
        assert_eq!(range, 1.5);
    }

    /// SGEN-56: groups no junction can join are bridged by the shortest
    /// chain of relays, fewest relays first.
    #[test]
    fn a_chain_of_relays_bridges_far_groups() {
        let t = GeneratorTunables::default();
        // Chosen at 0 and 4; relays possible at 1, 2, 3 (a straight
        // chain) and a detour at 2 off the line.
        let (positions, scores) = field_of(&[
            (0.0, 0.0),
            (4.0, 0.0),
            (1.0, 0.0),
            (2.0, 0.0),
            (3.0, 0.0),
            (2.0, 0.9),
        ]);
        let field = Field::new(&positions, &scores);
        let mut range = 1.01;
        let connectors = connect(&[0, 1], &field, &mut range, &t);
        let relays: Vec<usize> = connectors.iter().map(|(id, _)| *id).collect();
        assert_eq!(relays, vec![2, 3, 4]);
        assert!(connectors.iter().all(|(_, pick)| *pick == Pick::Relay));
    }

    /// SGEN-56, SGEN-60: when nothing in range can join two groups, the
    /// range grows until something does.
    #[test]
    fn the_range_grows_when_no_chain_exists() {
        let t = GeneratorTunables::default();
        let (positions, scores) = field_of(&[(0.0, 0.0), (3.0, 0.0)]);
        let field = Field::new(&positions, &scores);
        let mut range = 1.0;
        let connectors = connect(&[0, 1], &field, &mut range, &t);
        assert!(connectors.is_empty());
        assert!((3.0..3.0 * 1.1).contains(&range), "{range}");
    }
}

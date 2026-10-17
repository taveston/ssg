//! The old network's links, in three steps (SGEN-61): a spanning tree
//! over the gated systems by 3D distance; then links between pairs within
//! range, shortest first, wherever both ends have room and the link
//! crosses no other in the top-down view; then links for any system still
//! short of `links_min`, to its nearest neighbours. The link range grows
//! whenever a step cannot be done within it (SGEN-60).

use bevy::math::{DVec2, DVec3};

use super::crossing::{segments_cross, top_down};
use crate::content::generator::graph::Groups;
use crate::content::generator::network::{Link, LinkOrigin, LinkState};
use crate::content::generator::tunables::GeneratorTunables;

/// Every link between the systems at `positions` (whose entry ids are
/// `ids`, ascending), and the final link range in km (SGEN-60 to
/// SGEN-62, SGEN-65).
pub fn build(
    ids: &[usize],
    positions: &[DVec3],
    start_range_km: f64,
    t: &GeneratorTunables,
) -> (Vec<Link>, f64) {
    let mut mesh = Mesh::new(positions, start_range_km, t);
    mesh.spanning_tree();
    mesh.extra_links();
    mesh.second_links();
    let links = mesh
        .links
        .iter()
        .map(|&(a, b, origin)| Link {
            ends: [ids[a], ids[b]],
            length_km: positions[a].distance(positions[b]),
            origin,
            state: LinkState::Working,
        })
        .collect();
    (links, mesh.range_km)
}

/// A pair of systems, by index, and the distance between them in km.
#[derive(Debug, Clone, Copy)]
struct Pair {
    a: usize,
    b: usize,
    distance_km: f64,
}

/// The network as it is built, over systems by index.
struct Mesh<'a> {
    positions: &'a [DVec3],
    /// The positions seen from above.
    flat: Vec<DVec2>,
    /// Every pair, shortest first; ties by index.
    pairs: Vec<Pair>,
    /// Links so far: the lower index, the higher, and the step.
    links: Vec<(usize, usize, LinkOrigin)>,
    /// Whether each pair is linked, by index a × count + b.
    linked: Vec<bool>,
    degree: Vec<u32>,
    range_km: f64,
    t: &'a GeneratorTunables,
}

impl<'a> Mesh<'a> {
    fn new(positions: &'a [DVec3], range_km: f64, t: &'a GeneratorTunables) -> Self {
        let count = positions.len();
        let mut pairs = Vec::with_capacity(count * count.saturating_sub(1) / 2);
        for a in 0..count {
            for b in a + 1..count {
                let distance_km = positions[a].distance(positions[b]);
                pairs.push(Pair { a, b, distance_km });
            }
        }
        pairs.sort_by(|p, q| {
            p.distance_km
                .total_cmp(&q.distance_km)
                .then((p.a, p.b).cmp(&(q.a, q.b)))
        });
        Self {
            positions,
            flat: positions.iter().map(|&p| top_down(p)).collect(),
            pairs,
            links: Vec::new(),
            linked: vec![false; count * count],
            degree: vec![0; count],
            range_km,
            t,
        }
    }

    fn is_linked(&self, a: usize, b: usize) -> bool {
        self.linked[a * self.positions.len() + b]
    }

    fn has_room(&self, a: usize) -> bool {
        self.degree[a] < self.t.links_max
    }

    fn add(&mut self, a: usize, b: usize, origin: LinkOrigin) {
        let (a, b) = (a.min(b), a.max(b));
        let count = self.positions.len();
        self.linked[a * count + b] = true;
        self.linked[b * count + a] = true;
        self.degree[a] += 1;
        self.degree[b] += 1;
        self.links.push((a, b, origin));
    }

    /// The links the link from `a` to `b` would cross from above.
    fn crossings(&self, a: usize, b: usize) -> impl Iterator<Item = LinkOrigin> + '_ {
        let (p, q) = (self.flat[a], self.flat[b]);
        self.links.iter().filter_map(move |&(c, d, origin)| {
            segments_cross(p, q, self.flat[c], self.flat[d]).then_some(origin)
        })
    }

    /// Step 1: a minimum spanning tree by 3D distance (Kruskal's: the
    /// shortest pairs first, skipping any that would close a loop), never
    /// giving a system more than `links_max` links. The range then grows
    /// until it covers the tree's longest link.
    fn spanning_tree(&mut self) {
        let mut groups = Groups::new(self.positions.len());
        let mut longest: f64 = 0.0;
        for index in 0..self.pairs.len() {
            let Pair { a, b, distance_km } = self.pairs[index];
            if self.has_room(a) && self.has_room(b) && groups.join(a, b) {
                self.add(a, b, LinkOrigin::Spanning);
                longest = longest.max(distance_km);
            }
        }
        while self.range_km < longest {
            self.range_km *= 1.0 + self.t.link_range_growth;
        }
    }

    /// Step 2: every pair within range, shortest first, is linked if both
    /// ends have room and the link crosses no existing link from above.
    fn extra_links(&mut self) {
        for index in 0..self.pairs.len() {
            let Pair { a, b, distance_km } = self.pairs[index];
            if distance_km > self.range_km {
                break;
            }
            if !self.is_linked(a, b)
                && self.has_room(a)
                && self.has_room(b)
                && self.crossings(a, b).next().is_none()
            {
                self.add(a, b, LinkOrigin::Extra);
            }
        }
    }

    /// Step 3: a system with fewer than `links_min` links is linked to its
    /// nearest systems within range that have room and are not yet linked
    /// to it: one that crosses nothing if there is one, else one that
    /// crosses no step-2 link, else the nearest. If none is in range, the
    /// range grows (SGEN-60).
    fn second_links(&mut self) {
        let farthest = self.pairs.last().map_or(0.0, |p| p.distance_km);
        for a in 0..self.positions.len() {
            while self.degree[a] < self.t.links_min {
                match self.second_partner(a) {
                    Some(b) => self.add(a, b, LinkOrigin::Second),
                    // Every system is already in range: none has room.
                    None if self.range_km >= farthest => break,
                    None => self.range_km *= 1.0 + self.t.link_range_growth,
                }
            }
        }
    }

    /// The partner step 3 links `a` to, if any is in range (SGEN-61).
    fn second_partner(&self, a: usize) -> Option<usize> {
        let here = self.positions[a];
        let mut candidates: Vec<(f64, usize)> = (0..self.positions.len())
            .filter(|&b| b != a && !self.is_linked(a, b) && self.has_room(b))
            .map(|b| (here.distance(self.positions[b]), b))
            .filter(|&(distance, _)| distance <= self.range_km)
            .collect();
        candidates.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
        let clear = candidates
            .iter()
            .find(|&&(_, b)| self.crossings(a, b).next().is_none());
        let spares_extra = candidates.iter().find(|&&(_, b)| {
            self.crossings(a, b)
                .all(|origin| origin != LinkOrigin::Extra)
        });
        clear
            .or(spares_extra)
            .or(candidates.first())
            .map(|&(_, b)| b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<DVec3> {
        vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ]
    }

    /// SGEN-61: four corners of a square in range get the tree, then the
    /// last side, then no diagonal, since the two would cross.
    #[test]
    fn a_square_gets_its_sides_and_one_diagonal_at_most() {
        let t = GeneratorTunables::default();
        let (links, range) = build(&[0, 1, 2, 3], &square(), 2.0, &t);
        let spanning = links
            .iter()
            .filter(|l| l.origin == LinkOrigin::Spanning)
            .count();
        assert_eq!(spanning, 3);
        let extra: Vec<&Link> = links
            .iter()
            .filter(|l| l.origin == LinkOrigin::Extra)
            .collect();
        // The fourth side, and one diagonal; the other diagonal would cross it.
        assert_eq!(extra.len(), 2);
        assert_eq!(range, 2.0);
        assert!(links.iter().all(|l| l.state == LinkState::Working));
    }

    /// SGEN-60, SGEN-61: a system out of range of the rest still joins the
    /// tree, and the range grows to cover its link.
    #[test]
    fn the_range_grows_to_cover_the_spanning_tree() {
        let t = GeneratorTunables::default();
        let mut points = square();
        points.push(DVec3::new(10.0, 0.0, 0.0));
        let (links, range) = build(&[0, 1, 2, 3, 4], &points, 2.0, &t);
        assert!(range >= 9.0);
        assert!(links.iter().all(|l| l.length_km <= range));
        assert!(links.iter().any(|l| l.ends.contains(&4)));
    }

    /// SGEN-61: step 3 links a system short of links to its nearest
    /// partner that crosses nothing, else one that crosses no step-2
    /// link, else the nearest.
    #[test]
    fn step_three_prefers_partners_that_cross_nothing() {
        let t = GeneratorTunables::default();
        let points = square();
        let mut mesh = Mesh::new(&points, 2.0, &t);
        mesh.add(0, 2, LinkOrigin::Extra);
        // Corner 1 can reach 0 and 2 along the sides, or 3 across the
        // diagonal 0–2.
        assert_eq!(mesh.second_partner(1), Some(0));
        mesh.add(1, 0, LinkOrigin::Second);
        mesh.add(1, 2, LinkOrigin::Second);
        // Only 3 is left, and it crosses the step-2 diagonal: taken anyway.
        assert_eq!(mesh.second_partner(1), Some(3));
        // A spanning diagonal is crossed before a step-2 one would be.
        let mut mesh = Mesh::new(&points, 2.0, &t);
        mesh.add(0, 2, LinkOrigin::Spanning);
        mesh.add(1, 0, LinkOrigin::Spanning);
        mesh.add(1, 2, LinkOrigin::Spanning);
        assert_eq!(mesh.second_partner(1), Some(3));
    }

    /// SGEN-62: every system ends with at least `links_min` links when
    /// partners are in reach.
    #[test]
    fn every_system_ends_with_at_least_the_minimum() {
        let t = GeneratorTunables {
            links_max: 2,
            ..GeneratorTunables::default()
        };
        // A line of three: the middle takes both ends' first links; the ends
        // need a second, and the only partner left is each other.
        let points = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(2.0, 0.1, 0.0),
        ];
        let (links, _) = build(&[5, 6, 7], &points, 3.0, &t);
        assert_eq!(links.len(), 3);
        let degree = |id: usize| links.iter().filter(|l| l.ends.contains(&id)).count();
        assert!([5, 6, 7].iter().all(|&id| degree(id) == 2));
        assert!(links.iter().all(|l| l.ends[0] < l.ends[1]));
    }
}

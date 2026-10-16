//! Which things share a multi-marker (MUL-05, PHAS-02), as plain rules:
//! things **together** in one named orbit, and symbols that **overlap**
//! on screen. Both link pairs and take every chain of links as one group.
//!
//! They run every frame over every ship, so neither tests every pair.
//! Things together are neighbours in phase in their orbit, or a chain of
//! them is, so only neighbours are tested. Symbols that can overlap lie
//! within `cluster_split_px` of each other, so they are found through a
//! grid of cells that wide. The overlap latch, which keeps a joined pair
//! joined inside the hysteresis band, is kept per symbol by the thing
//! that leads it, with the lead each thing had, so it grows with the
//! symbols and things and not with their pairs.

use std::collections::{HashMap, HashSet};
use std::f64::consts::TAU;

use bevy::prelude::*;

use crate::sim::transfer::phase::{gap_rad, together};

/// A thing in a named orbit, for finding what is together (PHAS-02).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitSlot {
    pub thing: Entity,
    pub orbit: Entity,
    /// The orbit's radius, the same for every thing in it.
    pub radius_km: f64,
    pub phase_rad: f64,
}

/// The root of `i` in a union-find forest, flattening as it goes.
fn root(parents: &mut [usize], mut i: usize) -> usize {
    while parents[i] != i {
        parents[i] = parents[parents[i]];
        i = parents[i];
    }
    i
}

/// Groups `count` items by `links` between pairs of them: every item
/// linked to another by a chain of links is in one group. Groups and
/// their items keep the items' order, whatever the links' order.
fn link_groups(count: usize, links: impl IntoIterator<Item = (usize, usize)>) -> Vec<Vec<usize>> {
    let mut parents: Vec<usize> = (0..count).collect();
    for (a, b) in links {
        let (ra, rb) = (root(&mut parents, a), root(&mut parents, b));
        parents[ra.max(rb)] = ra.min(rb);
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut group_of: Vec<Option<usize>> = vec![None; count];
    for i in 0..count {
        let r = root(&mut parents, i);
        let index = *group_of[r].get_or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[index].push(i);
    }
    groups
}

#[cfg(test)]
thread_local! {
    /// How many pairs this thread's linking has compared, for the test
    /// that neither rule compares every pair.
    static COMPARED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Notes one comparison of a pair; counted only in tests.
fn note_comparison() {
    #[cfg(test)]
    COMPARED.with(|compared| compared.set(compared.get() + 1));
}

/// PHAS-02: whether two things of one orbit are together, by the shorter
/// way round between them.
fn slots_together(a: &OrbitSlot, b: &OrbitSlot, together_km: f64) -> bool {
    note_comparison();
    let gap = gap_rad(a.phase_rad, b.phase_rad).min(gap_rad(b.phase_rad, a.phase_rad));
    together(a.radius_km, gap, together_km)
}

/// MUL-05 / PHAS-02: the sets of two or more things together in one named
/// orbit: each thing closer than `together_km` to another of the set, in
/// a chain. Things keep their order within a set.
///
/// Things closer than `together_km` round one orbit are neighbours in
/// phase, or every thing between them is closer still, so linking each
/// thing to its neighbours (the last of an orbit to its first, across the
/// zero angle) finds the same sets as testing every pair.
pub fn together_sets(slots: &[OrbitSlot], together_km: f64) -> Vec<Vec<Entity>> {
    let phase = |i: usize| slots[i].phase_rad.rem_euclid(TAU);
    let mut order: Vec<usize> = (0..slots.len()).collect();
    order.sort_by(|&a, &b| {
        (slots[a].orbit.cmp(&slots[b].orbit)).then(phase(a).total_cmp(&phase(b)))
    });
    let mut links = Vec::new();
    for run in order.chunk_by(|&a, &b| slots[a].orbit == slots[b].orbit) {
        let wrap = (run.len() > 2).then(|| (run[run.len() - 1], run[0]));
        let neighbours = run.windows(2).map(|w| (w[0], w[1])).chain(wrap);
        for (a, b) in neighbours {
            let (a, b) = (a.min(b), a.max(b));
            if slots_together(&slots[a], &slots[b], together_km) {
                links.push((a, b));
            }
        }
    }
    link_groups(slots.len(), links)
        .into_iter()
        .filter(|set| set.len() > 1)
        .map(|set| set.into_iter().map(|i| slots[i].thing).collect())
        .collect()
}

/// MUL-05: whether two symbols `distance` pixels apart are joined: below
/// `join_px` they join, above `split_px` they part, and between the two
/// they keep `was_joined`.
pub fn next_joined(distance: f32, was_joined: bool, join_px: f32, split_px: f32) -> bool {
    if distance < join_px {
        true
    } else if distance > split_px {
        false
    } else {
        was_joined
    }
}

/// A pair of things, in a fixed order, for the overlap latch.
fn pair(a: Entity, b: Entity) -> (Entity, Entity) {
    if a < b { (a, b) } else { (b, a) }
}

/// A symbol on screen that can overlap others (MUL-05): the thing that
/// leads it, one of its things and in no other symbol; every thing it
/// stands for; and where it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Spot {
    pub lead: Entity,
    pub things: Vec<Entity>,
    pub pos: Vec2,
}

/// MUL-05: last frame's overlaps, for the hysteresis: the pairs of
/// symbols that were joined, by their leads, and the lead of the symbol
/// each thing was in. Two symbols were joined when any thing of one was
/// in a symbol joined to one that held any thing of the other, so they
/// stay joined while either gains, loses or changes its lead.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlapLatch {
    joined: HashSet<(Entity, Entity)>,
    lead_of: HashMap<Entity, Entity>,
}

impl OverlapLatch {
    /// The leads last frame of the symbols that held `things`, each once.
    fn leads_of(&self, things: &[Entity]) -> Vec<Entity> {
        let mut leads: Vec<Entity> = things
            .iter()
            .filter_map(|t| self.lead_of.get(t).copied())
            .collect();
        leads.sort();
        leads.dedup();
        leads
    }

    /// Whether a symbol last led by any of `a` was joined to one last led
    /// by any of `b`.
    fn were_joined(&self, a: &[Entity], b: &[Entity]) -> bool {
        a.iter()
            .any(|&x| b.iter().any(|&y| self.joined.contains(&pair(x, y))))
    }
}

/// The grid cell `pos` falls in, for cells `cell` pixels wide.
fn cell_of(pos: Vec2, cell: f32) -> IVec2 {
    (pos / cell).floor().as_ivec2()
}

/// The pairs of points no further apart than `reach`, each as `(a, b)`
/// with `a < b`: found through a grid of cells `reach` wide, so only
/// points in neighbouring cells are compared.
fn near_pairs(points: &[Vec2], reach: f32) -> Vec<(usize, usize)> {
    let cell = reach.max(1.0);
    let mut grid: HashMap<IVec2, Vec<usize>> = HashMap::new();
    for (i, &pos) in points.iter().enumerate() {
        grid.entry(cell_of(pos, cell)).or_default().push(i);
    }
    let mut pairs = Vec::new();
    for (a, &pos) in points.iter().enumerate() {
        let home = cell_of(pos, cell);
        for dx in -1..=1 {
            for dy in -1..=1 {
                let Some(others) = grid.get(&(home + IVec2::new(dx, dy))) else {
                    continue;
                };
                pairs.extend(
                    others
                        .iter()
                        .filter(|&&b| b > a)
                        .filter(|&&b| {
                            note_comparison();
                            pos.distance(points[b]) <= reach
                        })
                        .map(|&b| (a, b)),
                );
            }
        }
    }
    pairs
}

/// What the latch relies on: no thing is in two spots, and each spot's
/// lead is one of its own things, so no two spots share a lead.
fn spots_are_distinct(spots: &[Spot]) -> bool {
    let mut seen = HashSet::new();
    spots.iter().all(|s| s.things.contains(&s.lead))
        && spots
            .iter()
            .flat_map(|s| &s.things)
            .all(|t| seen.insert(*t))
}

/// MUL-05: the overlap clusters among `spots`: every symbol joined to
/// another by a chain of joined pairs, with the hysteresis of
/// `next_joined` read from `latch`, last frame's overlaps. Returns the
/// clusters, as indices into `spots` (lone symbols as clusters of one),
/// and this frame's latch. Symbols further apart than `split_px` never
/// join, so only those nearer are compared.
pub fn overlap_clusters(
    spots: &[Spot],
    latch: &OverlapLatch,
    join_px: f32,
    split_px: f32,
) -> (Vec<Vec<usize>>, OverlapLatch) {
    debug_assert!(
        spots_are_distinct(spots),
        "each thing in one spot at most, each spot led by one of its own things, no lead twice"
    );
    let points: Vec<Vec2> = spots.iter().map(|s| s.pos).collect();
    let leads: Vec<Vec<Entity>> = spots.iter().map(|s| latch.leads_of(&s.things)).collect();
    let mut links = Vec::new();
    let mut next = OverlapLatch::default();
    for (a, b) in near_pairs(&points, split_px) {
        let was = latch.were_joined(&leads[a], &leads[b]);
        if next_joined(points[a].distance(points[b]), was, join_px, split_px) {
            links.push((a, b));
            next.joined.insert(pair(spots[a].lead, spots[b].lead));
        }
    }
    next.lead_of = spots
        .iter()
        .flat_map(|s| s.things.iter().map(move |&t| (t, s.lead)))
        .collect();
    (link_groups(spots.len(), links), next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entities(n: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..n).map(|_| world.spawn_empty().id()).collect()
    }

    /// A lone symbol of one thing at `pos`.
    fn spot(thing: Entity, pos: Vec2) -> Spot {
        Spot {
            lead: thing,
            things: vec![thing],
            pos,
        }
    }

    /// MUL-05 / PHAS-02: things in one orbit closer than `together_km` are
    /// together, in a chain; another orbit at the same phase is not.
    #[test]
    fn things_within_together_km_in_one_orbit_are_one_set() {
        let e = entities(6);
        let slot = |thing, orbit, phase_rad| OrbitSlot {
            thing,
            orbit,
            radius_km: 40_000.0,
            phase_rad,
        };
        // 10 km is 2.5e-4 rad at 40,000 km.
        let slots = [
            slot(e[0], e[5], 1.0),
            slot(e[1], e[5], 1.0 + 1.5e-4),
            slot(e[2], e[5], 1.0 + 3.0e-4),
            slot(e[3], e[4], 1.0),
            slot(e[4], e[5], 2.0),
        ];
        assert_eq!(together_sets(&slots, 10.0), vec![vec![e[0], e[1], e[2]]]);
        // Across the zero angle, either way round.
        let wrap = [
            slot(e[0], e[5], 0.0),
            slot(e[1], e[5], std::f64::consts::TAU - 1e-5),
        ];
        assert_eq!(together_sets(&wrap, 10.0).len(), 1);
        let wrap_of_three = [
            slot(e[0], e[5], 1e-5),
            slot(e[1], e[5], 3.0),
            slot(e[2], e[5], -1e-5),
        ];
        assert_eq!(together_sets(&wrap_of_three, 10.0), vec![vec![e[0], e[2]]]);
    }

    /// MUL-05: symbols join below `cluster_join_px`, part only above
    /// `cluster_split_px`, and keep their state between the two.
    #[test]
    fn two_symbols_join_below_the_join_distance_and_part_above_the_split() {
        let t = crate::view::tunables::Tunables::default();
        let (join, split) = (t.cluster_join_px, t.cluster_split_px);
        assert!(join < split);
        assert!(next_joined(join - 0.1, false, join, split));
        assert!(!next_joined(split + 0.1, true, join, split));
        let between = (join + split) / 2.0;
        assert!(next_joined(between, true, join, split));
        assert!(!next_joined(between, false, join, split));

        let e = entities(2);
        let at = |d: f32| [spot(e[0], Vec2::ZERO), spot(e[1], Vec2::new(d, 0.0))];
        let mut latch = OverlapLatch::default();
        let mut clusters_at = |d: f32| {
            let (clusters, next) = overlap_clusters(&at(d), &latch, join, split);
            latch = next;
            clusters.len()
        };
        assert_eq!(clusters_at(between), 2, "apart, and not yet joined");
        assert_eq!(clusters_at(join - 1.0), 1, "joined");
        assert_eq!(clusters_at(between), 1, "stays joined in the band");
        // Exactly the split apart, one grid cell over: still in the band.
        assert_eq!(clusters_at(split), 1, "stays joined at the split");
        assert_eq!(clusters_at(split + 1.0), 2, "parted");
        assert_eq!(clusters_at(between), 2, "stays parted in the band");
    }

    /// MUL-05: a cluster is every symbol joined to another by a chain of
    /// joined pairs, so three in a row 20 px apart are one.
    #[test]
    fn a_chain_of_joined_pairs_is_one_cluster() {
        let e = entities(4);
        let spots = [
            spot(e[0], Vec2::new(0.0, 0.0)),
            spot(e[1], Vec2::new(20.0, 0.0)),
            spot(e[2], Vec2::new(40.0, 0.0)),
            spot(e[3], Vec2::new(200.0, 0.0)),
        ];
        let (clusters, _) = overlap_clusters(&spots, &OverlapLatch::default(), 24.0, 28.0);
        assert_eq!(clusters, vec![vec![0, 1, 2], vec![3]]);
    }

    /// A small deterministic generator, so the comparisons below see the
    /// same "random" scenes on every run.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// The together sets by testing every pair, as the rule is written.
    fn together_sets_by_pairs(slots: &[OrbitSlot], together_km: f64) -> Vec<Vec<Entity>> {
        let mut links = Vec::new();
        for a in 0..slots.len() {
            for b in a + 1..slots.len() {
                if slots[a].orbit == slots[b].orbit
                    && slots_together(&slots[a], &slots[b], together_km)
                {
                    links.push((a, b));
                }
            }
        }
        link_groups(slots.len(), links)
            .into_iter()
            .filter(|set| set.len() > 1)
            .map(|set| set.into_iter().map(|i| slots[i].thing).collect())
            .collect()
    }

    /// MUL-05 / PHAS-02: linking only neighbours in phase finds the same
    /// sets as testing every pair, over many crowded orbits of different
    /// radii, with things at the very same phase and across the zero
    /// angle.
    #[test]
    fn neighbours_in_phase_find_the_same_sets_as_every_pair() {
        let e = entities(400);
        let orbits = &e[390..];
        let mut rng = Lcg(7);
        let (mut chains, mut wraps) = (0, 0);
        for scene in 0..20 {
            let slots: Vec<OrbitSlot> = e[..390]
                .iter()
                .map(|&thing| {
                    let o = (rng.next() * orbits.len() as f64) as usize;
                    // Clustered near a few phases, some just either side
                    // of zero, so sets form, chain and wrap; one in five
                    // exactly on its cluster's phase.
                    let centre = ((rng.next() * 6.0).floor() - 1.0) * 0.5;
                    let spread = (rng.next() - 0.5) * 2e-3;
                    let exact = rng.next() < 0.2;
                    OrbitSlot {
                        thing,
                        orbit: orbits[o],
                        radius_km: 30_000.0 + 2_000.0 * o as f64,
                        phase_rad: if exact { centre } else { centre + spread },
                    }
                })
                .collect();
            let sets = together_sets(&slots, 10.0);
            assert_eq!(sets, together_sets_by_pairs(&slots, 10.0), "scene {scene}");
            chains += sets.iter().filter(|set| set.len() > 2).count();
            let phase_of = |t: &Entity| slots.iter().find(|s| s.thing == *t).unwrap().phase_rad;
            wraps += sets
                .iter()
                .filter(|set| {
                    set.iter().any(|t| phase_of(t) < 0.0) && set.iter().any(|t| phase_of(t) > 0.0)
                })
                .count();
        }
        assert!(chains > 0, "the scenes have chains to find");
        assert!(wraps > 0, "the scenes have sets across the zero angle");
    }

    /// The overlap clusters by testing every pair, with the latch kept as
    /// every pair of things joined last frame: the rule as it is written.
    /// Also counts the pairs the latch kept joined inside the band.
    fn overlap_by_pairs(
        spots: &[Spot],
        latch: &HashSet<(Entity, Entity)>,
        (join_px, split_px): (f32, f32),
    ) -> (Vec<Vec<usize>>, HashSet<(Entity, Entity)>, usize) {
        let mut links = Vec::new();
        let mut joined = HashSet::new();
        let mut held = 0;
        for a in 0..spots.len() {
            for b in a + 1..spots.len() {
                let (sa, sb) = (&spots[a], &spots[b]);
                let was = sa
                    .things
                    .iter()
                    .any(|&x| sb.things.iter().any(|&y| latch.contains(&pair(x, y))));
                let distance = sa.pos.distance(sb.pos);
                if next_joined(distance, was, join_px, split_px) {
                    held += usize::from(distance >= join_px);
                    links.push((a, b));
                    for &x in &sa.things {
                        joined.extend(sb.things.iter().map(|&y| pair(x, y)));
                    }
                }
            }
        }
        (link_groups(spots.len(), links), joined, held)
    }

    /// MUL-05: the grid and the latch by leads give the same clusters as
    /// testing every pair with a latch of every pair of things, frame
    /// after frame, while symbols drift through the hysteresis band and
    /// things move between symbols, changing their leads.
    #[test]
    fn the_grid_and_lead_latch_cluster_as_every_pair_would() {
        let things = entities(300);
        let mut rng = Lcg(11);
        let mut pos: Vec<Vec2> = (0..things.len())
            .map(|_| Vec2::new(rng.next() as f32 * 600.0, rng.next() as f32 * 400.0))
            .collect();
        let (mut latch, mut by_pairs) = (OverlapLatch::default(), HashSet::new());
        let mut held = 0;
        for frame in 0..40 {
            // Things gather into symbols of one to four; each symbol is
            // drawn where its first thing is, led by a random one of them.
            let mut spots = Vec::new();
            let mut i = 0;
            while i < things.len() {
                let size = 1 + (rng.next() * 4.0) as usize;
                let members = things[i..(i + size).min(things.len())].to_vec();
                let lead = members[(rng.next() * members.len() as f64) as usize];
                spots.push(Spot {
                    lead,
                    things: members,
                    pos: pos[i],
                });
                i += size;
            }
            let (clusters, next) = overlap_clusters(&spots, &latch, 24.0, 28.0);
            let (want, joined, kept) = overlap_by_pairs(&spots, &by_pairs, (24.0, 28.0));
            assert_eq!(clusters, want, "frame {frame}");
            assert!(want.len() > 1 && want.iter().any(|c| c.len() > 2));
            held += kept;
            (latch, by_pairs) = (next, joined);
            for p in &mut pos {
                *p += Vec2::new(rng.next() as f32 - 0.5, rng.next() as f32 - 0.5) * 6.0;
            }
        }
        assert!(held > 0, "the latch is put to the test");
    }

    /// MUL-05: both rules keep up with far more ships than any scene has.
    /// 40,000 things in one orbit and 40,000 symbols on screen are linked
    /// with a few comparisons each; testing every pair would take 800
    /// million. Counted, not timed, so a slow machine can't fail it.
    #[test]
    fn linking_many_things_does_not_test_every_pair() {
        let count = 40_000;
        let e = entities(count + 1);
        let (things, orbit) = (&e[..count], e[count]);
        let mut rng = Lcg(3);
        let slots: Vec<OrbitSlot> = things
            .iter()
            .map(|&thing| OrbitSlot {
                thing,
                orbit,
                radius_km: 40_000.0,
                phase_rad: rng.next() * TAU,
            })
            .collect();
        let spots: Vec<Spot> = things
            .iter()
            .map(|&thing| {
                let pos = Vec2::new(rng.next() as f32, rng.next() as f32) * 20_000.0;
                spot(thing, pos)
            })
            .collect();
        COMPARED.with(|compared| compared.set(0));
        let sets = together_sets(&slots, 10.0);
        let (clusters, latch) = overlap_clusters(&spots, &OverlapLatch::default(), 24.0, 28.0);
        overlap_clusters(&spots, &latch, 24.0, 28.0);
        let compared = COMPARED.with(std::cell::Cell::get);
        assert!(!sets.is_empty() && clusters.len() < count, "some link");
        assert!(compared > 0, "the comparisons are counted");
        assert!(compared < 10 * count, "{compared} comparisons");
    }
}

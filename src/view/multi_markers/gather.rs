//! Which multi-markers there are this frame (MUL-03 to MUL-06, GRP-05):
//! things together in one named orbit, symbols that overlap on screen,
//! and the satellites folded into each parent. Writes `MultiMarkers` and
//! each thing's `JoinSlide` (`view/multi.rs`).
//!
//! **Together** comes from game state: ships, groups and stations whose
//! separation in one named orbit is under `together_km` (PHAS-02) are one
//! marker at every zoom, a station marker when a station is among them.
//! **Overlap** comes from the screen: two symbols closer than
//! `cluster_join_px` join, and part only beyond `cluster_split_px`, so a
//! pair at the edge doesn't flicker. **Fold** comes from `grouping.rs`.
//! The rules are plain functions here, tested without a camera; the
//! system gathers what they need.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::look::{MultiLook, count_text, multi_frames, station_frames};
use crate::sim::body::{Affiliation, BodyClass};
use crate::sim::named_orbit::{InNamedOrbit, NamedOrbit};
use crate::sim::orbit::Orbit;
use crate::sim::ship::InFlight;
use crate::sim::track::ShipRecord;
use crate::sim::transfer::phase::{gap_rad, together};
use crate::sim::tunables::SimTunables;
use crate::view::anim::{advance_towards, step_per_frame};
use crate::view::fold::{FoldSummary, SatelliteFoldState};
use crate::view::markers::{marker_alpha, marker_position, parent_clear_px, symbol_settings};
use crate::view::motion_mode::MotionModeState;
use crate::view::multi::{JoinSlide, MultiKey, MultiMarker, MultiMarkers};
use crate::view::screen::{MapView, ScreenPositions, snap_to_pixel};
use crate::view::symbols::symbol_look;
use crate::view::true_size::DiscRadii;
use crate::view::tunables::Tunables;

/// Below this strength a symbol isn't drawn, so it can't overlap another
/// (the markers' own cut-off).
const MIN_VISIBLE_ALPHA: f32 = 0.02;

/// A thing in a named orbit, for finding what is together (PHAS-02).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitSlot {
    pub thing: Entity,
    pub orbit: Entity,
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

/// Groups `count` items by a pairwise link: every item linked to another
/// by a chain of links is in one group. Groups and their items keep the
/// items' order.
fn link_groups(count: usize, linked: impl Fn(usize, usize) -> bool) -> Vec<Vec<usize>> {
    let mut parents: Vec<usize> = (0..count).collect();
    for a in 0..count {
        for b in a + 1..count {
            if linked(a, b) {
                let (ra, rb) = (root(&mut parents, a), root(&mut parents, b));
                parents[ra.max(rb)] = ra.min(rb);
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut group_of: HashMap<usize, usize> = HashMap::new();
    for i in 0..count {
        let r = root(&mut parents, i);
        let index = *group_of.entry(r).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[index].push(i);
    }
    groups
}

/// MUL-05 / PHAS-02: the sets of two or more things together in one named
/// orbit: each thing closer than `together_km` to another of the set, in
/// a chain. Things keep their order within a set.
pub fn together_sets(slots: &[OrbitSlot], together_km: f64) -> Vec<Vec<Entity>> {
    link_groups(slots.len(), |a, b| {
        let (a, b) = (slots[a], slots[b]);
        a.orbit == b.orbit
            && together(
                a.radius_km,
                gap_rad(a.phase_rad, b.phase_rad).min(gap_rad(b.phase_rad, a.phase_rad)),
                together_km,
            )
    })
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

/// A pair of symbols, in a fixed order, for the overlap latch.
fn pair(a: Entity, b: Entity) -> (Entity, Entity) {
    if a < b { (a, b) } else { (b, a) }
}

/// MUL-05: the overlap clusters among symbols at `units` (each named by
/// its lead thing): every symbol joined to another by a chain of joined
/// pairs, with the hysteresis of `next_joined` read from `latch`, last
/// frame's joined pairs. Returns the clusters, as indices into `units`
/// (lone symbols as clusters of one), and this frame's joined pairs.
pub fn overlap_clusters(
    units: &[(Entity, Vec2)],
    latch: &HashSet<(Entity, Entity)>,
    join_px: f32,
    split_px: f32,
) -> (Vec<Vec<usize>>, HashSet<(Entity, Entity)>) {
    let mut joined = HashSet::new();
    for a in 0..units.len() {
        for b in a + 1..units.len() {
            let key = pair(units[a].0, units[b].0);
            let distance = units[a].1.distance(units[b].1);
            if next_joined(distance, latch.contains(&key), join_px, split_px) {
                joined.insert(key);
            }
        }
    }
    let clusters = link_groups(units.len(), |a, b| {
        joined.contains(&pair(units[a].0, units[b].0))
    });
    (clusters, joined)
}

/// MUL-06 / GRP-05: where a fold marker's front frame is centred: level
/// with its parent's marker and beside it, its box (HOV-07) clear of the
/// parent's box, which reaches `parent_clear` from the parent's centre.
pub fn fold_marker_centre(parent: Vec2, parent_clear: f32, half: f32, margin: f32) -> Vec2 {
    parent + Vec2::new(parent_clear + margin + half, 0.0)
}

/// Last frame's joined pairs of symbols (MUL-05).
#[derive(Resource, Default, Debug)]
pub struct ClusterLatch(pub HashSet<(Entity, Entity)>);

/// What the gathering reads of each thing.
type ThingQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static BodyClass,
        &'static Affiliation,
        Option<&'static ShipRecord>,
        Option<&'static Orbit>,
        Option<&'static InNamedOrbit>,
        Option<&'static ChildOf>,
        Option<&'static SatelliteFoldState>,
        Option<&'static MotionModeState>,
        Has<InFlight>,
    ),
>;

/// The things and the state that decides how they gather.
#[derive(SystemParam)]
pub struct GatherInputs<'w, 's> {
    things: ThingQuery<'w, 's>,
    named: Query<'w, 's, &'static NamedOrbit>,
    screen: Res<'w, ScreenPositions>,
    discs: Res<'w, DiscRadii>,
    folds: Res<'w, FoldSummary>,
    sim: Res<'w, SimTunables>,
    tunables: Res<'w, Tunables>,
    map: MapView<'w, 's>,
}

/// A ship, group or station as the gathering sees it this frame.
#[derive(Debug, Clone, Copy)]
struct Symbol {
    thing: Entity,
    station: bool,
    /// Where its own symbol is drawn, sliding with its fold (GRP-03).
    pos: Option<Vec2>,
    /// How strongly its own symbol is drawn, before any join.
    alpha: f32,
    /// Folded into its parent (the latch, GRP-01), or sliding there.
    folded: bool,
    in_flight: bool,
    /// Its fold slide has run its course.
    settled: bool,
}

/// A symbol on screen that can overlap others: a lone thing, a station
/// with what is docked at it, or things together.
#[derive(Debug, Clone)]
struct Unit {
    lead: Entity,
    station: Option<Entity>,
    /// The things it counts: the station alone, or every thing together.
    things: Vec<Entity>,
    docked: Vec<Entity>,
    pos: Vec2,
    alpha: f32,
    visible: bool,
}

impl GatherInputs<'_, '_> {
    /// Every ship, group and station.
    fn symbols(&self) -> Vec<Symbol> {
        let mut symbols: Vec<Symbol> = self
            .things
            .iter()
            .filter(|(_, _, _, record, ..)| record.is_some())
            .map(
                |(thing, class, _, _, orbit, _, child_of, fold, motion, flying)| {
                    let fold_t = fold.map_or(1.0, |f| f.anim_t);
                    let parent = child_of.and_then(|c| self.screen.get(c.parent()));
                    let pos = self
                        .screen
                        .get(thing)
                        .map(|own| marker_position(own, parent, fold_t));
                    Symbol {
                        thing,
                        station: *class == BodyClass::Station,
                        pos,
                        alpha: marker_alpha(
                            fold_t,
                            fold.is_some_and(|f| f.effective_hidden),
                            motion.map_or(1.0, MotionModeState::dot_alpha),
                        ),
                        folded: fold.is_some_and(|f| f.folded || f.effective_hidden),
                        in_flight: flying || orbit.is_none(),
                        settled: fold_t >= 1.0,
                    }
                },
            )
            .collect();
        symbols.sort_by_key(|s| s.thing);
        symbols
    }

    /// MUL-05: the sets of things together, from where each sits in its
    /// named orbit.
    fn together(&self) -> Vec<Vec<Entity>> {
        let mut slots: Vec<OrbitSlot> = self
            .things
            .iter()
            .filter(|(_, _, _, record, _, _, _, _, _, flying)| record.is_some() && !flying)
            .filter_map(|(thing, _, _, _, orbit, in_orbit, ..)| {
                let (orbit, in_orbit) = (orbit?, in_orbit?);
                Some(OrbitSlot {
                    thing,
                    orbit: in_orbit.0,
                    radius_km: self
                        .named
                        .get(in_orbit.0)
                        .map_or(orbit.radius_km, |n| n.radius_km),
                    phase_rad: orbit.phase0_rad,
                })
            })
            .collect();
        slots.sort_by_key(|s| s.thing);
        together_sets(&slots, self.sim.together_km)
    }

    fn class_and_affiliation(&self, thing: Entity) -> Option<(BodyClass, Affiliation)> {
        let (_, class, affiliation, ..) = self.things.get(thing).ok()?;
        Some((*class, *affiliation))
    }

    /// MUL-02: the frames and count of a marker holding `things`. What is
    /// docked at a station among them is listed under the station and,
    /// like the count, the frames leave it out (MUL-04).
    fn look(&self, things: &[Entity]) -> MultiLook {
        let looks: Vec<(BodyClass, Affiliation)> = things
            .iter()
            .filter_map(|&t| self.class_and_affiliation(t))
            .collect();
        let (front, back) = multi_frames(&looks, symbol_settings(&self.tunables));
        MultiLook {
            front,
            back,
            count: count_text(things.len()),
        }
    }

    /// MUL-03: a station marker's look: the station's own frame, and the
    /// back frame over what is docked.
    fn station_look(&self, station: Entity, docked: &[Entity]) -> Option<MultiLook> {
        let (_, _, affiliation, record, ..) = self.things.get(station).ok()?;
        let settings = symbol_settings(&self.tunables);
        let symbol = symbol_look(*affiliation, record?, settings);
        let docked_looks: Vec<(BodyClass, Affiliation)> = docked
            .iter()
            .filter_map(|&t| self.class_and_affiliation(t))
            .collect();
        let (front, back) = station_frames(&symbol, &docked_looks, settings);
        Some(MultiLook {
            front,
            back,
            count: count_text(docked.len()),
        })
    }

    fn in_viewport(&self, pos: Vec2) -> bool {
        self.screen.in_viewport(pos)
    }

    /// GRP-05 / MUL-06: where a parent's fold marker goes this frame, on
    /// a whole pixel, while the parent is in the viewport.
    fn fold_centre(&self, parent: Entity) -> Option<Vec2> {
        let pos = self.screen.get(parent).filter(|p| self.in_viewport(*p))?;
        let (_, class, _, record, ..) = self.things.get(parent).ok()?;
        let tunables = &self.tunables;
        let clear = parent_clear_px(*class, record.is_some(), self.discs.get(parent), tunables);
        let centre = fold_marker_centre(
            pos,
            clear,
            tunables.ship_symbol_px / 2.0,
            tunables.bracket_margin_px,
        );
        Some(snap_to_pixel(centre, self.map.scale_factor()))
    }
}

/// MUL-05: the units that can overlap, from the symbols and the sets of
/// things together. Things folded into a parent, or in flight, are in
/// none; a set together is one unit led by its station, if it has one.
fn units(
    symbols: &[Symbol],
    sets: &[Vec<Entity>],
    in_viewport: impl Fn(Vec2) -> bool,
) -> Vec<Unit> {
    let by_thing: HashMap<Entity, &Symbol> = symbols.iter().map(|s| (s.thing, s)).collect();
    let in_set: HashSet<Entity> = sets.iter().flatten().copied().collect();
    let mut units = Vec::new();
    let mut push = |members: Vec<&Symbol>| {
        if members.iter().any(|s| s.folded || s.in_flight) {
            return;
        }
        let station = members.iter().find(|s| s.station).map(|s| s.thing);
        let lead = station.unwrap_or(members[0].thing);
        // Off screen or behind the camera, things together still hold
        // each other; they just aren't drawn.
        let pos = by_thing.get(&lead).and_then(|s| s.pos);
        let alpha = members.iter().map(|s| s.alpha).fold(0.0, f32::max);
        let (things, docked) = match station {
            Some(station) => (
                vec![station],
                members
                    .iter()
                    .map(|s| s.thing)
                    .filter(|&t| t != station)
                    .collect(),
            ),
            None => (members.iter().map(|s| s.thing).collect(), Vec::new()),
        };
        units.push(Unit {
            lead,
            station,
            things,
            docked,
            pos: pos.unwrap_or_default(),
            alpha,
            visible: alpha > MIN_VISIBLE_ALPHA && pos.is_some_and(&in_viewport),
        });
    };
    for set in sets {
        push(
            set.iter()
                .filter_map(|t| by_thing.get(t).copied())
                .collect(),
        );
    }
    for symbol in symbols.iter().filter(|s| !in_set.contains(&s.thing)) {
        push(vec![symbol]);
    }
    units
}

/// MUL-03 / MUL-05: the markers made from clusters of units: a lone unit
/// is a station marker with things docked, a together marker of two or
/// more things, or nothing; two or more units are an overlap marker
/// centred on the mean of their centres.
fn cluster_markers(
    units: &[Unit],
    clusters: &[Vec<usize>],
    inputs: &GatherInputs,
) -> Vec<MultiMarker> {
    let scale = inputs.map.scale_factor();
    let mut markers = Vec::new();
    for cluster in clusters {
        let members: Vec<&Unit> = cluster.iter().map(|&i| &units[i]).collect();
        let alpha = members.iter().map(|u| u.alpha).fold(0.0, f32::max);
        let docked: Vec<(Entity, Vec<Entity>)> = members
            .iter()
            .filter_map(|u| Some((u.station?, u.docked.clone())))
            .filter(|(_, d)| !d.is_empty())
            .collect();
        let (key, things, centre, drawn) = match members.as_slice() {
            [unit] => {
                let key = match unit.station {
                    Some(station) if !unit.docked.is_empty() => MultiKey::Station(station),
                    None if unit.things.len() > 1 => MultiKey::Together(unit.lead),
                    _ => continue,
                };
                let centre = snap_to_pixel(unit.pos, scale);
                (key, unit.things.clone(), centre, unit.visible)
            }
            _ => {
                let mean = members.iter().map(|u| u.pos).sum::<Vec2>() / members.len() as f32;
                let lead = members
                    .iter()
                    .map(|u| u.lead)
                    .min()
                    .unwrap_or(members[0].lead);
                let things = members.iter().flat_map(|u| u.things.clone()).collect();
                (
                    MultiKey::Overlap(lead),
                    things,
                    snap_to_pixel(mean, scale),
                    true,
                )
            }
        };
        let look = match key {
            MultiKey::Station(station) => inputs.station_look(station, &docked[0].1),
            _ => Some(inputs.look(&things)),
        };
        let Some(look) = look else {
            continue;
        };
        markers.push(MultiMarker {
            key,
            centre,
            things,
            docked,
            look,
            alpha,
            drawn,
        });
    }
    markers
}

/// GRP-05 / MUL-04: each parent's fold marker, holding its folded
/// satellites. A thing docked at a station folded with it is listed under
/// the station and not counted.
fn fold_markers(inputs: &GatherInputs, sets: &[Vec<Entity>]) -> Vec<MultiMarker> {
    let station_of = |thing: Entity| -> Option<(Entity, &Vec<Entity>)> {
        let set = sets.iter().find(|set| set.contains(&thing))?;
        let station = set.iter().copied().find(|&t| {
            inputs
                .class_and_affiliation(t)
                .is_some_and(|(class, _)| class == BodyClass::Station)
        })?;
        Some((station, set))
    };
    let mut parents: Vec<Entity> = inputs.folds.0.keys().copied().collect();
    parents.sort();
    let mut markers = Vec::new();
    for parent in parents {
        let mut members = inputs.folds.0[&parent].members.clone();
        if members.is_empty() {
            continue;
        }
        members.sort();
        let mut things = Vec::new();
        let mut docked: Vec<(Entity, Vec<Entity>)> = Vec::new();
        for &member in &members {
            match station_of(member) {
                Some((station, _)) if station != member && members.contains(&station) => {}
                Some((station, set)) if station == member => {
                    things.push(member);
                    let at: Vec<Entity> = set
                        .iter()
                        .copied()
                        .filter(|&t| t != station && members.contains(&t))
                        .collect();
                    if !at.is_empty() {
                        docked.push((station, at));
                    }
                }
                _ => things.push(member),
            }
        }
        let centre = inputs.fold_centre(parent);
        let look = inputs.look(&things);
        markers.push(MultiMarker {
            key: MultiKey::Fold(parent),
            centre: centre.unwrap_or_default(),
            things,
            docked,
            look,
            alpha: 1.0,
            drawn: centre.is_some(),
        });
    }
    markers
}

/// MUL-05: each unit's cluster: the units drawn on screen clustered by
/// overlap, with `latch` (last frame's joined pairs) updated; units off
/// screen or faded out each alone.
fn clusters_of(
    units: &[Unit],
    latch: &mut HashSet<(Entity, Entity)>,
    tunables: &Tunables,
) -> Vec<Vec<usize>> {
    let visible: Vec<usize> = (0..units.len()).filter(|&i| units[i].visible).collect();
    let spots: Vec<(Entity, Vec2)> = visible
        .iter()
        .map(|&i| (units[i].lead, units[i].pos))
        .collect();
    let (clusters, joined) = overlap_clusters(
        &spots,
        latch,
        tunables.cluster_join_px,
        tunables.cluster_split_px,
    );
    *latch = joined;
    let mut all: Vec<Vec<usize>> = clusters
        .into_iter()
        .map(|c| c.into_iter().map(|i| visible[i]).collect())
        .collect();
    all.extend(
        (0..units.len())
            .filter(|i| !units[*i].visible)
            .map(|i| vec![i]),
    );
    all
}

/// MUL-05: the things drawn into a marker rather than as their own
/// symbols: everything in a together or overlap marker, and what is
/// docked at a station marker's station. A station marker's station is
/// its own marker, and folded satellites are the fold's business.
pub fn joined_things(markers: &[MultiMarker]) -> HashMap<Entity, MultiKey> {
    let mut joined = HashMap::new();
    for marker in markers {
        match marker.key {
            MultiKey::Fold(_) => {}
            MultiKey::Station(_) => {
                joined.extend(marker.all_things().skip(1).map(|t| (t, marker.key)));
            }
            _ => joined.extend(marker.all_things().map(|t| (t, marker.key))),
        }
    }
    joined
}

/// MUL-05: a thing's slide after one frame of `step`: towards the marker
/// that holds it, `offset` from its own place, or back out when nothing
/// does. A thing seen for the first time (`slide` is `None`), or whose
/// fold is still sliding (`settled` false), takes its state at once, so
/// it never slides two ways.
pub fn next_slide(
    slide: Option<JoinSlide>,
    offset: Option<Vec2>,
    settled: bool,
    step: f32,
) -> JoinSlide {
    let held = offset.is_some();
    let target = if held { 1.0 } else { 0.0 };
    match slide {
        Some(slide) if settled => JoinSlide {
            t: advance_towards(slide.t, target, step),
            offset: offset.unwrap_or(slide.offset),
            held,
        },
        _ => JoinSlide {
            t: target,
            offset: offset.or(slide.map(|s| s.offset)).unwrap_or_default(),
            held,
        },
    }
}

/// MUL-05: slide each ship, group and station towards the marker that
/// holds it, or back out, over `fold_unfold_anim_ms`.
fn slide_things(
    commands: &mut Commands,
    symbols: &[Symbol],
    (markers, joined): (&[MultiMarker], &HashMap<Entity, MultiKey>),
    step: f32,
    slides: &mut Query<&mut JoinSlide>,
) {
    for symbol in symbols {
        let holder = joined
            .get(&symbol.thing)
            .and_then(|key| markers.iter().find(|m| m.key == *key));
        // A thing with no place on screen keeps its offset, but is held.
        let offset = holder.map(|m| symbol.pos.map_or(Vec2::ZERO, |pos| m.centre - pos));
        match slides.get_mut(symbol.thing) {
            Ok(mut slide) => {
                let next = next_slide(Some(*slide), offset, symbol.settled, step);
                slide.set_if_neq(next);
            }
            Err(_) => {
                let next = next_slide(None, offset, symbol.settled, step);
                commands.entity(symbol.thing).insert(next);
            }
        }
    }
}

/// MUL-03 to MUL-06 / GRP-05: gather this frame's multi-markers and slide
/// each thing into or out of the marker that holds it.
pub fn gather_multi_markers(
    mut commands: Commands,
    time: Res<Time>,
    inputs: GatherInputs,
    mut latch: ResMut<ClusterLatch>,
    mut multis: ResMut<MultiMarkers>,
    mut slides: Query<&mut JoinSlide>,
) {
    let symbols = inputs.symbols();
    let sets = inputs.together();
    let units = units(&symbols, &sets, |pos| inputs.in_viewport(pos));
    let clusters = clusters_of(&units, &mut latch.0, &inputs.tunables);
    let mut markers = cluster_markers(&units, &clusters, &inputs);
    markers.extend(fold_markers(&inputs, &sets));
    let joined = joined_things(&markers);
    let step = step_per_frame(time.delta_secs(), inputs.tunables.fold_unfold_anim_ms);
    slide_things(
        &mut commands,
        &symbols,
        (&markers, &joined),
        step,
        &mut slides,
    );
    multis.markers = markers;
    multis.joined = joined;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entities(n: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..n).map(|_| world.spawn_empty().id()).collect()
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
    }

    /// MUL-05: symbols join below `cluster_join_px`, part only above
    /// `cluster_split_px`, and keep their state between the two.
    #[test]
    fn two_symbols_join_below_the_join_distance_and_part_above_the_split() {
        let t = Tunables::default();
        let (join, split) = (t.cluster_join_px, t.cluster_split_px);
        assert!(join < split);
        assert!(next_joined(join - 0.1, false, join, split));
        assert!(!next_joined(split + 0.1, true, join, split));
        let between = (join + split) / 2.0;
        assert!(next_joined(between, true, join, split));
        assert!(!next_joined(between, false, join, split));

        let e = entities(2);
        let at = |d: f32| [(e[0], Vec2::ZERO), (e[1], Vec2::new(d, 0.0))];
        let mut latch = HashSet::new();
        let mut clusters_at = |d: f32| {
            let (clusters, joined) = overlap_clusters(&at(d), &latch, join, split);
            latch = joined;
            clusters.len()
        };
        assert_eq!(clusters_at(between), 2, "apart, and not yet joined");
        assert_eq!(clusters_at(join - 1.0), 1, "joined");
        assert_eq!(clusters_at(between), 1, "stays joined in the band");
        assert_eq!(clusters_at(split + 1.0), 2, "parted");
        assert_eq!(clusters_at(between), 2, "stays parted in the band");
    }

    /// MUL-05: a cluster is every symbol joined to another by a chain of
    /// joined pairs, so three in a row 20 px apart are one.
    #[test]
    fn a_chain_of_joined_pairs_is_one_cluster() {
        let e = entities(4);
        let units = [
            (e[0], Vec2::new(0.0, 0.0)),
            (e[1], Vec2::new(20.0, 0.0)),
            (e[2], Vec2::new(40.0, 0.0)),
            (e[3], Vec2::new(200.0, 0.0)),
        ];
        let (clusters, _) = overlap_clusters(&units, &HashSet::new(), 24.0, 28.0);
        assert_eq!(clusters, vec![vec![0, 1, 2], vec![3]]);
    }

    /// GRP-05 / MUL-06: a fold marker sits level with its parent and
    /// beside it, its box clear of the parent's box.
    #[test]
    fn a_fold_marker_sits_beside_its_parent_clear_of_its_box() {
        let t = Tunables::default();
        let half = t.ship_symbol_px / 2.0;
        let margin = t.bracket_margin_px;
        let clear = 13.0;
        let centre = fold_marker_centre(Vec2::new(100.0, 50.0), clear, half, margin);
        assert_eq!(centre.y, 50.0);
        assert_eq!(
            centre.x - half - margin,
            100.0 + clear,
            "boxes touch, no overlap"
        );
    }

    /// MUL-05: a held thing slides into its marker a step at a time and
    /// back out when let go, from where the marker was; one seen for the
    /// first time, or still sliding with its fold, takes its state at
    /// once.
    #[test]
    fn a_thing_slides_into_its_marker_and_back_out() {
        let offset = Vec2::new(6.0, -2.0);
        let start = JoinSlide::default();
        let joining = next_slide(Some(start), Some(offset), true, 0.25);
        assert_eq!(
            joining,
            JoinSlide {
                t: 0.25,
                offset,
                held: true
            }
        );
        let parting = next_slide(Some(joining), None, true, 0.25);
        assert_eq!(
            (parting.t, parting.offset, parting.held),
            (0.0, offset, false)
        );
        assert_eq!(next_slide(None, Some(offset), true, 0.25).t, 1.0);
        assert_eq!(next_slide(Some(start), Some(offset), false, 0.25).t, 1.0);
    }

    /// MUL-05: the things of a together or overlap marker are joined into
    /// it; a station marker joins what is docked but not the station, and
    /// a fold marker joins nothing.
    #[test]
    fn joined_things_are_what_markers_draw_in_place_of_symbols() {
        let e = entities(6);
        let marker = |key, things: Vec<Entity>, docked| MultiMarker {
            key,
            centre: Vec2::ZERO,
            things,
            docked,
            look: crate::view::multi_markers::draw::blank_look(),
            alpha: 1.0,
            drawn: true,
        };
        let markers = [
            marker(
                MultiKey::Station(e[0]),
                vec![e[0]],
                vec![(e[0], vec![e[1]])],
            ),
            marker(MultiKey::Together(e[2]), vec![e[2], e[3]], vec![]),
            marker(MultiKey::Fold(e[5]), vec![e[4]], vec![]),
        ];
        let joined = joined_things(&markers);
        assert_eq!(joined.get(&e[1]), Some(&MultiKey::Station(e[0])));
        assert!(!joined.contains_key(&e[0]));
        assert_eq!(joined.get(&e[3]), Some(&MultiKey::Together(e[2])));
        assert!(!joined.contains_key(&e[4]));
    }

    fn symbol(thing: Entity, station: bool) -> Symbol {
        Symbol {
            thing,
            station,
            pos: Some(Vec2::new(10.0, 10.0)),
            alpha: 1.0,
            folded: false,
            in_flight: false,
            settled: true,
        }
    }

    /// MUL-05 / GRP-08: a ship in flight is never a symbol that joins
    /// another, and a folded thing is in its parent's fold marker instead.
    #[test]
    fn ships_in_flight_and_folded_things_never_join() {
        let e = entities(3);
        let symbols = [
            symbol(e[0], false),
            Symbol {
                in_flight: true,
                ..symbol(e[1], false)
            },
            Symbol {
                folded: true,
                ..symbol(e[2], false)
            },
        ];
        let units = units(&symbols, &[], |_| true);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].lead, e[0]);
    }

    /// MUL-03 / MUL-05: things together are one symbol on screen: led by
    /// their station, which counts alone with the rest docked at it, or by
    /// the first of them when there is no station. Off screen they still
    /// hold together, just not drawn.
    #[test]
    fn things_together_are_one_unit_led_by_their_station() {
        let e = entities(5);
        let symbols = [
            symbol(e[0], false),
            symbol(e[1], true),
            symbol(e[2], false),
            symbol(e[3], false),
            Symbol {
                pos: None,
                ..symbol(e[4], false)
            },
        ];
        let sets = [vec![e[0], e[1], e[2]], vec![e[3], e[4]]];
        let units = units(&symbols, &sets, |_| true);
        assert_eq!(units.len(), 2);
        assert_eq!(
            (units[0].lead, &units[0].things, &units[0].docked),
            (e[1], &vec![e[1]], &vec![e[0], e[2]])
        );
        assert_eq!(units[1].things, [e[3], e[4]]);
        assert!(units[1].visible, "led by a thing on screen");
        let drawn = set_drawn(&symbols, &[vec![e[4], e[3]]]);
        assert!(!drawn, "led by a thing off screen");
    }

    /// Whether the unit of things together made of `sets` is drawn.
    fn set_drawn(symbols: &[Symbol], sets: &[Vec<Entity>]) -> bool {
        units(symbols, sets, |_| true)
            .iter()
            .find(|u| u.things.len() > 1)
            .is_some_and(|u| u.visible)
    }
}

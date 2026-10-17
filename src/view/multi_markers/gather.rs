//! Which multi-markers there are this frame (MUL-03 to MUL-06, GRP-12):
//! things together in one named orbit, symbols that overlap on screen,
//! and the ships, groups and stations folded into each parent, drawn as
//! a pip or popped out as `pop.rs` says. Writes `MultiMarkers` and
//! each thing's `JoinSlide` (`view/multi.rs`).
//!
//! **Together** comes from game state: ships, groups and stations whose
//! separation in one named orbit is under `together_km` (PHAS-02) are one
//! marker at every zoom, a station marker when a station is among them.
//! **Overlap** comes from the screen: two symbols closer than
//! `cluster_join_px` join, and part only beyond `cluster_split_px`, so a
//! pair at the edge doesn't flicker. **Fold** comes from `grouping.rs`.
//! The rules are plain functions, tested without a camera (`link.rs`
//! links things and symbols); the system gathers what they need.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::link::{OrbitSlot, OverlapLatch, Spot, overlap_clusters, sole_station, together_sets};
use super::look::MultiLook;
use super::pop::{ObjectGeometry, ObjectPops, object_geometry};
use crate::sim::body::{Affiliation, BodyClass};
use crate::sim::named_orbit::{InNamedOrbit, NamedOrbit};
use crate::sim::orbit::Orbit;
use crate::sim::ship::InFlight;
use crate::sim::track::ShipRecord;
use crate::sim::tunables::SimTunables;
use crate::view::anim::{advance_towards, step_per_frame};
use crate::view::fold::{FoldSummary, SatelliteFoldState};
use crate::view::markers::{marker_alpha, marker_position, parent_clear_px, symbol_settings};
use crate::view::motion_mode::MotionModeState;
use crate::view::multi::{JoinSlide, MultiKey, MultiMarker, MultiMarkers, back_offset};
use crate::view::screen::{MapView, ScreenPositions, snap_to_pixel};
use crate::view::symbols::symbol_look;
use crate::view::true_size::DiscRadii;
use crate::view::tunables::Tunables;

/// Below this strength a symbol isn't drawn, so it can't overlap another
/// (the markers' own cut-off).
const MIN_VISIBLE_ALPHA: f32 = 0.02;

/// MUL-06 / SYM-16: where an overlap cluster's marker is centred: the
/// mean of its symbols' centres, on a whole physical pixel (`scale`
/// physical pixels per logical one).
pub fn overlap_centre(centres: &[Vec2], scale: f32) -> Vec2 {
    let mean = centres.iter().sum::<Vec2>() / centres.len().max(1) as f32;
    snap_to_pixel(mean, scale)
}

/// Last frame's overlaps, for the hysteresis (MUL-05).
#[derive(Resource, Default, Debug)]
pub struct ClusterLatch(pub OverlapLatch);

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
    pops: Res<'w, ObjectPops>,
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
    /// named orbit. The radius is the orbit's, one for all its members;
    /// a thing whose orbit can't be found is together with nothing.
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
                    radius_km: self.named.get(in_orbit.0).ok()?.radius_km,
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
    /// docked at a station among them is listed under the station and not
    /// counted (MUL-04), but counts for the frames, so a hostile ship
    /// docked there still shows.
    fn look(&self, things: &[Entity], docked: &[(Entity, Vec<Entity>)]) -> MultiLook {
        let looks: Vec<(BodyClass, Affiliation)> = things
            .iter()
            .chain(docked.iter().flat_map(|(_, at)| at))
            .filter_map(|&t| self.class_and_affiliation(t))
            .collect();
        MultiLook::of_things(&looks, things.len(), symbol_settings(&self.tunables))
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
        Some(MultiLook::of_station(
            &symbol,
            &docked_looks,
            docked.len(),
            settings,
        ))
    }

    fn in_viewport(&self, pos: Vec2) -> bool {
        self.screen.in_viewport(pos)
    }

    /// GRP-12 / GRP-13 / MUL-06: where a parent's object marker holding
    /// `count` things is drawn this frame and how big, as far as it has
    /// popped out, its centre on a whole pixel; and how strongly: as its
    /// parent's marker is (MOT-11), faded in as it appears (GRP-03). `None`
    /// while the parent's marker isn't drawn.
    fn fold_geometry(&self, parent: Entity, count: usize) -> Option<(ObjectGeometry, f32)> {
        let pos = self.screen.get(parent).filter(|p| self.in_viewport(*p))?;
        let (_, class, _, record, _, _, _, fold, motion, _) = self.things.get(parent).ok()?;
        let tunables = &self.tunables;
        let parent_alpha = marker_alpha(
            fold.map_or(1.0, |f| f.anim_t),
            fold.is_some_and(|f| f.effective_hidden),
            motion.map_or(1.0, MotionModeState::dot_alpha),
        );
        let h = parent_clear_px(*class, record.is_some(), self.discs.get(parent), tunables);
        let pop = self.pops.of(parent);
        let mut geometry = object_geometry(pos, h, (pop.t, count), tunables);
        geometry.centre = snap_to_pixel(geometry.centre, self.map.scale_factor());
        let alpha = parent_alpha * pop.shown;
        (alpha > MIN_VISIBLE_ALPHA).then_some((geometry, alpha))
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
        let station = sole_station(members.iter().filter(|s| s.station).map(|s| s.thing));
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
                let centres: Vec<Vec2> = members.iter().map(|u| u.pos).collect();
                let lead = members
                    .iter()
                    .map(|u| u.lead)
                    .min()
                    .unwrap_or(members[0].lead);
                let things = members.iter().flat_map(|u| u.things.clone()).collect();
                (
                    MultiKey::Overlap(lead),
                    things,
                    overlap_centre(&centres, scale),
                    true,
                )
            }
        };
        let look = match key {
            MultiKey::Station(station) => inputs.station_look(station, &docked[0].1),
            _ => Some(inputs.look(&things, &docked)),
        };
        let Some(look) = look else {
            continue;
        };
        let tunables = &inputs.tunables;
        markers.push(MultiMarker {
            key,
            centre,
            frame_px: tunables.ship_symbol_px,
            back: Some(back_offset(tunables.back_frame_offset_px)),
            count_shown: true,
            things,
            docked,
            look,
            alpha,
            drawn,
        });
    }
    markers
}

/// GRP-12 / MUL-04: each parent's fold marker, holding its folded ships,
/// groups and stations; its folded natural bodies are its moon ring's. A
/// thing docked at a station folded with it is listed under the station
/// and not counted. It is drawn as a pip or popped out (GRP-13).
fn fold_markers(inputs: &GatherInputs, sets: &[Vec<Entity>]) -> Vec<MultiMarker> {
    // Each thing together with a station, by that station's set.
    let mut station_of: HashMap<Entity, (Entity, &Vec<Entity>)> = HashMap::new();
    for set in sets {
        let station = sole_station(set.iter().copied().filter(|&t| {
            inputs
                .class_and_affiliation(t)
                .is_some_and(|(class, _)| class == BodyClass::Station)
        }));
        if let Some(station) = station {
            station_of.extend(set.iter().map(|&t| (t, (station, set))));
        }
    }
    let mut parents: Vec<Entity> = inputs.folds.0.keys().copied().collect();
    parents.sort();
    let mut markers = Vec::new();
    for parent in parents {
        let mut members = inputs.folds.0[&parent].objects.clone();
        if members.is_empty() {
            continue;
        }
        members.sort();
        let folded: HashSet<Entity> = members.iter().copied().collect();
        let mut things = Vec::new();
        let mut docked: Vec<(Entity, Vec<Entity>)> = Vec::new();
        for &member in &members {
            match station_of.get(&member) {
                Some(&(station, _)) if station != member && folded.contains(&station) => {}
                Some(&(station, set)) if station == member => {
                    things.push(member);
                    let at: Vec<Entity> = set
                        .iter()
                        .copied()
                        .filter(|&t| t != station && folded.contains(&t))
                        .collect();
                    if !at.is_empty() {
                        docked.push((station, at));
                    }
                }
                _ => things.push(member),
            }
        }
        let drawn = inputs.fold_geometry(parent, things.len());
        let look = inputs.look(&things, &docked);
        let geometry = drawn.map_or(
            object_geometry(Vec2::ZERO, 0.0, (0.0, things.len()), &inputs.tunables),
            |(geometry, _)| geometry,
        );
        markers.push(MultiMarker {
            key: MultiKey::Fold(parent),
            centre: geometry.centre,
            frame_px: geometry.frame_px,
            back: geometry.back,
            count_shown: geometry.count_shown,
            things,
            docked,
            look,
            alpha: drawn.map_or(0.0, |(_, alpha)| alpha),
            drawn: drawn.is_some(),
        });
    }
    markers
}

/// MUL-05: each unit's cluster: the units drawn on screen clustered by
/// overlap, with `latch` (last frame's overlaps) updated; units off
/// screen or faded out each alone.
fn clusters_of(units: &[Unit], latch: &mut OverlapLatch, tunables: &Tunables) -> Vec<Vec<usize>> {
    let visible: Vec<usize> = (0..units.len()).filter(|&i| units[i].visible).collect();
    let spots: Vec<Spot> = visible
        .iter()
        .map(|&i| {
            let unit = &units[i];
            Spot {
                lead: unit.lead,
                things: unit.things.iter().chain(&unit.docked).copied().collect(),
                pos: unit.pos,
            }
        })
        .collect();
    let (clusters, next) = overlap_clusters(
        &spots,
        latch,
        tunables.cluster_join_px,
        tunables.cluster_split_px,
    );
    *latch = next;
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
    multis: &MultiMarkers,
    step: f32,
    slides: &mut Query<&mut JoinSlide>,
) {
    for symbol in symbols {
        let holder = multis
            .joined
            .get(&symbol.thing)
            .and_then(|key| multis.get(*key));
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
    multis.set(markers, joined);
    let step = step_per_frame(time.delta_secs(), inputs.tunables.fold_unfold_anim_ms);
    slide_things(&mut commands, &symbols, &multis, step, &mut slides);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entities(n: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..n).map(|_| world.spawn_empty().id()).collect()
    }

    /// MUL-05: two joined symbols stay joined inside the hysteresis band
    /// when one of them changes its lead: here a ship that sorts first
    /// comes together with it, and then the station it docks at leads it.
    #[test]
    fn joined_symbols_stay_joined_in_the_band_when_a_lead_changes() {
        let t = Tunables::default();
        let between = (t.cluster_join_px + t.cluster_split_px) / 2.0;
        let e = entities(4);
        let (newcomer, ship, other, station) = (e[0], e[1], e[2], e[3]);
        let unit = |lead, things: Vec<Entity>, docked: Vec<Entity>, x: f32| Unit {
            lead,
            station: None,
            things,
            docked,
            pos: Vec2::new(x, 0.0),
            alpha: 1.0,
            visible: true,
        };
        let mut latch = OverlapLatch::default();
        let near = [
            unit(ship, vec![ship], vec![], 0.0),
            unit(other, vec![other], vec![], t.cluster_join_px - 1.0),
        ];
        assert_eq!(clusters_of(&near, &mut latch, &t).len(), 1, "joined");
        let led_by_newcomer = [
            unit(newcomer, vec![newcomer, ship], vec![], 0.0),
            unit(other, vec![other], vec![], between),
        ];
        assert_eq!(
            clusters_of(&led_by_newcomer, &mut latch, &t).len(),
            1,
            "a new lead stays joined in the band"
        );
        let led_by_station = [
            Unit {
                station: Some(station),
                ..unit(station, vec![station], vec![newcomer, ship], 0.0)
            },
            unit(other, vec![other], vec![], between),
        ];
        assert_eq!(clusters_of(&led_by_station, &mut latch, &t).len(), 1);
    }

    /// MUL-06 / SYM-16: an overlap marker sits on the mean of its
    /// symbols' centres, moved to a whole physical pixel.
    #[test]
    fn an_overlap_marker_sits_on_the_mean_of_its_symbols() {
        let centres = [Vec2::new(0.0, 0.0), Vec2::new(10.0, 3.0)];
        assert_eq!(overlap_centre(&centres, 1.0), Vec2::new(5.0, 2.0));
        assert_eq!(overlap_centre(&centres, 2.0), Vec2::new(5.0, 1.5));
        let three = [Vec2::ZERO, Vec2::new(9.0, 0.0), Vec2::new(0.0, 9.0)];
        assert_eq!(overlap_centre(&three, 1.0), Vec2::new(3.0, 3.0));
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
            frame_px: 22.0,
            back: None,
            count_shown: true,
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

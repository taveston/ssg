//! The order a multi-marker lists its things in (MUL-09): the station
//! first, when there is one, then the others in the order of the inset's
//! "In orbit" outline (INS-05). A click on a marker selects the first of
//! them (MUL-07), and the tray's rows follow the same order.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::sim::body::BodyClass;
use crate::view::inset::outline::{OutlineBodies, outline_positions};
use crate::view::multi::{MultiKey, MultiMarker};

/// MUL-09: `things` in tray order: the first station among them, then
/// the rest by their place in the outline (`outline` maps a body to its
/// row with everything open), anything not in it last.
pub fn tray_order(
    things: &[Entity],
    is_station: impl Fn(Entity) -> bool,
    outline: &HashMap<Entity, usize>,
) -> Vec<Entity> {
    let station = things.iter().copied().find(|&t| is_station(t));
    let mut rest: Vec<Entity> = things
        .iter()
        .copied()
        .filter(|&t| Some(t) != station)
        .collect();
    rest.sort_by_key(|t| (outline.get(t).copied().unwrap_or(usize::MAX), *t));
    station.into_iter().chain(rest).collect()
}

/// MUL-05: whether a marker draws one tail of its own in dot mode: a
/// together or overlap marker does. A station marker's station keeps its
/// own tail, and a fold marker has none.
fn lays_a_tail(marker: &MultiMarker) -> bool {
    matches!(marker.key, MultiKey::Together(_) | MultiKey::Overlap(_))
}

/// MUL-05: the things that lay a marker's tail in dot mode: the first
/// tray row (`tray_order`) of each together or overlap marker.
pub fn tail_leads(
    markers: &[MultiMarker],
    is_station: impl Fn(Entity) -> bool,
    outline: &HashMap<Entity, usize>,
) -> HashSet<Entity> {
    markers
        .iter()
        .filter(|m| lays_a_tail(m))
        .filter_map(|m| tray_order(&m.things, &is_station, outline).first().copied())
        .collect()
}

/// What a marker's order is read from: the outline and each body's
/// class.
#[derive(SystemParam)]
pub struct TrayOrder<'w, 's> {
    outline: OutlineBodies<'w, 's>,
    classes: Query<'w, 's, &'static BodyClass>,
}

impl TrayOrder<'_, '_> {
    /// MUL-09: a marker's things in tray order. Docked things are listed
    /// under their station's row and are not among them.
    pub fn things(&self, marker: &MultiMarker) -> Vec<Entity> {
        self.sorted(&marker.things)
    }

    /// MUL-09: any things in tray order, such as those docked at a
    /// station, listed under it.
    pub fn sorted(&self, things: &[Entity]) -> Vec<Entity> {
        let positions = outline_positions(&self.outline.nodes());
        tray_order(
            things,
            |t| self.classes.get(t) == Ok(&BodyClass::Station),
            &positions,
        )
    }

    /// MUL-05: the things that lay a marker's tail in dot mode
    /// (`tail_leads`).
    pub fn tail_leads(&self, markers: &[MultiMarker]) -> HashSet<Entity> {
        if !markers.iter().any(lays_a_tail) {
            return HashSet::new();
        }
        let positions = outline_positions(&self.outline.nodes());
        tail_leads(
            markers,
            |t| self.classes.get(t) == Ok(&BodyClass::Station),
            &positions,
        )
    }

    /// MUL-07: the thing a click on a marker other than a fold marker
    /// selects: its first row.
    pub fn first(&self, marker: &MultiMarker) -> Option<Entity> {
        self.things(marker).first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MUL-05: a together or overlap marker's tail is laid by its first
    /// tray row, and follows it when the order changes; a station marker's
    /// station keeps its own tail and a fold marker has none.
    #[test]
    fn the_markers_tail_follows_its_first_tray_row() {
        let mut world = World::new();
        let [a, b, c, d, station, yard, parent] = [(); 7].map(|_| world.spawn_empty().id());
        let marker = |key, things: Vec<Entity>| MultiMarker {
            key,
            centre: Vec2::ZERO,
            frame_px: 22.0,
            back: None,
            count_shown: true,
            things,
            docked: vec![],
            look: crate::view::multi_markers::draw::blank_look(),
            alpha: 1.0,
            drawn: true,
        };
        let markers = [
            marker(MultiKey::Together(a), vec![a, b]),
            marker(MultiKey::Overlap(c), vec![c, station]),
            marker(MultiKey::Station(yard), vec![yard]),
            marker(MultiKey::Fold(parent), vec![d]),
        ];
        let is_station = |t| t == station || t == yard;
        let outline: HashMap<Entity, usize> =
            [(a, 1), (b, 2), (c, 3), (d, 0)].into_iter().collect();
        let leads = tail_leads(&markers, is_station, &outline);
        assert_eq!(leads, HashSet::from([a, station]));
        let reordered: HashMap<Entity, usize> =
            [(a, 2), (b, 1), (c, 3), (d, 0)].into_iter().collect();
        let leads = tail_leads(&markers, is_station, &reordered);
        assert_eq!(leads, HashSet::from([b, station]));
    }

    /// MUL-09: the station comes first, then the rest in outline order,
    /// whatever order they were gathered in.
    #[test]
    fn the_station_comes_first_then_the_outline_order() {
        let mut world = World::new();
        let [a, b, station, c] = [(); 4].map(|_| world.spawn_empty().id());
        let outline: HashMap<Entity, usize> =
            [(station, 9), (a, 5), (b, 2), (c, 7)].into_iter().collect();
        assert_eq!(
            tray_order(&[a, b, station, c], |t| t == station, &outline),
            [station, b, a, c]
        );
        assert_eq!(tray_order(&[c, a], |_| false, &outline), [a, c]);
    }
}

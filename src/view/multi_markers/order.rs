//! The order a multi-marker lists its things in (MUL-09): the station
//! first, when there is one, then the others in the order of the inset's
//! "In orbit" outline (INS-05). A click on a marker selects the first of
//! them (MUL-07), and the tray's rows follow the same order.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::sim::body::BodyClass;
use crate::view::inset::outline::{OutlineBodies, outline_positions};
use crate::view::multi::MultiMarker;

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
        let positions = outline_positions(&self.outline.nodes());
        tray_order(
            &marker.things,
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

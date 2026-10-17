//! Which marker an open tray belongs to as things come and go (MUL-10).
//! A together or overlap marker is keyed by one of its things, so its key
//! changes when that thing leaves or another joins ahead of it. The tray
//! therefore keeps the things it listed and, each frame, moves to the
//! marker that holds most of them, whatever that marker's key now is.

use std::cmp::Reverse;
use std::collections::HashMap;

use bevy::prelude::*;

use crate::view::multi::{MultiKey, MultiMarker};

/// MUL-10: the marker a tray that listed `listed`, in row order, follows:
/// the one holding the most of those things, docked things included. A
/// tie goes to the marker holding the earliest-listed thing, so to the
/// first row's thing when one of them holds it. `None` once no marker
/// holds any of them, which closes the tray.
pub fn marker_to_follow(listed: &[Entity], markers: &[MultiMarker]) -> Option<MultiKey> {
    let place: HashMap<Entity, usize> = listed.iter().enumerate().map(|(i, &t)| (t, i)).collect();
    markers
        .iter()
        .filter_map(|marker| {
            let rows: Vec<usize> = marker
                .all_things()
                .filter_map(|t| place.get(&t).copied())
                .collect();
            let earliest = *rows.iter().min()?;
            Some(((rows.len(), Reverse(earliest)), marker.key))
        })
        .max_by_key(|(rank, _)| *rank)
        .map(|(_, key)| key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::multi_markers::draw::blank_look;

    fn entities(n: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..n).map(|_| world.spawn_empty().id()).collect()
    }

    fn marker(key: MultiKey, things: &[Entity], docked: Vec<(Entity, Vec<Entity>)>) -> MultiMarker {
        MultiMarker {
            key,
            centre: Vec2::ZERO,
            frame_px: 22.0,
            back: None,
            count_shown: true,
            things: things.to_vec(),
            docked,
            look: blank_look(),
            alpha: 1.0,
            drawn: true,
        }
    }

    /// MUL-10: when the thing a marker is keyed by leaves, the tray
    /// follows the rest to the marker they make, under its new key.
    #[test]
    fn the_tray_follows_the_rest_when_the_lead_leaves() {
        let e = entities(5);
        let listed = [e[0], e[1], e[2]];
        let markers = [
            marker(MultiKey::Overlap(e[0]), &[e[0], e[3], e[4]], vec![]),
            marker(MultiKey::Together(e[1]), &[e[1], e[2]], vec![]),
        ];
        assert_eq!(
            marker_to_follow(&listed, &markers),
            Some(MultiKey::Together(e[1]))
        );
    }

    /// MUL-10: a thing joining ahead of the first changes the marker's
    /// key, and the tray stays with it.
    #[test]
    fn the_tray_stays_when_a_thing_joins_ahead_of_the_first() {
        let e = entities(3);
        let markers = [marker(
            MultiKey::Together(e[0]),
            &[e[0], e[1], e[2]],
            vec![],
        )];
        assert_eq!(
            marker_to_follow(&[e[1], e[2]], &markers),
            Some(MultiKey::Together(e[0]))
        );
    }

    /// MUL-10: a tie goes to the marker holding the first row's thing,
    /// or failing that the earliest row's; docked things count.
    #[test]
    fn a_tie_goes_to_the_marker_holding_the_earliest_row() {
        let e = entities(7);
        let first = marker(MultiKey::Together(e[5]), &[e[5], e[1]], vec![]);
        let other = marker(MultiKey::Together(e[6]), &[e[6], e[0]], vec![]);
        let listed = [e[1], e[0], e[2], e[3]];
        assert_eq!(
            marker_to_follow(&listed, &[other.clone(), first.clone()]),
            Some(first.key)
        );
        let station = marker(
            MultiKey::Station(e[4]),
            &[e[4]],
            vec![(e[4], vec![e[2], e[3]])],
        );
        assert_eq!(
            marker_to_follow(&listed, &[first.clone(), station.clone()]),
            Some(station.key),
            "two docked things outnumber one"
        );
        let later = marker(MultiKey::Together(e[4]), &[e[4], e[3]], vec![]);
        let earlier = marker(MultiKey::Overlap(e[6]), &[e[6], e[2]], vec![]);
        assert_eq!(
            marker_to_follow(&listed, &[later, earlier.clone()]),
            Some(earlier.key)
        );
    }

    /// MUL-10: once no marker holds any listed thing, there is none to
    /// follow, and the tray closes.
    #[test]
    fn nothing_to_follow_once_everything_has_parted() {
        let e = entities(4);
        let markers = [marker(MultiKey::Together(e[2]), &[e[2], e[3]], vec![])];
        assert_eq!(marker_to_follow(&[e[0], e[1]], &markers), None);
        assert_eq!(marker_to_follow(&[], &markers), None);
    }
}

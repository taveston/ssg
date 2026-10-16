//! Several objects on one marker (MUL-01 to MUL-06): the multi-markers on
//! the map this frame, what each holds, and which one holds each thing.
//! A **thing** is what a multi-marker counts: a natural body, a ship, a
//! group or a station.
//!
//! Shared state (ARCH-03): `multi_markers/gather.rs` is the one writer;
//! the markers, brackets, labels, picking, clicks and the tray that lists
//! a marker's things read it.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::view::multi_markers::look::MultiLook;
use crate::view::tunables::Tunables;

/// What made a multi-marker, and so where it sits and what a click on it
/// does (MUL-05, MUL-07). Each names the entity that identifies it, so
/// a marker keeps its key from frame to frame while it holds the same
/// things.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MultiKey {
    /// A parent's fold marker, beside its marker, holding the satellites
    /// folded into it (GRP-05).
    Fold(Entity),
    /// A station with things docked at it (MUL-03). It is drawn by the
    /// station's own marker, which gains the count and the back frame.
    Station(Entity),
    /// Ships and groups together at one point, with no station among them
    /// (MUL-05), by the first of them.
    Together(Entity),
    /// Symbols that overlap on screen (MUL-05), by the first of them.
    Overlap(Entity),
}

/// One multi-marker this frame.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiMarker {
    pub key: MultiKey,
    /// The front frame's centre on screen, on a whole pixel (MUL-06,
    /// SYM-16).
    pub centre: Vec2,
    /// What it counts, in no particular order (MUL-04). A station marker
    /// counts only its docked things, so for one this is the station alone
    /// and the docked things are in `docked`.
    pub things: Vec<Entity>,
    /// For each station among `things` with things docked at it, those
    /// things: listed under the station's row and not counted (MUL-04).
    pub docked: Vec<(Entity, Vec<Entity>)>,
    pub look: MultiLook,
    /// How strongly it is drawn: as strongly as the strongest thing in it
    /// would be on its own (MOT-11, GRP-03).
    pub alpha: f32,
    /// Whether it is on screen this frame; a marker off screen or faded
    /// out still holds its things, so their symbols stay hidden.
    pub drawn: bool,
}

impl MultiMarker {
    /// MUL-01 / MUL-03 / MUL-04: the number its count shows: the docked
    /// things for a station marker, otherwise its things.
    pub fn count(&self) -> usize {
        match self.key {
            MultiKey::Station(_) => self.docked.iter().map(|(_, d)| d.len()).sum(),
            _ => self.things.len(),
        }
    }

    /// Every thing it holds, docked things included.
    pub fn all_things(&self) -> impl Iterator<Item = Entity> + '_ {
        self.things
            .iter()
            .copied()
            .chain(self.docked.iter().flat_map(|(_, d)| d.iter().copied()))
    }
}

/// Every multi-marker this frame, and the joined things each holds.
#[derive(Resource, Default, Debug)]
pub struct MultiMarkers {
    pub markers: Vec<MultiMarker>,
    /// MUL-05: each thing drawn into a marker other than its own symbol:
    /// joined into a together or overlap marker, or docked into its
    /// station's. Folded satellites are in `FoldSummary` instead.
    pub joined: HashMap<Entity, MultiKey>,
}

impl MultiMarkers {
    pub fn get(&self, key: MultiKey) -> Option<&MultiMarker> {
        self.markers.iter().find(|m| m.key == key)
    }

    /// HOV-07: the box marker `key`'s two frames take on screen, before
    /// any margin, while it is drawn. A station marker is its station's
    /// own marker, so `station_marker` says where that is drawn now: its
    /// centre, half size and back frame.
    pub fn frames_box(
        &self,
        key: MultiKey,
        station_marker: impl FnOnce(Entity) -> Option<(Vec2, f32, Option<Vec2>)>,
        tunables: &Tunables,
    ) -> Option<Rect> {
        if let MultiKey::Station(station) = key {
            let (centre, half, back) = station_marker(station)?;
            return Some(frames_rect(centre, half, back));
        }
        let marker = self.get(key).filter(|m| m.drawn)?;
        let back = back_offset(tunables.back_frame_offset_px);
        Some(frames_rect(
            marker.centre,
            tunables.ship_symbol_px / 2.0,
            Some(back),
        ))
    }

    /// MUL-03: how many things are docked at a station, zero if it isn't
    /// drawn as a station marker.
    pub fn docked_count(&self, station: Entity) -> usize {
        self.get(MultiKey::Station(station))
            .map_or(0, MultiMarker::count)
    }
}

/// MUL-05: a thing's slide into the marker that holds it, or back out:
/// `t` 0 is at its own place and drawn, 1 is in the marker and not drawn.
/// `offset` is from its own place to the marker's centre, kept while it
/// slides out so it leaves from where the marker was. `held` is whether a
/// marker holds it now; a held thing draws no tail (MUL-05).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct JoinSlide {
    pub t: f32,
    pub offset: Vec2,
    pub held: bool,
}

/// On a marker node, or a multi-marker's node: where its back frame is
/// drawn while it has one, as an offset from the front frame's centre
/// (MUL-01). Brackets, labels and picking take in both frames (HOV-07,
/// LBL-01, MUL-11).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct BackFrame(pub Option<Vec2>);

/// MUL-01: where a back frame sits from its front frame: `offset_px` to
/// the right and up.
pub fn back_offset(offset_px: f32) -> Vec2 {
    Vec2::new(offset_px, -offset_px)
}

/// HOV-07 / MUL-11: the box a front frame `half` pixels from `centre` and
/// its back frame at `back` take together, before any margin.
pub fn frames_rect(centre: Vec2, half: f32, back: Option<Vec2>) -> Rect {
    let front = Rect::from_center_half_size(centre, Vec2::splat(half));
    match back {
        Some(back) => front.union(Rect::from_center_half_size(
            centre + back,
            Vec2::splat(half),
        )),
        None => front,
    }
}

/// A multi-marker's drawn node (`multi_markers/draw.rs`), with the
/// invisible entity that takes its hover and clicks (MUL-11).
#[derive(Component, Debug, Clone, Copy)]
pub struct MultiNode {
    pub target: Entity,
}

/// The hover and click target standing for a multi-marker: which marker
/// it is this frame, if any.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct MultiTarget(pub Option<MultiKey>);

/// What a tray lists (MUL-08): the things of a multi-marker gathered
/// here, or the members of a shared orbit at its exposure ring's identity
/// mark (MOT-14, HOV-05), which `rings/marks.rs` draws, by the named
/// orbit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrayFor {
    Marker(MultiKey),
    Orbit(Entity),
}

/// The tray that is open, if any (MUL-07, MUL-10). At most one is. While
/// one is, the arrow keys are the tray's and do not pan (CAM-14), and Esc
/// closes it before anything else (INS-03). The tray (`view/tray/`)
/// opens and closes it.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct OpenTray(pub Option<TrayFor>);

/// A click this frame on a multi-marker, or on a shared orbit's ring or
/// identity mark, asks for its tray (MUL-07, HOV-05). The tray opens it
/// in place of any other; a click elsewhere, which asks for none, closes
/// the open one (MUL-10).
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct TrayRequest(pub Option<TrayFor>);

#[cfg(test)]
mod tests {
    use super::*;

    /// MUL-01 / HOV-07: the two frames together reach from the front
    /// frame's left and bottom to the back frame's right and top.
    #[test]
    fn the_frames_rect_takes_in_the_back_frame() {
        let rect = frames_rect(Vec2::new(100.0, 50.0), 11.0, Some(back_offset(4.0)));
        assert_eq!(rect.min, Vec2::new(89.0, 35.0));
        assert_eq!(rect.max, Vec2::new(115.0, 61.0));
        assert_eq!(rect.width(), rect.height(), "a square");
        let alone = frames_rect(Vec2::ZERO, 11.0, None);
        assert_eq!(alone.size(), Vec2::splat(22.0));
    }
}

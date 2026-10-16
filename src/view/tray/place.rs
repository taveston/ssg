//! Where a tray sits (MUL-08): to the right of its marker's box (HOV-07)
//! with a gap, centred on the marker vertically; if it doesn't fit there,
//! to the left, except for a fold marker, whose left would cover its
//! parent: that tray goes below the marker, or above it, centred on the
//! marker horizontally. Then it is moved to lie wholly inside the map
//! area (INS-07). The notch on the edge facing the marker points at it.
//! A plain function of the boxes, so it can be tested apart from the UI.

use bevy::prelude::*;

/// Which side of its marker a tray opens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraySide {
    Right,
    Left,
    Below,
    Above,
}

/// Where a tray goes this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayPlace {
    /// Its top-left corner, in window pixels.
    pub top_left: Vec2,
    /// The side of the marker it opens on; its notch is on the edge that
    /// faces the marker.
    pub side: TraySide,
    /// The notch's centre along the facing edge: from the tray's top for
    /// a tray beside the marker, from its left for one below or above.
    pub notch: f32,
    /// The tallest it may be: the map area's height. A taller tray
    /// scrolls inside it.
    pub max_height: f32,
}

/// The boxes a tray is placed against: its marker's box, and for a fold
/// marker its parent's box, which the tray must not cover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayAnchor {
    pub marker: Rect,
    pub parent: Option<Rect>,
}

/// The tray's size, the map area it must lie in, the gap from the
/// marker's box and how far the notch keeps from the tray's corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayRoom {
    pub size: Vec2,
    pub area: Rect,
    pub gap: f32,
    pub notch_inset: f32,
}

/// MUL-08: place a tray beside `anchor` in `room`. A fold marker's tray
/// (one with a parent) never opens to the left, where its parent is;
/// below or above, it also keeps `gap` clear of the parent's box, so it
/// never covers the parent.
pub fn place_tray(anchor: TrayAnchor, room: TrayRoom) -> TrayPlace {
    let TrayRoom {
        size,
        area,
        gap,
        notch_inset,
    } = room;
    let marker = anchor.marker;
    let height = size.y.min(area.height());
    let fits_right = marker.max.x + gap + size.x <= area.max.x;
    let side = match anchor.parent {
        _ if fits_right => TraySide::Right,
        None => TraySide::Left,
        Some(parent) if parent.union(marker).max.y + gap + height <= area.max.y => TraySide::Below,
        Some(_) => TraySide::Above,
    };
    let clear = anchor.parent.map_or(marker, |parent| parent.union(marker));
    let (x, y) = match side {
        TraySide::Right | TraySide::Left => {
            let x = if side == TraySide::Right {
                marker.max.x + gap
            } else {
                marker.min.x - gap - size.x
            };
            (x, marker.center().y - height / 2.0)
        }
        TraySide::Below => (marker.center().x - size.x / 2.0, clear.max.y + gap),
        TraySide::Above => (marker.center().x - size.x / 2.0, clear.min.y - gap - height),
    };
    let x = x.min(area.max.x - size.x).max(area.min.x);
    let y = y.min(area.max.y - height).max(area.min.y);
    let notch = match side {
        TraySide::Right | TraySide::Left => {
            (marker.center().y - y).clamp(notch_inset, (height - notch_inset).max(notch_inset))
        }
        TraySide::Below | TraySide::Above => {
            (marker.center().x - x).clamp(notch_inset, (size.x - notch_inset).max(notch_inset))
        }
    };
    TrayPlace {
        top_left: Vec2::new(x, y),
        side,
        notch,
        max_height: area.height(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect {
        min: Vec2::new(0.0, 40.0),
        max: Vec2::new(1200.0, 900.0),
    };
    const SIZE: Vec2 = Vec2::new(240.0, 160.0);

    fn room(size: Vec2) -> TrayRoom {
        TrayRoom {
            size,
            area: AREA,
            gap: 8.0,
            notch_inset: 10.0,
        }
    }

    fn anchor_at(centre: Vec2) -> TrayAnchor {
        TrayAnchor {
            marker: Rect::from_center_half_size(centre, Vec2::splat(15.0)),
            parent: None,
        }
    }

    /// MUL-08: with room, the tray sits right of the marker's box by the
    /// gap, centred on it vertically, the notch level with the marker.
    #[test]
    fn a_tray_opens_to_the_right_centred_on_its_marker() {
        let place = place_tray(anchor_at(Vec2::new(400.0, 400.0)), room(SIZE));
        assert_eq!(place.side, TraySide::Right);
        assert_eq!(place.top_left, Vec2::new(423.0, 320.0));
        assert_eq!(place.notch, 80.0);
    }

    /// MUL-08: without room on the right it opens to the left, its right
    /// edge the gap clear of the box.
    #[test]
    fn a_tray_opens_to_the_left_near_the_right_edge() {
        let anchor = anchor_at(Vec2::new(1100.0, 400.0));
        let place = place_tray(anchor, room(SIZE));
        assert_eq!(place.side, TraySide::Left);
        assert_eq!(place.top_left.x + SIZE.x, anchor.marker.min.x - 8.0);
    }

    /// MUL-08: a fold marker's tray never opens to the left over its
    /// parent: near the right edge it goes below, centred on the marker,
    /// or above near the bottom too; either way it never covers the
    /// parent's box.
    #[test]
    fn a_fold_markers_tray_goes_below_or_above_never_over_its_parent() {
        let fold = |centre: Vec2| {
            let marker = Rect::from_center_half_size(centre, Vec2::splat(15.0));
            let parent = Rect::from_center_half_size(centre - Vec2::splat(21.0), Vec2::splat(10.0));
            TrayAnchor {
                marker,
                parent: Some(parent),
            }
        };
        let covers = |anchor: TrayAnchor, place: TrayPlace| {
            let tray = Rect::from_corners(place.top_left, place.top_left + SIZE);
            let shared = tray.intersect(anchor.parent.unwrap());
            shared.width() > 0.0 && shared.height() > 0.0
        };
        let right = fold(Vec2::new(400.0, 400.0));
        assert_eq!(place_tray(right, room(SIZE)).side, TraySide::Right);
        let below = fold(Vec2::new(1100.0, 400.0));
        let place = place_tray(below, room(SIZE));
        assert_eq!(place.side, TraySide::Below);
        assert_eq!(place.top_left.y, below.marker.max.y + 8.0);
        assert_eq!(place.notch, below.marker.center().x - place.top_left.x);
        assert!(!covers(below, place));
        let above = fold(Vec2::new(1100.0, 850.0));
        let place = place_tray(above, room(SIZE));
        assert_eq!(place.side, TraySide::Above);
        assert!(!covers(above, place), "{place:?}");
        let parent_top = above.parent.unwrap().min.y;
        assert_eq!(place.top_left.y + SIZE.y, parent_top - 8.0);
    }

    /// MUL-08: near the top or bottom of the map area the tray is moved
    /// to lie inside it, and the notch moves along its edge to keep
    /// pointing at the marker, never past its corners.
    #[test]
    fn a_tray_stays_inside_the_map_area_and_its_notch_follows() {
        for centre in [Vec2::new(300.0, 45.0), Vec2::new(5.0, 895.0)] {
            let place = place_tray(anchor_at(centre), room(SIZE));
            let tray = Rect::from_corners(place.top_left, place.top_left + SIZE);
            assert!(
                AREA.contains(tray.min) && AREA.contains(tray.max),
                "{tray:?}"
            );
            assert!((10.0..=SIZE.y - 10.0).contains(&place.notch));
        }
        let top = place_tray(anchor_at(Vec2::new(300.0, 60.0)), room(SIZE));
        assert_eq!(top.top_left.y, AREA.min.y);
        assert_eq!(top.notch, 20.0, "still level with the marker");
    }

    /// MUL-08: a tray taller than the map area fills its height and no
    /// more, from its top; the rest scrolls.
    #[test]
    fn a_tray_taller_than_the_map_area_is_cut_to_it() {
        let tall = Vec2::new(240.0, 2000.0);
        let place = place_tray(anchor_at(Vec2::new(300.0, 500.0)), room(tall));
        assert_eq!(place.top_left.y, AREA.min.y);
        assert_eq!(place.max_height, AREA.height());
    }
}

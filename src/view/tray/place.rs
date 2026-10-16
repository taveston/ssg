//! Where a tray sits (MUL-08): to the right of its marker's box (HOV-07)
//! with a gap, or to the left if it doesn't fit there, centred on the
//! marker vertically, then moved to lie wholly inside the map area
//! (INS-07). The notch on the edge facing the marker points at it. A
//! plain function of the boxes, so it can be tested apart from the UI.

use bevy::prelude::*;

/// Where a tray goes this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayPlace {
    /// Its top-left corner, in window pixels.
    pub top_left: Vec2,
    /// It opens to the marker's left, so its notch is on its right edge.
    pub left_side: bool,
    /// The notch's centre down the facing edge, from the tray's top.
    pub notch_y: f32,
    /// The tallest it may be: the map area's height. A taller tray
    /// scrolls inside it.
    pub max_height: f32,
}

/// MUL-08: place a tray of `size` beside the marker box `anchor`, `gap`
/// clear of it, inside `area`. The notch stays at least `notch_inset`
/// from the tray's top and bottom corners.
pub fn place_tray(anchor: Rect, size: Vec2, area: Rect, gap: f32, notch_inset: f32) -> TrayPlace {
    let height = size.y.min(area.height());
    let right_x = anchor.max.x + gap;
    let left_side = right_x + size.x > area.max.x;
    let x = if left_side {
        anchor.min.x - gap - size.x
    } else {
        right_x
    };
    let x = x.min(area.max.x - size.x).max(area.min.x);
    let y = (anchor.center().y - height / 2.0)
        .min(area.max.y - height)
        .max(area.min.y);
    let notch_y = (anchor.center().y - y).clamp(notch_inset, (height - notch_inset).max(notch_inset));
    TrayPlace {
        top_left: Vec2::new(x, y),
        left_side,
        notch_y,
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

    fn anchor_at(centre: Vec2) -> Rect {
        Rect::from_center_half_size(centre, Vec2::splat(15.0))
    }

    /// MUL-08: with room, the tray sits right of the marker's box by the
    /// gap, centred on it vertically, the notch level with the marker.
    #[test]
    fn a_tray_opens_to_the_right_centred_on_its_marker() {
        let anchor = anchor_at(Vec2::new(400.0, 400.0));
        let place = place_tray(anchor, SIZE, AREA, 8.0, 10.0);
        assert!(!place.left_side);
        assert_eq!(place.top_left, Vec2::new(423.0, 320.0));
        assert_eq!(place.notch_y, 80.0);
    }

    /// MUL-08: without room on the right it opens to the left, its right
    /// edge the gap clear of the box.
    #[test]
    fn a_tray_opens_to_the_left_near_the_right_edge() {
        let anchor = anchor_at(Vec2::new(1100.0, 400.0));
        let place = place_tray(anchor, SIZE, AREA, 8.0, 10.0);
        assert!(place.left_side);
        assert_eq!(place.top_left.x + SIZE.x, anchor.min.x - 8.0);
    }

    /// MUL-08: near the top or bottom of the map area the tray is moved
    /// to lie inside it, and the notch moves along its edge to keep
    /// pointing at the marker, never past its corners.
    #[test]
    fn a_tray_stays_inside_the_map_area_and_its_notch_follows() {
        for centre in [Vec2::new(300.0, 45.0), Vec2::new(5.0, 895.0)] {
            let place = place_tray(anchor_at(centre), SIZE, AREA, 8.0, 10.0);
            let tray = Rect::from_corners(place.top_left, place.top_left + SIZE);
            assert!(AREA.contains(tray.min) && AREA.contains(tray.max), "{tray:?}");
            assert!((10.0..=SIZE.y - 10.0).contains(&place.notch_y));
        }
        let top = place_tray(anchor_at(Vec2::new(300.0, 60.0)), SIZE, AREA, 8.0, 10.0);
        assert_eq!(top.top_left.y, AREA.min.y);
        assert_eq!(top.notch_y, 20.0, "still level with the marker");
    }

    /// MUL-08: a tray taller than the map area fills its height and no
    /// more, from its top; the rest scrolls.
    #[test]
    fn a_tray_taller_than_the_map_area_is_cut_to_it() {
        let tall = Vec2::new(240.0, 2000.0);
        let place = place_tray(anchor_at(Vec2::new(300.0, 500.0)), tall, AREA, 8.0, 10.0);
        assert_eq!(place.top_left.y, AREA.min.y);
        assert_eq!(place.max_height, AREA.height());
    }
}

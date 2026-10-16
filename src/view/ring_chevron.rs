//! Where an exposure ring's direction chevron goes (MOT-14): on the ring,
//! a fixed gap beyond the edge of the identity mark in the direction the
//! ring's bodies circle, turned along the ring there. Fixed to the mark,
//! it says which way the orbit runs without implying a position.
//!
//! Also which side of the mark the ring's label takes, so the chevron
//! never runs under it. Plain geometry in screen pixels (y down), tested
//! without a camera; `ring_marks.rs` draws the result.

use bevy::prelude::*;

/// A small step round the ring, in radians, for the ring's direction.
const TANGENT_STEP_RAD: f32 = 1e-3;
/// Halvings when searching round the ring for the chevron's spot.
const SEARCH_STEPS: u32 = 24;

/// How far from `origin` a ray in direction `dir` (a unit vector) leaves
/// `rect`, which contains `origin`.
pub fn exit_distance(rect: Rect, dir: Vec2) -> f32 {
    let axis = |d: f32, min: f32, max: f32| {
        if d > f32::EPSILON {
            max / d
        } else if d < -f32::EPSILON {
            min / d
        } else {
            f32::INFINITY
        }
    };
    axis(dir.x, rect.min.x, rect.max.x).min(axis(dir.y, rect.min.y, rect.max.y))
}

/// The chevron's open arrowhead is half as long as it is wide, so its
/// centre is a quarter of its width from its back.
pub fn chevron_half_length(chevron_px: f32) -> f32 {
    chevron_px / 4.0
}

/// The chevron's centre on screen and the unit direction it points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChevronSpot {
    pub centre: Vec2,
    pub dir: Vec2,
}

/// What places a chevron: the mark's box relative to the ring point, the
/// gap beyond its edge, and the chevron's width, in pixels.
#[derive(Debug, Clone, Copy)]
pub struct ChevronFit {
    pub mark: Rect,
    pub gap_px: f32,
    pub chevron_px: f32,
}

/// MOT-14: the chevron's spot: on the ring, `gap_px` beyond the mark's
/// edge in the direction the ring's bodies circle (`sense` +1 for growing
/// angle, -1 for shrinking), pointing along the ring there. `point` gives
/// the screen point at an angle round the ring; `angle` is the mark's.
/// `None` when the ring is too small on screen to reach that far.
pub fn chevron_spot(
    point: impl Fn(f32) -> Option<Vec2>,
    angle: f32,
    sense: f32,
    fit: ChevronFit,
) -> Option<ChevronSpot> {
    let at = point(angle)?;
    let ahead = (point(angle + sense * TANGENT_STEP_RAD)? - at).try_normalize()?;
    let reach = exit_distance(fit.mark, ahead) + fit.gap_px + chevron_half_length(fit.chevron_px);
    // Round the ring from the mark, find the turn at which the ring is
    // `reach` from the mark's point; half a turn is as far as it gets.
    let away = |turn: f32| point(angle + sense * turn).map(|p| p.distance(at));
    if away(std::f32::consts::PI)? < reach {
        return None;
    }
    let (mut low, mut high) = (0.0, std::f32::consts::PI);
    for _ in 0..SEARCH_STEPS {
        let mid = (low + high) / 2.0;
        if away(mid)? < reach {
            low = mid;
        } else {
            high = mid;
        }
    }
    let turn = (low + high) / 2.0;
    let centre = point(angle + sense * turn)?;
    let before = point(angle + sense * (turn - TANGENT_STEP_RAD))?;
    let after = point(angle + sense * (turn + TANGENT_STEP_RAD))?;
    let dir = (after - before).try_normalize()?;
    Some(ChevronSpot { centre, dir })
}

/// The box a chevron takes on screen: a square its width across, round
/// its centre, so it covers the arrowhead however it is turned.
pub fn chevron_box(spot: ChevronSpot, chevron_px: f32) -> Rect {
    Rect::from_center_size(spot.centre, Vec2::splat(chevron_px))
}

/// Which side of its mark a ring's label sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelSide {
    Right,
    Left,
}

/// The box a ring's label of `size` takes on `side` of the mark (both
/// relative to the ring point), `gap_px` from its edge, with its first
/// line centred on the ring point (`first_line_px` high).
pub fn label_box(mark: Rect, size: Vec2, side: LabelSide, gap_px: f32, first_line_px: f32) -> Rect {
    let top = -first_line_px / 2.0;
    let left = match side {
        LabelSide::Right => mark.max.x + gap_px,
        LabelSide::Left => mark.min.x - gap_px - size.x,
    };
    Rect::new(left, top, left + size.x, top + size.y)
}

/// MOT-14: the label sits right of its mark, and moves to the left when
/// the chevron (its box relative to the ring point) would run under it.
pub fn label_side(
    mark: Rect,
    label_size: Vec2,
    chevron: Option<Rect>,
    (gap_px, first_line_px): (f32, f32),
) -> LabelSide {
    let right = label_box(mark, label_size, LabelSide::Right, gap_px, first_line_px);
    match chevron {
        Some(chevron) if !chevron.intersect(right).is_empty() => LabelSide::Left,
        _ => LabelSide::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A face-on ring of radius 100 px round the origin, y down: angle 0
    /// is to the right, and a growing angle runs clockwise on screen.
    fn flat(angle: f32) -> Option<Vec2> {
        Some(Vec2::new(angle.cos(), angle.sin()) * 100.0)
    }

    fn fit() -> ChevronFit {
        ChevronFit {
            mark: Rect::new(-4.0, -4.0, 4.0, 4.0),
            gap_px: 8.0,
            chevron_px: 8.0,
        }
    }

    /// MOT-14: a ray leaves a box at its nearest wall.
    #[test]
    fn a_ray_leaves_a_box_at_its_wall() {
        let square = Rect::new(-4.0, -4.0, 4.0, 4.0);
        assert!((exit_distance(square, Vec2::X) - 4.0).abs() < 1e-5);
        let diagonal = Vec2::ONE.normalize();
        assert!((exit_distance(square, diagonal) - 4.0 * 2f32.sqrt()).abs() < 1e-4);
        // A pill reaching further right than left.
        let pill = Rect::new(-8.0, -8.0, 26.0, 8.0);
        assert!((exit_distance(pill, Vec2::NEG_X) - 8.0).abs() < 1e-5);
        assert!((exit_distance(pill, Vec2::X) - 26.0).abs() < 1e-5);
    }

    /// MOT-14: the chevron sits on the ring, `chevron_gap_px` beyond the
    /// mark's edge, ahead of the mark the way the bodies circle, and
    /// points along the ring that way. Reversing the sense puts it on the
    /// other side, pointing back.
    #[test]
    fn the_chevron_sits_on_the_ring_ahead_of_the_mark_the_way_bodies_circle() {
        let fit = fit();
        let reach = 4.0 + fit.gap_px + chevron_half_length(fit.chevron_px);
        // The mark at the top of the ring; growing angle runs right there.
        let top = -std::f32::consts::FRAC_PI_2;
        let spot = chevron_spot(flat, top, 1.0, fit).unwrap();
        assert!((spot.centre.length() - 100.0).abs() < 1e-3, "on the ring");
        assert!((spot.centre.distance(flat(top).unwrap()) - reach).abs() < 1e-2);
        assert!(
            spot.centre.x > 0.0 && spot.dir.x > 0.9,
            "ahead, to the right"
        );
        let back = chevron_spot(flat, top, -1.0, fit).unwrap();
        assert!(back.centre.x < 0.0 && back.dir.x < -0.9, "the other way");
    }

    /// MOT-14: a ring too small to reach past the mark has no chevron.
    #[test]
    fn a_ring_smaller_than_the_mark_has_no_chevron() {
        let tiny = |angle: f32| Some(Vec2::new(angle.cos(), angle.sin()) * 5.0);
        assert_eq!(chevron_spot(tiny, 0.0, 1.0, fit()), None);
    }

    /// MOT-14: the label sits right of its mark, and moves left when the
    /// chevron would run under it, but not when the chevron is above or
    /// to the left.
    #[test]
    fn the_label_moves_left_when_the_chevron_would_run_under_it() {
        let fit = fit();
        let label = Vec2::new(40.0, 25.0);
        let spacing = (4.0, 13.0);
        let at = |centre: Vec2| Some(Rect::from_center_size(centre, Vec2::splat(8.0)));
        let side = |chevron| label_side(fit.mark, label, chevron, spacing);
        assert_eq!(side(at(Vec2::new(16.0, 0.0))), LabelSide::Left);
        assert_eq!(side(at(Vec2::new(-16.0, 0.0))), LabelSide::Right);
        assert_eq!(side(at(Vec2::new(0.0, -16.0))), LabelSide::Right);
        assert_eq!(side(None), LabelSide::Right);
        // On the left the label ends the gap before the mark.
        let left = label_box(fit.mark, label, LabelSide::Left, 4.0, 13.0);
        assert_eq!(left.max.x, -8.0);
        assert_eq!(left.min.y, -6.5);
    }
}

//! Where the frame cue lies on screen (REF-01 to REF-08), as plain
//! functions of projected points: each arm's start, end and ticks, the
//! two halves of the vertical axis and the place of the "N". The systems
//! in `draw.rs` gather the camera and the frame body and apply the result.
//!
//! Distances along a stroke are in logical pixels from its start; screen
//! positions are logical pixels from the window's top left, y down.

use bevy::prelude::*;

use crate::view::screen::ray_inside;

/// The fewest pixels between two ticks on an arm. Towards the plane's
/// horizon the ticks crowd without end; past the first pair closer than
/// this the rest would only read as a solid bar, so they are left off.
pub const MIN_TICK_SPACING_PX: f32 = 4.0;
/// The most ticks one arm carries (the shader's limit).
pub const MAX_TICKS: usize = 64;

/// How a stroke is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeStyle {
    /// Round dots `dot_px` across, their centres `pitch_px` apart (REF-01).
    Dotted { dot_px: f32, pitch_px: f32 },
    /// A solid line `width_px` wide, ending in a filled arrowhead
    /// `arrow_px` long, or none at 0 (REF-06, REF-07).
    Solid { width_px: f32, arrow_px: f32 },
}

/// One stroke of the cue on screen: it starts at `start` and runs
/// `length` pixels along the unit vector `dir`, fading from `alpha_start`
/// to `alpha_end`, with a tick `tick_px` long across it at each distance
/// in `ticks`.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub start: Vec2,
    pub dir: Vec2,
    pub length: f32,
    pub alpha_start: f32,
    pub alpha_end: f32,
    pub style: StrokeStyle,
    pub ticks: Vec<f32>,
    pub tick_px: f32,
}

impl Stroke {
    /// The screen point `along` pixels from the start.
    pub fn at(&self, along: f32) -> Vec2 {
        self.start + self.dir * along
    }
}

/// REF-02 / REF-03: where an arm runs, as distances from the frame body's
/// centre along its screen direction: from `start_px` (half the body's
/// box plus the gap) to the edge of the map area, or to the plane's
/// horizon at `horizon_px` if that is nearer. A stretch before the arm
/// enters the map area is left out too. `None` if nothing is left.
pub fn arm_extent(
    centre: Vec2,
    dir: Vec2,
    start_px: f32,
    horizon_px: Option<f32>,
    map: Rect,
) -> Option<(f32, f32)> {
    let (inside_from, inside_to) = ray_inside(centre, dir, map)?;
    let start = start_px.max(inside_from);
    let end = horizon_px.map_or(inside_to, |h| inside_to.min(h));
    (end > start).then_some((start, end))
}

/// REF-05: the arm's ticks, as distances from the body's centre along the
/// arm. `along(k)` is where the k-th multiple of the scale bar's distance
/// from the body lands on the arm, or `None` once it can't be seen. Ticks
/// before `start` (in the gap round the body) are skipped; they end past
/// `end`, where two crowd closer than `MIN_TICK_SPACING_PX` towards the
/// horizon, or at `MAX_TICKS`.
pub fn arm_ticks(along: impl Fn(u32) -> Option<f32>, start: f32, end: f32) -> Vec<f32> {
    let mut ticks = Vec::new();
    let mut previous = 0.0;
    for k in 1.. {
        let Some(at) = along(k) else {
            break;
        };
        if at > end || at - previous < MIN_TICK_SPACING_PX || ticks.len() == MAX_TICKS {
            break;
        }
        previous = at;
        if at >= start {
            ticks.push(at);
        }
    }
    ticks
}

/// REF-06: how much the view shortens the plane's normal at a point seen
/// along the unit vector `view` (from the eye to the point): nothing when
/// looking straight down it, all of its length edge-on.
pub fn normal_foreshortening(view: Vec3) -> f32 {
    (1.0 - view.y * view.y).max(0.0).sqrt()
}

/// The tunables the vertical axis is drawn with (REF-06 to REF-08).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalLook {
    pub up_px: f32,
    pub down_fraction: f32,
    pub min_px: f32,
    pub width_px: f32,
    pub arrow_px: f32,
    pub alpha_near: f32,
    pub alpha_far: f32,
}

/// REF-06 to REF-08: the vertical axis's two halves. `centre` is the
/// frame body on screen and `up` the unit screen direction of the plane's
/// normal on the camera's side. The upper half starts `upper_from` pixels
/// from the centre (at the pole of a true-size disc, or the gap beyond a
/// marker's box) and is `up_px` long shortened by `shortening`, ending in
/// an arrowhead. The lower half starts at the centre, runs the other way
/// for `down_fraction` of the upper half's length and has no arrowhead.
/// Both fade as the arms do. A half shorter than `min_px` is not drawn.
pub fn vertical_halves(
    centre: Vec2,
    up: Vec2,
    upper_from: f32,
    shortening: f32,
    look: &VerticalLook,
) -> (Option<Stroke>, Option<Stroke>) {
    let upper_px = look.up_px * shortening;
    let lower_px = upper_px * look.down_fraction;
    let half = |start: Vec2, dir: Vec2, length: f32, arrow_px: f32| {
        (length >= look.min_px).then(|| Stroke {
            start,
            dir,
            length,
            alpha_start: look.alpha_near,
            alpha_end: look.alpha_far,
            style: StrokeStyle::Solid {
                width_px: look.width_px,
                arrow_px,
            },
            ticks: Vec::new(),
            tick_px: 0.0,
        })
    };
    (
        half(centre + up * upper_from, up, upper_px, look.arrow_px),
        half(centre, -up, lower_px, 0.0),
    )
}

/// REF-04: where the "N" sits: `inset_px` back from the north arm's end,
/// or nowhere if the arm is too short to hold it.
pub fn north_label_at(arm: &Stroke, inset_px: f32) -> Option<Vec2> {
    (arm.length > inset_px).then(|| arm.at(arm.length - inset_px))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: Rect = Rect {
        min: Vec2::new(0.0, 40.0),
        max: Vec2::new(800.0, 600.0),
    };

    /// REF-03: an arm runs from its start to the map area's edge.
    #[test]
    fn an_arm_runs_from_its_gap_to_the_edge_of_the_map() {
        let centre = Vec2::new(400.0, 300.0);
        let (start, end) = arm_extent(centre, Vec2::X, 20.0, None, MAP).unwrap();
        assert_eq!((start, end), (20.0, 400.0));
        let (_, end) = arm_extent(centre, Vec2::NEG_Y, 20.0, None, MAP).unwrap();
        assert_eq!(end, 260.0, "stops at the top bar, not the window");
    }

    /// REF-03: where the plane's horizon crosses the view nearer than the
    /// edge, the arm stops there.
    #[test]
    fn an_arm_stops_at_the_horizon_if_that_is_nearer() {
        let centre = Vec2::new(400.0, 300.0);
        let (_, end) = arm_extent(centre, Vec2::NEG_Y, 20.0, Some(120.0), MAP).unwrap();
        assert_eq!(end, 120.0);
        assert!(arm_extent(centre, Vec2::NEG_Y, 20.0, Some(15.0), MAP).is_none());
    }

    /// REF-03: a frame body off the map still gets the part of an arm that
    /// crosses the map area; an arm pointing away from it gets none.
    #[test]
    fn an_arm_from_off_the_map_keeps_only_what_lies_on_it() {
        let centre = Vec2::new(-100.0, 300.0);
        let (start, end) = arm_extent(centre, Vec2::X, 20.0, None, MAP).unwrap();
        assert_eq!((start, end), (100.0, 900.0));
        assert!(arm_extent(centre, Vec2::NEG_X, 20.0, None, MAP).is_none());
    }

    /// REF-05: ticks fall at each multiple's place, skipping those in the
    /// gap and ending at the arm's end.
    #[test]
    fn ticks_skip_the_gap_and_end_with_the_arm() {
        let ticks = arm_ticks(|k| Some(15.0 * k as f32), 20.0, 100.0);
        assert_eq!(ticks, [30.0, 45.0, 60.0, 75.0, 90.0]);
    }

    /// REF-05: crowding towards the horizon, ticks end once two would sit
    /// closer than a few pixels.
    #[test]
    fn ticks_end_where_they_crowd_towards_the_horizon() {
        // Each tick halves the remaining distance to a horizon at 200 px.
        let along = |k: u32| Some(200.0 - 200.0 * 0.5_f32.powi(k as i32));
        let ticks = arm_ticks(along, 0.0, 200.0);
        assert!(ticks.windows(2).all(|w| w[1] - w[0] >= MIN_TICK_SPACING_PX));
        assert!(ticks.len() < 10 && *ticks.last().unwrap() < 200.0);
    }

    /// REF-06: the normal is nothing seen straight down and whole edge-on.
    #[test]
    fn the_normal_shortens_with_the_view() {
        assert_eq!(normal_foreshortening(Vec3::NEG_Y), 0.0);
        assert_eq!(normal_foreshortening(Vec3::X), 1.0);
        let tilt = 30f32.to_radians();
        let view = Vec3::new(0.0, -tilt.cos(), -tilt.sin());
        assert!((normal_foreshortening(view) - tilt.sin()).abs() < 1e-6);
    }

    fn look() -> VerticalLook {
        VerticalLook {
            up_px: 100.0,
            down_fraction: 0.7,
            min_px: 10.0,
            width_px: 1.5,
            arrow_px: 8.0,
            alpha_near: 1.0,
            alpha_far: 0.45,
        }
    }

    /// REF-06 / REF-07: the upper half starts beyond the body and ends in
    /// an arrowhead; the lower half starts at the centre, runs down for
    /// its fraction of the upper's length, with no arrowhead.
    #[test]
    fn the_vertical_halves_start_and_run_as_set() {
        let centre = Vec2::new(400.0, 300.0);
        let (upper, lower) = vertical_halves(centre, Vec2::NEG_Y, 12.0, 0.5, &look());
        let (upper, lower) = (upper.unwrap(), lower.unwrap());
        assert_eq!(upper.start, Vec2::new(400.0, 288.0));
        assert_eq!(upper.length, 50.0);
        assert_eq!(upper.at(upper.length), Vec2::new(400.0, 238.0));
        assert!(matches!(upper.style, StrokeStyle::Solid { arrow_px, .. } if arrow_px == 8.0));
        assert_eq!(lower.start, centre);
        assert_eq!(lower.dir, Vec2::Y);
        assert!((lower.length - 35.0).abs() < 1e-4);
        assert!(matches!(lower.style, StrokeStyle::Solid { arrow_px, .. } if arrow_px == 0.0));
        assert_eq!((upper.alpha_start, upper.alpha_end), (1.0, 0.45));
    }

    /// REF-08: a half shorter than the minimum is not drawn, so the lower
    /// half goes first as the view turns towards straight down.
    #[test]
    fn a_short_vertical_half_is_not_drawn() {
        let centre = Vec2::ZERO;
        let (upper, lower) = vertical_halves(centre, Vec2::NEG_Y, 12.0, 0.12, &look());
        assert!(upper.is_some() && lower.is_none(), "12 px up, 8.4 px down");
        let (upper, lower) = vertical_halves(centre, Vec2::NEG_Y, 12.0, 0.0, &look());
        assert!(upper.is_none() && lower.is_none());
    }

    /// REF-04: the "N" sits its inset back from the north arm's end.
    #[test]
    fn the_n_sits_inside_the_arm_end() {
        let arm = Stroke {
            start: Vec2::new(0.0, 100.0),
            dir: Vec2::NEG_Y,
            length: 80.0,
            alpha_start: 1.0,
            alpha_end: 0.45,
            style: StrokeStyle::Dotted {
                dot_px: 2.0,
                pitch_px: 5.0,
            },
            ticks: Vec::new(),
            tick_px: 9.0,
        };
        assert_eq!(north_label_at(&arm, 13.0), Some(Vec2::new(0.0, 33.0)));
        assert_eq!(north_label_at(&arm, 90.0), None);
    }
}

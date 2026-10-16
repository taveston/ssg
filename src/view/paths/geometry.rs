//! Where a path lies and how its line is broken up (PLNR-60, PLNR-61), as
//! plain functions of the rail, the day and the camera.
//!
//! A path is sampled along its rail by day, finer where the camera is
//! close, so it stays within half a pixel of the true curve as an orbit
//! ring does (MRK-05). It splits at today into the part flown and the part
//! to come. Dashes and dots are laid along the line on screen, so they
//! keep their length in pixels at any zoom and run on unbroken from one
//! piece of the line to the next.

use bevy::prelude::*;

use crate::view::rings::{RingView, clip_to_front};

/// How many pieces a stretch of rail starts as, enough for a path seen
/// from afar.
const BASE_PIECES: usize = 64;
/// How many times a piece may be halved near the camera: as for rings,
/// far more than any zoom needs, and a bound on the work.
const MAX_SPLITS: u32 = 12;

/// One sampled point of a path: where it is in render space and the day
/// the ship is there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathPoint {
    pub world: Vec3,
    pub day: f64,
}

/// PLNR-60 / MRK-05: points along a rail from day `from` to day `to`,
/// with `at` giving a day's point in render space. A piece whose middle
/// strays from its chord by more than `view.max_error_rad`, as seen from
/// the camera, is halved, so the line stays within half a pixel of the
/// true path at any zoom. Days `at` can't place are left out.
pub fn sample_rail(
    at: &impl Fn(f64) -> Option<Vec3>,
    (from, to): (f64, f64),
    view: RingView,
) -> Vec<PathPoint> {
    let mut points = Vec::new();
    if !(to > from) {
        return points;
    }
    let step = (to - from) / BASE_PIECES as f64;
    let first = PathPoint {
        world: match at(from) {
            Some(world) => world,
            None => return points,
        },
        day: from,
    };
    points.push(first);
    let mut last = first;
    for i in 1..=BASE_PIECES {
        let day = if i == BASE_PIECES {
            to
        } else {
            from + step * i as f64
        };
        let Some(world) = at(day) else {
            continue;
        };
        let next = PathPoint { world, day };
        split_piece(at, (last, next), view, 0, &mut points);
        last = next;
    }
    points
}

/// Adds the points of one piece after its start, halving it while its
/// middle strays too far from its chord.
fn split_piece(
    at: &impl Fn(f64) -> Option<Vec3>,
    (a, b): (PathPoint, PathPoint),
    view: RingView,
    depth: u32,
    out: &mut Vec<PathPoint>,
) {
    let mid_day = (a.day + b.day) / 2.0;
    if depth < MAX_SPLITS
        && let Some(mid) = at(mid_day)
    {
        let gap = mid.distance((a.world + b.world) / 2.0);
        let distance = mid.distance(view.camera_pos).max(f32::EPSILON);
        if gap / distance > view.max_error_rad {
            let middle = PathPoint {
                world: mid,
                day: mid_day,
            };
            split_piece(at, (a, middle), view, depth + 1, out);
            split_piece(at, (middle, b), view, depth + 1, out);
            return;
        }
    }
    out.push(b);
}

/// PLNR-61: a path split at `today` into the part the ship has flown and
/// the part it has yet to fly. The split point is shared by both.
pub fn split_at_day(points: &[PathPoint], today: f64) -> (Vec<PathPoint>, Vec<PathPoint>) {
    let mut flown = Vec::new();
    let mut ahead = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if b.day <= today {
            push_once(&mut flown, a);
            flown.push(b);
        } else if a.day >= today {
            push_once(&mut ahead, a);
            ahead.push(b);
        } else {
            let t = ((today - a.day) / (b.day - a.day)) as f32;
            let cut = PathPoint {
                world: a.world.lerp(b.world, t),
                day: today,
            };
            push_once(&mut flown, a);
            flown.push(cut);
            ahead.push(cut);
            ahead.push(b);
        }
    }
    (flown, ahead)
}

fn push_once(points: &mut Vec<PathPoint>, point: PathPoint) {
    if points.last() != Some(&point) {
        points.push(point);
    }
}

/// A straight piece of line in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub from: Vec3,
    pub to: Vec3,
}

/// PLNR-61: the solid pieces of a polyline, each cut at the camera's near
/// limit (MRK-05).
pub fn solid_pieces(points: &[PathPoint], view: RingView) -> Vec<Piece> {
    points
        .windows(2)
        .filter_map(|pair| clip_to_front((pair[0].world, pair[1].world), view))
        .map(|(from, to)| Piece { from, to })
        .collect()
}

/// PLNR-61: a dash pattern laid along a line on screen: `on` pixels drawn,
/// then `off` pixels gap, starting with a dash, carried on from one piece
/// of the line to the next.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dashes {
    pub on: f32,
    pub off: f32,
    /// How far into the pattern the next piece starts, in pixels.
    pub phase: f32,
}

impl Dashes {
    pub fn new(on: f32, off: f32) -> Self {
        Self {
            on,
            off,
            phase: 0.0,
        }
    }

    /// The drawn stretches of the next piece of line, `length` pixels
    /// long on screen, as fractions of it from its start.
    pub fn walk(&mut self, length: f32) -> Vec<(f32, f32)> {
        let period = self.on + self.off;
        if length <= 0.0 || period <= 0.0 || self.on <= 0.0 {
            return Vec::new();
        }
        let mut stretches = Vec::new();
        let mut at = 0.0;
        while at < length {
            let into = self.phase % period;
            let (drawn, left) = if into < self.on {
                (true, self.on - into)
            } else {
                (false, period - into)
            };
            let end = (at + left).min(length);
            if drawn {
                stretches.push((at / length, end / length));
            }
            self.phase = (self.phase + end - at) % period;
            at = end;
        }
        stretches
    }
}

/// PLNR-61: the dashes of a polyline: each chord is cut at the camera's
/// near limit, measured on screen with `project`, and the pattern laid
/// along it. A chord that can't be placed on screen breaks the pattern
/// nowhere; it is skipped.
pub fn dashed_pieces(
    points: &[PathPoint],
    view: RingView,
    mut dashes: Dashes,
    project: impl Fn(Vec3) -> Option<Vec2>,
) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for pair in points.windows(2) {
        let Some((from, to)) = clip_to_front((pair[0].world, pair[1].world), view) else {
            continue;
        };
        let (Some(a), Some(b)) = (project(from), project(to)) else {
            continue;
        };
        for (t0, t1) in dashes.walk(a.distance(b)) {
            pieces.push(Piece {
                from: from.lerp(to, t0),
                to: from.lerp(to, t1),
            });
        }
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> RingView {
        RingView {
            camera_pos: Vec3::new(0.0, 100.0, 0.0),
            camera_forward: Vec3::NEG_Y,
            max_error_rad: 0.0005,
        }
    }

    /// A circle of radius 50 round the origin, a quarter turn per 10 days.
    fn circle(day: f64) -> Option<Vec3> {
        let angle = (day / 40.0 * std::f64::consts::TAU) as f32;
        Some(Vec3::new(50.0 * angle.cos(), 0.0, 50.0 * angle.sin()))
    }

    /// PLNR-60 / MRK-05: the sampled path runs from its first day to its
    /// last, in order, and every chord's middle stays within the allowed
    /// error of the true curve as seen from the camera.
    #[test]
    fn a_sampled_path_follows_its_rail_within_half_a_pixel() {
        let view = view();
        let points = sample_rail(&circle, (0.0, 30.0), view);
        assert_eq!(points.first().unwrap().day, 0.0);
        assert_eq!(points.last().unwrap().day, 30.0);
        assert!(points.windows(2).all(|p| p[0].day < p[1].day));
        for pair in points.windows(2) {
            let mid_day = (pair[0].day + pair[1].day) / 2.0;
            let truth = circle(mid_day).unwrap();
            let chord = (pair[0].world + pair[1].world) / 2.0;
            let error = truth.distance(chord) / truth.distance(view.camera_pos);
            assert!(error <= view.max_error_rad * 1.01, "{error}");
        }
        assert!(sample_rail(&circle, (5.0, 5.0), view).is_empty());
    }

    /// PLNR-61: the path splits at today into the flown part and the part
    /// to come, which meet at today's point.
    #[test]
    fn a_path_splits_at_today() {
        let points: Vec<PathPoint> = (0..=4)
            .map(|i| PathPoint {
                world: Vec3::X * i as f32,
                day: i as f64 * 10.0,
            })
            .collect();
        let (flown, ahead) = split_at_day(&points, 15.0);
        assert_eq!(flown.first().unwrap().day, 0.0);
        assert_eq!(flown.last().unwrap().day, 15.0);
        assert_eq!(flown.last().unwrap().world, Vec3::X * 1.5);
        assert_eq!(ahead.first(), flown.last());
        assert_eq!(ahead.last().unwrap().day, 40.0);
        let (flown, ahead) = split_at_day(&points, -1.0);
        assert!(flown.is_empty());
        assert_eq!(ahead, points);
        let (flown, ahead) = split_at_day(&points, 50.0);
        assert_eq!(flown, points);
        assert!(ahead.is_empty());
    }

    /// PLNR-61: dashes are `on` pixels long with `off` pixel gaps on
    /// screen, and the pattern carries on across pieces of the line.
    #[test]
    fn dashes_keep_their_length_across_pieces() {
        let mut dashes = Dashes::new(5.0, 4.0);
        // 20 px: dash 0-5, gap 5-9, dash 9-14, gap 14-18, dash 18-20.
        let first = dashes.walk(20.0);
        assert_eq!(first, vec![(0.0, 0.25), (0.45, 0.7), (0.9, 1.0)]);
        // The next piece finishes the dash begun at 18: 3 px more.
        let second = dashes.walk(10.0);
        assert_eq!(second[0], (0.0, 0.3));
        assert!((second[1].0 - 0.7).abs() < 1e-6);
        assert!(Dashes::new(5.0, 4.0).walk(0.0).is_empty());
    }

    /// PLNR-61: dashed pieces lie on the line, each `on` pixels long on
    /// screen, `off` apart.
    #[test]
    fn dashed_pieces_measure_their_dashes_on_screen() {
        let points = [
            PathPoint {
                world: Vec3::ZERO,
                day: 0.0,
            },
            PathPoint {
                world: Vec3::X * 10.0,
                day: 1.0,
            },
        ];
        // Ten pixels on screen per render unit.
        let project = |p: Vec3| Some(Vec2::new(p.x * 10.0, p.z * 10.0));
        let pieces = dashed_pieces(&points, view(), Dashes::new(5.0, 4.0), project);
        assert_eq!(pieces.len(), 12);
        for piece in &pieces[..11] {
            assert!((piece.from.distance(piece.to) * 10.0 - 5.0).abs() < 1e-3);
        }
        let gap = pieces[1].from.distance(pieces[0].to) * 10.0;
        assert!((gap - 4.0).abs() < 1e-3);
    }

    /// MRK-05: a piece behind the camera is cut at its near limit, not
    /// drawn through it.
    #[test]
    fn pieces_behind_the_camera_are_cut() {
        let points = [
            PathPoint {
                world: Vec3::new(0.0, 50.0, 0.0),
                day: 0.0,
            },
            PathPoint {
                world: Vec3::new(0.0, 150.0, 0.0),
                day: 1.0,
            },
        ];
        let pieces = solid_pieces(&points, view());
        assert_eq!(pieces.len(), 1);
        assert!(pieces[0].to.y < 100.0);
    }
}

//! A trajectory: a list of segments, each a path evaluated by day, laid
//! end to end so that any day in the trajectory's span reads from exactly
//! one of them (RAIL-01, RAIL-04).
//!
//! The crate's own paths (circular orbits, conics, spirals) all implement
//! [`Rail`], and so does [`Path`], an enum of the three. A caller with its
//! own kind of segment implements `Rail` for it and uses
//! `Trajectory<ItsKind>`.

use crate::circular::CircularOrbit;
use crate::conic::Conic;
use crate::spiral::Spiral;
use crate::state::State;

/// Something that has a position and velocity on any day: a path a body
/// follows. The crate's paths implement it, and so can a caller's own
/// segment kinds.
///
/// ```
/// use glam::DVec3;
/// use orbits::{Rail, State};
///
/// // A beacon that never moves.
/// struct Fixed(DVec3);
/// impl Rail for Fixed {
///     fn state_at(&self, _day: f64) -> State {
///         State::new(self.0, DVec3::ZERO)
///     }
/// }
/// let beacon = Fixed(DVec3::new(1.0, 0.0, 0.0));
/// assert_eq!(beacon.state_at(42.0).position.x, 1.0);
/// ```
pub trait Rail {
    /// The position (km) and velocity (km/day) on `day`, relative to the
    /// primary the path goes round.
    fn state_at(&self, day: f64) -> State;
}

/// One of the crate's paths: a circular orbit, a conic (an ellipse or
/// hyperbola) or a low-thrust spiral. A ready-made segment kind for a
/// [`Trajectory`].
///
/// ```
/// use orbits::{CircularOrbit, Path, Rail};
///
/// let orbit = CircularOrbit { radius_km: 1_000.0, period_days: 10.0, phase0_rad: 0.0 };
/// let path = Path::Circular(orbit);
/// assert_eq!(path.state_at(0.0).position, orbit.position_at(0.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Path {
    /// A circular orbit.
    Circular(CircularOrbit),
    /// An ellipse or hyperbola.
    Conic(Conic),
    /// A low-thrust spiral between two circular orbits.
    Spiral(Spiral),
}

impl Rail for Path {
    fn state_at(&self, day: f64) -> State {
        match self {
            Path::Circular(orbit) => orbit.state_at(day),
            Path::Conic(conic) => conic.state_at(day),
            Path::Spiral(spiral) => spiral.state_at(day),
        }
    }
}

/// A stretch of a trajectory: the path followed from `start_day` until
/// `end_day`, or for ever if `end_day` is `None`. A segment owns its start
/// day but not its end day, which belongs to the next segment.
///
/// ```
/// use orbits::{CircularOrbit, Path, Segment};
///
/// let orbit = CircularOrbit { radius_km: 1_000.0, period_days: 10.0, phase0_rad: 0.0 };
/// let parked = Segment { start_day: 0.0, end_day: Some(5.0), kind: Path::Circular(orbit) };
/// assert!(parked.contains(0.0));
/// assert!(parked.contains(4.999));
/// assert!(!parked.contains(5.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment<K> {
    /// The first day the segment is followed.
    pub start_day: f64,
    /// The day the segment hands over to the next one; `None` for the last
    /// segment, which goes on without end.
    pub end_day: Option<f64>,
    /// The path followed.
    pub kind: K,
}

impl<K> Segment<K> {
    /// Whether `day` falls in this segment's span: at or after the start,
    /// and before the end if there is one.
    ///
    /// ```
    /// use orbits::Segment;
    ///
    /// let open = Segment { start_day: 10.0, end_day: None, kind: () };
    /// assert!(!open.contains(9.0));
    /// assert!(open.contains(1e9));
    /// ```
    pub fn contains(&self, day: f64) -> bool {
        day >= self.start_day && self.end_day.is_none_or(|end| day < end)
    }
}

/// Why a list of segments does not make a trajectory.
///
/// ```
/// use orbits::{Trajectory, TrajectoryError};
///
/// let empty: Result<Trajectory<()>, _> = Trajectory::new(vec![]);
/// assert_eq!(empty.unwrap_err(), TrajectoryError::Empty);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrajectoryError {
    /// No segments at all.
    Empty,
    /// The last segment has an end day; it must go on without end.
    LastSegmentBounded,
    /// A segment before the last has no end day.
    UnboundedBeforeLast {
        /// The index of the offending segment.
        index: usize,
    },
    /// A segment ends before it starts.
    EndsBeforeStart {
        /// The index of the offending segment.
        index: usize,
    },
    /// A segment does not start on the day the previous one ends.
    Gap {
        /// The index of the segment that starts on the wrong day.
        index: usize,
    },
}

/// An ordered list of segments laid end to end: each starts on the day
/// the previous one ends, and the last goes on for ever. Reading the
/// trajectory on a day is a pure function of the two.
///
/// ```
/// use orbits::{CircularOrbit, Path, Rail, Segment, Trajectory};
///
/// let inner = CircularOrbit { radius_km: 1_000.0, period_days: 10.0, phase0_rad: 0.0 };
/// let outer = CircularOrbit { radius_km: 2_000.0, period_days: 30.0, phase0_rad: 0.0 };
/// let trajectory = Trajectory::new(vec![
///     Segment { start_day: 0.0, end_day: Some(5.0), kind: Path::Circular(inner) },
///     Segment { start_day: 5.0, end_day: None, kind: Path::Circular(outer) },
/// ])
/// .unwrap();
/// assert!((trajectory.state_at(2.0).unwrap().radius() - 1_000.0).abs() < 1e-9);
/// assert!((trajectory.state_at(5.0).unwrap().radius() - 2_000.0).abs() < 1e-9);
/// assert!(trajectory.state_at(-1.0).is_none());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Trajectory<K> {
    segments: Vec<Segment<K>>,
}

impl<K> Trajectory<K> {
    /// Builds a trajectory from segments, checking that they are
    /// contiguous and that only the last is unbounded.
    ///
    /// ```
    /// use orbits::{Segment, Trajectory, TrajectoryError};
    ///
    /// let gap = Trajectory::new(vec![
    ///     Segment { start_day: 0.0, end_day: Some(5.0), kind: () },
    ///     Segment { start_day: 6.0, end_day: None, kind: () },
    /// ]);
    /// assert_eq!(gap.unwrap_err(), TrajectoryError::Gap { index: 1 });
    /// ```
    pub fn new(segments: Vec<Segment<K>>) -> Result<Self, TrajectoryError> {
        let last = segments
            .len()
            .checked_sub(1)
            .ok_or(TrajectoryError::Empty)?;
        let mut previous_end = None;
        for (index, segment) in segments.iter().enumerate() {
            match segment.end_day {
                None if index != last => {
                    return Err(TrajectoryError::UnboundedBeforeLast { index });
                }
                Some(_) if index == last => return Err(TrajectoryError::LastSegmentBounded),
                Some(end) if end < segment.start_day => {
                    return Err(TrajectoryError::EndsBeforeStart { index });
                }
                _ => {}
            }
            if previous_end.is_some_and(|end| end != segment.start_day) {
                return Err(TrajectoryError::Gap { index });
            }
            previous_end = segment.end_day;
        }
        Ok(Self { segments })
    }

    /// The segments in order.
    ///
    /// ```
    /// use orbits::{Segment, Trajectory};
    ///
    /// let t = Trajectory::new(vec![Segment { start_day: 1.0, end_day: None, kind: () }]).unwrap();
    /// assert_eq!(t.segments().len(), 1);
    /// ```
    pub fn segments(&self) -> &[Segment<K>] {
        &self.segments
    }

    /// The first day the trajectory covers.
    ///
    /// ```
    /// use orbits::{Segment, Trajectory};
    ///
    /// let t = Trajectory::new(vec![Segment { start_day: 7.0, end_day: None, kind: () }]).unwrap();
    /// assert_eq!(t.start_day(), 7.0);
    /// ```
    pub fn start_day(&self) -> f64 {
        self.segments[0].start_day
    }

    /// The segment followed on `day`: the one whose span contains it, or
    /// `None` before the trajectory starts.
    ///
    /// ```
    /// use orbits::{Segment, Trajectory};
    ///
    /// let t = Trajectory::new(vec![
    ///     Segment { start_day: 0.0, end_day: Some(5.0), kind: 'a' },
    ///     Segment { start_day: 5.0, end_day: None, kind: 'b' },
    /// ])
    /// .unwrap();
    /// assert_eq!(t.segment_at(5.0).unwrap().kind, 'b');
    /// assert!(t.segment_at(-0.5).is_none());
    /// ```
    pub fn segment_at(&self, day: f64) -> Option<&Segment<K>> {
        self.segments.iter().find(|segment| segment.contains(day))
    }
}

impl<K: Rail> Trajectory<K> {
    /// The state on `day`, read from the one segment whose span contains
    /// it; `None` before the trajectory starts.
    ///
    /// ```
    /// use orbits::{CircularOrbit, Path, Segment, Trajectory};
    ///
    /// let orbit = CircularOrbit { radius_km: 500.0, period_days: 4.0, phase0_rad: 0.0 };
    /// let t = Trajectory::new(vec![Segment { start_day: 0.0, end_day: None, kind: Path::Circular(orbit) }]).unwrap();
    /// let state = t.state_at(1.0).unwrap();
    /// assert!(state.position.x.abs() < 1e-9);
    /// assert!((state.position.z - 500.0).abs() < 1e-9);
    /// ```
    pub fn state_at(&self, day: f64) -> Option<State> {
        self.segment_at(day)
            .map(|segment| segment.kind.state_at(day))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: f64, end: Option<f64>) -> Segment<()> {
        Segment {
            start_day: start,
            end_day: end,
            kind: (),
        }
    }

    /// Every malformed list is rejected with the reason (RAIL-01).
    #[test]
    fn malformed_segment_lists_are_rejected() {
        assert_eq!(
            Trajectory::new(vec![seg(0.0, Some(1.0))]).unwrap_err(),
            TrajectoryError::LastSegmentBounded
        );
        assert_eq!(
            Trajectory::new(vec![seg(0.0, None), seg(0.0, None)]).unwrap_err(),
            TrajectoryError::UnboundedBeforeLast { index: 0 }
        );
        assert_eq!(
            Trajectory::new(vec![seg(3.0, Some(2.0)), seg(2.0, None)]).unwrap_err(),
            TrajectoryError::EndsBeforeStart { index: 0 }
        );
        assert_eq!(
            Trajectory::new(vec![seg(0.0, Some(2.0)), seg(2.5, None)]).unwrap_err(),
            TrajectoryError::Gap { index: 1 }
        );
    }

    /// A segment owns its start day, not its end day, so the handover day
    /// reads from the later segment (RAIL-04).
    #[test]
    fn the_handover_day_belongs_to_the_later_segment() {
        let t = Trajectory::new(vec![
            seg(0.0, Some(2.0)),
            seg(2.0, Some(4.0)),
            seg(4.0, None),
        ])
        .unwrap();
        assert_eq!(t.segment_at(1.999).unwrap().start_day, 0.0);
        assert_eq!(t.segment_at(2.0).unwrap().start_day, 2.0);
        assert_eq!(t.segment_at(4.0).unwrap().start_day, 4.0);
        assert_eq!(t.segment_at(1e6).unwrap().start_day, 4.0);
        assert!(t.segment_at(-1e-9).is_none());
    }

    /// A zero-length segment is allowed and never owns a day.
    #[test]
    fn a_zero_length_segment_owns_no_day() {
        let t = Trajectory::new(vec![
            seg(0.0, Some(2.0)),
            seg(2.0, Some(2.0)),
            seg(2.0, None),
        ])
        .unwrap();
        assert_eq!(t.segments().len(), 3);
        assert!(std::ptr::eq(t.segment_at(2.0).unwrap(), &t.segments()[2]));
    }
}

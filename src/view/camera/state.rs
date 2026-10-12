//! Camera state: the rig (pivot, heading, tilt and distance), the animated
//! move in flight, and the bookmarks (ZOOM-01, ZOOM-02, ZOOM-10, ZOOM-11,
//! CAM-02, CAM-05). Shared by the input handlers and the rig systems.

use bevy::prelude::*;

use super::zoom::animated_distance;
use crate::view::anim::{MS_PER_SEC, ease_out_cubic};
use crate::view::focus::Focus;

/// Where the camera looks from. The pivot is a point on the orbital plane
/// in render units; heading and tilt are in radians, tilt from straight
/// down (`rules.rs` has the conventions); distance is from the pivot.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct CameraRig {
    pub pivot: Vec3,
    pub heading: f32,
    pub tilt: f32,
    pub distance: f32,
}

/// Replaced at startup by the system frame, once the stops have been
/// computed from the real orbits.
const PLACEHOLDER_DISTANCE: f32 = 300.0;

impl Default for CameraRig {
    /// CAM-02: before start-up applies the standard tilt, the rig is north
    /// up, over the star.
    fn default() -> Self {
        Self {
            pivot: Vec3::ZERO,
            heading: 0.0,
            tilt: 0.0,
            distance: PLACEHOLDER_DISTANCE,
        }
    }
}

/// Where a move takes the pivot: a fixed point, or a body (plus offset)
/// wherever it is when the move gets there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PivotTarget {
    Fixed(Vec3),
    Body { body: Entity, offset: Vec3 },
}

/// Where a move ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveTarget {
    pub pivot: PivotTarget,
    pub heading: f32,
    pub tilt: f32,
    pub distance: f32,
}

impl MoveTarget {
    /// A move that changes only the distance and pivot, keeping the view's
    /// heading and tilt (zoom steps, focus jumps).
    pub fn keeping_view(rig: &CameraRig, pivot: PivotTarget, distance: f32) -> Self {
        Self {
            pivot,
            heading: rig.heading,
            tilt: rig.tilt,
            distance,
        }
    }

    /// The focus a move leaves behind: on the body it follows, or none.
    pub fn focus(&self) -> Focus {
        match self.pivot {
            PivotTarget::Fixed(_) => Focus::default(),
            PivotTarget::Body { body, offset } => Focus {
                body: Some(body),
                offset,
            },
        }
    }
}

/// An animated move: a zoom step, a focus jump, Home or a bookmark. New
/// input retargets from the *current* rig rather than queueing (ZOOM-02).
#[derive(Debug, Clone, Copy)]
pub struct CameraMove {
    pub start: CameraRig,
    pub target: MoveTarget,
    pub elapsed: f32,
    pub duration: f32,
    /// For a move onto a body: where the pivot started, relative to that
    /// body, noted on the move's first frame. The pivot blends *this
    /// offset* into the target's, so the move rides along with the body's
    /// orbit instead of lerping from a stale world point.
    pub origin_offset: Option<Vec3>,
}

impl CameraMove {
    pub fn new(start: CameraRig, target: MoveTarget, duration_ms: f32) -> Self {
        Self {
            start,
            target,
            elapsed: 0.0,
            origin_offset: None,
            duration: (duration_ms / MS_PER_SEC).max(f32::EPSILON),
        }
    }

    pub fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }
}

/// The move in flight, if any.
#[derive(Resource, Default, Debug)]
pub struct ActiveMove(pub Option<CameraMove>);

/// ZOOM-01: the rig part-way through a move, eased out. Pivot and distance
/// share one eased factor, so a step anchored on a point (pivot and
/// distance scaled about it) keeps that point still throughout.
/// `target_pivot` is where the target pivot is *now*, so a move onto a
/// moving body arrives on it.
pub fn blend_rig(start: &CameraRig, target: &MoveTarget, target_pivot: Vec3, t: f32) -> CameraRig {
    let eased = ease_out_cubic(t);
    CameraRig {
        pivot: start.pivot.lerp(target_pivot, eased),
        heading: start.heading + (target.heading - start.heading) * eased,
        tilt: start.tilt + (target.tilt - start.tilt) * eased,
        // ZOOM-01's own step easing, the same curve as the rest.
        distance: animated_distance(start.distance, target.distance, t, 1.0),
    }
}

/// ZOOM-11: a saved view - the whole rig, and the focus with its offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bookmark {
    pub rig: CameraRig,
    pub focus: Focus,
}

/// ZOOM-11: Ctrl+1 to Ctrl+4.
pub const BOOKMARK_SLOTS: usize = 4;

#[derive(Resource, Default)]
pub struct Bookmarks(pub [Option<Bookmark>; BOOKMARK_SLOTS]);

/// Absolute safety rails, independent of the computed stops.
pub const MIN_DISTANCE: f32 = 0.002;
pub const MAX_DISTANCE: f32 = 5_000.0;

#[derive(Component)]
pub struct OrbitCamera;

pub fn camera_fov(projection: &Projection) -> f32 {
    match projection {
        Projection::Perspective(perspective) => perspective.fov,
        _ => std::f32::consts::FRAC_PI_4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig(pivot: Vec3, distance: f32) -> CameraRig {
        CameraRig {
            pivot,
            distance,
            ..default()
        }
    }

    /// The default rig is north up over the star; start-up adds the tilt.
    #[test]
    fn default_rig_is_north_up_over_the_star() {
        let start = CameraRig::default();
        assert_eq!(
            (start.heading, start.tilt, start.pivot),
            (0.0, 0.0, Vec3::ZERO)
        );
    }

    /// ZOOM-01: a move eases from the start to exactly the target.
    #[test]
    fn move_runs_from_start_to_target() {
        let start = rig(Vec3::ZERO, 100.0);
        let target = MoveTarget {
            pivot: PivotTarget::Fixed(Vec3::X),
            heading: 1.0,
            tilt: 0.5,
            distance: 50.0,
        };
        assert_eq!(blend_rig(&start, &target, Vec3::X, 0.0), start);
        let end = blend_rig(&start, &target, Vec3::X, 1.0);
        assert_eq!(end.pivot, Vec3::X);
        assert_eq!((end.heading, end.tilt, end.distance), (1.0, 0.5, 50.0));
    }

    /// ZOOM-13: a step scaled about an anchor keeps the anchor still all
    /// the way, because pivot and distance share one eased factor.
    #[test]
    fn anchor_stays_put_through_the_move() {
        let anchor = Vec3::new(10.0, 0.0, 0.0);
        let start = rig(Vec3::ZERO, 100.0);
        let k = 0.5;
        let target_pivot = anchor + (start.pivot - anchor) * k;
        let target = MoveTarget::keeping_view(&start, PivotTarget::Fixed(target_pivot), 100.0 * k);
        for t in [0.1, 0.4, 0.8] {
            let mid = blend_rig(&start, &target, target_pivot, t);
            // The pivot and distance are the start scaled about the anchor
            // by the same factor.
            let scale = mid.distance / start.distance;
            let expected = anchor + (start.pivot - anchor) * scale;
            assert!((mid.pivot - expected).length() < 1e-4, "at t = {t}");
        }
    }

    /// A move onto a body leaves it focused with the move's offset; a move
    /// to a fixed point leaves nothing focused.
    /// Covers CAM-06.
    #[test]
    fn moves_leave_the_matching_focus() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let start = CameraRig::default();
        let onto = MoveTarget::keeping_view(
            &start,
            PivotTarget::Body {
                body,
                offset: Vec3::X,
            },
            1.0,
        );
        assert_eq!(
            onto.focus(),
            Focus {
                body: Some(body),
                offset: Vec3::X
            }
        );
        let fixed = MoveTarget::keeping_view(&start, PivotTarget::Fixed(Vec3::ZERO), 1.0);
        assert_eq!(fixed.focus(), Focus::default());
    }
}

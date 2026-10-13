//! A frame body that turns into an exposure ring can't stay the frame
//! (CAM-24): the frame moves up to the nearest ancestor that isn't in ring
//! mode and the view moves as a frame jump to it would, or to the star's
//! frame at the system fit if no ancestor is left.

use bevy::prelude::*;

use super::control::CameraControl;
use super::fits::Fits;
use super::state::{MoveTarget, PivotTarget};
use crate::view::motion_mode::MotionModeState;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// CAM-24: the nearest ancestor of `body` (starting with its parent) that
/// isn't in ring mode. A body with no mode state, like the star, never is.
/// `None` when the chain runs out: the frame goes to the star.
pub fn nearest_unringed_ancestor(
    body: Entity,
    parent_of: impl Fn(Entity) -> Option<Entity>,
    in_ring_mode: impl Fn(Entity) -> bool,
) -> Option<Entity> {
    let mut current = parent_of(body);
    // The hierarchy is a tree; the bound only guards against a cycle.
    for _ in 0..64 {
        let candidate = current?;
        if !in_ring_mode(candidate) {
            return Some(candidate);
        }
        current = parent_of(candidate);
    }
    None
}

/// CAM-24: move the frame off a body in ring mode, with the move a frame
/// jump to its new frame body makes (ZOOM-16): the ancestor's satellite
/// fit, or the system fit over the star if no ancestor is left. Waits for a
/// move in flight to finish, and never moves the frame back when the body
/// leaves ring mode.
pub fn release_ring_mode_frame(
    tunables: Res<Tunables>,
    fits: Fits,
    parents: Query<&ChildOf>,
    modes: Query<&MotionModeState>,
    positions: Res<WorldPositions>,
    mut view: CameraControl,
) {
    let Some(body) = view.frame.non_star_body() else {
        return;
    };
    let ringed = |entity: Entity| modes.get(entity).is_ok_and(|mode| mode.ring_mode);
    if view.active.0.is_some() || !ringed(body) {
        return;
    }
    let parent_of = |entity: Entity| parents.get(entity).ok().map(ChildOf::parent);
    let tilt = view.rig.tilt;
    view.settle(&positions);
    let (body, distance) = match nearest_unringed_ancestor(body, parent_of, ringed) {
        Some(ancestor) => (ancestor, fits.jump_distance(ancestor, tilt)),
        None => (view.frame.star, fits.system_fit(tilt)),
    };
    let pivot = PivotTarget {
        body,
        offset: Vec3::ZERO,
    };
    let target = MoveTarget::keeping_view(&view.rig, pivot, distance);
    view.start(target, tunables.focus_jump_ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CAM-24: the first ancestor not in ring mode becomes the frame; if
    /// all are in ring mode, or there are none, the frame goes to the star.
    #[test]
    fn the_frame_moves_to_the_nearest_ancestor_not_in_ring_mode() {
        let mut world = World::new();
        let [moon, planet, grand] = [(); 3].map(|_| world.spawn_empty().id());
        let parent_of = |e: Entity| {
            if e == moon {
                Some(planet)
            } else if e == planet {
                Some(grand)
            } else {
                None
            }
        };
        assert_eq!(
            nearest_unringed_ancestor(moon, parent_of, |e| e == moon),
            Some(planet)
        );
        assert_eq!(
            nearest_unringed_ancestor(moon, parent_of, |e| e != grand),
            Some(grand)
        );
        assert_eq!(nearest_unringed_ancestor(moon, parent_of, |_| true), None);
        assert_eq!(nearest_unringed_ancestor(grand, parent_of, |_| false), None);
    }
}

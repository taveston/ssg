//! A focused object that turns into an exposure ring can't stay focused
//! (CAM-24): focus moves up to the nearest ancestor that isn't in ring mode
//! and the view moves to frame it, or focus ends and the view returns to the
//! system frame if none is.

use bevy::prelude::*;

use super::control::CameraControl;
use super::frames::Frames;
use super::state::{MoveTarget, PivotTarget};
use crate::view::focus::Focus;
use crate::view::motion_mode::MotionModeState;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// CAM-24: the nearest ancestor of `body` (starting with its parent) that
/// isn't in ring mode. A body with no mode state, like the star, never is.
/// `None` when the chain runs out: focus ends.
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

/// CAM-24: move focus off a body in ring mode, with a move that brings the
/// new focus back into view: the ancestor framed as a focus jump frames it
/// (ZOOM-16), or the system frame over the star if no ancestor is left.
/// Waits for a move in flight to finish, and never moves focus back when
/// the body leaves ring mode.
pub fn release_ring_mode_focus(
    tunables: Res<Tunables>,
    frames: Frames,
    parents: Query<&ChildOf>,
    modes: Query<&MotionModeState>,
    positions: Res<WorldPositions>,
    mut view: CameraControl,
) {
    let Some(body) = view.focus.body else {
        return;
    };
    let ringed = |entity: Entity| modes.get(entity).is_ok_and(|mode| mode.ring_mode);
    if view.active.0.is_some() || !ringed(body) {
        return;
    }
    let parent_of = |entity: Entity| parents.get(entity).ok().map(ChildOf::parent);
    let tilt = view.rig.tilt;
    view.settle(&positions);
    let target = match nearest_unringed_ancestor(body, parent_of, ringed) {
        Some(ancestor) => MoveTarget::keeping_view(
            &view.rig,
            PivotTarget::Body {
                body: ancestor,
                offset: Vec3::ZERO,
            },
            frames.framing(ancestor, tilt),
        ),
        None => {
            *view.focus = Focus::default();
            MoveTarget::keeping_view(
                &view.rig,
                PivotTarget::Fixed(Vec3::ZERO),
                frames.system(tilt),
            )
        }
    };
    view.start(target, tunables.focus_jump_ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CAM-24: the first ancestor not in ring mode takes the focus; if all
    /// are in ring mode, or there are none, focus ends.
    #[test]
    fn focus_moves_to_the_nearest_ancestor_not_in_ring_mode() {
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

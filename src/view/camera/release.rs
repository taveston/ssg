//! A focused object that turns into an exposure ring can't stay focused
//! (CAM-24): focus moves up to the nearest ancestor that isn't in ring mode,
//! or ends if none is, with the offset that keeps the view where it is.

use bevy::prelude::*;

use super::state::{ActiveMove, CameraRig};
use crate::view::focus::Focus;
use crate::view::motion_mode::MotionModeState;
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

/// CAM-24: move focus off a body in ring mode. Waits for a move in flight to
/// finish, and never moves focus back when the body leaves ring mode.
pub fn release_ring_mode_focus(
    active: Res<ActiveMove>,
    positions: Res<WorldPositions>,
    rig: Res<CameraRig>,
    parents: Query<&ChildOf>,
    modes: Query<&MotionModeState>,
    mut focus: ResMut<Focus>,
) {
    let Some(body) = focus.body else {
        return;
    };
    let ringed = |entity: Entity| modes.get(entity).is_ok_and(|mode| mode.ring_mode);
    if active.0.is_some() || !ringed(body) {
        return;
    }
    let parent_of = |entity: Entity| parents.get(entity).ok().map(ChildOf::parent);
    let next = nearest_unringed_ancestor(body, parent_of, ringed)
        .and_then(|ancestor| Some((ancestor, *positions.0.get(&ancestor)?)));
    *focus = match next {
        Some((ancestor, position)) => Focus {
            body: Some(ancestor),
            offset: rig.pivot - position,
        },
        None => Focus::default(),
    };
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

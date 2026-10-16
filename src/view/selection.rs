//! Selection: the one object whose inspection inset is open (INS-01,
//! INS-03). Shared state, read by everything that highlights the selected
//! object (ARCH-03); clicks in `hover.rs`, the inset's buttons in
//! `inset/inspect.rs` and a click on empty map space change it.
//!
//! Selection never moves the camera; the reference frame does (CAM-05).

use bevy::prelude::*;

/// The selected object, if any. There is at most one, and one inset
/// (INS-03).
#[derive(Resource, Default)]
pub struct Selection {
    pub selected: Option<Entity>,
    /// HOV-05: a named orbit of the selected object whose row the inset
    /// opens expanded, set by a click on that orbit's shared ring.
    pub revealed_orbit: Option<Entity>,
}

impl Selection {
    /// Whether `entity` is the selected object.
    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected == Some(entity)
    }

    /// INS-01 / INS-03: select a body; its inset replaces any other's.
    pub fn select(&mut self, body: Entity) {
        self.selected = Some(body);
        self.revealed_orbit = None;
    }

    /// HOV-05: a click on a shared orbit's ring selects the orbit's parent
    /// and opens its inset with that orbit's row expanded.
    pub fn select_through_orbit(&mut self, parent: Entity, orbit: Entity) {
        self.selected = Some(parent);
        self.revealed_orbit = Some(orbit);
    }

    /// INS-03: Close, Esc or a click on empty map space clear the
    /// selection and close the inset.
    pub fn clear(&mut self) {
        self.selected = None;
        self.revealed_orbit = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INS-03: there is one selection; selecting another object replaces
    /// it, and clearing leaves nothing selected.
    #[test]
    fn one_selection_replaced_in_place() {
        let mut world = World::new();
        let [a, b] = [(); 2].map(|_| world.spawn_empty().id());
        let mut selection = Selection::default();
        selection.select(a);
        assert!(selection.is_selected(a));
        selection.select(b);
        assert!(selection.is_selected(b) && !selection.is_selected(a));
        selection.clear();
        assert_eq!(selection.selected, None);
    }

    /// HOV-05: selecting through a shared orbit selects the parent and
    /// remembers the orbit to reveal; any other selection forgets it.
    #[test]
    fn selecting_through_an_orbit_reveals_it_until_the_next_selection() {
        let mut world = World::new();
        let [parent, orbit, other] = [(); 3].map(|_| world.spawn_empty().id());
        let mut selection = Selection::default();
        selection.select_through_orbit(parent, orbit);
        assert_eq!(
            (selection.selected, selection.revealed_orbit),
            (Some(parent), Some(orbit))
        );
        selection.select(other);
        assert_eq!(selection.revealed_orbit, None);
        selection.select_through_orbit(parent, orbit);
        selection.clear();
        assert_eq!(selection.revealed_orbit, None);
    }
}

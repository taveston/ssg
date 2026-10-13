//! Selection: the one object whose inspection inset is open (INS-01,
//! INS-03). Shared state, read by everything that highlights the selected
//! object (ARCH-03); clicks in `hover.rs`, the inset's buttons in
//! `inspect.rs` and a click on empty map space change it.
//!
//! Selection never moves the camera; the reference frame does (CAM-05).

use bevy::prelude::*;

/// The selected object, if any. There is at most one, and one inset
/// (INS-03).
#[derive(Resource, Default)]
pub struct Selection {
    pub selected: Option<Entity>,
}

impl Selection {
    /// Whether `entity` is the selected object.
    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected == Some(entity)
    }

    /// INS-01 / INS-03: select a body; its inset replaces any other's.
    pub fn select(&mut self, body: Entity) {
        self.selected = Some(body);
    }

    /// INS-03: Close, Esc or a click on empty map space clear the
    /// selection and close the inset.
    pub fn clear(&mut self) {
        self.selected = None;
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
}

//! Selection: which objects have an inspection inset open (INS-01, INS-03).
//! Shared state, read by everything that highlights a selected object
//! (ARCH-03); clicks in `hover.rs` and the inset's buttons in `inspect.rs`
//! change it.
//!
//! Selection never moves the camera; focus does (CAM-05, not yet built).

use bevy::prelude::*;

/// The open insets: the unpinned one that follows each new selection, and
/// any pinned for comparison (INS-03).
#[derive(Resource, Default)]
pub struct Selection {
    pub primary: Option<Entity>,
    pub pinned: Vec<Entity>,
}

impl Selection {
    /// Whether `entity` has an inset open, pinned or not.
    pub fn is_open(&self, entity: Entity) -> bool {
        self.primary == Some(entity) || self.pinned.contains(&entity)
    }

    /// INS-01: select a body, opening its inset in place of the unpinned
    /// one. A body that already has an inset (pinned or not) keeps it, so
    /// selecting a pinned body never opens a second copy.
    pub fn select(&mut self, body: Entity) {
        if !self.is_open(body) {
            self.primary = Some(body);
        }
    }

    /// INS-03: pin an inset so it stays open when the selection changes.
    pub fn pin(&mut self, body: Entity) {
        if self.primary == Some(body) {
            self.primary = None;
        }
        if !self.pinned.contains(&body) {
            self.pinned.push(body);
        }
    }

    pub fn unpin(&mut self, body: Entity) {
        self.pinned.retain(|&e| e != body);
    }

    /// Close a body's inset, pinned or not.
    pub fn close(&mut self, body: Entity) {
        if self.primary == Some(body) {
            self.primary = None;
        }
        self.unpin(body);
    }

    /// INS-03: Esc closes the unpinned inset and leaves pinned ones open.
    pub fn close_unpinned(&mut self) {
        self.primary = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bodies() -> (Entity, Entity) {
        let mut world = World::new();
        (world.spawn_empty().id(), world.spawn_empty().id())
    }

    /// INS-01: selecting a body opens its inset, replacing the unpinned one.
    #[test]
    fn select_replaces_the_unpinned_inset() {
        let (a, b) = bodies();
        let mut selection = Selection::default();
        selection.select(a);
        assert_eq!(selection.primary, Some(a));
        selection.select(b);
        assert_eq!(selection.primary, Some(b));
        assert!(!selection.is_open(a));
    }

    /// INS-03: a pinned inset stays open through a new selection, so two
    /// objects can be compared, and Esc only closes the unpinned one.
    #[test]
    fn pinned_inset_survives_selection_and_esc() {
        let (a, b) = bodies();
        let mut selection = Selection::default();
        selection.select(a);
        selection.pin(a);
        selection.select(b);
        assert!(selection.is_open(a) && selection.is_open(b));
        selection.close_unpinned();
        assert!(selection.is_open(a));
        assert!(!selection.is_open(b));
    }

    /// INS-03: selecting a pinned body doesn't open a second inset for it.
    #[test]
    fn selecting_a_pinned_body_keeps_one_inset() {
        let (a, _) = bodies();
        let mut selection = Selection::default();
        selection.select(a);
        selection.pin(a);
        selection.select(a);
        assert_eq!(selection.primary, None);
        assert_eq!(selection.pinned, vec![a]);
    }

    /// Covers INS-02.
    #[test]
    fn closing_or_unpinning_an_inset() {
        let (a, b) = bodies();
        let mut selection = Selection::default();
        selection.select(a);
        selection.pin(a);
        selection.select(b);
        selection.close(a);
        assert!(!selection.is_open(a));
        selection.close(b);
        assert!(!selection.is_open(b));
        selection.select(a);
        selection.pin(a);
        selection.unpin(a);
        assert!(!selection.is_open(a));
    }
}

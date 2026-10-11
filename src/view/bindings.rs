//! Every camera and clock key and mouse button, in one resource (CAM-18).
//!
//! No other module names a `KeyCode` or a `MouseButton` (checked by
//! `arch_*` in `tests/architecture.rs`): systems ask this resource whether
//! an action's binding is pressed. A rebinding screen is not yet specced;
//! when it is, it edits this resource and nothing else changes.

use bevy::prelude::*;

/// Which keys and buttons drive which action. Each action takes a list, so
/// an action can have alternatives (W and the up arrow, = and numpad +).
#[derive(Resource, Debug, Clone)]
pub struct Bindings {
    // --- Clock (CLK-05) ---
    pub pause: Vec<KeyCode>,
    pub rate_up: Vec<KeyCode>,
    pub rate_down: Vec<KeyCode>,

    // --- Zoom (CAM-16) ---
    pub zoom_in: Vec<KeyCode>,
    pub zoom_out: Vec<KeyCode>,

    // --- Camera moves ---
    /// ZOOM-10: back to the system frame.
    pub home: Vec<KeyCode>,
    /// ZOOM-16: focus the selection.
    pub focus_selection: Vec<KeyCode>,
    /// ZOOM-11: held with a bookmark slot key, saves instead of recalling.
    pub bookmark_save: Vec<KeyCode>,
    /// ZOOM-11: the bookmark slots, in order.
    pub bookmark_slots: Vec<KeyCode>,

    // --- Mouse ---
    /// Drag to rotate the camera.
    pub rotate_button: MouseButton,
    /// Drag to pan.
    pub pan_buttons: Vec<MouseButton>,

    // --- Inspection ---
    /// INS-03: close the unpinned inset.
    pub close_inset: Vec<KeyCode>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self {
            pause: vec![KeyCode::Space],
            rate_up: vec![KeyCode::Period],
            rate_down: vec![KeyCode::Comma],

            zoom_in: vec![KeyCode::Equal, KeyCode::NumpadAdd],
            zoom_out: vec![KeyCode::Minus, KeyCode::NumpadSubtract],

            home: vec![KeyCode::Home],
            focus_selection: vec![KeyCode::KeyF],
            bookmark_save: vec![KeyCode::ControlLeft, KeyCode::ControlRight],
            bookmark_slots: vec![
                KeyCode::Digit1,
                KeyCode::Digit2,
                KeyCode::Digit3,
                KeyCode::Digit4,
            ],

            rotate_button: MouseButton::Right,
            pan_buttons: vec![MouseButton::Middle],

            close_inset: vec![KeyCode::Escape],
        }
    }
}

/// Whether any of `bound` is held.
pub fn held<T>(input: &ButtonInput<T>, bound: &[T]) -> bool
where
    T: Copy + Eq + std::hash::Hash + Send + Sync + 'static,
{
    input.any_pressed(bound.iter().copied())
}

/// Whether any of `bound` went down this frame.
pub fn pressed<T>(input: &ButtonInput<T>, bound: &[T]) -> bool
where
    T: Copy + Eq + std::hash::Hash + Send + Sync + 'static,
{
    input.any_just_pressed(bound.iter().copied())
}

pub struct BindingsPlugin;

impl Plugin for BindingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bindings>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CLK-05: Space pauses, `.` and `,` change the rate, and + and - are
    /// no longer clock keys.
    #[test]
    fn clk_05_clock_keys() {
        let bindings = Bindings::default();
        assert_eq!(bindings.pause, vec![KeyCode::Space]);
        assert_eq!(bindings.rate_up, vec![KeyCode::Period]);
        assert_eq!(bindings.rate_down, vec![KeyCode::Comma]);
        for key in [KeyCode::Equal, KeyCode::Minus, KeyCode::NumpadAdd] {
            assert!(!bindings.rate_up.contains(&key) && !bindings.rate_down.contains(&key));
        }
    }

    /// CAM-18: actions are read through the bindings, alternatives included.
    #[test]
    fn cam_18_any_bound_key_triggers_the_action() {
        let mut input = ButtonInput::<KeyCode>::default();
        let bound = [KeyCode::KeyW, KeyCode::ArrowUp];
        assert!(!held(&input, &bound));
        input.press(KeyCode::ArrowUp);
        assert!(held(&input, &bound));
        assert!(pressed(&input, &bound));
        input.clear();
        assert!(held(&input, &bound), "still held");
        assert!(!pressed(&input, &bound), "but not newly pressed");
    }
}

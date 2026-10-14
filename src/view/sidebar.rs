//! The sidebar's state (INS-07): whether the inspection sidebar is open
//! and how far it has slid in. Shared state, read by everything that
//! lays out against the map area (ARCH-03): the camera's viewport, the
//! fits, the pan clamp, the edge tabs and the plot marks. The panel
//! itself is `inspect.rs`.
//!
//! The sidebar opens with a selection and closes with none; the slide
//! runs over `sidebar_slide_ms`, and while it runs the map area follows
//! it frame by frame, so the view narrows with the panel rather than
//! jumping.

use bevy::prelude::*;

use super::anim::{advance_towards, step_per_frame};
use super::selection::Selection;
use super::tunables::Tunables;
use crate::Stage;

/// How far the sidebar has slid in: 0 is hidden, 1 is fully open.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct SidebarState {
    pub slide: f32,
}

impl SidebarState {
    /// INS-07: how much of the window's right edge the sidebar takes this
    /// frame: its full width once open, less while it slides.
    pub fn width_px(&self, tunables: &Tunables) -> f32 {
        sidebar_width_px(self.slide, tunables.inset_width_px)
    }

    /// Whether anything of the sidebar is on screen.
    pub fn visible(&self) -> bool {
        self.slide > 0.0
    }
}

/// INS-07: the width the sidebar takes at `slide` of its travel.
pub fn sidebar_width_px(slide: f32, inset_width_px: f32) -> f32 {
    slide.clamp(0.0, 1.0) * inset_width_px
}

/// INS-07: the slide after one frame: towards open while something is
/// selected, towards hidden otherwise, over `sidebar_slide_ms`.
pub fn next_slide(slide: f32, open: bool, delta_secs: f32, slide_ms: f32) -> f32 {
    let target = if open { 1.0 } else { 0.0 };
    advance_towards(slide, target, step_per_frame(delta_secs, slide_ms))
}

pub struct SidebarPlugin;

impl Plugin for SidebarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SidebarState>()
            .add_systems(Update, slide_sidebar.in_set(Stage::Projection));
    }
}

/// INS-07: the sidebar slides in while something is selected and out
/// once nothing is, ahead of the camera, so this frame's map area is
/// already the narrower one.
fn slide_sidebar(
    time: Res<Time>,
    tunables: Res<Tunables>,
    selection: Res<Selection>,
    mut sidebar: ResMut<SidebarState>,
) {
    let next = next_slide(
        sidebar.slide,
        selection.selected.is_some(),
        time.delta_secs(),
        tunables.sidebar_slide_ms,
    );
    if next != sidebar.slide {
        sidebar.slide = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INS-07: the slide takes `sidebar_slide_ms` each way and never
    /// leaves 0..1; selecting another object while open changes nothing.
    #[test]
    fn the_sidebar_slides_in_and_out_over_its_duration() {
        let mut slide = 0.0;
        for _ in 0..10 {
            slide = next_slide(slide, true, 0.02, 200.0);
        }
        assert!((slide - 1.0).abs() < 1e-5);
        assert_eq!(next_slide(1.0, true, 0.5, 200.0), 1.0, "stays open");
        let half = next_slide(1.0, false, 0.1, 200.0);
        assert!((half - 0.5).abs() < 1e-5);
        assert_eq!(next_slide(0.0, false, 1.0, 200.0), 0.0);
        assert_eq!(
            next_slide(0.0, true, 1.0, 0.0),
            1.0,
            "a zero duration is instant"
        );
    }

    /// INS-07: the width the map loses follows the slide.
    #[test]
    fn the_width_follows_the_slide() {
        assert_eq!(sidebar_width_px(0.0, 320.0), 0.0);
        assert_eq!(sidebar_width_px(0.25, 320.0), 80.0);
        assert_eq!(sidebar_width_px(1.0, 320.0), 320.0);
        assert_eq!(sidebar_width_px(2.0, 320.0), 320.0);
        let state = SidebarState { slide: 0.5 };
        assert_eq!(state.width_px(&Tunables::default()), 160.0);
        assert!(state.visible() && !SidebarState::default().visible());
    }
}

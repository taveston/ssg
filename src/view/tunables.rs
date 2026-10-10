//! Every number the presentation spec marks *tunable*, in one place
//! (ARCH-22).
//!
//! The tunables table in `docs/spec/system-view.md` lists them with the
//! spec IDs that use them; `docs/ideas/presentation/presentation.md`
//! section 6 lists several as still-open design questions. They live
//! here, documented field by field, so they can be changed without hunting
//! through the systems that consume them.
//!
//! Consumed as an ordinary `Res<Tunables>` system parameter, the same way
//! `GameClock` and `Fonts` are.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct Tunables {
    // --- Satellite grouping ---
    /// A satellite folds into its parent once its orbit's on-screen radius
    /// drops below this. Lower than `unfold_at_px` on purpose: the gap is
    /// the hysteresis band that stops flicker at the boundary.
    pub fold_below_px: f32,
    /// R_show: a satellite is drawn individually once its orbit's on-screen
    /// radius reaches this.
    pub unfold_at_px: f32,
    /// How long a satellite takes to slide out from (or back into) its
    /// parent when it unfolds or folds.
    pub fold_unfold_anim_ms: f32,

    // --- Motion rule ---
    /// T_dot: at or above this apparent period (orbital period divided by
    /// the time rate, in real seconds) a satellite is drawn as a dot at its
    /// true position with a short trail.
    pub t_dot_secs: f32,
    /// T_ring: below this apparent period a satellite switches to
    /// exposure-ring mode. Between the two, whichever mode is already
    /// active is kept.
    pub t_ring_secs: f32,
    /// Cross-fade time when a satellite switches between dot and ring.
    pub motion_crossfade_ms: f32,
    /// How much of the orbit a dot-mode trail covers, in degrees of arc.
    pub trail_arc_degrees: f32,
    /// Line width of an exposure ring. The ordinary dim orbit ring is 1px.
    pub exposure_ring_width_px: f32,

    // --- Zoom and navigation ---
    /// Each wheel notch multiplies the view scale by this factor.
    pub zoom_step_factor: f32,
    /// Duration of one zoom step's ease-out animation.
    pub zoom_anim_ms: f32,
    /// Extra fraction of `zoom_anim_ms` spent on a step that lands on a
    /// stop, so the stop reads as slight resistance rather than a snag.
    pub zoom_stop_resistance: f32,
    /// On release, a continuous (trackpad) zoom settles to the nearest stop
    /// if it is within this many zoom steps. Waits for ZOOM-07, which is not
    /// built: continuous pinch input is deferred until there's hardware to
    /// test it on.
    #[allow(dead_code)]
    pub pinch_settle_steps: f32,
    /// Duration of the combined pan-and-zoom when framing a body's
    /// satellites (double-click, or the focus key).
    pub focus_jump_ms: f32,
    /// Fraction of the viewport half-height a framed radius should fill.
    /// Below 1.0 so a framed thing has a margin rather than touching the
    /// screen edge.
    pub zoom_fit_margin: f32,
    /// Fallback double-click window, used only if the picking backend's own
    /// click-count proves unreliable. Waits for ZOOM-16 and ZOOM-17, which
    /// are not built; the current code reads the backend's click count.
    #[allow(dead_code)]
    pub double_click_window_ms: f32,
}

impl Default for Tunables {
    fn default() -> Self {
        Self {
            fold_below_px: 20.0,
            unfold_at_px: 24.0,
            fold_unfold_anim_ms: 200.0,

            t_dot_secs: 25.0,
            t_ring_secs: 20.0,
            motion_crossfade_ms: 200.0,
            trail_arc_degrees: 30.0,
            exposure_ring_width_px: 3.0,

            zoom_step_factor: 2.0,
            zoom_anim_ms: 200.0,
            zoom_stop_resistance: 0.35,
            pinch_settle_steps: 1.0,
            focus_jump_ms: 400.0,
            zoom_fit_margin: 0.8,
            double_click_window_ms: 350.0,
        }
    }
}

pub struct TunablesPlugin;

impl Plugin for TunablesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tunables>();
    }
}

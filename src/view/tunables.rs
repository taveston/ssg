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
    /// An orbit's tail covers the arc swept in this many real seconds; an
    /// apparent period (orbital period over the time rate) shorter than
    /// this closes the tail into the exposure ring (MOT-01, MOT-02,
    /// MOT-10).
    pub tail_secs: f32,
    /// As a tail grows from this many degrees to a full circle, the dot
    /// fades out and the ring widens to the exposure width; paused, tails
    /// are capped here (MOT-11, MOT-12).
    pub dot_fade_from_degrees: f32,
    /// A tail's shown length eases to a new length (after a change of rate,
    /// a pause or an unpause) within this many milliseconds (MOT-13).
    pub tail_ease_ms: f32,
    /// Line width of an exposure ring (MOT-02).
    pub exposure_ring_width_px: f32,
    /// An exposure ring's identity mark and label fade in once its tail has
    /// closed, and out when it opens, over this many milliseconds (MOT-14).
    pub identity_mark_fade_ms: f32,
    /// Alpha of an exposure ring and of a tail's head, so an exposure ring
    /// never matches the full-white selected ring (MOT-01, MOT-02).
    pub exposure_ring_alpha: f32,
    /// Line width of a normal orbit ring (MRK-05, MOT-11).
    pub orbit_ring_width_px: f32,
    /// The alpha of a faint orbit ring in the orbit colour, and of a tail's
    /// far end (MRK-05, MOT-01).
    pub orbit_ring_alpha: f32,
    /// Line width of a hovered or selected orbit ring (HOV-03).
    pub orbit_highlight_width_px: f32,
    /// An orbit ring starts to fade out once its on-screen radius passes
    /// this many times half the map area's diagonal (MRK-14).
    pub orbit_fade_from: f32,
    /// ...and is gone, not drawn at all, at this many times (MRK-14).
    pub orbit_fade_to: f32,

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
    /// The inner zoom limit with nothing focused: the view radius at the
    /// closest, in AU (ZOOM-14).
    pub min_view_radius_au: f32,
    /// The inner zoom limit with a body focused: its true diameter spans
    /// this fraction of the map area's height (ZOOM-14).
    pub body_max_screen_fraction: f32,
    /// The double-click window. Double-clicks use the picking backend's
    /// click count, and since the outline selects on a single click at
    /// once (INS-05) nothing reads this any more (F-148).
    #[allow(dead_code)]
    pub double_click_window_ms: f32,

    // --- Camera (section 5a). Tilt is measured from straight down. ---
    /// The steepest tilt from straight down, in degrees: the camera never
    /// goes below the orbital plane (CAM-01).
    pub max_pitch_deg: f32,
    /// A press and release that moves less than this is a click, never a
    /// drag (CAM-10, CAM-13).
    pub drag_threshold_px: f32,
    /// Degrees of heading or tilt per pixel of right drag (CAM-12).
    pub rotate_deg_per_px: f32,
    /// Key panning speed, as a fraction of the visible height at the pivot
    /// per second, so it looks the same at every zoom and tilt (CAM-14,
    /// CAM-17).
    pub key_pan_screen_per_s: f32,
    /// Time for key panning to ramp up to speed, and back down (CAM-14).
    pub key_pan_ramp_ms: f32,
    /// Holding Shift multiplies the key panning speed by this (CAM-14).
    pub key_pan_fast_factor: f32,
    /// Heading and tilt speed for the rotate keys, in degrees per second
    /// (CAM-15).
    pub key_rotate_deg_per_s: f32,
    /// Whether a cursor at the window edge pans the view. Off by default
    /// (CAM-17).
    pub edge_scroll_enabled: bool,
    /// How close to a window edge the cursor must be to edge-scroll
    /// (CAM-17).
    pub edge_scroll_px: f32,
    /// The standard orientation's tilt from straight down, in degrees
    /// (CAM-02, CAM-23).
    pub standard_tilt_deg: f32,
    /// While focused, panning keeps the focused body at least this far
    /// inside the map area's edges (CAM-07).
    pub focus_keep_margin_px: f32,
    /// The pivot stays within this many system-frame radii of the star
    /// (CAM-19).
    pub pan_bound_factor: f32,
    /// Space between the frame brackets and the selection ring's size, on
    /// each side (CAM-22).
    pub focus_bracket_margin_px: f32,

    // --- Hover, selection and the inset ---
    /// Every marker and symbol can be hit at least this far from its
    /// centre, even where it is drawn smaller (MRK-08).
    pub min_hit_radius_px: f32,
    /// A pointer this close to a drawn orbit ring hits it (HOV-02).
    pub ring_hit_px: f32,
    /// A hovered object's orbit ring is drawn white at this alpha (HOV-03).
    pub orbit_hover_alpha: f32,
    /// The selected object's orbit ring is drawn white at this alpha
    /// (HOV-03).
    pub orbit_selected_alpha: f32,
    /// The inspection sidebar's width (INS-07).
    pub inset_width_px: f32,
    /// How long the sidebar takes to slide open or closed (INS-07).
    pub sidebar_slide_ms: f32,
    /// The gap between the map area's edges and the scale bar and north
    /// dial (MRK-16, MRK-17).
    pub plot_margin_px: f32,
    /// The scale bar is the longest round distance that fits in this many
    /// pixels (MRK-16).
    pub scale_bar_max_px: f32,
    /// The north dial's diameter (MRK-17).
    pub north_dial_px: f32,
    /// The length of the north dial's filled head, tip to base (MRK-17).
    pub north_head_px: f32,
    /// Distances from this many AU up read in AU; shorter ones in km
    /// (UNIT-01).
    pub distance_au_threshold_au: f64,

    // --- Natural body markers ---
    /// How much of the neutral colour is mixed into the ground colour for
    /// a true-size disc's opaque fill (MRK-11).
    pub body_fill_mix: f32,
    /// The width of a natural body's rim, on markers and true-size discs
    /// (MRK-11, MRK-13).
    pub body_rim_px: f32,
    /// The ground-coloured gap round a natural body's rim, which keeps
    /// overlapping discs apart (MRK-11).
    pub body_gap_px: f32,
    /// The star's marker circle, in pixels across (MRK-03, MRK-13).
    pub star_marker_px: f32,
    /// A planet's marker circle, in pixels across (MRK-03, MRK-13).
    pub planet_marker_px: f32,
    /// A moon's marker circle, in pixels across (MRK-03, MRK-13).
    pub moon_marker_px: f32,
    /// How many radial ticks ring the star's marker (MRK-13).
    pub star_tick_count: u32,
    /// The length of each star tick, outside the rim (MRK-13).
    pub star_tick_px: f32,
    /// The diameter of a planet marker's centre dot (MRK-13).
    pub planet_dot_px: f32,

    // --- Ship and station symbols ---
    /// A symbol's frame fits a square this many pixels across, at every
    /// zoom (SYM-04, SYM-14).
    pub ship_symbol_px: f32,
    /// A symbol's size in lists such as the inset's "In orbit" (INS-05),
    /// and the parent's marker in its edge tab (MRK-15).
    pub list_symbol_px: f32,
    /// A symbol's outline width (SYM-02).
    pub symbol_stroke_px: f32,
    /// The outline width of secondary buttons and edge tabs (BTN-01,
    /// EDGE-03).
    pub button_stroke_px: f32,
    /// How much of a symbol's colour is mixed into the ground colour for
    /// the opaque inside of objects the state commands (SYM-03).
    pub symbol_fill_state: f32,
    /// The same mix for every other affiliation but own-flag civilians,
    /// whose inside is plain ground (SYM-03).
    pub symbol_fill_other: f32,
    /// A stale track's symbol is drawn at this opacity (SYM-08).
    pub stale_symbol_alpha: f32,

    // --- The sidebar ---
    /// The sidebar's running text, facts' values and outline entries
    /// (INS-05, INS-09).
    pub sidebar_text_px: f32,
    /// The sidebar's small spaced capitals: fact labels and section
    /// headings (INS-09).
    pub sidebar_caps_px: f32,
    /// The sidebar's header, the selection's name (INS-02).
    pub sidebar_header_px: f32,
    /// A natural body's marker in the outline, in pixels across (INS-05).
    pub list_marker_px: f32,

    // --- Labels ---
    /// A label sits this far from its marker's edge (LBL-01).
    pub label_gap_px: f32,
    /// A label's change of place or visibility fades over this (LBL-04).
    pub label_fade_ms: f32,
}

impl Default for Tunables {
    fn default() -> Self {
        Self {
            fold_below_px: 20.0,
            unfold_at_px: 24.0,
            fold_unfold_anim_ms: 200.0,

            tail_secs: 8.0,
            dot_fade_from_degrees: 300.0,
            tail_ease_ms: 500.0,
            exposure_ring_width_px: 3.0,
            identity_mark_fade_ms: 150.0,
            exposure_ring_alpha: 0.5,
            orbit_ring_width_px: 1.0,
            orbit_ring_alpha: 0.35,
            orbit_highlight_width_px: 1.0,
            orbit_fade_from: 1.5,
            orbit_fade_to: 3.0,

            zoom_step_factor: 2.0,
            zoom_anim_ms: 200.0,
            zoom_stop_resistance: 0.35,
            pinch_settle_steps: 1.0,
            focus_jump_ms: 400.0,
            zoom_fit_margin: 0.8,
            min_view_radius_au: 0.05,
            body_max_screen_fraction: 0.25,
            double_click_window_ms: 350.0,

            max_pitch_deg: 85.0,
            drag_threshold_px: 4.0,
            rotate_deg_per_px: 0.25,
            key_pan_screen_per_s: 0.6,
            key_pan_ramp_ms: 120.0,
            key_pan_fast_factor: 3.0,
            key_rotate_deg_per_s: 90.0,
            edge_scroll_enabled: false,
            edge_scroll_px: 8.0,
            standard_tilt_deg: 60.0,
            focus_keep_margin_px: 40.0,
            pan_bound_factor: 1.5,
            focus_bracket_margin_px: 2.0,

            min_hit_radius_px: 10.0,
            ring_hit_px: 7.0,
            orbit_hover_alpha: 0.6,
            orbit_selected_alpha: 0.85,
            inset_width_px: 320.0,
            sidebar_slide_ms: 200.0,
            plot_margin_px: 24.0,
            scale_bar_max_px: 160.0,
            north_dial_px: 34.0,
            north_head_px: 10.0,
            distance_au_threshold_au: 0.1,

            body_fill_mix: 0.32,
            body_rim_px: 1.0,
            body_gap_px: 2.0,
            star_marker_px: 16.0,
            planet_marker_px: 12.0,
            moon_marker_px: 8.0,
            star_tick_count: 8,
            star_tick_px: 3.0,
            planet_dot_px: 3.0,

            ship_symbol_px: 22.0,
            list_symbol_px: 14.0,
            symbol_stroke_px: 1.5,
            button_stroke_px: 1.0,
            symbol_fill_state: 0.30,
            symbol_fill_other: 0.16,
            stale_symbol_alpha: 0.55,

            sidebar_text_px: 13.0,
            sidebar_caps_px: 11.0,
            sidebar_header_px: 17.0,
            list_marker_px: 10.0,

            label_gap_px: 6.0,
            label_fade_ms: 150.0,
        }
    }
}

pub struct TunablesPlugin;

impl Plugin for TunablesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tunables>();
    }
}

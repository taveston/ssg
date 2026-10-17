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
    /// A parent's moon ring sits this far right of and above its marker
    /// (GRP-10).
    pub moon_ring_offset_px: f32,
    /// The size of the "+N" moon count under a parent's name, in the data
    /// face (GRP-11).
    pub moon_count_px: f32,
    /// How strongly the "+N" moon count is drawn, in the secondary text
    /// colour (GRP-11).
    pub moon_count_alpha: f32,
    /// The object pip's size: a copy of the fold marker's front frame
    /// this many pixels across (GRP-12).
    pub object_pip_px: f32,
    /// The pip's centre sits this far beyond the corner of its parent's
    /// box, to the right and below (GRP-12).
    pub object_pip_gap_px: f32,
    /// The back pip sits this far right of and above the pip (GRP-12).
    pub pip_back_offset_px: f32,
    /// The pip pops out into the fold marker while the pointer is within
    /// this many pixels of its parent's centre, and nearer to it than to
    /// any other parent's (GRP-13).
    pub pop_reach_px: f32,
    /// Once out, the fold marker stays out while the pointer is within
    /// this many pixels of its parent's centre; the gap from
    /// `pop_reach_px` stops it flickering at the edge (GRP-13).
    pub pop_release_px: f32,
    /// How long the pip takes to grow into the fold marker, or shrink
    /// back (GRP-13).
    pub pop_anim_ms: f32,

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
    /// The direction chevron beside an exposure ring's identity mark is an
    /// open arrowhead this many pixels across (MOT-14).
    pub chevron_px: f32,
    /// Space between an identity mark's edge and its direction chevron,
    /// along the ring, in pixels (MOT-14).
    pub chevron_gap_px: f32,
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

    // --- Belt ---
    /// The belt band's alpha along its centre line; it falls to nothing at
    /// both edges (MRK-18).
    pub belt_band_alpha: f32,
    /// How many faint points make the belt's grain (MRK-18).
    pub belt_speck_count: usize,
    /// The alpha of each of the belt's grain points (MRK-18).
    pub belt_speck_alpha: f32,

    // --- Frame cue (REF) ---
    /// The frame cue's arms are strokes of round dots this wide (REF-01)...
    pub ref_dot_px: f32,
    /// ...with their centres this far apart along the arm (REF-01).
    pub ref_dot_pitch_px: f32,
    /// An arm starts this far beyond the frame body's box, and the
    /// vertical axis this far beyond it when the body is a marker (REF-02,
    /// REF-06).
    pub ref_gap_px: f32,
    /// A stroke of the cue fades from this opacity at its start... (REF-03)
    pub ref_alpha_near: f32,
    /// ...to this at its end (REF-03).
    pub ref_alpha_far: f32,
    /// The three arms other than north are drawn at this times the north
    /// arm's strength (REF-03).
    pub ref_minor_factor: f32,
    /// The north arm's "N" sits this far inside the arm's end (REF-04).
    pub ref_n_inset_px: f32,
    /// A tick across an arm is this long (REF-05).
    pub ref_tick_px: f32,
    /// The vertical axis's upper half is this fraction of the map area's
    /// height on screen when the plane is seen edge-on, so it follows the
    /// window and the inset, and shortens with the view (REF-06).
    pub ref_up_fraction: f32,
    /// The upper half ends in a filled arrowhead this long (REF-06).
    pub ref_arrow_px: f32,
    /// The lower half is this fraction of the upper half's length
    /// (REF-07).
    pub ref_down_fraction: f32,
    /// A half of the vertical axis shorter than this on screen is not
    /// drawn (REF-08).
    pub ref_vertical_min_px: f32,

    // --- Zoom and navigation ---
    /// Each wheel notch multiplies the view scale by this factor.
    pub zoom_step_factor: f32,
    /// Duration of one zoom step's ease-out animation.
    pub zoom_anim_ms: f32,
    /// Extra fraction of `zoom_anim_ms` spent on a step that lands on a
    /// stop, so the stop reads as slight resistance rather than a snag.
    pub zoom_stop_resistance: f32,
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
    /// The hover mark is the selection's corner brackets drawn at this
    /// alpha (HOV-06).
    pub bracket_hover_alpha: f32,
    /// Each bracket arm is this share of its box's shorter side long
    /// (HOV-06).
    pub bracket_arm_fraction: f32,
    /// A bracket arm is never longer than this, so the corners stay small
    /// round a close disc's large box (HOV-06).
    pub bracket_arm_max_px: f32,
    /// Width of a bracket arm's stroke (HOV-06).
    pub bracket_stroke_px: f32,
    /// The bracket box clears what is drawn by this much on every side
    /// (HOV-07).
    pub bracket_margin_px: f32,
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
    /// the opaque inside of objects the faction commands (SYM-03).
    pub symbol_fill_faction: f32,
    /// The same mix for every other affiliation but own-flag civilians,
    /// whose inside is plain ground (SYM-03).
    pub symbol_fill_other: f32,
    /// A stale track's symbol is drawn at this opacity (SYM-08).
    pub stale_symbol_alpha: f32,
    /// An own ship's condition bar turns orange below this condition
    /// (SYM-12).
    pub condition_warn_fraction: f32,
    /// The space between a symbol's box (HOV-07) and the amplifiers
    /// outside it: a group's count, a task force's bracket and the
    /// condition bar (SYM-09, SYM-12).
    pub amplifier_gap_px: f32,
    /// How much of the condition bar's colour is mixed into the ground for
    /// its unfilled part, so a part-full bar shows its whole length
    /// (SYM-12).
    pub condition_track_mix: f32,

    // --- Several objects on one marker ---
    /// Two symbols whose on-screen centres come closer than this join one
    /// multi-marker (MUL-05).
    pub cluster_join_px: f32,
    /// Joined symbols part once their centres are farther apart than this;
    /// between the two they keep their state, so a pair at the edge
    /// doesn't flicker (MUL-05).
    pub cluster_split_px: f32,
    /// A multi-marker's back frame sits this far right of and above its
    /// front frame (MUL-01).
    pub back_frame_offset_px: f32,
    /// The size of a multi-marker's count, in the data face (MUL-01).
    pub multi_count_px: f32,
    /// A tray is never narrower than this, and otherwise as wide as its
    /// widest row (MUL-08).
    pub tray_min_width_px: f32,
    /// The height of each row of a tray (MUL-08).
    pub tray_row_px: f32,
    /// The height of a tray's header, which says what the marker holds
    /// (MUL-08).
    pub tray_header_px: f32,
    /// The height of a tray's footer, with the key hint (MUL-08).
    pub tray_footer_px: f32,
    /// The space between a tray and its marker's box (HOV-07), on the
    /// side the tray opens (MUL-08).
    pub tray_gap_px: f32,

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

    // --- Map and top bar text ---
    /// A name on the map: a marker's, an exposure ring's, an edge tab's,
    /// and a path's or ghost's label (MRK-19).
    pub map_label_px: f32,
    /// A second line under a name on the map (a marker's, an exposure
    /// ring's period), the scale bar and the "N" of the north dial and
    /// the frame cue (MRK-19).
    pub map_second_line_px: f32,
    /// The top bar's breadcrumb and date (MRK-19).
    pub bar_text_px: f32,

    // --- Labels ---
    /// A label sits this far from the edge of its marker's box (HOV-07),
    /// so it doesn't move when the brackets appear (LBL-01).
    pub label_gap_px: f32,
    /// A label's change of place or visibility fades over this (LBL-04).
    pub label_fade_ms: f32,
    /// Every label has an outline this wide round its glyphs, in the
    /// ground colour, so a line it crosses breaks round the letters
    /// (LBL-05).
    pub label_halo_px: f32,

    // --- Transfer planner (docs/spec/planner.md) ---
    /// A slider's track is this high (PLNR-23).
    pub slider_track_px: f32,
    /// A slider's thumb is this wide (PLNR-23).
    pub slider_thumb_w_px: f32,
    /// A slider's thumb is this high (PLNR-23).
    pub slider_thumb_h_px: f32,
    /// The dimmest an affordable setting is drawn on a track: own blue
    /// mixed into the empty colour at this, for a setting that needs all
    /// the ship has (PLNR-28).
    pub track_dim_mix: f32,
    /// A setting over budget is drawn on a track in the warning colour at
    /// this opacity (PLNR-28).
    pub track_over_alpha: f32,
    /// The selected option of a two-way choice is filled with the neutral
    /// colour mixed into the ground at this (PLNR-14).
    pub segment_fill_mix: f32,
    /// The Delta-v bar is this high (PLNR-27).
    pub delta_bar_px: f32,
    /// A label's tooltip shows after the pointer has rested on it this
    /// long (PLNR-91, PLNR-94).
    pub tooltip_delay_ms: f32,

    // --- Transfer paths on the map (docs/spec/planner.md section 6) ---
    /// A path's line width while it is planned, selected or hovered
    /// (PLNR-61, PLNR-62).
    pub path_width_px: f32,
    /// The part of a path the ship has yet to fly is dashed: dashes this
    /// long on screen... (PLNR-61)
    pub path_dash_px: f32,
    /// ...with gaps this long between them (PLNR-61).
    pub path_gap_px: f32,
    /// The dot that marks where a path starts is this wide (PLNR-61).
    pub departure_dot_px: f32,
    /// The coast after an Intercept is a dotted line this wide (PLNR-61,
    /// PLNR-67)...
    pub consequence_width_px: f32,
    /// ...of dots this long on screen... (PLNR-61)
    pub consequence_dot_px: f32,
    /// ...this far apart (PLNR-61).
    pub consequence_gap_px: f32,
    /// A ship's path in flight, while the ship is neither hovered nor
    /// selected, is drawn at this opacity... (PLNR-62)
    pub traffic_path_alpha: f32,
    /// ...and this wide (PLNR-62).
    pub traffic_path_width_px: f32,
    /// A ghost, an object's marker where it will be, is drawn at this
    /// opacity, times its object's dot opacity (PLNR-63, PLNR-64).
    pub ghost_alpha: f32,
    /// A ghost's dashed outline is this wide... (PLNR-63)
    pub ghost_width_px: f32,
    /// ...with dashes this long and as far apart (PLNR-63).
    pub ghost_dash_px: f32,

    // --- The sky (docs/spec/sky.md) ---
    /// The sky's seed while no sector gives one (SKY-01, SKY-02).
    pub sky_default_seed: u64,
    /// How many field stars the sky holds, scattered over the whole
    /// sphere (SKY-04, SKY-05).
    pub sky_field_stars: usize,
    /// How many stars make up the band, the Milky Way (SKY-04, SKY-06).
    pub sky_band_stars: usize,
    /// The brightest a field star can be, as a magnitude: smaller is
    /// brighter, each step of 1 about 2.5 times fainter (SKY-05).
    pub sky_mag_min: f32,
    /// The faintest a field star can be, as a magnitude (SKY-05).
    pub sky_mag_max: f32,
    /// How steeply field-star counts climb towards the faint end: the
    /// number at magnitude m goes as 10 to the power (slope × m) (SKY-05).
    pub sky_mag_slope: f32,
    /// The brightest a band star can be, as a magnitude (SKY-06).
    pub sky_band_mag_min: f32,
    /// The faintest a band star can be, as a magnitude (SKY-06).
    pub sky_band_mag_max: f32,
    /// How steeply band-star counts climb towards the faint end, as
    /// `sky_mag_slope` (SKY-06).
    pub sky_band_mag_slope: f32,
    /// The spread of band stars either side of the band's centre line: the
    /// standard deviation of their latitude, in degrees (SKY-06).
    pub sky_band_sigma_deg: f32,
    /// How far along the band its dense core reaches: the standard
    /// deviation, in degrees of longitude from the core, of the extra
    /// density of stars and glow there (SKY-06, SKY-13).
    pub sky_core_sigma_deg: f32,
    /// How dense the band is far from the core, as a fraction of its
    /// density at the core (SKY-06, SKY-13).
    pub sky_core_floor: f32,
    /// The least angle between the band and the orbital plane, in degrees
    /// (SKY-07).
    pub sky_tilt_min_deg: f32,
    /// The greatest angle between the band and the orbital plane, in
    /// degrees (SKY-07).
    pub sky_tilt_max_deg: f32,
    /// The band's centre line passes within this many degrees of the
    /// standard view's line of sight, so a new game opens with the band on
    /// screen (SKY-08).
    pub sky_band_view_margin_deg: f32,
    /// The sky tone: the neutral cool white the glow is drawn in and star
    /// colours are paled towards (SKY-12, SKY-13).
    pub sky_tone: Srgba,
    /// The opacity of the brightest possible star (SKY-09).
    pub sky_peak: f32,
    /// How fast opacity falls with magnitude: a star of magnitude m is
    /// drawn at `sky_peak` × 10 to the power (−0.4 × gamma × m) (SKY-09).
    pub sky_gamma: f32,
    /// The least opacity any star is drawn at, so the faintest stay
    /// visible (SKY-09).
    pub sky_faint_floor: f32,
    /// The softness of the faintest star: the standard deviation of its
    /// Gaussian, in screen pixels (SKY-10).
    pub sky_sigma_px: f32,
    /// How much wider a star is drawn per magnitude it is brighter than
    /// `sky_bright_mag`, in screen pixels (SKY-10).
    pub sky_sigma_bright_px: f32,
    /// Stars fainter than this magnitude are all drawn at `sky_sigma_px`;
    /// brighter ones grow (SKY-10).
    pub sky_bright_mag: f32,
    /// How far, as a fraction of its mean, the total light one star
    /// leaves on the screen may vary as it moves between pixels (SKY-11).
    pub sky_flicker_tolerance: f32,
    /// The colour of a bluish-white star, before it is paled (SKY-12).
    pub sky_type_blue: Srgba,
    /// The colour of a white star, before it is paled (SKY-12).
    pub sky_type_white: Srgba,
    /// The colour of a yellow-white star, before it is paled (SKY-12).
    pub sky_type_yellow: Srgba,
    /// The colour of an orange star, before it is paled (SKY-12).
    pub sky_type_orange: Srgba,
    /// How far a star's colour moves from the sky tone towards its type's
    /// colour: 0 is the tone, 1 the full colour (SKY-12).
    pub sky_colour_keep: f32,
    /// No star colour is more saturated than this (HSV saturation), so
    /// none reads as an affiliation colour or the warning orange (SKY-12).
    pub sky_max_saturation: f32,
    /// The glow's opacity at its brightest: on the band's centre line, at
    /// the core (SKY-13).
    pub sky_glow_alpha: f32,
    /// How fast the glow falls off across the band: the standard deviation
    /// of its normal curve, in degrees from the centre line (SKY-13).
    pub sky_glow_sigma_deg: f32,
    /// How much a dark lane can take from the glow, as a fraction of it
    /// (SKY-14).
    pub sky_lane_depth: f32,
    /// How much the glow's mottle can raise or lower it, as a fraction of
    /// it (SKY-14).
    pub sky_glow_mottle: f32,
    /// The most the glow's dither moves a pixel, either way, in display
    /// levels (of 255) (SKY-15).
    pub sky_dither_levels: f32,

    // --- The sector view (docs/spec/sector-view.md) ---
    /// A gated system's node: a star marker this many pixels across
    /// (SECT-03).
    pub sector_node_px: f32,
    /// An ungated system's node: a plain circle outline this many pixels
    /// across (SECT-03).
    pub sector_dot_px: f32,
    /// How strongly an ungated system's node is drawn (SECT-03).
    pub sector_dot_alpha: f32,
    /// How strongly a brown dwarf's node is drawn: the same circle as an
    /// ungated system's, fainter (SECT-03).
    pub sector_faint_alpha: f32,
    /// A working link's line width (SECT-04).
    pub link_working_px: f32,
    /// How strongly a working link is drawn (SECT-04).
    pub link_working_alpha: f32,
    /// How strongly a dormant link's dashes are drawn (SECT-04).
    pub link_dormant_alpha: f32,
    /// The length of a dormant link's dashes, on screen (SECT-04).
    pub link_dash_px: f32,
    /// The gap between a dormant link's dashes, on screen (SECT-04).
    pub link_gap_px: f32,
    /// A collapsed link's line width (SECT-04).
    pub link_collapsed_px: f32,
    /// How strongly a collapsed link is drawn (SECT-04).
    pub link_collapsed_alpha: f32,
    /// The sector view's inner zoom limit: the view radius, in light
    /// years, closer than which the camera doesn't zoom (SECT-11).
    pub sector_min_view_radius_ly: f32,
}

impl Default for Tunables {
    fn default() -> Self {
        Self {
            fold_below_px: 20.0,
            unfold_at_px: 24.0,
            fold_unfold_anim_ms: 200.0,
            moon_ring_offset_px: 3.0,
            moon_count_px: 9.0,
            moon_count_alpha: 0.7,
            object_pip_px: 6.0,
            object_pip_gap_px: 2.0,
            pip_back_offset_px: 3.0,
            pop_reach_px: 24.0,
            pop_release_px: 32.0,
            pop_anim_ms: 120.0,

            tail_secs: 8.0,
            dot_fade_from_degrees: 300.0,
            tail_ease_ms: 500.0,
            exposure_ring_width_px: 3.0,
            identity_mark_fade_ms: 150.0,
            chevron_px: 8.0,
            chevron_gap_px: 8.0,
            exposure_ring_alpha: 0.5,
            orbit_ring_width_px: 1.0,
            orbit_ring_alpha: 0.35,
            orbit_highlight_width_px: 1.0,
            orbit_fade_from: 4.0,
            orbit_fade_to: 12.0,

            belt_band_alpha: 0.10,
            belt_speck_count: 1200,
            belt_speck_alpha: 0.5,

            ref_dot_px: 2.0,
            ref_dot_pitch_px: 5.0,
            ref_gap_px: 8.0,
            ref_alpha_near: 1.0,
            ref_alpha_far: 0.45,
            ref_minor_factor: 0.6,
            ref_n_inset_px: 13.0,
            ref_tick_px: 9.0,
            ref_up_fraction: 0.45,
            ref_arrow_px: 8.0,
            ref_down_fraction: 0.7,
            ref_vertical_min_px: 10.0,

            zoom_step_factor: 2.0,
            zoom_anim_ms: 200.0,
            zoom_stop_resistance: 0.35,
            focus_jump_ms: 400.0,
            zoom_fit_margin: 0.8,
            min_view_radius_au: 0.05,
            body_max_screen_fraction: 0.25,

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

            min_hit_radius_px: 10.0,
            ring_hit_px: 7.0,
            orbit_hover_alpha: 0.6,
            orbit_selected_alpha: 0.85,
            bracket_hover_alpha: 0.6,
            bracket_arm_fraction: 0.3,
            bracket_arm_max_px: 10.0,
            bracket_stroke_px: 2.0,
            bracket_margin_px: 4.0,
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
            symbol_fill_faction: 0.30,
            symbol_fill_other: 0.16,
            stale_symbol_alpha: 0.55,
            condition_warn_fraction: 0.5,
            amplifier_gap_px: 1.0,
            condition_track_mix: 0.25,

            cluster_join_px: 24.0,
            cluster_split_px: 28.0,
            back_frame_offset_px: 4.0,
            multi_count_px: 10.5,
            tray_min_width_px: 240.0,
            tray_row_px: 26.0,
            tray_header_px: 28.0,
            tray_footer_px: 26.0,
            tray_gap_px: 8.0,

            sidebar_text_px: 13.0,
            sidebar_caps_px: 11.0,
            sidebar_header_px: 17.0,
            list_marker_px: 10.0,

            map_label_px: 12.0,
            map_second_line_px: 10.0,
            bar_text_px: 14.0,

            label_gap_px: 4.0,
            label_fade_ms: 150.0,
            label_halo_px: 2.0,

            slider_track_px: 10.0,
            slider_thumb_w_px: 14.0,
            slider_thumb_h_px: 18.0,
            track_dim_mix: 0.35,
            track_over_alpha: 0.45,
            segment_fill_mix: 0.30,
            delta_bar_px: 8.0,
            tooltip_delay_ms: 400.0,

            path_width_px: 1.5,
            path_dash_px: 5.0,
            path_gap_px: 4.0,
            departure_dot_px: 5.0,
            consequence_width_px: 1.2,
            consequence_dot_px: 2.0,
            consequence_gap_px: 4.0,
            traffic_path_alpha: 0.35,
            traffic_path_width_px: 1.0,
            ghost_alpha: 0.5,
            ghost_width_px: 1.0,
            ghost_dash_px: 2.0,

            sky_default_seed: 1,
            sky_field_stars: 40_000,
            sky_band_stars: 110_000,
            sky_mag_min: 0.0,
            sky_mag_max: 8.6,
            sky_mag_slope: 0.42,
            sky_band_mag_min: 2.5,
            sky_band_mag_max: 9.2,
            sky_band_mag_slope: 0.5,
            sky_band_sigma_deg: 9.0,
            sky_core_sigma_deg: 40.0,
            sky_core_floor: 0.4,
            sky_tilt_min_deg: 45.0,
            sky_tilt_max_deg: 75.0,
            sky_band_view_margin_deg: 10.0,
            sky_tone: Srgba::rgb_u8(0xC7, 0xD6, 0xE0),
            sky_peak: 0.55,
            sky_gamma: 0.22,
            sky_faint_floor: 0.04,
            sky_sigma_px: 0.55,
            sky_sigma_bright_px: 0.17,
            sky_bright_mag: 3.2,
            sky_flicker_tolerance: 0.05,
            sky_type_blue: Srgba::rgb_u8(0xA8, 0xC7, 0xFF),
            sky_type_white: Srgba::rgb_u8(0xE6, 0xF0, 0xFF),
            sky_type_yellow: Srgba::rgb_u8(0xFF, 0xF2, 0xD1),
            sky_type_orange: Srgba::rgb_u8(0xFF, 0xC7, 0x94),
            sky_colour_keep: 0.4,
            sky_max_saturation: 0.25,
            sky_glow_alpha: 0.085,
            sky_glow_sigma_deg: 10.0,
            sky_lane_depth: 0.6,
            sky_glow_mottle: 0.25,
            sky_dither_levels: 1.0,

            sector_node_px: 14.0,
            sector_dot_px: 5.0,
            sector_dot_alpha: 0.5,
            sector_faint_alpha: 0.3,
            link_working_px: 1.5,
            link_working_alpha: 0.9,
            link_dormant_alpha: 0.7,
            link_dash_px: 6.0,
            link_gap_px: 4.0,
            link_collapsed_px: 1.0,
            link_collapsed_alpha: 0.2,
            sector_min_view_radius_ly: 3.0,
        }
    }
}

pub struct TunablesPlugin;

impl Plugin for TunablesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tunables>();
    }
}

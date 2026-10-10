# Spec: System view

*Scope:* the strategic vector display of one star system, as built in the proof-of-concept iterations. *Source ideas:* `docs/ideas/presentation/presentation.md` (section 2), `docs/ideas/presentation/presentation-sketches.md` (boards 1, 2, 5, 6), mock-ups in `docs/ideas/presentation/mockups/`. Where a mock-up and this spec disagree, this spec wins.

*How to read it:* every requirement has a stable ID (`ZOOM-03`). IDs are never reused or renumbered; a withdrawn item stays in place marked ~~struck~~ with a note. *Since* says which iteration first built it. Numbers in `code` are tunables (table at the end); they live in `src/tunables.rs` and must never be hard-coded. Checks for each item are in `docs/validation/system-view.md`.

---

## 1. Data and scale

| ID | Requirement | Since |
|---|---|---|
| SCL-01 | Simulation data is stored in real units (km, kg, days) as `f64`, and converted to render space in one place. | it1 |
| SCL-02 | Positions and orbits are drawn at true scale, with one linear scale for planetary and satellite orbits alike. Only markers are stylised, never geometry. | it2 |
| SCL-03 | The system is data (a tree of body records) that a generator can replace without changing rendering code. | it2 |
| SCL-04 | Orbits are circular and coplanar, with periods from Kepler's third law using the parent's mass. | it1 |

## 2. Clock and top bar

| ID | Requirement | Since |
|---|---|---|
| CLK-01 | The game clock runs continuously at a default of 1 game day per real second. | it1 |
| CLK-02 | The top bar shows the breadcrumb ("Cluster / Corvane system"), the calendar date, and Pause, 1 d/s and 5 d/s buttons; the active rate is highlighted. | it2 |
| CLK-03 | ~~Space toggles pause; + doubles and − halves the rate (minimum 1/16 d/s).~~ Withdrawn 2026-10-10: + and − now zoom (CAM-16); replaced by CLK-05. | it1 |
| CLK-05 | Space toggles pause; `.` doubles and `,` halves the rate (minimum 1/16 d/s). | not built |
| CLK-04 | Pausing stops all orbital motion; every view stays fully interactive while paused. | it1 |

## 3. Markers and palette

| ID | Requirement | Since |
|---|---|---|
| MRK-01 | Palette: ground #070A0E; neutral bodies and text #DCE6EC; orbits and secondary text #9FB6C4; own assets #5AA9E6 (blue); foreign contacts and warnings #F0A040 (orange); selection and hover as a white ring. | it2 |
| MRK-02 | Type: Barlow for labels and UI; JetBrains Mono for dates, figures and status readouts. | it2 |
| MRK-03 | Each object is a fixed-size screen marker by class, projected from its 3D position each frame; marker size never depends on physical size. | it2 |
| MRK-04 | Marker shapes: filled disc = body; hollow square = station; filled triangle = own ship; filled diamond = foreign contact; X = salvage site. | it2 |
| MRK-05 | Orbits are faint rings; the belt is a translucent shaded band. | it2 |
| MRK-06 | Separate planets are never merged, however close on screen. | it3 |
| EDGE-01 | A gate outside the viewport is shown as an arrow pinned to the screen edge in its direction, labelled with its destination. | it2 |

## 4. Hover and inspection

| ID | Requirement | Since |
|---|---|---|
| HOV-01 | Hovering a marker draws a white ring and a tooltip: name · ownership · contents summary — "Click: inspect · Double-click: frame its moons · Double-click again: planetary view". | it2, text it3 |
| INS-01 | Clicking a marker selects it and opens an inspection inset anchored to the marker by a leader line; the clock keeps running. | it2 |
| INS-02 | The inset shows: header (name, class and ownership line), Focus, Pin and Close buttons; an image for the knowledge state (INS-04); key facts; and an "Open planetary view" button, disabled until the planetary view exists. | it2 |
| INS-03 | Pin keeps an inset open when the selection changes, so two objects can be compared; Esc closes the unpinned inset. | it2 |
| INS-04 | The inset image follows knowledge: **surveyed** = clean render, status "SURVEYED · *date*"; **unsurveyed** = grainy telescope frame, "TELESCOPE FRAME · UNSURVEYED" in orange, estimates as ranges; **stale** = sepia image, "LAST IMAGED · *date*". | it2 |
| INS-05 | The inset of a body with satellites has an "In orbit" list: one entry per satellite (class marker and name, own and foreign colours per MRK-01), in orbit order from the inside out. Clicking an entry selects that satellite (its inset replaces the parent's unless pinned); double-clicking it behaves as ZOOM-16 and ZOOM-17. | not built |
| INS-06 | When the selected object is folded into its parent (GRP-01), its inset's leader line goes to the parent's pill, and the pill carries the white selection ring. When the object unfolds, both move to its own marker without a jump. | not built |

## 5. Navigation and zoom

| ID | Requirement | Since |
|---|---|---|
| ZOOM-01 | Each wheel notch multiplies the view scale by `zoom_step_factor`, animated over `zoom_anim_ms` with ease-out. Raw wheel deltas are never used. | it3 |
| ZOOM-02 | New input during a zoom animation retargets from the current scale; it is not queued. | it3 |
| ZOOM-03 | ~~Zoom is anchored on the cursor, or on the selected object when there is one.~~ Withdrawn 2026-10-10: replaced by ZOOM-13, which adds the focused object and tilt. | it3 |
| ZOOM-04 | ~~Stops are computed per system from its real orbits: **system frame** (all planetary orbits fit; gates stay as edge indicators), **region frame** (the inner system, or around the selected planet's orbit), **satellite frame** (the selected planet's satellites fit).~~ Withdrawn 2026-10-10 (selection no longer moves the camera; focus does): replaced by ZOOM-15. | it3 |
| ZOOM-05 | A zoom step that lands on a stop takes longer by `zoom_stop_resistance`, then settles exactly on the stop. | it3 |
| ZOOM-06 | ~~Hard limits: outer = system frame; inner = satellite frame of the selected body, or of the body nearest the cursor when nothing is selected. Scrolling past a limit does nothing and never changes layer.~~ Withdrawn 2026-10-10 (selection no longer moves the camera; focus does): replaced by ZOOM-14. | it3 |
| ZOOM-07 | Trackpad pinch is continuous, and on release settles to the nearest stop if within `pinch_settle_steps`. | not built |
| ZOOM-08 | ~~The first double-click on a body inspects it and frames its satellites in one pan-and-zoom of `focus_jump_ms`; F does the same for the current selection.~~ Withdrawn 2026-10-10 (selection no longer moves the camera; focus does): replaced by ZOOM-16. | it3 |
| ZOOM-09 | ~~A double-click on an already-inspected body is the planetary-view trigger. Until that view exists it calls a clearly named hook and does nothing visible.~~ Withdrawn 2026-10-10 ("already inspected" was ambiguous): replaced by ZOOM-17. | it3 |
| ZOOM-10 | Home returns to the system frame with the same animated move. | it3 |
| ZOOM-11 | Ctrl+1 to Ctrl+4 save a bookmark (scale and centre, or the focused object); 1 to 4 recall it with the same animated move. | it3 |
| ZOOM-12 | ~~Right drag tilts the camera; middle drag pans.~~ Withdrawn 2026-10-10: replaced by section 5a, CAM-10 to CAM-13. | it1 |
| ZOOM-13 | Zoom is anchored on the focused object when there is one (CAM-05); otherwise on the point where the cursor's ray meets the orbital plane, or the screen centre for keyboard zoom (CAM-16). Selection never sets the anchor. The anchor keeps its screen position through the zoom. | not built |
| ZOOM-14 | Hard limits: outer = system frame; inner = satellite frame of the focused body, or of the body nearest the zoom anchor when nothing is focused. Scrolling past a limit does nothing and never changes layer. | not built |
| ZOOM-15 | Stops are computed per system from its real orbits: **system frame** (all planetary orbits fit; gates stay as edge indicators), **region frame** (the inner system, or around the focused planet's orbit), **satellite frame** (the focused planet's satellites fit, or those of the body nearest the zoom anchor when nothing is focused). | not built |
| ZOOM-16 | Double-click on a body that is not focused selects it, focuses it (CAM-06) and frames its satellites in one pan-and-zoom of `focus_jump_ms`. F does the same for the current selection, and the inset's Focus button for the inset's object. | not built |
| ZOOM-17 | Double-click on the focused body selects it and is the planetary-view trigger. Until that view exists it calls a clearly named hook and does nothing visible. | not built |

## 5a. Camera

The camera looks at a **pivot**, a point on the orbital plane, from a distance set by the zoom scale. Its orientation is a **heading** (rotation about the plane's normal) and a **tilt** (0° = looking straight down). **North** is a fixed direction in the orbital plane, the same for every system. There are two separate states. **Selection** (INS-01) is the object whose details are shown; it never moves the camera. **Focus** is the object the camera is locked onto: the camera tracks it as it moves. Selection and focus may be different objects (for example, focus on Halden IV while IVc is selected).

| ID | Requirement | Since |
|---|---|---|
| CAM-01 | Heading is unlimited. Tilt is clamped between 0° and `max_pitch_deg`; the camera never goes below the orbital plane. | not built |
| CAM-02 | The start state is tilt 0° with north at the top of the screen. | not built |
| CAM-03 | N animates to tilt 0° and north up over `focus_jump_ms`, keeping the pivot and the scale. | not built |
| CAM-04 | Home (ZOOM-10) also returns to tilt 0° and north up, in the same animated move. | not built |
| CAM-05 | While an object is focused, the pivot is recomputed every frame as the object's position plus a pan offset, so the object keeps its place on screen at any time rate. The offset does not rotate with the object's orbit. | not built |
| CAM-06 | Focus is set by: double-click, F or the inset's Focus button (ZOOM-16), each with zero offset; zooming in (CAM-21); and recalling a bookmark. A bookmark (ZOOM-11) also stores heading, tilt and any focused object with its offset; recalling it restores all of them. | not built |
| CAM-07 | Panning while focused changes the offset and does not end focus. | not built |
| CAM-08 | Focus ends only on: Home; a zoom that reaches the system frame (ZOOM-15); focusing another object (which becomes the focus); recalling a bookmark saved without a focused object; or the × on the focus chip (CAM-09). Selecting another object, Esc, rotating and zooming that stays inside the system frame do not end it. | not built |
| CAM-09 | While an object is focused, a chip under the breadcrumb reads "Focus: *name*" with a × that ends focus. | not built |
| CAM-10 | Left drag pans: the point on the orbital plane under the cursor when the drag starts stays under the cursor throughout, with no inertia after release. A press and release that moves less than `drag_threshold_px` is a click, never a pan. Shift+left drag is reserved and does nothing. | not built |
| CAM-11 | Middle drag pans exactly as CAM-10. | not built |
| CAM-12 | Right drag rotates about the pivot: horizontal movement changes heading and vertical movement changes tilt, both at `rotate_deg_per_px`, with tilt clamped (CAM-01). | not built |
| CAM-13 | A right press and release that moves less than `drag_threshold_px` is a right-click. It is reserved for context menus and orders: until those exist it calls a clearly named hook and does nothing visible. | not built |
| CAM-14 | W, A, S, D and the arrow keys pan relative to the screen (up = towards the top of the screen, along the plane) at `key_pan_screen_per_s` of the visible height at the pivot per second, so the speed looks the same at every zoom and tilt. Movement ramps up and down over `key_pan_ramp_ms`. Holding Shift multiplies the speed by `key_pan_fast_factor`. | not built |
| CAM-15 | Q and E change heading, Page Up tilts towards the horizon and Page Down towards straight down, all at `key_rotate_deg_per_s`, with tilt clamped (CAM-01). | not built |
| CAM-16 | + (also = and numpad +) zooms in one step and − (also numpad −) zooms out one step, exactly as a wheel notch (ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-14), anchored per ZOOM-13. | not built |
| CAM-17 | Edge scrolling: when `edge_scroll_enabled` is true, a cursor within `edge_scroll_px` of a window edge pans that way at the CAM-14 speed. The default is off. | not built |
| CAM-18 | Every camera and clock key and mouse button is read from one bindings resource; no system hard-codes a key. (A rebinding screen is not yet specced.) | not built |
| CAM-19 | The pivot never moves further than `pan_bound_factor` × the system frame radius from the star, whether panning freely or by offset while focused. | not built |
| CAM-20 | Pan or rotate input during an animated move (ZOOM-10, ZOOM-11, ZOOM-16, CAM-03) ends the move and takes over from the camera's current state, with no jump. | not built |
| CAM-21 | With nothing focused, a zoom-in step whose target is closer than the region frame focuses the body whose satellite frame is the inner limit (ZOOM-14), with the offset that keeps the view where it is (no jump). The step in progress keeps its anchor; later steps anchor on the focused body. | not built |
| CAM-22 | The focused object's marker carries four corner brackets of `focus_bracket_px`, in the neutral colour (MRK-01), so focus is never mistaken for the white selection ring. Both can show on one marker. | not built |

## 6. Satellite grouping

A **satellite** is anything orbiting a body other than the star: moons, stations, ships parked in orbit. Its **parent** is the body it orbits. Moons of moons follow the same rules. Ships in transit are not satellites.

| ID | Requirement | Since |
|---|---|---|
| GRP-01 | A satellite is drawn individually only while its orbit's on-screen radius is at least `unfold_at_px`; below `fold_below_px` it folds into its parent and its marker, label, ring and trail are hidden. Between the two it keeps its current state. | it3 |
| GRP-02 | The test is per satellite and re-evaluated as zoom changes, so outer satellites unfold before inner ones. | it3 |
| GRP-03 | Folding and unfolding animate the satellite into or out of its parent over `fold_unfold_anim_ms`, including during zoom animations. | it3 |
| GRP-04 | At the system frame every moon and planetary station is folded. | it3 |
| GRP-05 | A parent with folded satellites shows a "+N" pill beside its marker without overlapping it. | it3 |
| GRP-06 | The pill has an orange outline if any folded satellite is foreign, and a small blue dot if own ships are parked there. | it3 |
| GRP-07 | Hovering the pill lists the folded satellites; clicking it does exactly what clicking the parent does. | it3 |
| GRP-08 | Ships in transit are never folded. | it3 (rule only; no transit ships yet) |

## 7. Fan-out magnifier (withdrawn)

The whole section is withdrawn; IDs stay for reference. Satellites are reached by focus (ZOOM-16, CAM-21) or the inset's "In orbit" list (INS-05).

| ID | Requirement | Since |
|---|---|---|
| MAG-01 | ~~Selecting a body with folded satellites (marker or pill) opens the magnifier together with the inspection inset, as one selection.~~ Withdrawn 2026-10-10 with the magnifier (F3-07): focus now reaches a planet's satellites in one move, and the magnifier duplicated them at a second scale. | it3 |
| MAG-02 | ~~Form: a circle of `magnifier_radius_px` with a dashed frame, centred on the parent and following it as it orbits, over the dimmed map; a label shows the magnification (for example "×40").~~ Withdrawn 2026-10-10 with the magnifier (F3-07): focus now reaches a planet's satellites in one move, and the magnifier duplicated them at a second scale. | it3 |
| MAG-03 | ~~Geometry inside is true: real phases and real relative radii, scaled so the outermost included orbit fills the frame. Radii are never compressed.~~ Withdrawn 2026-10-10 with the magnifier (F3-07): focus now reaches a planet's satellites in one move, and the magnifier duplicated them at a second scale. | it3 |
| MAG-04 | ~~Contents: all stations and parked ships, and moons with radius at least `magnifier_moon_radius_threshold_km`.~~ Withdrawn 2026-10-10 with the magnifier (F3-07): focus now reaches a planet's satellites in one move, and the magnifier duplicated them at a second scale. | it3 |
| MAG-05 | ~~If the innermost included orbit would fall within `magnifier_inner_min_px` of the centre, the outermost small moons are dropped and counted as "+N outer" on the frame.~~ Withdrawn 2026-10-10 with the magnifier (F3-07): focus now reaches a planet's satellites in one move, and the magnifier duplicated them at a second scale. | it3 |
| MAG-06 | ~~Satellites inside are interactive: hover shows the tooltip; click inspects that satellite (the inset switches); double-click follows ZOOM-08.~~ Withdrawn 2026-10-10 with the magnifier (F3-07): focus now reaches a planet's satellites in one move, and the magnifier duplicated them at a second scale. | it3 |
| MAG-07 | ~~The magnifier closes when the selection changes, on Esc, or when zooming unfolds the satellites normally; in the last case the hand-over has no visual jump.~~ Withdrawn 2026-10-10 with the magnifier (F3-07): focus now reaches a planet's satellites in one move, and the magnifier duplicated them at a second scale. | it3 |
| MAG-08 | ~~Satellites inside are interactive: hover shows the tooltip; click selects that satellite (the inset switches) without changing focus; double-click behaves as ZOOM-16 and ZOOM-17.~~ Withdrawn 2026-10-10 with the magnifier (F3-07): focus now reaches a planet's satellites in one move, and the magnifier duplicated them at a second scale. | not built |

## 8. Motion rule

The **apparent period** of an orbit is its orbital period divided by the time rate, in real seconds; infinite when paused.

| ID | Requirement | Since |
|---|---|---|
| MOT-01 | Apparent period at or above `t_dot_secs`: the object is a dot at its true position with a fading trail of `trail_arc_degrees` of arc. | it3 |
| MOT-02 | Apparent period below `t_ring_secs`: exposure-ring mode. No dot; the orbit ring is brighter and `exposure_ring_width_px` wide; the label sits on the ring at a fixed spot. | it3 |
| MOT-03 | Between the two thresholds the current mode is kept; switching cross-fades over `motion_crossfade_ms`. | it3 |
| MOT-04 | There is no special case for pause: an infinite apparent period gives dot mode. | it3 |
| MOT-05 | Hover and click on an exposure ring target its object. | it3 |
| MOT-06 | ~~The rule applies to every orbiting object (moons, stations, parked ships, close-in planets), inside the magnifier too; ships in transit are ignored.~~ Withdrawn 2026-10-10 with the magnifier: replaced by MOT-08. | it3 |
| MOT-07 | A selected object in ring mode keeps its ring highlighted, and the inset gives its position as "shown when paused". | it3 |
| MOT-08 | The rule applies to every orbiting object (moons, stations, parked ships, close-in planets); ships in transit are ignored. | it3 (as MOT-06) |

## 9. Test content: Corvane

The prototype system, kept as data (SCL-03). It exists to exercise the rules above, so its values serve testing first. Its orbits do not yet match the reference layout in `docs/ideas/mechanics/drives-and-transit.md` section 2 (see `docs/findings/iteration-3.md`).

| ID | Requirement | Since |
|---|---|---|
| TEST-01 | Corvane contains at least: the star; planets Pell, Iver (with Iver Yard station), Halden IV (capital, own) and Morrow (outer giant with many moons); a belt with salvage sites Wreck Field and Lantern; and off-screen gates. | it2 |
| TEST-02 | Halden IV's satellites have periods of about: Arden Highport 0.1 d, IVa 1.5 d, IVb 4.2 d, IVc 31 d, IVd 68 d, so both motion modes show at 1 d/s. Radii follow Kepler from Halden IV's mass. | it3 |
| TEST-03 | Halden IV also has a foreign contact K-3 (diamond, slow orbit) and one own ship parked in orbit (triangle), as static orbiting objects. | it3 |
| TEST-04 | The inspection data covers all three knowledge states: Halden IV surveyed, Halden IVc unsurveyed, Lantern stale. | it2 |

## Tunables

All in the `Tunables` resource (`src/tunables.rs`; `src/view/tunables.rs` from iteration 4). Defaults are starting values; changing one is a spec change only if the default is being settled.

| Tunable | Default | Used by |
|---|---|---|
| `unfold_at_px` (R_show) | 24 px | GRP-01 |
| `fold_below_px` | 20 px | GRP-01 |
| `fold_unfold_anim_ms` | 200 | GRP-03 |
| `t_dot_secs` (T_dot) | 25 s | MOT-01 |
| `t_ring_secs` (T_ring) | 20 s | MOT-02 |
| `motion_crossfade_ms` | 200 | MOT-03 |
| `trail_arc_degrees` | 30 | MOT-01 |
| `exposure_ring_width_px` | 3 | MOT-02 |
| ~~`magnifier_radius_px`~~ | ~~80~~ | MAG-02 (withdrawn) |
| ~~`magnifier_inner_min_px`~~ | ~~12~~ | MAG-05 (withdrawn) |
| ~~`magnifier_moon_radius_threshold_km`~~ | ~~200~~ | MAG-04 (withdrawn) |
| `zoom_step_factor` | ×2 | ZOOM-01 |
| `zoom_anim_ms` | 200 (range 150–250) | ZOOM-01 |
| `zoom_stop_resistance` | 0.35 | ZOOM-05 |
| `pinch_settle_steps` | 1 | ZOOM-07 |
| `focus_jump_ms` | 400 | ZOOM-10, ZOOM-11, ZOOM-16, CAM-03 |
| `zoom_fit_margin` | 0.8 | ZOOM-15 (was ZOOM-04; added in it3, not in the original ideas) |
| `double_click_window_ms` | 350 | ZOOM-16, ZOOM-17 (fallback only) |
| `max_pitch_deg` | 65° | CAM-01, CAM-12, CAM-15 |
| `drag_threshold_px` | 4 px | CAM-10, CAM-13 |
| `rotate_deg_per_px` | 0.25 | CAM-12 |
| `key_pan_screen_per_s` | 0.6 | CAM-14, CAM-17 |
| `key_pan_ramp_ms` | 120 | CAM-14 |
| `key_pan_fast_factor` | 3 | CAM-14 |
| `key_rotate_deg_per_s` | 90 | CAM-15 |
| `edge_scroll_enabled` | false | CAM-17 |
| `edge_scroll_px` | 8 px | CAM-17 |
| `pan_bound_factor` | 1.5 | CAM-19 |
| `focus_bracket_px` | 14 px (bracket box side) | CAM-22 |

## Open points

- Per-system camera memory, a frame that rotates with the focused body, selection cycling (Tab), and gamepad and touch input: in `docs/ideas/presentation/presentation.md` (Camera), not yet specced.

## Not yet specced

From the ideas, not to be built until specced: layer toggles, alerts, legend, Administration button, inspection fidelity for foreign contacts (thermal track), ships in transit and trajectories, uncertainty ellipses, the planetary view and its transition, the cluster view, gate distance labels, and every open question in `docs/ideas/presentation/presentation.md` section 6.

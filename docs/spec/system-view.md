# Spec: System view

*Scope:* the strategic vector display of one star system: scale, clock, markers, inspection, zoom, camera, satellite grouping, the motion rule and the Corvane test system. *Source ideas:* `docs/ideas/presentation/presentation.md` (section 2), `docs/ideas/presentation/presentation-sketches.md` (boards 1, 2, 5, 6), mock-ups in `docs/ideas/presentation/mockups/`. Where a mock-up and this spec disagree, this spec wins.

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables (table at the end), read from `src/view/tunables.rs`, never hard-coded.

*Retired IDs* (never reuse): CLK-03, ZOOM-03, ZOOM-04, ZOOM-06, ZOOM-08, ZOOM-09, ZOOM-12, MAG-01 to MAG-08, MOT-06.

---

## 1. Data and scale

| ID | Requirement | Built |
|---|---|---|
| SCL-01 | Simulation data is stored in real units (km, kg, days) as `f64`, and converted to render space in one place. | ✓ |
| SCL-02 | Positions and orbits are drawn at true scale, with one linear scale for planetary and satellite orbits alike. Only markers are stylised, never geometry. | ✓ |
| SCL-03 | The system is data (a tree of body records) that a generator can replace without changing rendering code. | ✓ |
| SCL-04 | Orbits are circular and coplanar, with periods from Kepler's third law using the parent's mass. | ✓ |

## 2. Clock and top bar

| ID | Requirement | Built |
|---|---|---|
| CLK-01 | The game clock runs continuously at a default of 1 game day per real second. | ✓ |
| CLK-02 | The top bar shows the breadcrumb ("Cluster / Corvane system"), the calendar date, and Pause, 1 d/s and 5 d/s buttons; the active rate is highlighted. | ✓ |
| CLK-05 | Space toggles pause; `.` doubles and `,` halves the rate (minimum 1/16 d/s). | ✓ |
| CLK-04 | Pausing stops all orbital motion; every view stays fully interactive while paused. | ✓ |

## 3. Markers and palette

| ID | Requirement | Built |
|---|---|---|
| MRK-01 | Palette: ground #070A0E; neutral bodies and text #DCE6EC; orbits and secondary text #9FB6C4; own assets #5AA9E6 (blue); foreign contacts and warnings #F0A040 (orange); selection and hover as a white ring. | ✓ |
| MRK-02 | Type: Barlow for labels and UI; JetBrains Mono for dates, figures and status readouts. | ✓ |
| MRK-03 | Each object is a fixed-size screen marker by class, projected from its 3D position each frame; marker size never depends on physical size. | ✓ |
| MRK-04 | Marker shapes: filled disc = body; hollow square = station; filled triangle = own ship; filled diamond = foreign contact; X = salvage site. | ✓ |
| MRK-05 | Orbits are faint rings; the belt is a translucent shaded band. A ring stays within half a pixel of the true circle, also when the camera is close to it, and a ring passing behind the camera is cut at the near limit, not dropped. | ✓ |
| MRK-06 | Separate planets are never merged, however close on screen. | ✓ |
| EDGE-01 | A gate outside the viewport is shown as an arrow pinned to the edge of the map area (below the top bar) in its direction, labelled with its destination. A gate behind the camera is pinned by its direction in the camera's own right and up axes. Arrows slide round the edge as the heading turns. | ✓ |

## 4. Hover and inspection

| ID | Requirement | Built |
|---|---|---|
| HOV-01 | Hovering a marker draws a white ring and a tooltip: name · ownership · contents summary — "Click: inspect · Double-click: frame its moons · Double-click again: planetary view". | ✓ |
| INS-01 | Clicking a marker selects it and opens an inspection inset anchored to the marker by a leader line; the clock keeps running. Only the primary button selects, and only when the release is within `drag_threshold_px` of the press, so the end of a pan never selects. | ✓ |
| INS-02 | The inset shows: header (name, class and ownership line), Focus, Pin and Close buttons; an image for the knowledge state (INS-04); key facts; and an "Open planetary view" button, disabled until the planetary view exists. | ✓ |
| INS-03 | Pin keeps an inset open when the selection changes, so two objects can be compared; Esc closes the unpinned inset. | ✓ |
| INS-04 | The inset image follows knowledge: **surveyed** = clean render, status "SURVEYED · *date*"; **unsurveyed** = grainy telescope frame, "TELESCOPE FRAME · UNSURVEYED" in orange, estimates as ranges; **stale** = sepia image, "LAST IMAGED · *date*". | ✓ |
| INS-05 | The inset of a body with satellites has an "In orbit" list: one entry per satellite (class marker and name, own and foreign colours per MRK-01), in orbit order from the inside out. Clicking an entry selects that satellite (its inset replaces the parent's unless pinned) once `double_click_window_ms` passes without a second click; double-clicking it behaves as ZOOM-16 and ZOOM-17. | ✓ |
| INS-06 | When the selected object is folded into its parent (GRP-01), its inset's leader line goes to the parent's pill, and the pill carries the white selection ring. When the object unfolds, both move to its own marker without a jump. | ✓ |

## 5. Navigation and zoom

| ID | Requirement | Built |
|---|---|---|
| ZOOM-01 | Each wheel notch multiplies the view scale by `zoom_step_factor`, animated over `zoom_anim_ms` with ease-out. Raw wheel deltas are never used: line-unit input counts notches, and pixel-unit input is converted at 20 px per notch, rounded towards zero. | ✓ |
| ZOOM-02 | New input during a zoom animation retargets from the current scale; it is not queued. | ✓ |
| ZOOM-05 | A zoom step that lands on a stop takes longer by `zoom_stop_resistance`, then settles exactly on the stop. | ✓ |
| ZOOM-07 | Trackpad pinch is continuous, and on release settles to the nearest stop if within `pinch_settle_steps`. | |
| ZOOM-10 | Home returns to the system frame with the same animated move. | ✓ |
| ZOOM-11 | Ctrl+1 to Ctrl+4 save a bookmark (scale and centre, or the focused object); 1 to 4 recall it with the same animated move. | ✓ |
| ZOOM-13 | Zoom is anchored on the focused object when there is one (CAM-05); otherwise on the point where the cursor's ray meets the orbital plane, or the screen centre for keyboard zoom (CAM-16). Selection never sets the anchor. The anchor keeps its screen position through the zoom. The anchored pivot is clamped to the pan bound (CAM-19); a ray that misses the plane anchors on the pivot. | ✓ |
| ZOOM-14 | Hard limits: outer = system frame; inner = satellite frame of the focused body, or of the body nearest the zoom anchor when nothing is focused. Scrolling past a limit does nothing and never changes layer. | ✓ |
| ZOOM-15 | Stops are computed per system from its real orbits: **system frame** (all planetary orbits fit; gates stay as edge indicators), **region frame** (the inner system, or, with a planet focused, a frame whose radius is that planet's orbit, so the star sits at its edge), **satellite frame** (the focused planet's satellites fit, or those of the body nearest the zoom anchor when nothing is focused). A satellite uses the satellite frame of the body it orbits; a body orbiting the star with no satellites gets a close-up of 5% of the region frame. "Nearest" counts bodies orbiting the star, except gates. | ✓ |
| ZOOM-16 | Double-click on a body that is not focused selects it, focuses it (CAM-06) and frames its satellites in one pan-and-zoom of `focus_jump_ms`. F does the same for the current selection, and the inset's Focus button for the inset's object. | ✓ |
| ZOOM-17 | Double-click on the focused body selects it and is the planetary-view trigger. Until that view exists it calls a clearly named hook and does nothing visible. | ✓ |

## 5a. Camera

The camera looks at a **pivot**, a point on the orbital plane, from a distance set by the zoom scale. Its orientation is a **heading** (rotation about the plane's normal) and a **tilt** (0° = looking straight down). **North** is a fixed direction in the orbital plane, the same for every system. There are two separate states. **Selection** (INS-01) is the object whose details are shown; it never moves the camera. **Focus** is the object the camera is locked onto: the camera tracks it as it moves. Selection and focus may be different objects (for example, focus on Halden IV while IVc is selected).

| ID | Requirement | Built |
|---|---|---|
| CAM-01 | Heading is unlimited. Tilt is clamped between 0° and `max_pitch_deg`; the camera never goes below the orbital plane. | ✓ |
| CAM-02 | The start state is tilt 0° with north at the top of the screen. | ✓ |
| CAM-03 | N animates to tilt 0° and north up over `focus_jump_ms`, keeping the pivot and the scale. | ✓ |
| CAM-04 | Home (ZOOM-10) also returns to tilt 0° and north up, in the same animated move. | ✓ |
| CAM-05 | While an object is focused, the pivot is recomputed every frame as the object's position plus a pan offset, so the object keeps its place on screen at any time rate. The offset does not rotate with the object's orbit. The pivot and the markers use the same frame's positions. | ✓ |
| CAM-06 | Focus is set by: double-click, F or the inset's Focus button (ZOOM-16), each with zero offset; zooming in (CAM-21); and recalling a bookmark. A bookmark (ZOOM-11) also stores heading, tilt and any focused object with its offset; recalling it restores all of them. | ✓ |
| CAM-07 | Panning while focused changes the offset and does not end focus. | ✓ |
| CAM-08 | Focus ends only on: Home; a zoom that reaches the system frame (ZOOM-15); focusing another object (which becomes the focus); recalling a bookmark saved without a focused object; or the × on the focus chip (CAM-09). Selecting another object, Esc, rotating and zooming that stays inside the system frame do not end it. | ✓ |
| CAM-09 | While an object is focused, a chip under the breadcrumb reads "Focus: *name*" with a × that ends focus. | ✓ |
| CAM-10 | Left drag pans: the point on the orbital plane under the cursor when the drag starts stays under the cursor throughout, with no inertia after release. A press and release that moves less than `drag_threshold_px` is a click, never a pan. A press that starts on the top bar or an inset never pans. Shift+left drag is reserved and does nothing. | ✓ |
| CAM-11 | Middle drag pans exactly as CAM-10. | ✓ |
| CAM-12 | Right drag rotates about the pivot: horizontal movement changes heading and vertical movement changes tilt, both at `rotate_deg_per_px`, with tilt clamped (CAM-01). Dragging left turns the camera anticlockwise seen from above (the scene turns clockwise on screen); dragging down tilts towards the horizon. | ✓ |
| CAM-13 | A right press and release that moves less than `drag_threshold_px` is a right-click. It is reserved for context menus and orders: until those exist it calls a clearly named hook and does nothing visible. | ✓ |
| CAM-14 | W, A, S, D and the arrow keys pan relative to the screen (up = towards the top of the screen, along the plane) at `key_pan_screen_per_s` of the visible height at the pivot per second, so the speed looks the same at every zoom and tilt. Movement ramps up and down over `key_pan_ramp_ms`. Holding Shift multiplies the speed by `key_pan_fast_factor`. | ✓ |
| CAM-15 | Q and E change heading (Q as dragging left, E as dragging right), Page Up tilts towards the horizon and Page Down towards straight down, all at `key_rotate_deg_per_s`, with tilt clamped (CAM-01). | ✓ |
| CAM-16 | + (also = and numpad +) zooms in one step and − (also numpad −) zooms out one step, exactly as a wheel notch (ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-14), anchored per ZOOM-13. | ✓ |
| CAM-17 | Edge scrolling: when `edge_scroll_enabled` is true, a cursor within `edge_scroll_px` of a window edge pans that way at the CAM-14 speed. The default is off. | ✓ |
| CAM-18 | Every camera and clock key and mouse button is read from one bindings resource; no system hard-codes a key. (A rebinding screen is not yet specced.) | ✓ |
| CAM-19 | The pivot never moves further than `pan_bound_factor` × the system frame radius from the star, whether panning freely or by offset while focused. | ✓ |
| CAM-20 | Pan or rotate input during an animated move (ZOOM-10, ZOOM-11, ZOOM-16, CAM-03) ends the move and takes over from the camera's current state, with no jump. | ✓ |
| CAM-21 | With nothing focused, a zoom-in step whose target is closer than the region frame focuses the body whose satellite frame is the inner limit (ZOOM-14), with the offset that keeps the view where it is (no jump): the offset is the pivot minus the body's position when focus starts, and is taken again when that step ends. The step in progress keeps its anchor; later steps anchor on the focused body. | ✓ |
| CAM-22 | The focused object's marker carries four corner brackets in the neutral colour (MRK-01), so focus is never mistaken for the white selection ring. Both can show on one marker: the bracket box is `focus_bracket_px`, or the selection ring plus 2 px if that is larger. | ✓ |

## 6. Satellite grouping

A **satellite** is anything orbiting a body other than the star: moons, stations, ships parked in orbit. Its **parent** is the body it orbits. Moons of moons follow the same rules. Ships in transit are not satellites.

| ID | Requirement | Built |
|---|---|---|
| GRP-01 | A satellite is drawn individually only while its orbit's on-screen radius is at least `unfold_at_px`; below `fold_below_px` it folds into its parent and its marker, label, ring and trail are hidden. Between the two it keeps its current state. | ✓ |
| GRP-02 | The test is per satellite and re-evaluated as zoom changes, so outer satellites unfold before inner ones. | ✓ |
| GRP-03 | Folding and unfolding animate the satellite into or out of its parent over `fold_unfold_anim_ms`, including during zoom animations. | ✓ |
| GRP-04 | At the system frame every moon and planetary station is folded. | ✓ |
| GRP-05 | A parent with folded satellites shows a "+N" pill beside its marker without overlapping it. | ✓ |
| GRP-06 | The pill has an orange outline if any folded satellite is foreign, and a small blue dot if own ships are parked there. | ✓ |
| GRP-07 | Hovering the pill lists the folded satellites; clicking it does exactly what clicking the parent does. | ✓ |
| GRP-08 | Ships in transit are never folded. | ✓ (rule only; no transit ships yet) |

## 7. Motion rule

The **apparent period** of an orbit is its orbital period divided by the time rate, in real seconds; infinite when paused.

| ID | Requirement | Built |
|---|---|---|
| MOT-01 | Apparent period at or above `t_dot_secs`: the object is a dot at its true position with a fading trail of `trail_arc_degrees` of arc. | ✓ |
| MOT-02 | Apparent period below `t_ring_secs`: exposure-ring mode. No dot; the orbit ring is brighter and `exposure_ring_width_px` wide; the label sits on the ring at a fixed spot. | ✓ |
| MOT-03 | Between the two thresholds the current mode is kept; switching cross-fades over `motion_crossfade_ms`. | ✓ |
| MOT-04 | There is no special case for pause: an infinite apparent period gives dot mode. | ✓ |
| MOT-05 | Hover and click on an exposure ring target its object. A pointer within 7 px of the ring hits it; a marker, pill, label or panel over the ring wins, and of two rings in reach the nearer wins. | ✓ |
| MOT-07 | A selected object in ring mode keeps its ring highlighted, and its inset has the line "Position: shown when paused" while it is in ring mode. | ✓ |
| MOT-08 | The rule applies to every orbiting object (moons, stations, parked ships, close-in planets); ships in transit are ignored. | ✓ |

## 8. Test content: Corvane

The prototype system, kept as data (SCL-03). It exists to exercise the rules above, so its values serve testing first. Its orbits don't match the reference layout in `docs/ideas/mechanics/drives-and-transit.md` section 2 yet (F3-05).

| ID | Requirement | Built |
|---|---|---|
| TEST-01 | Corvane contains at least: the star; planets Pell, Iver (with Iver Yard station), Halden IV (capital, own) and Morrow (outer giant with many moons); a belt with salvage sites Wreck Field and Lantern; and off-screen gates. | ✓ |
| TEST-02 | Halden IV's satellites have periods of about: Arden Highport 0.1 d, IVa 1.5 d, IVb 4.2 d, IVc 31 d, IVd 68 d, so both motion modes show at 1 d/s. Radii follow Kepler from Halden IV's mass. | ✓ |
| TEST-03 | Halden IV also has a foreign contact K-3 (diamond, slow orbit) and one own ship parked in orbit (triangle), as static orbiting objects. | ✓ |
| TEST-04 | The inspection data covers all three knowledge states: Halden IV surveyed, Halden IVc unsurveyed, Lantern stale. | ✓ |

## Tunables

All in the `Tunables` resource (`src/view/tunables.rs`). Defaults are starting values; changing one is a spec change only if the default is being settled.

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
| `zoom_step_factor` | ×2 | ZOOM-01 |
| `zoom_anim_ms` | 200 (range 150–250) | ZOOM-01 |
| `zoom_stop_resistance` | 0.35 | ZOOM-05 |
| `pinch_settle_steps` | 1 | ZOOM-07 |
| `focus_jump_ms` | 400 | ZOOM-10, ZOOM-11, ZOOM-16, CAM-03 |
| `zoom_fit_margin` | 0.8 | ZOOM-15 |
| `double_click_window_ms` | 350 | INS-05 |
| `max_pitch_deg` | 65° | CAM-01, CAM-12, CAM-15 |
| `drag_threshold_px` | 4 px | INS-01, CAM-10, CAM-13 |
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

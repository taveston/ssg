# Findings: iteration 5

*Iteration:* 5, camera and the selection / focus model (ZOOM-13 to ZOOM-17, CAM-01 to CAM-22, CLK-05, INS-05, INS-06, MOT-08). *Status:* not started. Carried in from iteration 4: F4-13, F4-27, F4-28, F4-29 and F4-31 are resolved by this build; F4-30 (zoom setting focus, CAM-21) is deferred to the playtest of it. Add findings from the build from F5-01 on. Format and types are explained in `docs/README.md`.

---

### F5-01 · How CAM-21's "no jump" offset is computed
- **Observation:** CAM-21 says the zoom step that starts focus keeps its anchor and the view doesn't jump, but not how the offset is worked out while that step is still moving. I chose: at the moment focus starts, the offset is the pivot minus the body's position, so the pivot doesn't move; the step in flight carries on to its anchored, fixed pivot; when it ends (or a new step or pan replaces it), the offset is re-taken from where the pivot then is (`settle_focus`). From then on the pivot follows the body (CAM-05) and later steps anchor on it. While the first step is in flight the body drifts with its orbit (about 0.2 render units at 5 d/s over a 200 ms step, a fraction of the satellite frame), which the playtest should judge.
- **Affects:** CAM-21, CAM-05, ZOOM-02 · **Type:** spec gap
- **Disposition:** built this way; check in the playtest of this build alongside F4-30.

### F5-02 · Frames for bodies without satellites, and which bodies are "nearest"
- **Observation:** ZOOM-14 and ZOOM-15 use "the satellite frame of the focused body, or of the body nearest the anchor" and a region frame "around the focused planet's orbit", without saying (a) what the satellite frame is for a body that has no satellites (a moon, Pell, a salvage site), (b) which bodies count as "nearest" when nothing is focused, or (c) how large the region frame around a planet's orbit is. I chose: (a) a satellite uses the frame of the body it orbits; a moonless body orbiting the star gets a close-up of 5% of the region frame (the iteration 3 fallback); (b) bodies orbiting the star, except gates, which stay edge indicators (so zooming over Pell no longer snaps to Iver's frame); (c) a frame whose radius is the focused planet's orbit radius (for a focused moon, its planet's), so the star sits at its edge; with nothing focused it is the inner system, as before.
- **Affects:** ZOOM-14, ZOOM-15, CAM-21 · **Type:** spec gap
- **Disposition:** built this way (`ZoomStops::satellite_frame`, `ZoomStops::region_frame`); review the region-frame choice in the playtest.

### F5-03 · Zoom anchor near the horizon, and the pan bound
- **Observation:** at full tilt (65°), the cursor's ray near the top of the screen meets the plane far away, so a zoom anchored there moves the pivot a long way. ZOOM-13 doesn't cover it. I chose: a zoom step's fixed pivot is clamped to the pan bound (CAM-19), and a cursor ray that misses the plane falls back to the pivot (the screen centre). With `max_pitch_deg` at 65° and a 45° field of view the top edge still meets the plane, so the fallback only matters if the limit is raised.
- **Affects:** ZOOM-13, CAM-19, CAM-01 · **Type:** spec gap
- **Disposition:** built this way.

### F5-04 · Clicks now that left drag pans
- **Observation:** with left drag panning (CAM-10), a press can be a click or a pan, and both picking and the camera see it. I chose: every press records where it started (`gesture.rs`); a marker, pill or ring click selects only if the release is within `drag_threshold_px` of the press, so the release of a pan never also selects. A left click on empty space still does nothing (F2-04 stays as it is). Only the primary button selects: a right-click on a marker used to select it, and is now reserved for the CAM-13 hook. A press that starts on the top bar or an inset never starts a pan or rotation, so dragging across a panel's buttons doesn't move the map.
- **Affects:** CAM-10, CAM-13, INS-01, F2-04 · **Type:** spec gap
- **Disposition:** built this way.

### F5-05 · Which way the rotation controls turn
- **Observation:** CAM-12 and CAM-15 don't say which way is which. I chose: Q, and dragging left, turn the camera anticlockwise as seen from above, so the scene appears to turn clockwise on screen; E, and dragging right, the other way. Dragging down tilts towards the horizon, dragging up back towards straight down; Page Up and Page Down tilt as specced. Rotation is about the pivot, which is the focused object when there is one.
- **Affects:** CAM-12, CAM-15 · **Type:** spec gap
- **Disposition:** built this way; check the feel in the playtest.

### F5-06 · The double-click window tunable has nothing to do
- **Observation:** `double_click_window_ms` is specced as a fallback for ZOOM-16 and ZOOM-17 if the picking backend's click count proves unreliable. The double-click actions now read Bevy's click count, which has worked in every test, so the tunable is unused and stays marked as waiting.
- **Affects:** ZOOM-16, ZOOM-17 · **Type:** spec gap
- **Disposition:** open for triage: drop the tunable from the spec, or keep it until double-clicks have been checked on more hardware.

### F5-07 · A single click on an "In orbit" entry waits for the double-click window
- **Observation:** INS-05 asks for click (select) and double-click (ZOOM-16 / ZOOM-17) on the same entry. But selecting a satellite replaces the parent's inset with the satellite's own, which removes the list before a double-click's second click can land. So a single click on an entry selects once `double_click_window_ms` (350 ms) has passed with no second click; a double-click selects and focuses at once. This gives `double_click_window_ms` a job (F5-06), but list clicks feel a third of a second slow.
- **Affects:** INS-05, ZOOM-16, ZOOM-17, F5-06 · **Type:** spec gap
- **Disposition:** built this way. Alternative for triage: keep the parent's inset open when a satellite is picked from its list, so selection can be immediate.

### F5-08 · Focus brackets sized to clear the selection ring
- **Observation:** CAM-22 sets the bracket box at `focus_bracket_px` (14 px), but a planet's 12 px marker plus the selection ring (2 px out, 2 px wide) is about 20 px across, so 14 px brackets sat under the ring and couldn't be seen; CAM-22 asks that both read on one marker. The box is now the larger of `focus_bracket_px` and the marker's ring plus 2 px (`bracket_side`): 24 px on a planet, 30 px on the star.
- **Affects:** CAM-22, MRK-01 · **Type:** spec wrong
- **Disposition:** built this way; triage whether `focus_bracket_px` should become a margin round the ring rather than a box side.

<!-- Findings from the build: copy this block.
### F5-01 · Short title
- **Observation:**
- **Affects:** spec IDs, or "new" · **Type:** bug / spec gap / spec wrong / idea / conflict
- **Disposition:**
-->

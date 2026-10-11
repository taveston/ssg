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

<!-- Findings from the build: copy this block.
### F5-01 · Short title
- **Observation:**
- **Affects:** spec IDs, or "new" · **Type:** bug / spec gap / spec wrong / idea / conflict
- **Disposition:**
-->

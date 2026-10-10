# Findings: iteration 3

*Iteration:* 3, satellite grouping, stepped zoom and the motion rule (commit `0c1ce1a`). *Status:* awaiting your playtest. F3-01 to F3-06 come from the implementation notes in the code and from cross-checking the docs during the restructure on 2026-10-10. Add your own observations from F3-07 on. Format and types are explained in `docs/README.md`.

---

### F3-01 · Trackpad pinch not implemented
- **Observation:** `src/camera.rs` handles wheel notches only; continuous pinch and settle-on-release were deferred for lack of hardware to test on.
- **Affects:** ZOOM-07 · **Type:** bug (deferred)
- **Disposition:** open. Build when it can be tested, or withdraw ZOOM-07 if trackpads aren't a target.

### F3-02 · Framing margin added
- **Observation:** the spec didn't say how much of the screen a framed stop should fill; the build added `zoom_fit_margin` = 0.8.
- **Affects:** ZOOM-04 · **Type:** spec gap
- **Disposition:** added to the spec's tunables table. **Closed.**

### F3-03 · Magnifier moon size threshold chosen
- **Observation:** the spec left the "size threshold" for moons in the magnifier open; the build uses 200 km radius.
- **Affects:** MAG-04 · **Type:** spec gap
- **Disposition:** recorded as tunable `magnifier_moon_radius_threshold_km`. **Closed** (value still open to tuning).

### F3-04 · "Shown when paused" text may be missing
- **Observation:** while writing the spec, no "shown when paused" text was found in the source.
- **Affects:** MOT-07 · **Type:** bug (suspected)
- **Disposition:** confirmed by code review in iteration 4 (the ring highlight is there, the text is not). Fix in iteration 4, step 7. **Closed** in iteration 4 step 7 (`1659df5`), see F4-24.

### F3-05 · Prototype Corvane doesn't match the reference layout
- **Observation:** the prototype has a 0.78 solar-mass star with Pell 0.35 AU, Iver 0.7 AU, Halden IV 2.0 AU, belt sites near 1.2 to 1.3 AU and Morrow 4.3 AU, gates at about 6 AU, and no Corvane II. *Drives & Transit* section 2 has a 0.75 solar-mass star with Pell 0.25, Corvane II 0.36, Iver 0.50, belt 0.58 to 0.70, Halden IV 0.80, Morrow about 4.0 and gates at 2.0 AU. *Component Stats* and *Requirements & Bids* use the reference layout for HR-1.
- **Affects:** TEST-01, `docs/ideas/mechanics/drives-and-transit.md` · **Type:** idea conflict
- **Disposition:** open. Decide whether to move the test system to the reference layout (that changes zoom stops and Halden IV's year) and spec the orbits in TEST-01.

### F3-06 · Gate placement on screen
- **Observation:** the prototype puts gates at about 6 AU, so they show as edge indicators at the system frame. *Drives & Transit* now recommends gates at about 2 AU by travel time, inside Morrow's orbit, which would put them on screen.
- **Affects:** EDGE-01, ZOOM-04 · **Type:** idea
- **Disposition:** open, linked to F3-05 and the open question on gate distance in `presentation.md` section 6.

### F3-07 · Magnifier looks bad and is no longer needed
- **Observation:** the magnifier looks bad on screen. With the iteration 4 focus model, one double-click focuses a planet and zooms to its satellites, so the magnifier only duplicates the same objects at a second scale.
- **Affects:** MAG-01 to MAG-08, MOT-06 · **Type:** spec wrong
- **Disposition:** amend spec: MAG-01 to MAG-08 and the magnifier tunables withdrawn; MOT-06 replaced by MOT-08; INS-05 ("In orbit" list) and INS-06 (selected folded satellite) added. Code to be removed in iteration 4. **Closed** (spec).

<!-- Your playtest observations: copy this block.
### F3-07 · Short title
- **Observation:**
- **Affects:** spec IDs, or "new" · **Type:** bug / spec gap / spec wrong / idea
- **Disposition:**
-->

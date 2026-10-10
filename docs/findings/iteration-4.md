# Findings: iteration 4

*Iteration:* 4, refactoring, code hygiene and tests. *Status:* in progress. F4-01 to F4-06 come from a review of the iteration 3 code (commit `0c1ce1a`) on 2026-10-10 and are the reason for `docs/spec/architecture.md`. F4-07 to F4-15 come from step 1, the review of that code against `docs/spec/system-view.md` (results in the "it4 code review" column of `docs/validation/system-view.md`). Add findings from the build from F4-16 on. Format and types are explained in `docs/README.md`.

---

### F4-01 · Orbit state is stored in render units and in `Transform`
- **Observation:** `OrbitalBody` holds `orbit_radius` as an `f32` in render units, converted at spawn (`scene.rs`), and `update_orbital_positions` writes positions straight into `Transform`. There is no simulation position in km; the render transform hierarchy *is* the simulation.
- **Affects:** SCL-01, ARCH-10, ARCH-11, ARCH-12 · **Type:** bug
- **Disposition:** fix code in iteration 4.

### F4-02 · Simulation code depends on view code, and view modules form cycles
- **Observation:** `orbits.rs` (motion of bodies) also draws orbit rings and reads `grouping`, `motion` and `hover_inspect`. `grouping`, `motion`, `markers`, `hover_inspect` and `magnifier` all import each other; `scene.rs` (spawning) imports `grouping`. `hover_inspect::Inspections` is the de-facto selection state for every module.
- **Affects:** ARCH-02, ARCH-03 · **Type:** bug
- **Disposition:** fix code in iteration 4.

### F4-03 · Large systems with rules buried inside
- **Observation:** the longest functions are `markers::update_markers` (about 260 lines), `hover_inspect::spawn_panel` (about 245), `system_data::hand_made_system` (about 230, data), `markers::spawn_markers` (about 210), `magnifier::rebuild_magnifier_ui` (about 170), `magnifier::position_magnifier` (about 145), `camera::handle_zoom_input` (about 125), `grouping::update_grouping` (about 110) and `motion::update_trails` (about 110). Spec rules such as fold hysteresis, motion mode switching and magnifier dropping sit inside them and can't be tested alone.
- **Affects:** ARCH-20, ARCH-21 · **Type:** bug
- **Disposition:** fix code in iteration 4. `hand_made_system` is content data; it may stay long but moves to `src/content/`.

### F4-04 · No automated tests
- **Observation:** the crate had no `#[test]` functions. Since then, commit `2745bca` added unit tests for SCL-04, TEST-01 to TEST-03, GRP-01, MOT-03, MOT-04 and the stop radii (ZOOM-15). Zoom stepping, stop snapping, apparent period, trails, edge clamping and every headless and architecture test are still missing.
- **Affects:** ARCH-30, ARCH-31; every check marked Auto · **Type:** bug
- **Disposition:** partly fixed; finish in iteration 4.

### F4-05 · Simulation data, content and display text mixed in one module
- **Observation:** `system_data.rs` holds simulation components (`BodyClass`, `Ownership`, `Knowledge`), display text components (`Description`, `KeyFacts`), the blueprint types and the Corvane content, plus Kepler maths.
- **Affects:** ARCH-01, SCL-03 · **Type:** bug
- **Disposition:** fix code in iteration 4: types to `sim/`, Corvane to `content/`.

### F4-06 · Top bar writes the clock directly
- **Observation:** `topbar.rs` sets `GameClock.speed_days_per_sec` and `paused` from button clicks.
- **Affects:** ARCH-14 · **Type:** bug (minor)
- **Disposition:** fix code in iteration 4.

### F4-07 · Ship markers are text glyphs, not filled shapes
- **Observation:** `markers.rs` draws an own ship as the text "^" and a foreign contact as "<>". MRK-04 asks for a filled triangle and a filled diamond. Disc, hollow square and X are as specced.
- **Affects:** MRK-04 · **Type:** bug
- **Disposition:** fix in step 7 if small (for example a rotated square node for the diamond and a clipped or image triangle), else leave open for triage.

### F4-08 · Gate edge arrows can vanish when tilted or zoomed in
- **Observation:** edge targets come from `world_to_viewport`, which fails for points behind the camera. With the default steep tilt, a far gate behind the camera's heading gets no arrow at all, rather than one pinned to the bottom edge.
- **Affects:** EDGE-01 · **Type:** bug (suspected; confirm by running)
- **Disposition:** confirm in the playtest; the fix (project the direction in the plane instead) is small enough for step 7.

### F4-09 · Inset details
- **Observation:** the class line prints the internal enum name (`SalvageSite`, `Planet`) rather than a readable class; the inset already has an unordered row of satellite chips without class markers or colours, a partial start on INS-05.
- **Affects:** INS-02, INS-05 · **Type:** bug (minor)
- **Disposition:** readable class names in step 7. The chips stay until INS-05 is built in the camera iteration.

### F4-10 · Stale status shows time since, not the date
- **Observation:** the stale inset reads "LAST IMAGED · 2 Y 9 M AGO". INS-04 says "LAST IMAGED · *date*"; sketch board 5 shows both, the date stamp on the image and the time since in the status.
- **Affects:** INS-04 · **Type:** conflict
- **Disposition:** open for triage: amend INS-04 to "date, and time since", or change the code. Not changed in iteration 4.

### F4-11 · Zoom steps are taken from the raw scroll delta
- **Observation:** `handle_zoom_input` uses `AccumulatedMouseScroll.delta.y` directly as the number of notches, without checking the scroll unit. A device that reports pixels would zoom by many steps per event.
- **Affects:** ZOOM-01 · **Type:** bug (suspected)
- **Disposition:** step 7: count line-unit events as notches and treat pixel-unit input as continuous (ZOOM-07 territory) or convert at a fixed rate. Pure function plus test.

### F4-12 · Unspecced FPS readout in the top bar
- **Observation:** the top bar shows an FPS counter that no spec item covers.
- **Affects:** CLK-02 · **Type:** spec gap
- **Disposition:** open for triage: spec it as a debug readout or remove it.

### F4-13 · Current zoom behaviour follows withdrawn IDs
- **Observation:** the code anchors zoom on the selection (withdrawn ZOOM-03), limits and stops follow the selection (withdrawn ZOOM-04, ZOOM-06), double-click uses "already inspected" (withdrawn ZOOM-09), and zooming in latches the camera onto the nearest body, which no current ID describes (CAM-21 is close). + and − still change the time rate (withdrawn CLK-03).
- **Affects:** ZOOM-13 to ZOOM-17, CAM-21, CLK-05 · **Type:** conflict
- **Disposition:** leave as is in iteration 4 (no behaviour change); replaced by the camera iteration.

### F4-14 · Exposure rings are only hittable at 16 points
- **Observation:** ring hover and click use 16 invisible 14 px squares spaced round the ring. On a large ring most of the circumference is not hittable.
- **Affects:** MOT-05 · **Type:** bug
- **Disposition:** step 7 if small: hit-test by distance from the projected ring (pure function, testable) instead of the necklace.

### F4-15 · Withdrawn magnifier still in the code
- **Observation:** `magnifier.rs` and its tunables are still built and running, although MAG-01 to MAG-08 were withdrawn (F3-07).
- **Affects:** MAG-01 to MAG-08, ARCH-33 · **Type:** bug
- **Disposition:** remove in iteration 4 (step 2), with its tunables.

### F4-16 · Physical body radii removed with the magnifier
- **Observation:** `BodyRadius` and the blueprints' `radius_km` had exactly one reader, the magnifier's size threshold (MAG-04). With the magnifier gone they were dead data, which ARCH-33 does not allow outside tunables, so both were removed in step 2. No current ID needs a body's physical radius: marker size is a presentation choice (MRK-03) and the inset's facts are authored text (INS-02). The authored values are in git history at `3c91a59` if they are wanted again.
- **Affects:** ARCH-33, SCL-03, INS-02 · **Type:** spec gap
- **Disposition:** removed in iteration 4. Re-add `radius_km` to the body records when a spec ID uses it (physical facts in the inset, or a generator that derives mass from radius).

<!-- Findings from the build: copy this block.
### F4-16 · Short title
- **Observation:**
- **Affects:** spec IDs, or "new" · **Type:** bug / spec gap / spec wrong / idea / conflict
- **Disposition:**
-->

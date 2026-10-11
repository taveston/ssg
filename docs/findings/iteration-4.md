# Findings: iteration 4

*Iteration:* 4, refactoring, code hygiene and tests. *Status:* built; awaiting playtest. F4-01 to F4-06 come from a review of the iteration 3 code (commit `0c1ce1a`) on 2026-10-10 and are the reason for `docs/spec/architecture.md`. F4-07 to F4-15 come from step 1, the review of that code against `docs/spec/system-view.md` (results in the "it4 code review" column of `docs/validation/system-view.md`). Add findings from the build from F4-16 on. Format and types are explained in `docs/README.md`.

---

### F4-01 · Orbit state is stored in render units and in `Transform`
- **Observation:** `OrbitalBody` holds `orbit_radius` as an `f32` in render units, converted at spawn (`scene.rs`), and `update_orbital_positions` writes positions straight into `Transform`. There is no simulation position in km; the render transform hierarchy *is* the simulation.
- **Affects:** SCL-01, ARCH-10, ARCH-11, ARCH-12 · **Type:** bug
- **Disposition:** fix code in iteration 4. **Closed** in iteration 4 step 3 (`e9b84ce`): `Orbit` and `OrbitPositionKm` in km, `Transform` written only by `view/projection.rs`.

### F4-02 · Simulation code depends on view code, and view modules form cycles
- **Observation:** `orbits.rs` (motion of bodies) also draws orbit rings and reads `grouping`, `motion` and `hover_inspect`. `grouping`, `motion`, `markers`, `hover_inspect` and `magnifier` all import each other; `scene.rs` (spawning) imports `grouping`. `hover_inspect::Inspections` is the de-facto selection state for every module.
- **Affects:** ARCH-02, ARCH-03 · **Type:** bug
- **Disposition:** fix code in iteration 4. **Closed** in iteration 4 step 4 (`8bf26bb`): domains split, ring drawing in `view/rings.rs`, shared state in `selection`, `fold`, `motion_mode`, `targets` and `screen`; `arch_02_*` and `arch_03_*` guard it.

### F4-03 · Large systems with rules buried inside
- **Observation:** the longest functions are `markers::update_markers` (about 260 lines), `hover_inspect::spawn_panel` (about 245), `system_data::hand_made_system` (about 230, data), `markers::spawn_markers` (about 210), `magnifier::rebuild_magnifier_ui` (about 170), `magnifier::position_magnifier` (about 145), `camera::handle_zoom_input` (about 125), `grouping::update_grouping` (about 110) and `motion::update_trails` (about 110). Spec rules such as fold hysteresis, motion mode switching and magnifier dropping sit inside them and can't be tested alone.
- **Affects:** ARCH-20, ARCH-21 · **Type:** bug
- **Disposition:** fix code in iteration 4. `hand_made_system` is content data; it may stay long but moves to `src/content/`. **Closed** in iteration 4 steps 2 and 5 (`1abc968`, `51a275e`); `arch_21_*` guards it.

### F4-04 · No automated tests
- **Observation:** the crate had no `#[test]` functions. Since then, commit `2745bca` added unit tests for SCL-04, TEST-01 to TEST-03, GRP-01, MOT-03, MOT-04 and the stop radii (ZOOM-15). Zoom stepping, stop snapping, apparent period, trails, edge clamping and every headless and architecture test are still missing.
- **Affects:** ARCH-30, ARCH-31; every check marked Auto · **Type:** bug
- **Disposition:** partly fixed; finish in iteration 4. **Closed** in iteration 4 steps 2 and 6 (`1abc968`, `7a02bd0`).

### F4-05 · Simulation data, content and display text mixed in one module
- **Observation:** `system_data.rs` holds simulation components (`BodyClass`, `Ownership`, `Knowledge`), display text components (`Description`, `KeyFacts`), the blueprint types and the Corvane content, plus Kepler maths.
- **Affects:** ARCH-01, SCL-03 · **Type:** bug
- **Disposition:** fix code in iteration 4: types to `sim/`, Corvane to `content/`. **Closed** in iteration 4 step 4 (`8bf26bb`): types in `sim/`, Corvane in `content/` (see F4-18).

### F4-06 · Top bar writes the clock directly
- **Observation:** `topbar.rs` sets `GameClock.speed_days_per_sec` and `paused` from button clicks.
- **Affects:** ARCH-14 · **Type:** bug (minor)
- **Disposition:** fix code in iteration 4. **Closed** in iteration 4 step 3 (`e9b84ce`).

### F4-07 · Ship markers are text glyphs, not filled shapes
- **Observation:** `markers.rs` draws an own ship as the text "^" and a foreign contact as "<>". MRK-04 asks for a filled triangle and a filled diamond. Disc, hollow square and X are as specced.
- **Affects:** MRK-04 · **Type:** bug
- **Disposition:** fix in step 7 if small (for example a rotated square node for the diamond and a clipped or image triangle), else leave open for triage. **Closed** in iteration 4 step 7 (`1659df5`), see F4-23.

### F4-08 · Gate edge arrows can vanish when tilted or zoomed in
- **Observation:** edge targets come from `world_to_viewport`, which fails for points behind the camera. With the default steep tilt, a far gate behind the camera's heading gets no arrow at all, rather than one pinned to the bottom edge.
- **Affects:** EDGE-01 · **Type:** bug (suspected; confirm by running)
- **Disposition:** confirm in the playtest; the fix (project the direction in the plane instead) is small enough for step 7. **Closed** in iteration 4 step 7 (`1659df5`), confirmed by a headless test first; see F4-20.

### F4-09 · Inset details
- **Observation:** the class line prints the internal enum name (`SalvageSite`, `Planet`) rather than a readable class; the inset already has an unordered row of satellite chips without class markers or colours, a partial start on INS-05.
- **Affects:** INS-02, INS-05 · **Type:** bug (minor)
- **Disposition:** readable class names in step 7. The chips stay until INS-05 is built in the camera iteration. **Closed** in iteration 4 for the class names in step 7 (`1659df5`); the chips stay until INS-05.

### F4-10 · Stale status shows time since, not the date
- **Observation:** the stale inset reads "LAST IMAGED · 2 Y 9 M AGO". INS-04 says "LAST IMAGED · *date*"; sketch board 5 shows both, the date stamp on the image and the time since in the status.
- **Affects:** INS-04 · **Type:** conflict
- **Disposition:** open for triage: amend INS-04 to "date, and time since", or change the code. Not changed in iteration 4.

### F4-11 · Zoom steps are taken from the raw scroll delta
- **Observation:** `handle_zoom_input` uses `AccumulatedMouseScroll.delta.y` directly as the number of notches, without checking the scroll unit. A device that reports pixels would zoom by many steps per event.
- **Affects:** ZOOM-01 · **Type:** bug (suspected)
- **Disposition:** step 7: count line-unit events as notches and treat pixel-unit input as continuous (ZOOM-07 territory) or convert at a fixed rate. Pure function plus test. **Closed** in iteration 4 step 2 (`1abc968`), see F4-17.

### F4-12 · Unspecced FPS readout in the top bar
- **Observation:** the top bar shows an FPS counter that no spec item covers.
- **Affects:** CLK-02 · **Type:** spec gap
- **Disposition:** open for triage: spec it as a debug readout or remove it.

### F4-13 · Current zoom behaviour follows withdrawn IDs
- **Observation:** the code anchors zoom on the selection (withdrawn ZOOM-03), limits and stops follow the selection (withdrawn ZOOM-04, ZOOM-06), double-click uses "already inspected" (withdrawn ZOOM-09), and zooming in latches the camera onto the nearest body, which no current ID describes (CAM-21 is close). + and − still change the time rate (withdrawn CLK-03).
- **Affects:** ZOOM-13 to ZOOM-17, CAM-21, CLK-05 · **Type:** conflict
- **Disposition:** leave as is in iteration 4 (no behaviour change); replaced by the camera iteration. **Closed** in iteration 5 steps 3 and 4 (`eafafd4`, `146b765`): the zoom latch, selection-based anchoring, limits and stops are removed; zoom anchors and stops by focus (ZOOM-13 to ZOOM-15).

### F4-14 · Exposure rings are only hittable at 16 points
- **Observation:** ring hover and click use 16 invisible 14 px squares spaced round the ring. On a large ring most of the circumference is not hittable.
- **Affects:** MOT-05 · **Type:** bug
- **Disposition:** step 7 if small: hit-test by distance from the projected ring (pure function, testable) instead of the necklace. **Closed** in iteration 4 step 7 (`1659df5`), see F4-22.

### F4-15 · Withdrawn magnifier still in the code
- **Observation:** `magnifier.rs` and its tunables are still built and running, although MAG-01 to MAG-08 were withdrawn (F3-07).
- **Affects:** MAG-01 to MAG-08, ARCH-33 · **Type:** bug
- **Disposition:** remove in iteration 4 (step 2), with its tunables. **Closed** in iteration 4 step 2 (`1abc968`).

### F4-16 · Physical body radii removed with the magnifier
- **Observation:** `BodyRadius` and the blueprints' `radius_km` had exactly one reader, the magnifier's size threshold (MAG-04). With the magnifier gone they were dead data, which ARCH-33 does not allow outside tunables, so both were removed in step 2. No current ID needs a body's physical radius: marker size is a presentation choice (MRK-03) and the inset's facts are authored text (INS-02). The authored values are in git history at `3c91a59` if they are wanted again.
- **Affects:** ARCH-33, SCL-03, INS-02 · **Type:** spec gap
- **Disposition:** removed in iteration 4. Re-add `radius_km` to the body records when a spec ID uses it (physical facts in the inset, or a generator that derives mass from radius).

### F4-17 · Pixel-unit scroll converted at 20 px per notch
- **Observation:** F4-11 was fixed in step 2, earlier than planned, because the pure zoom-stepping function it needed was being written anyway. ZOOM-01 says raw deltas are never used but not how a pixel-unit device maps to notches. I chose a fixed 20 px per notch (the figure Bevy's examples use), truncated towards zero so a slow trackpad drag doesn't creep (`notches_from_scroll`, tested). Line-unit wheels are unchanged.
- **Affects:** ZOOM-01, ZOOM-07 · **Type:** spec gap
- **Disposition:** fixed in iteration 4. When ZOOM-07 (pinch) is built, pixel-unit input should become continuous instead.

### F4-18 · Authored body text sits in the simulation
- **Observation:** F4-05 asked for display text components to leave `system_data.rs`, but ARCH-02 only lets content and view depend on sim. `DisplayName`, `Description` and `KeyFacts` are what the player's records say about a body, read by the view and written by content, so I put them in `sim/body.rs` with ownership and knowledge. The alternative, a view-owned text component filled by content, would need content to depend on the view.
- **Affects:** ARCH-01, ARCH-02, F4-05 · **Type:** spec gap
- **Disposition:** done this way in iteration 4. Revisit when knowledge becomes simulated (what is known changes over time), when this text may want to be derived rather than stored.

### F4-19 · Test layout: bin crate, headless tests in `src/`, a strict length check
- **Observation:** integration tests in `tests/` need a library crate, and a library exports every `pub` item, which silences the dead-code lint ARCH-33 relies on. So the crate stays a binary: the headless tests are `#[cfg(test)]` apps in `src/headless_tests.rs` (ARCH-31 allows this), and only the text scans, which need no crate access, are in `tests/architecture.rs`. The ARCH-21 check fails the build above 60 lines rather than warning, because a test can't warn; content data and tests are exempt. Grouping needs a camera that can project, which normally needs the renderer, so the headless fold test fills the camera's computed projection in by hand, as Bevy's own camera tests do.
- **Affects:** ARCH-21, ARCH-31, ARCH-33 · **Type:** spec gap
- **Disposition:** done this way in iteration 4.

### F4-20 · Gates behind the camera now pinned by direction
- **Observation:** F4-08 confirmed by a headless test before fixing: at Halden IV's satellite frame with the default tilt, two of the three gates are behind the camera, have no projected position, and got no arrow. A gate that can't be projected is now pinned in the direction given by the camera's own right and up axes (`screen_direction`, `pin_direction_to_edge`), so a gate behind a steeply tilted camera sits on the bottom edge. Gates the camera can project are unchanged.
- **Affects:** EDGE-01 · **Type:** bug
- **Disposition:** fixed in iteration 4, test `edge_01_every_gate_has_an_arrow_when_zoomed_in`. Check the arrows' feel in the playtest: a gate passing from behind the camera to in front of it may jump along the edge.

### F4-21 · Gate edge labels show the gate's name
- **Observation:** EDGE-01 says the arrow is "labelled with its destination"; the label is the gate's display name, "Gate Ostra". The name contains the destination, so it reads correctly, but it isn't literally the destination.
- **Affects:** EDGE-01 · **Type:** conflict
- **Disposition:** open for triage: accept the name, or label with the destination alone. Not changed in iteration 4.

### F4-22 · Exposure rings hit-tested by a picking backend
- **Observation:** for F4-14, the 16 hit squares per ring are replaced by a small Bevy picking backend (`ring_picking.rs`): each active ring is traced on screen every frame, and a pointer within 7 px of it (the old squares' half-width, not a spec tunable) hits an invisible target entity carrying the ordinary marker observers, so tooltip, select and double-click work as before. Ring hits are reported below the UI's order, so a marker, pill, label or panel over the ring wins, and where two rings are in reach the nearer wins.
- **Affects:** MOT-05 · **Type:** bug
- **Disposition:** fixed in iteration 4. Check in the playtest that the 7 px reach feels right; promote it to a tunable if it needs tuning.

### F4-23 · Filled ship shapes are generated images
- **Observation:** for F4-07, Bevy UI has no triangle primitive, so the own-ship triangle and the foreign diamond are small white images generated at startup (antialiased by supersampling) and tinted per marker, which keeps fading and colour identical to the other markers. Box sizes are unchanged (12 px). The salvage-site X stays a glyph, as specced.
- **Affects:** MRK-04 · **Type:** bug
- **Disposition:** fixed in iteration 4; MRK-04 needs re-checking by eye.

### F4-24 · "Shown when paused" wording and timing
- **Observation:** for F3-04, MOT-07 asks that the inset give a ring-mode object's position as "shown when paused" without saying where or in what words. The inset now has a line "Position: shown when paused", in the facts' style, shown while the body's latched motion mode is ring (so it appears and disappears at the mode switch, not part-way through the cross-fade).
- **Affects:** MOT-07 · **Type:** spec gap
- **Disposition:** done in iteration 4. Check the wording in the playtest.

### F4-25 · One-frame ordering changes from the restructure
- **Observation:** ARCH-04's "simulation before view" puts the clock tick and orbit positions first in each frame; before, the clock tick was unordered and the camera ran before orbits. Ring drawing moved from the orbit stage to the overlay stage, so rings now use this frame's fold and motion state rather than last frame's. Count pills are spawned only for bodies with orbiting children, where before any `ChildOf` counted. None of these should be visible; listed because ARCH-35 asks that a refactor change no behaviour.
- **Affects:** ARCH-04, ARCH-35 · **Type:** spec gap
- **Disposition:** accepted in iteration 4; confirm nothing looks different in the playtest.

## Playtest, 2026-10-10

Each was checked against the iteration 3 code (`3c91a59`); none was introduced by the iteration 4 refactor.

### F4-26 · A planet's own orbit line flickers when zoomed in on it
- **Observation:** zoomed in so the planet's satellites fill the view, the line of the planet's own orbit round the star comes in and out of view. Cause: rings are drawn as 128 straight chords, and a chord is dropped whole if either end is behind the camera. At Halden IV's satellite frame the camera is about one render unit from an orbit 180 units in radius, so the chord through the view is about 9 units long and its far end is often behind the camera. The chords are also visibly wrong at this scale: a 9-unit chord on a 180-unit circle bows about 0.05 units from the arc, roughly 40 px at this zoom, so the line doesn't pass through the planet. Unchanged since iteration 3.
- **Affects:** MRK-05, SCL-02 · **Type:** bug
- **Disposition:** fix now (triaged 2026-10-11). Rings still start as 128 chords, but a chord is halved, up to 12 times, until it strays no more than half a pixel from the circle as seen from the camera, and a chord crossing the near limit is cut there instead of dropped (`ring_segments`, `clip_to_front`). From afar a ring is the same 128 chords as before. **Closed** in iteration 4: tests `mrk_05_*`; a screenshot at Halden IV's satellite frame shows the orbit line through the planet and steady while running.

### F4-27 · Zooming out drops the camera off the planet
- **Observation:** after zooming in onto a planet, zooming out detaches the camera, which stays where the planet was while the planet moves on. Cause: every zoom step that doesn't latch onto a body sets a fixed centre, and zooming out never latches. This is the iteration 3 zoom model (F4-13).
- **Affects:** CAM-05, CAM-08, ZOOM-13 · **Type:** conflict (built behaviour against unbuilt spec)
- **Disposition:** replaced by the camera iteration: CAM-08 says zooming that stays inside the system frame doesn't end focus. **Closed** in iteration 5 step 4 (`146b765`): focus ends only on reaching the system frame; headless test `cam_21_zoom_focuses_and_cam_08_zoom_out_ends_it`.

### F4-28 · Double-click doesn't frame a body
- **Observation:** double-clicking a planet does nothing visible. Cause: the first click of a double-click selects the body; the second arrives with a click count of 2, finds the body already selected, and takes the planetary-view hook (the withdrawn ZOOM-09) instead of framing it (the withdrawn ZOOM-08). The framing branch can never be reached by a double-click. This is the "already inspected" ambiguity that withdrew ZOOM-09. F still frames the selection.
- **Affects:** ZOOM-16, ZOOM-17 · **Type:** bug
- **Disposition:** replaced by the camera iteration, where ZOOM-16 and ZOOM-17 decide by focus instead of selection, which removes the ambiguity. **Closed** in iteration 5 step 6 (`1ea4234`): double-click focuses and frames, or on the focused body calls the planetary-view hook; headless `zoom_16_cam_03_cam_04_focus_north_and_home_moves`. Re-check by hand.

### F4-29 · Bodies jitter while the camera follows a planet; orbit lines stay steady
- **Observation:** while the camera follows a planet, its markers and its satellites' markers shake, while the rings stay still. Cause: the camera centres on the body's position from the previous frame (it reads `GlobalTransform`, which Bevy only updates after the frame's systems have run), while the markers are placed from the body's newer position. The body therefore sits one frame of orbital motion away from the centre, and that gap changes with every frame's length. At Halden IV's satellite frame one frame of its orbital motion is about 13 px at 1 d/s and 65 px at 5 d/s, so uneven frame times show as jitter. Rings are drawn from the same frame as the camera, so they don't shake. Same frame ordering in iteration 3.
- **Affects:** CAM-05 ("keeps its place on screen at any time rate"), MRK-03 · **Type:** bug
- **Disposition:** open for triage. Fix: have the camera pivot use the followed body's current simulated position (`OrbitPositionKm` up its parent chain, through `scale.rs`) rather than last frame's transform, and project markers after transform propagation so camera and bodies come from the same frame. CAM-05's check at 5 d/s already covers it. **Closed** in iteration 5 step 3 (`eafafd4`) as described: `WorldPositions` gives this frame's positions to the camera and the markers; headless `cam_05_focused_body_keeps_its_place_on_screen` fails (66 px off) with the lag put back.

### F4-30 · Zooming in hops focus between satellites
- **Observation:** changing zoom inside a planet's satellite system switches the followed body unpredictably between satellites. Cause: each zoom-in step latches onto whichever body is nearest the anchor within a quarter of the new frame, satellites included, so successive steps can pick different moons. The playtester's view: zooming to focus is poor UX.
- **Affects:** CAM-21 · **Type:** spec wrong (suspected)
- **Disposition:** deferred (2026-10-11): CAM-21 stays as specced for now. As specced, CAM-21 would behave differently: it focuses only when nothing is focused, only the body whose satellite frame is the inner limit, and later steps anchor on that body, so it wouldn't hop between moons. The question is whether zoom should set focus at all, or focus only comes from double-click, F and the Focus button (CAM-06). Iteration 5 builds CAM-21 as specced (`146b765`, see F5-01); still deferred to the playtest of that build.

### F4-31 · Left drag doesn't pan
- **Observation:** left-click drag does nothing; middle drag pans (iteration 1).
- **Affects:** CAM-10 · **Type:** bug (not built)
- **Disposition:** camera iteration (CAM-10). **Closed** in iteration 5 step 5 (`70e40cb`): left and middle drag pan, the grabbed point staying under the cursor; headless `cam_10_left_drag_keeps_the_grabbed_point_under_the_cursor`.

<!-- Findings from the build: copy this block.
### F4-16 · Short title
- **Observation:**
- **Affects:** spec IDs, or "new" · **Type:** bug / spec gap / spec wrong / idea / conflict
- **Disposition:**
-->

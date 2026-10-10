# Orphan Worlds

Hard sci-fi space strategy game for PC. This repo holds the prototype and the whole design. Read `docs/README.md` first: it explains the layout and the design–build–refine cycle.

## Where things are

- `docs/spec/`: **the only basis for code.** Each requirement has a stable ID (`ZOOM-03`). Build what the current milestone lists; nothing else.
  - `system-view.md`: the system view (prefixes SCL, CLK, MRK, EDGE, HOV, INS, ZOOM, CAM, GRP, MAG, MOT, TEST) and its tunables table.
  - `ideology.md`: the ideology profile (IDEO). Specced, not yet scheduled.
  - `architecture.md`: how the code is organised, separated and tested (ARCH). Applies to all code.
- `docs/ideas/`: design thinking, back story and open questions. Read it for context and intent, but never implement from it. If something you need is only in `ideas/`, stop and say so.
- `docs/ideas/presentation/mockups/`: reference images (open them with your image reader). Look and layout only; where an image and the spec disagree, the spec wins.
- `docs/findings/iteration-N.md`: observations from each iteration and what was done about them.
- `docs/validation/`: per-ID checks, with results per iteration.

## Stack

- Rust (stable) and Bevy, pinned in `Cargo.toml`. Bevy's API changes between versions, so check the docs and examples for the pinned version rather than relying on memory.
- Keep dependencies minimal. Ask before adding crates beyond Bevy. Fonts are assets in `assets/fonts/` (Barlow, JetBrains Mono, OFL).

## Done

- **Iteration 1:** coarse 3D system with circular orbits, an orbit/zoom/pan camera and a game clock.
- **Iteration 2:** the Corvane system as data at true scale, fixed-size markers, top bar, hover tooltips, inspect in place with pinning, fidelity states, gate edge indicators. Findings: `docs/findings/iteration-2.md`.
- **Iteration 3:** satellite grouping, fan-out magnifier, stepped zoom with stops, motion rule, test content (spec IDs marked *it3*). Playtested 2026-10-10. Findings: `docs/findings/iteration-3.md`.
- **Iteration 4:** refactor to `docs/spec/architecture.md` (`sim/`, `content/`, `view/` domains; simulation in km and days; rule functions with tests named after their IDs; headless tests in `src/headless_tests.rs`; text-scan checks in `tests/architecture.rs`), magnifier removed, small spec fixes (F3-04, F4-07, F4-08, F4-09, F4-11, F4-14) and the orbit-line fix (F4-26). Findings: `docs/findings/iteration-4.md`, including the playtest of the iteration 3 behaviour (F4-26 to F4-31).

## Current milestone: iteration 5, camera and focus

Build the camera and the selection / focus model in `docs/spec/system-view.md` (section 5a and the zoom IDs that depend on focus). **Selection** is what the inset shows and never moves the camera; **focus** is what the camera tracks. Today the code mixes the two (F4-13, F4-27, F4-28, F4-30); this iteration separates them.

**In scope:** ZOOM-13 to ZOOM-17, CAM-01 to CAM-22, CLK-05, INS-05, INS-06 and MOT-08 (already working by code review; confirm, cite and test it). ZOOM-10 and ZOOM-11 are reworked to carry heading, tilt and focus (CAM-04, CAM-06). Fix F4-29 (jitter while following) as part of CAM-05.

**Remove, don't keep alongside:** code for the withdrawn ZOOM-03, ZOOM-04, ZOOM-06, ZOOM-08, ZOOM-09, ZOOM-12, CLK-03 and MOT-06. In particular: the zoom latch in `camera/input.rs` that follows whichever body is nearest the anchor (F4-13, F4-30; replaced by CAM-21 as specced), anchoring, limits and stops that follow the selection, the "already selected" double-click branch (F4-28), + and − changing the time rate in `topbar.rs` (now zoom, CAM-16), the hard-coded `PITCH_LIMIT` and start pitch (now `max_pitch_deg`, CAM-01, CAM-02), and the inset's satellite chips (replaced by the INS-05 list, F4-09). The iteration 3 tests for the latch (F4-13) go with it.

**Steps, in order.** Keep the app runnable and `cargo test` green after each, and commit after each.

1. **Tunables and bindings (CAM-18, CLK-05).** Add the camera tunables from the spec's table to `Tunables` (`max_pitch_deg` to `focus_bracket_px`), each documented. Add one bindings resource for every camera and clock key and mouse button, and read them only through it (add an `arch_*` text scan so camera and clock code names no `KeyCode` or `MouseButton` outside the bindings module; Esc for the inset, INS-03, may stay where it is). Rebind the clock: Space pause, `.` and `,` rate. Note the spec's convention: **tilt** 0° is looking straight down; the code's `pitch` is measured from the plane, so convert or rename in one place.
2. **Focus as shared state, and camera rules as plain functions.** Move focus out of the camera into its own small view module (like `selection`), so the camera, inset, markers and top bar can read it without cycles (ARCH-03). Focus is an optional entity plus a pan offset in the plane (km or render units, through `scale.rs`). Write and test pure functions for: tilt clamp and unlimited heading (CAM-01); pivot = focused position + offset (CAM-05); pan bound (CAM-19); cursor ray to orbital-plane point (ZOOM-13, CAM-10); drag threshold, click vs drag (CAM-10, CAM-13); key pan speed as a fraction of visible height, with ramp and Shift (CAM-14); the zoom anchor and inner limit by focus or nearest body (ZOOM-13, ZOOM-14, ZOOM-15); when a zoom step starts or ends focus (CAM-08, CAM-21).
3. **Pivot from this frame (CAM-05, F4-29).** The pivot of a focused body comes from its current simulated position (`OrbitPositionKm` up its parent chain, through `scale.rs`), not last frame's `GlobalTransform`, and markers are projected after transform propagation, so camera, bodies and rings all come from the same frame. Add a headless test that the focused body's projected position doesn't move between frames at 5 d/s.
4. **Zoom by focus (ZOOM-13 to ZOOM-15, CAM-16, CAM-21, CAM-08).** Anchor on the focus, else on the plane point under the cursor, else the screen centre for keys. Limits and the satellite stop follow the focus, else the body nearest the anchor. Build CAM-21 as specced: it focuses only when nothing is focused, only the body whose satellite frame is the inner limit, and later steps anchor on that body (F4-30 is deferred to the playtest of this build; don't change the rule). A zoom reaching the system frame ends focus. Keep ZOOM-01, ZOOM-02 and ZOOM-05 and their tests unchanged.
5. **Mouse and keyboard (CAM-10 to CAM-15, CAM-17, CAM-20).** Left and middle drag pan with the grabbed plane point staying under the cursor, also when tilted; no inertia; Shift+left drag does nothing. Right drag rotates about the pivot; a right click under the threshold calls a clearly named hook. WASD and arrows pan, Q / E turn, Page Up / Down tilt. Edge scrolling, off by default. Pan or rotate during an animated move ends it from the current state. Panning while focused changes the offset (CAM-07). A click that stays under the threshold must still select, on markers, pills, rings and the panel (check that picking and the drag don't both act on one press).
6. **Focus actions and moves (ZOOM-16, ZOOM-17, CAM-03, CAM-04, CAM-06, ZOOM-10, ZOOM-11).** Double-click, F and the inset's Focus button focus with zero offset and frame the satellites in one move; double-click on the focused body selects it and calls the planetary-view hook (this removes F4-28's ambiguity). N levels and turns north up keeping pivot and scale; Home also levels and ends focus. Bookmarks store and restore scale, pivot, heading, tilt and any focus with its offset.
7. **Focus and selection on screen (CAM-09, CAM-22, INS-05, INS-06).** Focus chip under the breadcrumb with a ×. Corner brackets on the focused marker, distinct from the white selection ring, both readable on one marker. The inset's "In orbit" list, inside out, with class markers and own / foreign colours; click selects, double-click acts as ZOOM-16 / ZOOM-17. A folded selected satellite's leader line and ring go to the parent's pill and move to its own marker without a jump when it unfolds.
8. **Tidy and check.** ARCH-21 to ARCH-23 and ARCH-30 to ARCH-34 still hold (new systems small, IDs cited, rule tests named after IDs). Re-check MOT-08 and the edge arrows (EDGE-01, F4-20) now that the camera starts level and can turn.

**Out of scope:** ZOOM-07 (pinch, F3-01); Corvane's layout (F3-05, F3-06); the open triage items F4-10 (stale date), F4-12 (FPS readout), F4-21 (gate label) and F2-04 (whether a click on empty space closes the inset: keep today's behaviour); the camera open points in the spec (per-system camera memory, a rotating frame, Tab cycling, gamepad, touch); a rebinding screen; context menus and the planetary view (hooks only); ideology (IDEO).

**Findings:** add them to `docs/findings/iteration-5.md`, from F5-01. Expect spec gaps around: how the CAM-21 "no jump" offset is computed; what a click on empty space does now that left drag pans; how the plane-point anchor behaves near the horizon at full tilt; and what edge arrows do as the heading turns.

**When done:** add an it5 column to `docs/validation/system-view.md` and `docs/validation/architecture.md` and fill it (Auto results and code review for every row you touched, ✓ auto with the test names). Close or update the iteration 4 findings this build resolves (F4-13, F4-27, F4-28, F4-29, F4-31; F4-30 stays deferred) with the commit. In your summary list: the IDs now built (so I can update the spec's *Since* column; don't edit it yourself), the new rule functions and their tests, the module layout of the camera, focus and bindings, and the hand checks I should run, by scene (validation file Setup, scenes A to C).

## How to work

- Implement the spec IDs listed for the milestone, as written. Cite the ID in a comment where a rule is implemented (`// GRP-01`) and in test names (`fn grp_01_hysteresis()`).
- Every number the spec marks as a tunable lives in the `Tunables` resource (`src/view/tunables.rs`), documented field by field. Never hard-code one.
- If the spec is ambiguous or doesn't work on screen, make a sensible choice and **add a finding** to the current iteration's file in `docs/findings/` (type "spec gap" or "spec wrong", with what you chose). Don't edit `docs/spec/` or `docs/ideas/` unless asked.
- Add `cargo test` cases for checks marked **Auto** in `docs/validation/` when you touch that area.
- Keep simulation data separate from rendering. Store everything in real units (km, days) using `f64` in simulation data, and convert to render space in one place. Marker size is a presentation choice in screen pixels, never derived from physical size.
- Keep the system as data, so a generator can replace it later (see `docs/ideas/mechanics/world-generator.md`).
- Organise code by domain (`docs/spec/architecture.md`): simulation, content and view, with Bevy plugins per area; prefer a new module per concern over growing an existing one.
- Work in steps and keep the app runnable after each one.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` before calling work done. I run the app myself to judge the visuals: list the validation IDs I should check and how, and fill in any Auto results in the validation file.

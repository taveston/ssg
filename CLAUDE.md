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
- **Iteration 3:** satellite grouping, fan-out magnifier, stepped zoom with stops, motion rule, test content (spec IDs marked *it3*). Built; awaiting playtest. Findings: `docs/findings/iteration-3.md`.

## Current milestone: iteration 4, refactor, hygiene and tests

No new features. Make the code match `docs/spec/architecture.md`, cover the rules with automated tests, and check the existing presentation code against `docs/spec/system-view.md`. The problems found in review are F4-01 to F4-15 in `docs/findings/iteration-4.md`.

**In scope:** ARCH-01 to ARCH-35; every check marked **Auto** in `docs/validation/system-view.md` and `docs/validation/architecture.md`; small fixes for system-view IDs the review finds broken (below).

**Steps, in order.** Keep the app runnable and `cargo test` green after each, and commit after each.

1. **Spec review: done** (2026-10-10, from code reading). Results are in the "it4 code review" column of `docs/validation/system-view.md`; deviations are F4-07 to F4-15 in `docs/findings/iteration-4.md`, plus F3-04 (confirmed). Re-check any row you touch.
2. **Remove withdrawn code, extract rules, test them (ARCH-20, ARCH-30, ARCH-33).** Delete `magnifier.rs` and the three magnifier tunables (F4-15). Unit tests already exist for SCL-04, TEST-01 to TEST-03, GRP-01, MOT-03, MOT-04 and the stop radii (`zoom_15_*`); keep them passing. Add plain functions with tests for: apparent period (MOT-01 to MOT-04), zoom stepping and easing (ZOOM-01, ZOOM-02), stop snapping with resistance (ZOOM-05), trail sampling (MOT-01), gate edge clamping (EDGE-01), fold animation stepping (GRP-03), and the pill summary flags (GRP-06). These tests are the safety net for the moves that follow.
3. **Simulation in real units (ARCH-10 to ARCH-14).** Orbits in km and days, positions in km in a sim component, one view system converting to `Transform` through `view/scale.rs`. Clock changes through a clock API (F4-06).
4. **Domains and dependencies (ARCH-01 to ARCH-04).** Move files into `src/sim/`, `src/content/` and `src/view/`; split `system_data.rs` (types to `sim/`, Corvane to `content/`); move ring drawing out of orbit motion; give selection, fold state and motion mode their own small shared modules so no view modules import each other in a cycle. Shape the camera module so the camera iteration (below) can replace its input handling without touching the rest.
5. **Small systems (ARCH-21 to ARCH-23).** Split the long functions listed in F4-03. Add spec ID comments.
6. **Headless and architecture tests (ARCH-13, ARCH-31 to ARCH-34).** Integration tests without a window; `tests/architecture.rs` text-scan checks; fmt and clippy with `-D warnings`.
7. **Fix small spec deviations** where the spec is unambiguous and the fix is a few lines: F3-04 (MOT-07 "shown when paused"), F4-07 (filled ship markers), F4-08 (edge arrows behind the camera, after confirming), F4-09 (readable class names), F4-11 (scroll units), F4-14 (ring hit-testing). Leave F4-10, F4-12 and F4-13 alone; they're for me to triage. Anything that turns out larger stays a finding.

**Out of scope:** new features, including the camera and focus work below; ZOOM-07 (pinch, F3-01); moving Corvane to the reference layout (F3-05, F3-06); visual changes other than step 7 fixes. If a refactor would change behaviour, don't: add a finding instead (ARCH-35).

**When done:** fill the it4 columns in `docs/validation/architecture.md` and `docs/validation/system-view.md` (Auto results and code review), add findings for anything you had to decide, and in your summary list the rule functions and their tests, the new module layout, and which system-view checks I should re-run by hand to confirm nothing regressed.

**Next, iteration 5 (not yet in scope; do not build until confirmed here):** the camera and the selection / focus model: ZOOM-13 to ZOOM-17, CAM-01 to CAM-22, INS-05, INS-06, MOT-08 and CLK-05. These replace the withdrawn ZOOM-03, ZOOM-04, ZOOM-06, ZOOM-08, ZOOM-09, ZOOM-12, CLK-03, MOT-06 and the whole magnifier (MAG-01 to MAG-08; its code goes in iteration 4), so existing code for those must be reworked or removed, not kept alongside.

## How to work

- Implement the spec IDs listed for the milestone, as written. Cite the ID in a comment where a rule is implemented (`// GRP-01`) and in test names (`fn grp_01_hysteresis()`).
- Every number the spec marks as a tunable lives in the `Tunables` resource (`src/tunables.rs`, moving to `src/view/tunables.rs` in iteration 4), documented field by field. Never hard-code one.
- If the spec is ambiguous or doesn't work on screen, make a sensible choice and **add a finding** to the current iteration's file in `docs/findings/` (type "spec gap" or "spec wrong", with what you chose). Don't edit `docs/spec/` or `docs/ideas/` unless asked.
- Add `cargo test` cases for checks marked **Auto** in `docs/validation/` when you touch that area.
- Keep simulation data separate from rendering. Store everything in real units (km, days) using `f64` in simulation data, and convert to render space in one place. Marker size is a presentation choice in screen pixels, never derived from physical size.
- Keep the system as data, so a generator can replace it later (see `docs/ideas/mechanics/world-generator.md`).
- Organise code by domain (`docs/spec/architecture.md`): simulation, content and view, with Bevy plugins per area; prefer a new module per concern over growing an existing one.
- Work in steps and keep the app runnable after each one.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` before calling work done. I run the app myself to judge the visuals: list the validation IDs I should check and how, and fill in any Auto results in the validation file.

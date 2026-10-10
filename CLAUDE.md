# Orphan Worlds

Hard sci-fi space strategy game for PC. This repo holds the prototype and the whole design. Read `docs/README.md` first: it explains the layout and the design–build–refine cycle.

## Where things are

- `docs/spec/`: **the only basis for code.** Each requirement has a stable ID (`ZOOM-03`). Build what the current milestone lists; nothing else.
  - `system-view.md`: the system view (prefixes SCL, CLK, MRK, EDGE, HOV, INS, ZOOM, GRP, MAG, MOT, TEST) and its tunables table.
  - `ideology.md`: the ideology profile (IDEO). Specced, not yet scheduled.
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
- **Iteration 3:** satellite grouping, fan-out magnifier, stepped zoom with stops, motion rule, test content (spec IDs marked *it3*). Built; awaiting validation and playtest. Findings so far: `docs/findings/iteration-3.md`.

## Current milestone

None in progress. The next milestone is set after iteration 3 is validated and its findings triaged; it will be listed here as spec IDs in scope.

## How to work

- Implement the spec IDs listed for the milestone, as written. Cite the ID in a comment where a rule is implemented (`// GRP-01`) and in test names (`fn grp_01_hysteresis()`).
- Every number the spec marks as a tunable lives in `src/tunables.rs`, documented field by field. Never hard-code one.
- If the spec is ambiguous or doesn't work on screen, make a sensible choice and **add a finding** to the current iteration's file in `docs/findings/` (type "spec gap" or "spec wrong", with what you chose). Don't edit `docs/spec/` or `docs/ideas/` unless asked.
- Add `cargo test` cases for checks marked **Auto** in `docs/validation/` when you touch that area.
- Keep simulation data separate from rendering. Store everything in real units (km, days) using `f64` in simulation data, and convert to render space in one place. Marker size is a presentation choice in screen pixels, never derived from physical size.
- Keep the system as data, so a generator can replace it later (see `docs/ideas/mechanics/world-generator.md`).
- Organise code with Bevy plugins, roughly one per area; prefer a new module per concern over growing an existing one.
- Work in steps and keep the app runnable after each one.
- Run `cargo check`, `cargo clippy` and `cargo test` before calling work done. I run the app myself to judge the visuals: list the validation IDs I should check and how, and fill in any Auto results in the validation file.

# Orphan Worlds

Hard sci-fi space strategy game for PC. This repo holds the prototype and the whole design. You build items from `docs/todo.md`, to the spec in `docs/spec/`, with automated tests. Read `docs/README.md` for the layout and how work flows.

## Where things are

- `docs/todo.md`: **what to build.** Build only the items you're given (or, if asked to "do the next item", the top one). Each item names its spec IDs.
- `docs/spec/`: **the only basis for code.** Each requirement has a stable ID (`ZOOM-13`) and a Built column.
  - `system-view.md`: the system view (prefixes SCL, CLK, BTN, MRK, SYM, EDGE, LBL, HOV, INS, ZOOM, CAM, GRP, MOT, TEST) and its tunables table.
  - `ideology.md`: the ideology profile (IDEO).
  - `sky.md`: the sky behind the system and sector views (SKY) and its tunables table.
  - `architecture.md`: how the code is organised, commented and tested (ARCH). Applies to all code.
- `docs/findings.md`: open observations waiting for a decision.
- `docs/ideas/`: design thinking, back story and open questions. Read it for context and intent, but never implement from it. If something you need is only in `ideas/`, stop and say so.
- `docs/ideas/presentation/mockups/`: reference images (open them with your image reader). Look and layout only; where an image and the spec disagree, the spec wins.

## Stack

- Rust (stable) and Bevy, pinned in `Cargo.toml`. Bevy's API changes between versions, so check the docs and examples for the pinned version rather than relying on memory.
- Keep dependencies minimal. Ask before adding crates beyond Bevy. Fonts are assets in `assets/fonts/`, all OFL; which font fills each type role is set by the type roles table in `docs/spec/system-view.md` (MRK-02).

## How to work

- Implement each item's spec IDs as written. Plan your own steps; keep the app runnable and `cargo test` green after each step, and commit after each.
- When a requirement is withdrawn, delete its code and tests. Never keep old behaviour alongside new.
- If the spec is ambiguous or doesn't work on screen, make a sensible choice and **add a finding** under "To triage" in `docs/findings.md` (type "spec gap" or "spec wrong", saying what you chose). Don't edit `docs/spec/` or `docs/ideas/` beyond the Built column unless asked.
- Check that every requirement with an empty Built cell is named on `docs/todo.md` (ranges like `IDEO-01 to IDEO-31` count), and that any requirement whose meaning changed has its ✓ cleared and is listed. Do it whenever you touch the spec or the to-do list, and when you finish an item. No test checks this. If one is missing, don't build it: add an item under "Spec changes" and mention it in your summary.
- When an item is done: mark its requirements ✓ in the spec's Built column, delete the item from `docs/todo.md`, and delete any finding it resolves.
- Every number the spec marks as a tunable lives in the `Tunables` resource (`src/view/tunables.rs`), documented field by field. Never hard-code one.
- Keep simulation data separate from rendering. Store everything in real units (km, days) using `f64` in simulation data, and convert to render space in one place. Marker size is a presentation choice in screen pixels, never derived from physical size.
- Keep the system as data, so a generator can replace it later (see `docs/ideas/mechanics/world-generator.md`).
- Organise code by domain (`docs/spec/architecture.md`): simulation, content and view, with Bevy plugins per area; prefer a new module per concern over growing an existing one.

## Seeing the game

- Run with `cargo run`, not the exe in `target/`: assets load from the crate root. Build first; a timeout during the build kills the linker.
- To check something on screen, take a screenshot of the game window itself with the `OW_SHOT` environment variable (`src/view/debug_shot.rs`). It is a list of `key=value` pairs separated by `;`: `out` (image path, required), `frame`, `select` and `hover` (body display names), `distance` (km from the pivot), `tilt` (degrees from straight down), `heading` (degrees, 0 is north up), `rate` (days per second, 0 pauses), `wait` (seconds to settle, default 2), `planner` (`open` opens the transfer planner for the selected ship; a body's name also picks it as the destination), `fly` (with `planner` naming a destination: schedules the planner's order, then moves the clock to that many days after the ship leaves, to show it in flight) and `tray` (a body's name: opens the tray of the multi-marker holding it, or of its own fold or station marker; a shot whose tray never opened fails). The app sets up that view, saves the image and exits; an unknown key or name is an error. So is `planner` without a selected ship that can open it, and, with `fly`, a destination the planner can't schedule: the app exits at once and logs the planner's reason. A shot not saved within `timeout` seconds (default 60) exits with an error naming the phase it was stuck in. Still run the game under a timeout of your own (`timeout 120 cargo run`), so a hang before the shot starts can't leave a window open. For example, in PowerShell: `$env:OW_SHOT='out=<scratchpad>/shot.png;frame=Halden IV;select=Halden IV;tilt=30;rate=0'; cargo run`. Then read the image. Don't capture the desktop: the game window may not be in front, and it would show the user's screen.
- For a playtest copy that later builds can't overwrite or lock, rebuild `master` (`touch src/main.rs` first, since other worktrees may share `target/`), then copy `target/debug/orphan_worlds.exe` and `assets/` into `play/` at the crate root and run it from there. `play/` is git-ignored. Keep this and any other scratch output inside the project, never elsewhere on the user's disk.
- Bevy 0.20 UI: every `Node` carries a `BackgroundColor` (and often a `BorderColor`), shown or not. When recolouring, write only the slot a part uses (`Tint`/`TintSlot` in `view/symbols/draw.rs`). Text in an absolute container as narrow as a marker wraps per word; use `TextLayout::linebreak(LineBreak::NoWrap)`.

## Tests

- Every requirement you build gets an automated test. A test comment or doc line names the ID it covers.
- Prefer testing through what drives the picture (projected positions, fold state, pixel sizes, simulated input in a headless app) over leaving a check to the eye. Only if that truly can't work, add *Manual:* and the reason to the requirement in the spec, and say so in your summary.
- Name tests for what they check (`satellites_stay_folded_inside_the_hysteresis_band`), not after a spec ID.
- Requirements about code structure (layout, dependencies, function size, comments, headers, which module reads a key) are marked *Checked in review.* Follow them, but don't write tests that read source text to enforce them. Tests check behaviour.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` before calling work done.

## Comments and docs

- A comment explains the code in its own terms, so it can be understood without the spec open. It may end with the spec ID it implements, e.g. `(GRP-01)`.
- Docs and findings use short, plain sentences, with a summary first (`docs/README.md`, Writing).

## Summary when done

Keep it short: the items built, the findings added, and anything marked *Manual*. I play the build myself and note what I find.

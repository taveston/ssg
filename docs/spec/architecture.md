# Spec: Code architecture

**In short:** the code is split into simulation, content and view domains, with dependencies pointing one way. Simulation works in real units (km, days) and never knows about rendering. Every spec rule is a plain function with tests, systems stay small, and text-scan tests enforce the layout.

*Scope:* how the prototype's code is organised, separated, commented and tested. It applies to all code. **Built** is ✓ once the code meets the requirement.

---

## 1. Domains and layout

The code is split into domains, each a folder under `src/` with its own Bevy plugin (or a few plugins) in its `mod.rs`.

| Domain | Folder | Owns | Examples today |
|---|---|---|---|
| Simulation | `src/sim/` | The game state and its rules, in real units: clock, bodies, orbits, ownership, knowledge, and the authored text the player's records hold about a body (name, description, key facts). Knows nothing about screens, cameras or input. | `GameClock`, orbital elements, body positions in km, Kepler periods |
| Content | `src/content/` | Data that seeds the simulation: the Corvane system as records. Replaced later by a generator. | `hand_made_system()` |
| View | `src/view/` | Everything drawn: scale conversion, markers, rings, pills, grouping, motion rule, inspection inset, top bar, palette, fonts, fidelity images, presentation tunables. | `markers`, `grouping`, `motion`, `inspect` |
| Camera and input | `src/view/camera/` (or `src/input/` if it grows) | Camera state, zoom stops, zoom animation, navigation keys and mouse. Focus and key bindings are shared view modules (`view/focus.rs`, `view/bindings.rs`) so other view modules can read them. | `camera/rig`, `camera/stops`, bookmarks |

| ID | Requirement | Built |
|---|---|---|
| ARCH-01 | Code is organised into the domain folders above. `main.rs` only assembles plugins and configures system sets. | ✓ |
| ARCH-02 | Dependencies point one way: view → sim → nothing; content → sim types only. No file under `src/sim/` or `src/content/` may `use crate::view` (or camera/input), Bevy rendering, UI, picking, gizmo or window types. | ✓ |
| ARCH-03 | No dependency cycles between modules inside a domain. When two view modules need the same state (selection, focus, fold state, motion mode), that state lives in a small shared module both read (for example `view/selection.rs`), not in one feature module the others reach into. | ✓ |
| ARCH-04 | Each plugin registers only its own systems, resources and components. Ordering between domains uses the system sets in `main.rs` (simulation before view), not cross-module `.after()` calls. | ✓ |

## 2. Simulation state

| ID | Requirement | Built |
|---|---|---|
| ARCH-10 | Simulation components hold real units as `f64`: orbit radius in km, period in days, phase in radians, positions in km (relative to the star, or to the parent with the parent named). SCL-01 holds without exception. | ✓ |
| ARCH-11 | Body positions are computed by a simulation system from the clock and orbital elements into a simulation component. A separate view system converts them to render space and writes `Transform`. `Transform` is never the simulation's source of truth. | ✓ |
| ARCH-12 | The only km → render conversion is in `view/scale.rs`. Nothing else multiplies by a render scale. | ✓ |
| ARCH-13 | Simulation systems can run in a headless `App` with `MinimalPlugins` and the sim plugin(s) only. | ✓ |
| ARCH-14 | Player input that changes the simulation (pause, rate) goes through a simulation-owned API (a method or message on the clock), not by view code writing simulation fields directly. | ✓ |

## 3. Functions, systems and comments

| ID | Requirement | Built |
|---|---|---|
| ARCH-20 | Every rule in a spec (fold hysteresis, motion mode, zoom stops, Kepler periods, apparent period, label rules) is a plain function with no Bevy queries or resources in its signature, taking and returning plain values. Systems gather inputs, call the function and apply the result. | ✓ |
| ARCH-21 | Systems stay small: as a guide, under about 50 lines. Longer systems are split into a gather / decide / apply sequence or into helper functions. UI spawning is split into one helper per panel section. A text-scan test fails any function over 60 lines; content data and tests are exempt. | ✓ |
| ARCH-22 | Magic numbers are named constants with a comment saying what they are. Numbers the spec calls tunable stay in `Tunables` (`src/view/tunables.rs`); simulation constants (for example `KM_PER_AU`, `G`) live in `src/sim/`. | ✓ |
| ARCH-23 | A comment at the function implementing a spec rule explains the rule in its own terms, so the code can be understood without the spec open. It may end with the rule's ID, e.g. `// Satellites fold below fold_px and unfold only above unfold_px, so they don't flicker (GRP-01)`. A comment that is only an ID doesn't meet this. | |

## 4. Tests and hygiene

| ID | Requirement | Built |
|---|---|---|
| ARCH-30 | Each rule function from ARCH-20 has unit tests in the same file (`#[cfg(test)] mod tests`). A test's name says what it checks (`satellites_stay_folded_inside_the_hysteresis_band`), never just a spec ID; the ID goes in a comment or doc line on the test. Edge cases include the hysteresis band, paused time (infinite apparent period) and empty inputs. | |
| ARCH-31 | Headless tests cover at least: orbital positions after a known time; fold state after a scale change; motion mode after a rate change; zoom stop computation for Corvane. They run without a window or GPU. The crate is a binary, so headless apps are `#[cfg(test)]` code in `src/headless_tests.rs`; only text scans, which need no crate access, live in `tests/`. | ✓ |
| ARCH-32 | `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` all pass. | ✓ |
| ARCH-33 | No dead code: no `#[allow(dead_code)]` or unused items, except tunables for requirements not yet built, each marked with the ID they wait for. | ✓ |
| ARCH-34 | Every module has a `//!` header saying what it owns and which spec IDs it implements. | ✓ |
| ARCH-35 | A refactor changes no behaviour: every test that passed before still passes, and nothing on screen changes. | ✓ |
| ARCH-36 | Every built requirement in a feature spec (system view, ideology) has an automated test whose comment or doc names its ID, unless the spec marks the requirement *Manual* with the reason. A text-scan test enforces this. | |

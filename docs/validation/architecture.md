# Validation: Code architecture

Checks for `docs/spec/architecture.md`. Record results per iteration as in `system-view.md`: ✓ pass, ✗ fail with a finding ID, – not applicable yet.

**Auto** checks should be tests in `tests/architecture.rs` that read the source files as text (no extra crates needed): they fail the build if the rule is broken again later.

| ID | How to check | Auto | it3 | it4 |
|---|---|---|---|---|
| ARCH-01 | `src/` has `sim/`, `content/` and `view/` folders, each with a `mod.rs` plugin; `main.rs` has no systems. | Auto (folders exist; no `fn` other than `main` in `main.rs`) | ✗ F4-05 | ✓ auto (`arch_01_*`) |
| ARCH-02 | No `use crate::view`, `bevy::ui`, `bevy::picking`, `Gizmos`, `Camera` or `Window` in any file under `src/sim/` or `src/content/`. | Auto (text scan) | ✗ F4-02 | ✓ auto (`arch_02_*`; also bans `bevy::render` and `bevy::window`) |
| ARCH-03 | Draw the module import graph inside `src/view/`; it has no cycles. | Auto (parse `use crate::view::…` lines, detect cycles) | ✗ F4-02 | ✓ auto (`arch_03_*`) |
| ARCH-04 | Code read: each plugin's `build` registers only its own module's items; no `.after(other_module::system)`. | | | ✓ code read: no `.after()` or `.before()` anywhere; ordering by the `Boot` and `Stage` sets in `main.rs`. `Selection`, shared with no single owner, is registered by `ViewPlugin` |
| ARCH-10 | Code read: simulation components carry `f64` km / days / radians; no render-unit fields. | | ✗ F4-01 | ✓ code read: `Orbit` (km, days, rad) and `OrbitPositionKm` (km, `DVec3`), all `f64` |
| ARCH-11 | A headless test advances the clock and reads body positions in km from the sim component, with no `Transform` involved. | Auto | ✗ F4-01 | ✓ auto (`arch_11_*`, headless `arch_31_orbital_positions_*`) |
| ARCH-12 | Search for the render scale constant: used only in `view/scale.rs`. | Auto (text scan) | ✗ F4-01 | ✓ auto (`arch_12_*`) |
| ARCH-13 | A test builds `App::new().add_plugins((MinimalPlugins, SimPlugin))`, runs updates, and passes. | Auto | ✗ | ✓ auto (`arch_13_*`) |
| ARCH-14 | Code read: view code changes the clock only through its API. | | ✗ F4-06 | ✓ code read: `GameClock`'s fields are private |
| ARCH-20 | Each rule in the spec maps to a named plain function (list them in the iteration 4 summary). | | ✗ F4-03 | ✓ (rule functions listed in the iteration 4 summary) |
| ARCH-21 | No system over about 50 lines (excluding content data). | Auto (rough line count per `fn`, warn above 60) | ✗ F4-03 | ✓ auto (`arch_21_*`: fails above 60 lines; content and tests exempt, F4-19) |
| ARCH-22 | Code read: no unexplained numeric literals in logic. | | | ✓ code read: numbers in logic are named; UI paddings, palette hex values and the fidelity image recipe stay inline as style data |
| ARCH-23 | Every spec ID in `docs/spec/system-view.md` marked built appears in at least one source comment. | Auto (text scan against the spec's ID list) | | ✓ auto (`arch_23_*`) |
| ARCH-30 | Every rule function has tests named after its ID. | | ✗ F4-04 | ✓ code read: every spec rule function has tests named after its ID; helpers with no ID of their own (`anim`, `distance_to_segments`) and iteration 3 behaviour with no current ID (the zoom latch, F4-13) have plainly named tests |
| ARCH-31 | The four integration tests exist and pass headless. | Auto | ✗ F4-04 | ✓ auto (four `arch_31_*` headless tests in `src/headless_tests.rs`, F4-19) |
| ARCH-32 | `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` all pass. | Auto | | ✓ (all three pass) |
| ARCH-33 | Search for `allow(dead_code)`: only on tunables waiting for an unbuilt ID, with that ID named. | Auto (text scan) | ✗ (`tunables.rs`) | ✓ auto (`arch_33_*`) |
| ARCH-34 | Every `src/**/*.rs` starts with a `//!` header. | Auto (text scan) | | ✓ auto (`arch_34_*`) |
| ARCH-35 | Re-run every check in `system-view.md` that passed in it3; none regress. | | – | ? screenshots of scenes A, C and D before and after match except for the intended step 7 fixes (scene A pixel-identical); hand re-run pending (F4-25) |

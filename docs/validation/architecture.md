# Validation: Code architecture

Checks for `docs/spec/architecture.md`. Record results per iteration as in `system-view.md`: ✓ pass, ✗ fail with a finding ID, – not applicable yet.

**Auto** checks should be tests in `tests/architecture.rs` that read the source files as text (no extra crates needed): they fail the build if the rule is broken again later.

| ID | How to check | Auto | it3 | it4 |
|---|---|---|---|---|
| ARCH-01 | `src/` has `sim/`, `content/` and `view/` folders, each with a `mod.rs` plugin; `main.rs` has no systems. | Auto (folders exist; no `fn` other than `main` in `main.rs`) | ✗ F4-05 | |
| ARCH-02 | No `use crate::view`, `bevy::ui`, `bevy::picking`, `Gizmos`, `Camera` or `Window` in any file under `src/sim/` or `src/content/`. | Auto (text scan) | ✗ F4-02 | |
| ARCH-03 | Draw the module import graph inside `src/view/`; it has no cycles. | Auto (parse `use crate::view::…` lines, detect cycles) | ✗ F4-02 | |
| ARCH-04 | Code read: each plugin's `build` registers only its own module's items; no `.after(other_module::system)`. | | | |
| ARCH-10 | Code read: simulation components carry `f64` km / days / radians; no render-unit fields. | | ✗ F4-01 | |
| ARCH-11 | A headless test advances the clock and reads body positions in km from the sim component, with no `Transform` involved. | Auto | ✗ F4-01 | |
| ARCH-12 | Search for the render scale constant: used only in `view/scale.rs`. | Auto (text scan) | ✗ F4-01 | |
| ARCH-13 | A test builds `App::new().add_plugins((MinimalPlugins, SimPlugin))`, runs updates, and passes. | Auto | ✗ | |
| ARCH-14 | Code read: view code changes the clock only through its API. | | ✗ F4-06 | |
| ARCH-20 | Each rule in the spec maps to a named plain function (list them in the iteration 4 summary). | | ✗ F4-03 | |
| ARCH-21 | No system over about 50 lines (excluding content data). | Auto (rough line count per `fn`, warn above 60) | ✗ F4-03 | |
| ARCH-22 | Code read: no unexplained numeric literals in logic. | | | |
| ARCH-23 | Every spec ID in `docs/spec/system-view.md` marked built appears in at least one source comment. | Auto (text scan against the spec's ID list) | | |
| ARCH-30 | Every rule function has tests named after its ID. | | ✗ F4-04 | |
| ARCH-31 | The four integration tests exist and pass headless. | Auto | ✗ F4-04 | |
| ARCH-32 | `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` all pass. | Auto | | |
| ARCH-33 | Search for `allow(dead_code)`: only on tunables waiting for an unbuilt ID, with that ID named. | Auto (text scan) | ✗ (`tunables.rs`) | |
| ARCH-34 | Every `src/**/*.rs` starts with a `//!` header. | Auto (text scan) | | |
| ARCH-35 | Re-run every check in `system-view.md` that passed in it3; none regress. | | – | |

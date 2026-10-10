# Findings: iteration 4

*Iteration:* 4, refactoring, code hygiene and tests. *Status:* planned. F4-01 to F4-06 come from a review of the iteration 3 code (commit `0c1ce1a`) on 2026-10-10 and are the reason for `docs/spec/architecture.md`. Add findings from the build from F4-07 on. Format and types are explained in `docs/README.md`.

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
- **Observation:** the crate has no `#[test]` functions; every check in `docs/validation/system-view.md` is manual.
- **Affects:** ARCH-30, ARCH-31; every check marked Auto · **Type:** bug
- **Disposition:** fix in iteration 4.

### F4-05 · Simulation data, content and display text mixed in one module
- **Observation:** `system_data.rs` holds simulation components (`BodyClass`, `Ownership`, `Knowledge`), display text components (`Description`, `KeyFacts`), the blueprint types and the Corvane content, plus Kepler maths.
- **Affects:** ARCH-01, SCL-03 · **Type:** bug
- **Disposition:** fix code in iteration 4: types to `sim/`, Corvane to `content/`.

### F4-06 · Top bar writes the clock directly
- **Observation:** `topbar.rs` sets `GameClock.speed_days_per_sec` and `paused` from button clicks.
- **Affects:** ARCH-14 · **Type:** bug (minor)
- **Disposition:** fix code in iteration 4.

<!-- Findings from the build: copy this block.
### F4-07 · Short title
- **Observation:**
- **Affects:** spec IDs, or "new" · **Type:** bug / spec gap / spec wrong / idea / conflict
- **Disposition:**
-->

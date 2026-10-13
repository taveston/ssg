# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-101.

---

### F5-07 · A single click on an "In orbit" entry waits for the double-click window
- **Observation:** selecting a satellite replaces the parent's inset, which removes the list before a double-click's second click can land. So a single click selects only after `double_click_window_ms` (350 ms) passes with no second click. List clicks feel a third of a second slow.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** parked (2026-10-12). Pinning is gone, so the parent's inset can't stay open beside the satellite's. Options: select at once and let a second click on the same spot in the satellite's inset do nothing; or give the satellite's inset a "back to parent" line in the list's place.

### F-101 · Frame bodies far from the star wobble on screen
- **Observation:** a frame body wobbles visibly when zoomed in. It is worst for the moons of the outer planets. The cause is precision: positions are relative to the star in `f32` render units. Morrow is about 387 units out, where an `f32` step is about 3 × 10⁻⁵ units. Zoomed in on a moon, the camera is only about 10⁻³ units away, so that step is a few per cent of the view, and the rounding changes from frame to frame.
- **Affects:** CAM-05, SCL-01 · **Type:** bug
- **Disposition:** proposed fix: render relative to the frame body (a floating origin). Subtract the frame body's position in `f64` km before converting to `f32`, in `view/scale.rs` and `projection.rs`, so everything near the frame body has full precision.

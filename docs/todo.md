# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Named orbits, shared rings and exposure-ring identity** · SCL-07, HOV-02, HOV-04, HOV-05, MOT-02, MOT-14, INS-05, TEST-10
  - Was F-131 (fast objects look alike) plus the gap that every object got its own ring. Orbits become data: a body's named orbits, referenced by the ships and stations in them (`sim/orbit.rs` and the Corvane content). One ring per named orbit; nothing is grouped by radius in the view. Per TEST-10, every ship parked at Halden IV goes in "High": no per-ship parking orbits. The identity mark reuses the fold pill and the list symbol. Build the INS-05 orbit row with the "In orbit" outline item below if that is still open.
- [ ] **Inset as a sidebar; "In orbit" outline** · INS-05, INS-07
  - `inset_dock_margin_px` is withdrawn; `sidebar_slide_ms` is new.
- [ ] **Inset panel anatomy** · INS-09
  - Was F-120. Build after the sidebar.
- [ ] **Label placement** · LBL-01 to LBL-04
  - Was F-115: labels overprinted each other round Halden IV. `label_gap_px` and `label_fade_ms` are new. The hardest item here; keep placement in its own view module.
- [ ] **Morrow IX's trader at about 9 d** · TEST-09
  - The build already uses 9 d (was F-113); check and mark ✓.
  - Was F-119: a label was clipped to "Gat" and one arrow had no label. Do it with the "Gate edge labels" fix below.
- [ ] **Edge arrows as tabs** · EDGE-02, EDGE-03
- [ ] **Ship groups and condition** · SYM-09, SYM-10, SYM-12, GRP-09, TEST-06
  - The symbols are built; groups add to them (`view/symbols.rs`, `symbol_draw.rs`).
- [ ] **Orbits crate** · ARCH-40 to ARCH-44, RAIL-01 to RAIL-07
  - A new library crate `crates/orbits` in a Cargo workspace with the game: conics, Lambert, the Edelbaum spiral, trajectories, the small formulas. Move the Kepler maths out of `sim/orbit.rs` into it and have the game call it (ARCH-35: nothing changes on screen). The documentation standard (ARCH-42, ARCH-43) is part of the item, not a follow-up: write the `lib.rs` worked example first.
- [ ] **Transfer simulation** · RAIL-08, RAIL-09, BAND-01 to BAND-08, XFER-01 to XFER-12, PLAN-01 to PLAN-30, TEST-20, ARCH-22
  - Builds on the orbits crate. Bands and costs, then legs against XFER-12's figures, then plans and delta-v accounting. No UI; nothing new is drawn. `SimTunables` is new (ARCH-22). Headless tests can schedule a plan on the TEST-20 warship and advance the clock.
- [ ] **Velocity leaders** · SYM-11
  - Blocked: ships in transit are not drawn yet (the simulation is in the item above; drawing them is not specced).
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.

## Fixes

- [ ] **Gate edge labels show the destination alone** · EDGE-01
  - Label reads "Ostra", not "Gate Ostra".

## Other

- [ ] **Screenshot debug option**
  - A built-in way to capture the game window without editing `main.rs`: for example, an environment variable naming a view (frame body, distance, tilt, time rate) and an output path. The app sets that view, waits for it to settle, saves a screenshot and exits.
  - Keep it in a view debug module, read through the bindings or a debug resource, not in `main.rs`. Then replace the "temporarily edit `main.rs`" steps in CLAUDE.md ("Seeing the game").

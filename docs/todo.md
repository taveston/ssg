# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Large orbits fade out; parent as an edge tab** · MRK-14, MRK-15
  - Was F-137. Remove the frame-body tail (`frame_orbit.rs`, `frame_orbit_arc_px`); the frame body's orbit is now an ordinary ring under the size fade. `orbit_fade_from` and `orbit_fade_to` are new. A ring faded to nothing is not drawn, so it's no target (HOV-02).
  - MRK-15 changes from a bearing tick to an edge tab for the parent; reuse the gate edge-tab code. `bearing_tick_px` and `bearing_tick_radius_px` are withdrawn.
- [ ] **Identity mark only on a closed ring** · MOT-14
  - Was F-139. The build fades the mark in as the dot fades out (300° to 360°), so a held rate shows both. Now the mark appears only once the displayed tail reaches 360°, with a time fade. `identity_mark_fade_ms` is new.
  - MOT-14 also now states the pill anchor the build already uses (was F-133); check it.
- [ ] **Close orbits pass in front of a true-size disc** · MRK-12
  - Was F-138. Rings are world gizmos and bodies UI nodes above them, so the near arc needs its own pass above the parent's disc: split the ring at the parent's centre depth into a far and a near arc. Only for satellites of a body drawn as a disc (MRK-07).
- [ ] **Frame body in a shared orbit** · HOV-02
  - Was F-134. The build makes the whole shared ring no target; the other members' dots and tails should still target them (HOV-05). Do it with or after "Large orbits fade out" above, which removes the frame-body tail.
- [ ] **Confirm built choices** · MOT-02, HOV-01
  - Was F-132 (shared ring labelled with the orbit name alone) and F-129 (the fold pill keeps its hover list). The build already does both; check and mark ✓.
- [ ] **Transfer simulation** · RAIL-02, RAIL-08, RAIL-09, BAND-01 to BAND-08, XFER-01 to XFER-12, PLAN-01 to PLAN-30, TEST-20, ARCH-22
  - Builds on the orbits crate (`crates/orbits`: `Trajectory<K>` takes any segment kind that implements `Rail`; `Path` is the crate's own Circular / Conic / Spiral). RAIL-02's game kinds (Parked with a named-orbit reference, Conic and Spiral with their primary) belong here, as a sim segment kind wrapping the crate's paths. Bands and costs, then legs against XFER-12's figures (see F-136), then plans and delta-v accounting. No UI; nothing new is drawn. `SimTunables` is new (ARCH-22). Headless tests can schedule a plan on the TEST-20 warship and advance the clock.
- [ ] **Inset as a sidebar; "In orbit" outline** · INS-05, INS-07
  - `inset_dock_margin_px` is withdrawn; `sidebar_slide_ms` is new.
  - The shared-orbit row ("Halden IV · High", expand and collapse, revealed by a ring click) is already in the flat list (`view/in_orbit.rs`, `ExpandedOrbits`); carry it into the outline.
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

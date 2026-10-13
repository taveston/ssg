# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Ship and station symbols** · SYM-01 to SYM-08, SYM-13 to SYM-15, TEST-05, and the reworded MRK-01, MRK-04, GRP-06, INS-05, TEST-03
  - Replaces the triangle, diamond and hollow-square markers; delete their code. The `Ownership` enum (own, neutral, foreign) gives way to affiliation and command (SYM-01).
  - Reference: mock-up `docs/ideas/presentation/mockups/7-symbology.png` for shapes and proportions. Ignore its installation role icons.
  - Most rules can be tested through what drives the picture: which frame, icon, fill and amplifiers a known-values record produces, and pixel sizes.
- [ ] **Ship groups and condition** · SYM-09, SYM-10, SYM-12, GRP-09, TEST-06
  - Needs the symbols item first.
- [ ] **Velocity leaders** · SYM-11
  - Blocked: ships in transit are not specced yet.
- [ ] **Ring selection and docked inset** · HOV-02, HOV-03, INS-01 to INS-03, INS-05 to INS-07, MOT-07, MRK-08, CAM-22, TEST-08
  - The inset docks at the right of the map with no leader line. Delete the leader-line drawing, the anchoring and clamping code, and Pin with all pinned-inset state.
  - MOT-05 and MOT-09 are retired: ring hit-testing now covers every drawn orbit (HOV-02), and a docked inset needs no anchor. Delete `inset_offset_px`.
  - `focus_bracket_px` becomes `focus_bracket_margin_px` (F5-08 described the built behaviour; keep it, just as a margin).
- [ ] **Orbit tails on the ring** · MOT-01, MOT-02, MOT-04, MOT-10, MOT-11, MOT-12, TEST-08
  - Replaces the sampled dot trail and the two-threshold switch. MOT-03 is retired: delete the cross-fade, `motion_crossfade_ms`, `t_dot_secs`, `t_ring_secs`, `trail_arc_degrees` and the trail sampling. Add `tail_secs` and `dot_fade_from_degrees`.
  - Draw the tail as a gradient on the ring's own stroke (per-vertex alpha), not as separate geometry.
  - Testable through tail arc length and dot alpha for known periods and rates.
- [ ] **Placeholder knowledge images** · INS-04, TEST-04
  - Remove the stale state, its sepia image and all status text and dates from the inset. Lantern no longer needs stale data.
- [ ] **Astronomical facts in the inset** · INS-08, UNIT-01 to UNIT-03
  - Needs SCL-05 (body radii) from "Deliberate focus and two-stop zoom" first.
- [ ] **FPS debug readout** · CLK-06
  - Add the toggle binding to the bindings resource (CAM-18).
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.

## Fixes

- [ ] **Gate edge labels show the destination alone** · EDGE-01
  - Label reads "Ostra", not "Gate Ostra".
- [ ] **Camera sometimes stops following the focus on zoom-out** · CAM-05, CAM-08, ZOOM-13
  - No reliable repro. That item has been built and rewrote this logic; check whether it still happens, and fix if so.

## Other

None yet.

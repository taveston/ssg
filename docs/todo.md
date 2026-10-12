# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Deliberate focus and two-stop zoom** · ZOOM-10, ZOOM-14 to ZOOM-16, CAM-02 to CAM-04, CAM-06 to CAM-08, CAM-23, CAM-24, MRK-07, SCL-05; withdraws CAM-21
  - Delete zoom-to-focus (CAM-21) and the region frame, with their code and tests.
  - Bodies need a physical radius (SCL-05). Give Corvane's bodies plausible ones: the star, Pell, Iver and Halden IV (rocky, a few thousand km), Morrow 71,500 km, moons a few hundred to a few thousand km.
  - The standard orientation now has a tilt, so tests that assume a top-down start (CAM-02) need updating.
  - CAM-24 needs the motion mode per object, which MOT-02 already computes.
  - F-105 (camera sometimes detaches on zoom-out) may go away with this; check it while here.

- [ ] **Ship and station symbols** · SYM-01 to SYM-08, SYM-13 to SYM-15, TEST-05, and the reworded MRK-01, MRK-04, GRP-06, INS-05, TEST-03
  - Replaces the triangle, diamond and hollow-square markers; delete their code. The `Ownership` enum (own, neutral, foreign) gives way to affiliation and command (SYM-01).
  - Reference: mock-up `docs/ideas/presentation/mockups/7-symbology.png` for shapes and proportions. Ignore its installation role icons.
  - Most rules can be tested through what drives the picture: which frame, icon, fill and amplifiers a known-values record produces, and pixel sizes.
- [ ] **Ship groups and condition** · SYM-09, SYM-10, SYM-12, GRP-09, TEST-06
  - Needs the symbols item first.
- [ ] **Velocity leaders** · SYM-11
  - Blocked: ships in transit are not specced yet.
- [ ] **Inset placement and dismissal** · INS-03, INS-07
  - Delete the old leader-line drawing; the leader is now one right-angled bend (INS-07).
- [ ] **Easier selection** · MRK-08, MOT-05, MOT-09, CAM-22
  - `focus_bracket_px` becomes `focus_bracket_margin_px` (F5-08 described the built behaviour; keep it, just as a margin).
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
  - No reliable repro. Check it after "Deliberate focus and two-stop zoom", which rewrites this logic; fix if it remains.

## Other

None yet.

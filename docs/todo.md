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
- [ ] **Trackpad pinch** · ZOOM-07
  - Blocked: see F3-01.
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.

## Fixes

None yet.

## Other

None yet.

# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Realistic Morrow** · TEST-07, TEST-08
  - Set Morrow's mass to 318 Earth masses and replace its moon radii (now 40,000 km plus 25,000 km a step) with radii from the target periods, as Halden IV's were.
  - The satellite frame now spans about 6.6 million km instead of 0.3 million, so Morrow's inner moons fold at the satellite frame and unfold as you zoom in (GRP-01, GRP-02). Check any test that depends on where Morrow's moons are.
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

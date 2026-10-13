# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Ship groups and condition** · SYM-09, SYM-10, SYM-12, GRP-09, TEST-06
  - The symbols are built; groups add to them (`view/symbols.rs`, `symbol_draw.rs`).
- [ ] **Velocity leaders** · SYM-11
  - Blocked: ships in transit are not specced yet.
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

## Other

None yet.

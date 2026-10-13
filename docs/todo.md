# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Floating origin at the frame body** · SCL-06
  - Fixes the wobble of distant frame bodies (was F-101). The subtraction belongs in `view/scale.rs` and `projection.rs`.
- [ ] **Orbit rings only for natural bodies; frame body's ring not a target** · MRK-05, MRK-09, HOV-02
  - Watch fast ships in dot mode: with no ring they are harder to click. The tail is a target (HOV-02); if that's not enough, add a finding.
- [ ] **Double-click on an object that can't be a frame moves to its parent's frame** · ZOOM-16
- [ ] **Natural bodies: neutral colour, larger markers** · MRK-01, MRK-03, MRK-10
  - Body marker sizes are hard-coded in `marker_half_size` (`view/markers.rs`); move them to `Tunables`. Check pills, labels and frame brackets still clear the bigger discs.
- [ ] **Satellites of a moon in Corvane** · TEST-09
- [ ] **Ship groups and condition** · SYM-09, SYM-10, SYM-12, GRP-09, TEST-06
  - The symbols are built; groups add to them (`view/symbols.rs`, `symbol_draw.rs`).
- [ ] **Velocity leaders** · SYM-11
  - Blocked: ships in transit are not specced yet.
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.

## Fixes

- [ ] **Gate edge labels show the destination alone** · EDGE-01
  - Label reads "Ostra", not "Gate Ostra".

## Other

- [ ] **Screenshot debug option**
  - A built-in way to capture the game window without editing `main.rs`: for example, an environment variable naming a view (frame body, distance, tilt, time rate) and an output path. The app sets that view, waits for it to settle, saves a screenshot and exits.
  - Keep it in a view debug module, read through the bindings or a debug resource, not in `main.rs`. Then replace the "temporarily edit `main.rs`" steps in CLAUDE.md ("Seeing the game").

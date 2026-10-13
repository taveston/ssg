# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Partly transparent exposure rings; thinner normal rings** · MOT-01, MOT-02, MRK-05, HOV-03
  - The tail's head uses the same alpha as the exposure ring, so the closing stays seamless (MOT-11).
  - The normal width is hard-coded as `NORMAL_RING_WIDTH_PX` in `view/rings.rs`; move it and the highlight width to `Tunables`.
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

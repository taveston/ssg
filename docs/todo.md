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
- [ ] **Stations in bands** · SCL-07, BAND-02, TEST-02, TEST-09, TEST-10
  - A station takes a place in a body's Low or High band, as a ship does; the Station place becomes the band it occupies. Move Arden Highport and Tern Relay into Low bands (`content/corvane.rs`) and drop the station's own-orbit placement.
  - A station has a phase in its band (BAND-02, PHAS-01). A ship docked at it sits at that phase; the **Phase in transfers** item below builds the arrival.
- [ ] **Phase in transfers** · BAND-05, BAND-06, XFER-02, XFER-10, PLAN-11, PHAS-01 to PHAS-13
  - Ships in one band are at real places. A same-band target is a phasing leg; a station or ship in another body's band is met by timing the arrival; leaving still ignores phase.
  - The maths goes in the `orbits` crate (PHAS-12): the phasing orbit and the capture angle. Lambert can't do the same-band case, because the path returns to its start point.
  - In `src/sim/transfer/`: the capture phase is now the closest point of the approach path, not the angle of the arrival velocity (`legs.rs`, BAND-06). A target ship or station is read at its own phase, not as its band. A leg that targets a ship stores the orbit and phase it was solved against (PLAN-11). Add `together_km` and `max_phasing_laps` to `SimTunables`.
  - Build after **Stations in bands**, so stations have phases to meet.
- [ ] **Velocity leaders** · SYM-11
  - Blocked: ships in transit are simulated (`src/sim/transfer/`) but drawing them is not specced.
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.
- [ ] **Sol test system** · SOL-01 to SOL-11, SOL-22 to SOL-23
  - New `src/content/sol.rs` beside `corvane.rs`. The `--system` choice goes in `content/mod.rs`.
  - Sol is about seven times wider than Corvane: Neptune is about 2,700 render units out. Check the far clip (20,000 units, `view/camera/rig.rs`) at the system fit and at the pan limit.

## Fixes


## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.
- [ ] **Screenshot debug option**
  - A built-in way to capture the game window without editing `main.rs`: for example, an environment variable naming a view (frame body, distance, tilt, time rate) and an output path. The app sets that view, waits for it to settle, saves a screenshot and exits.
  - Keep it in a view debug module, read through the bindings or a debug resource, not in `main.rs`. Then replace the "temporarily edit `main.rs`" steps in CLAUDE.md ("Seeing the game").

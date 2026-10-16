# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Velocity leaders** · SYM-11
  - Ships in flight are drawn now (PLNR-68, `view/paths/`); SYM-11 adds the leader PLNR-68 names. `OW_SHOT`'s `fly` key shows a ship in flight.
- [ ] **Label halo** · LBL-05
  - A 2 px ground-colour outline round every label's glyphs (`label_halo_px`), not a box. Mock-up 19, last board.
- [ ] **Reference frame cue** · REF-01 to REF-10, MRK-01, MRK-12, CAM-25
  - A dotted crosshair in the orbital plane through the frame body, with ticks, an N on the north arm and a solid vertical axis. Mock-up 19. Violet (#9B8CE8) is a starting hue: tune it on screen.
  - CAM-25: the breadcrumb entry for the current frame is violet too.
  - Build after **Label halo**: the N uses it.
  - Tests can run through projected positions: where an arm starts and ends, tick spacing against the scale bar, the vertical axis shortening with tilt, and the draw order.
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

## Fixes

## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Several objects on one marker, part 2: the tray** · MUL-07 to MUL-10, MUL-12, GRP-07, GRP-09, HOV-01, HOV-02, HOV-05, INS-03, INS-06, MRK-12, CAM-10, PLNR-11
  - Part 1 (the multi-marker on the map) is built. A click on a multi-marker now opens a tray of what it holds (MUL-07). Tunables: the five `tray_*` ones.
  - Meaning changed, so their ✓ is cleared: GRP-07, GRP-09, HOV-05, INS-03, INS-06, MRK-12, CAM-10 and PLNR-11. HOV-01 and HOV-02 were reworded only (pill to multi-marker), but their ✓ is cleared too, since the pill they name is gone.
  - MRK-12 needs only the tray layer; CAM-10 needs the tray's root to carry `MapInputBlocker`. INS-06's mark already moves onto the multi-marker.
  - Tests: a click opens one tray and selects the parent or the first row (MUL-07); Up, Down, Right, Left and Esc in the tray, with the arrow keys not panning; the tray stays inside the map area and follows its marker; with the planner open a row picks as its symbol would.

## Fixes

## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

## Parked

Specced but not built, because nothing needs it yet. Don't pick these for a coding session. Each says what brings it back. Their requirements still count for the coverage check in `docs/README.md`.

- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - **Revisit when:** a system reads ideology (a faction, settlement or character that carries a profile, or an effect that uses the scores) and the UI for showing it is specced. Until then the code would have no caller.
  - Decide the open point in `docs/spec/ideology.md` first (whether characters share the structure), and say where the option values are loaded from (IDEO-31).
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Transfer planner** · PLNR-01 to PLNR-05, PLNR-10 to PLNR-14, PLNR-20 to PLNR-32, PLNR-40 to PLNR-42, PLNR-50 to PLNR-54, PLNR-60 to PLNR-69, PLNR-87, PLNR-91, PLNR-94
  - In place: the search rules (`sim/transfer/` `ranges`, `grid`, `presets`, `search`, `coast`, reached through `Planner::search_context`), `SearchKey` for PLNR-87's recompute, and every planner string and line in `view/planner_words.rs` (the tooltips' text included). The panel and the map call these; drop their `expect(dead_code)` as they are wired.
  - Changes INS-01, INS-02, INS-03 and SYM-13 (their ✓ is cleared). The Transfer button, the planner panel and its destination picking, and the order countdown in a ship's label come from them.
  - SYM-13's group parts (the member count by class, and the label clear of the amplifiers) are built; only the countdown is left.
  - Intercept is offered for stations (PLNR-30) and shows its trim (PLNR-32), which needs **Phase in transfers** (PHAS-15).
  - PLNR-68 draws ships in flight for the first time, so GRP-08 and MOT-08 now have real ships to test against.
  - PLNR-40 to PLNR-42 need **Phase in transfers**. Build them after it, and build the rest first if the planner is wanted sooner.
  - The mock-ups (9 to 12) show a blue Schedule button and a choice labelled Stop / Pass; BTN-01, BTN-02 and PLNR-30 decide, so it is the neutral primary button and Rendezvous / Intercept.
- [ ] **Velocity leaders** · SYM-11
  - Build after **Transfer planner**, which draws ships in flight (PLNR-68).
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

## Fixes

- [ ] **Playtest bugs: brackets, disc clicks, empty-space clicks** · HOV-06, HOV-07, MOT-14, MRK-08, INS-03
  - F-185: brackets round an exposure ring's identity mark sit slightly off it. Put the box and the mark on the same point.
  - F-186: a body drawn at its true size (MRK-07) can't be clicked. Its hit area is at least the disc. Add that sentence to MRK-08 (agreed 2026-10-16).
  - F-187: a click on empty map space doesn't clear the selection (INS-03). Find what takes the click.

## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

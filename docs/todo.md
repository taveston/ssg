# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Transfer planner** · PLNR-60 to PLNR-69, SYM-13
  - In place: the search rules (`sim/transfer/`), every planner string (`view/planner_words.rs`) and the panel (`view/planner/`): the Transfer button, choosing and picking, tuning, orders, Change and Cancel. What is left is section 6, "On the map", and the countdown in a ship's label.
  - Draw from `PlannerState` (`view/planner_state.rs`): `session.leg` is the leg of the current setting (a `SolvedLeg` with its trajectory, `arrival_speed_kms` and `turn_rad`), `session.request` its destination and arrival mode, and `session.hover` the hovered place's Fastest leg while choosing (PLNR-12). A ship's order is its `Plan` component; `Leg` carries the same figures.
  - SYM-13's group parts (the member count by class, and the label clear of the amplifiers) are built; only the countdown is left (`LegWords::countdown`). Drop the `expect(dead_code)` on it, `ghost_label`, `coast_label` and `sim/transfer/coast.rs` as they are wired.
  - PLNR-68 draws ships in flight for the first time, so GRP-08 and MOT-08 now have real ships to test against.
  - The path, ghost and consequence tunables of the planner's table (`path_width_px` to `ghost_dash_px`) are not in `Tunables` yet.
- [ ] **Velocity leaders** · SYM-11
  - Build after **Transfer planner**, which draws ships in flight (PLNR-68).
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

## Fixes

## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

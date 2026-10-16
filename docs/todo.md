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
- [ ] **Bracket and label spacing** · LBL-01, SYM-09, SYM-12, HOV-06, INS-06, MRK-08
  - Settles the bracket and label spacing findings. Most of it is already how the build behaves; the spec now says so. The ✓ on these is cleared until the code is checked against the new text.
  - What changes: a label's gap is measured from the box (HOV-07) for every marker, so labels no longer move on hover, and `label_gap_px` drops to 4. Bracket arms are capped by `bracket_arm_max_px`. New tunables: `amplifier_gap_px`, `condition_track_mix`.
  - SYM-13's wording on the gap changed too; it lands with **Transfer planner**.
  - The playtest bug fixes are built (e3192a2). MRK-08 now measures a disc's hit area out to the edge of its rim (MRK-11); check `marker_picking.rs` against that and re-tick it.
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

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
- [ ] **Bracket and label spacing** · LBL-01, SYM-09, SYM-12, HOV-06, INS-06
  - Settles the bracket and label spacing findings. Most of it is already how the build behaves; the spec now says so. The ✓ on these is cleared until the code is checked against the new text.
  - What changes: a label's gap is measured from the box (HOV-07) for every marker, so labels no longer move on hover, and `label_gap_px` drops to 4. Bracket arms are capped by `bracket_arm_max_px`. New tunables: `amplifier_gap_px`, `condition_track_mix`.
  - SYM-13's wording on the gap changed too; it lands with **Transfer planner**.
  - Touches the same code as **Playtest bugs** (HOV-06). Build them together.
- [ ] **Label halo** · LBL-05
  - A 2 px ground-colour outline round every label's glyphs (`label_halo_px`), not a box. Mock-up 19, last board.
- [ ] **Reference frame cue** · REF-01 to REF-10, MRK-01, MRK-12
  - A dotted crosshair in the orbital plane through the frame body, with ticks, an N on the north arm and a solid vertical axis. Mock-up 19. Violet (#9B8CE8) is a starting hue: tune it on screen.
  - Build after **Label halo**: the N uses it.
  - Tests can run through projected positions: where an arm starts and ends, tick spacing against the scale bar, the vertical axis shortening with tilt, and the draw order.
- [ ] **Belt** · MRK-18
  - Replaces the hard-edged band: soft edges and a seeded scatter of faint points.
- [ ] **Exposure ring cues** · MOT-02, MOT-14
  - A period line under the ring's label and a direction chevron beside the identity mark. Mock-up 19, exposure rings board.
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

## Fixes

- [ ] **Playtest bugs: brackets, disc clicks, empty-space clicks** · HOV-06, HOV-07, MOT-14, MRK-08, INS-03
  - Brackets round an exposure ring's identity mark sit slightly off it. Put the box and the mark on the same point.
  - A body drawn at its true size (MRK-07) can't be clicked. Its hit area is at least the disc. MRK-08 now says so.
  - A click on empty map space doesn't clear the selection (INS-03). Find what takes the click.

## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

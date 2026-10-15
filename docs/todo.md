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
  - The ships of a group share one phase (PHAS-14, TEST-10), so TEST-10's check that no two ships share a phase must skip ships of one group.
- [ ] **Stations in bands** · SCL-07, BAND-02, TEST-02, TEST-09, TEST-10
  - A station takes a place in a body's Low or High band, as a ship does; the Station place becomes the band it occupies. Move Arden Highport and Tern Relay into Low bands (`content/corvane.rs`) and drop the station's own-orbit placement.
  - A station has a phase in its band (BAND-02, PHAS-01). A ship docked at it sits at that phase; the **Phase in transfers** item below builds the arrival.
- [ ] **Phase in transfers** · BAND-05, BAND-06, XFER-02, XFER-05, XFER-10, PLAN-11, PHAS-01 to PHAS-15
  - Ships in one band are at real places. A same-band target is a phasing leg; a station or ship in another body's band is met by timing the arrival; leaving an impulsive drive's band waits for the exit point. Ships in a group share one phase.
  - The maths goes in the `orbits` crate (PHAS-12): the phasing orbit, the capture angle and the exit angle. Lambert can't do the same-band case, because the path returns to its start point.
  - In `src/sim/transfer/`: the capture phase is now the closest point of the approach path, not the angle of the arrival velocity (`legs.rs`, BAND-06). A target ship or station is read at its own phase, not as its band. A leg that targets a ship stores the orbit and phase it was solved against (PLAN-11). Add `together_km`, `max_phasing_laps` and `trim_max_iterations` to `SimTunables`.
  - A Pass at a station or a parked ship in another body's band is trimmed to meet it (PHAS-15): the solver lengthens `travel_days` by under one period of the band and repeats until the target is at the Pass's point.
  - Build after **Stations in bands**, so stations have phases to meet.
- [ ] **Transfer planner** · PLNR-01 to PLNR-05, PLNR-10 to PLNR-14, PLNR-20 to PLNR-32, PLNR-40 to PLNR-42, PLNR-50 to PLNR-54, PLNR-60 to PLNR-69, PLNR-80 to PLNR-88, PLNR-90 to PLNR-93, UNIT-04, TEST-30
  - Changes INS-01, INS-02, INS-03 and SYM-13 (their ✓ is cleared). The Transfer button, the planner panel and its destination picking, and the order countdown in a ship's label come from them.
  - The simulation is built. New in `src/sim/transfer/`: the search rules of PLNR-80 to PLNR-88 (ranges, grid, presets, coast consequence) as pure functions. A drive also needs a display name (`sim/ship.rs`, `content/corvane.rs`, TEST-30).
  - Pass is offered for stations (PLNR-30) and shows its trim (PLNR-32), which needs **Phase in transfers** (PHAS-15). A station in a Low band can't be passed until a Pass through Low is specced (XFER-08), so a raid on today's stations needs that first.
  - PLNR-68 draws ships in flight for the first time, so GRP-08 and MOT-08 now have real ships to test against.
  - PLNR-40 to PLNR-42 need **Phase in transfers**. Build them after it, and build the rest first if the planner is wanted sooner.
  - The mock-ups (9 to 12) show a blue Schedule button; BTN-01 and BTN-02 decide, so it is the neutral primary button.
- [ ] **Velocity leaders** · SYM-11
  - Build after **Transfer planner**, which draws ships in flight (PLNR-68).
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.

## Fixes

- [ ] **Keep arcs out of the planet** · XFER-07
  - The build exempts every crossing that starts or ends in an orbit of the primary (`check_periapsis` in `src/sim/transfer/legs.rs`), so a High → Low arc to a station on the far side of the planet can pass through it. Replace the exemption with the Low band limit of the new XFER-07, and drop the F-148 comment.
  - Tests: a far-side arc to a Low band is infeasible with "Path passes too close to *primary*" (PLNR-92); a near-side arc to a Low band, and a departure from one, are feasible. Arriving exactly at the Low radius must pass, so allow for rounding.

## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.
- [ ] **Screenshot debug option**
  - A built-in way to capture the game window without editing `main.rs`: for example, an environment variable naming a view (frame body, distance, tilt, time rate) and an output path. The app sets that view, waits for it to settle, saves a screenshot and exits.
  - Keep it in a view debug module, read through the bindings or a debug resource, not in `main.rs`. Then replace the "temporarily edit `main.rs`" steps in CLAUDE.md ("Seeing the game").

# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Orbit fade defaults** · MRK-14
  - Changes MRK-14 (its ✓ is cleared): `orbit_fade_from` becomes 4 and `orbit_fade_to` 12 (they were 1.5 and 3). A ring now keeps its full alpha until its arc across the view is only mildly curved, and is gone once it is near-straight. Adjust in play if it still feels wrong.
  - The measure stays as built (`ring_screen_radius_px` in `view/rings.rs`); MRK-14 now says so. Change the defaults in `view/tunables.rs`, and any test that assumes the old ones.
- [ ] **Edge-tab arrow outside the tab** · EDGE-03, MRK-15
  - Changes EDGE-03 and MRK-15 (their ✓ is cleared): the arrow leaves the tab. It sits at the pinned point on the map area's edge and the tab beside it on the inner side, so the arrow is outside the tab's outline. The parent's tab keeps its marker and name inside.
  - One change in `view/edge_tab.rs`, shared by the gates (`view/edge.rs`) and the parent (`view/parent_tab.rs`): take the chevron out of the tab node, and move it with the tab as `tab_origin` slides the tab along the edge, so arrow and tab together stay whole inside the map area. Gates and the parent keep the same look.
- [ ] **Orbit rows in the outline** · INS-05
  - Changes INS-05 (its ✓ is cleared). In `view/outline.rs`: drop the step in `children_of` that turns a one-member orbit back into a plain entry, so every named orbit with members gets its row. Name the row "High orbit · 14" (the orbit's name, "orbit", the member count) instead of "Halden IV · High · 14". An orbit with no members still gets no row.
  - Tests: Halden IV's outline has "Low orbit · 1" (Arden Highport) and "High orbit" with its ships; a body with empty bands has no orbit rows; selecting a lone member opens its row.
- [ ] **Condition bar** · SYM-12
  - The symbols are built; the bar adds to them (`view/symbols.rs`, `symbol_draw.rs`).
  - Build a group's mean as a function of its members' conditions, tested on its own, so **Ship groups** can use it.
- [ ] **Ship groups** · SYM-09, SYM-10, GRP-09, TEST-06
  - Nothing forms groups in play yet (F-159), so only the test content (TEST-06) has them. TEST-06 now has the one task force; give the own ships of TEST-03 and TEST-05 conditions above `condition_warn_fraction`.
  - The symbols are built; groups add to them (`view/symbols.rs`, `symbol_draw.rs`).
  - A group holds its members and takes its class, armed status, load and condition from them (SYM-01, SYM-09).
  - SYM-13 is still open for the order countdown (**Transfer planner**). Its group parts come with this item: the member count by class, and the label clear of the symbol's amplifiers.
  - The ships of a group share one phase (PHAS-14, TEST-10), so TEST-10's check that no two ships share a phase must skip ships of one group.
- [ ] **Phase in transfers** · BAND-05, BAND-06, XFER-02, XFER-05, XFER-10, PLAN-11, PHAS-01 to PHAS-15
  - Ships in one band are at real places. A same-band target is a phasing leg; a station or ship in another body's band is met by timing the arrival; leaving an impulsive drive's band waits for the exit point. Ships in a group share one phase.
  - The maths goes in the `orbits` crate (PHAS-12): the phasing orbit, the capture angle and the exit angle. Lambert can't do the same-band case, because the path returns to its start point.
  - In `src/sim/transfer/`: the capture phase is now the closest point of the approach path, not the angle of the arrival velocity (`legs.rs`, BAND-06). A target ship or station is read at its own phase, not as its band. A leg that targets a ship stores the orbit and phase it was solved against (PLAN-11). Add `together_km`, `max_phasing_laps` and `trim_max_iterations` to `SimTunables`.
  - An Intercept at a station or a parked ship in another body's band is trimmed to meet it (PHAS-15): the solver lengthens `travel_days` by under one period of the band and repeats until the target is at the Intercept's point.
- [ ] **Intercept through Low** · XFER-08, XFER-13, XFER-14, XFER-15
  - Changes XFER-08 (its ✓ is cleared): an Intercept may take a Low band. In `src/sim/transfer/legs.rs` an Intercept through Low ends the crossing on the crossing end day and starts a new Conic round the primary with the turned velocity (XFER-13).
  - The turn angle and the closing speed go in the `orbits` crate (XFER-15). The closing speed at a station (XFER-14) is read only once **Phase in transfers** gives stations phases.
  - The planner's PLNR-30 no longer disables Intercept for a Low band, so build this before **Transfer planner**.
- [ ] **Transfer planner** · PLNR-01 to PLNR-05, PLNR-10 to PLNR-14, PLNR-20 to PLNR-32, PLNR-40 to PLNR-42, PLNR-50 to PLNR-54, PLNR-60 to PLNR-69, PLNR-80 to PLNR-88, PLNR-90 to PLNR-94, UNIT-04, TEST-30
  - Changes INS-01, INS-02, INS-03 and SYM-13 (their ✓ is cleared). The Transfer button, the planner panel and its destination picking, and the order countdown in a ship's label come from them.
  - The simulation is built. New in `src/sim/transfer/`: the search rules of PLNR-80 to PLNR-88 (ranges, grid, presets, coast consequence) as pure functions. A drive also needs a display name (`sim/ship.rs`, `content/corvane.rs`, TEST-30).
  - Intercept is offered for stations (PLNR-30) and shows its trim (PLNR-32), which needs **Phase in transfers** (PHAS-15). A raid on today's stations, which are in Low bands, needs **Intercept through Low**.
  - PLNR-68 draws ships in flight for the first time, so GRP-08 and MOT-08 now have real ships to test against.
  - Build after **Intercept through Low**, because Intercept is offered for a Low band (PLNR-30).
  - PLNR-40 to PLNR-42 need **Phase in transfers**. Build them after it, and build the rest first if the planner is wanted sooner.
  - The mock-ups (9 to 12) show a blue Schedule button and a choice labelled Stop / Pass; BTN-01, BTN-02 and PLNR-30 decide, so it is the neutral primary button and Rendezvous / Intercept.
- [ ] **Velocity leaders** · SYM-11
  - Build after **Transfer planner**, which draws ships in flight (PLNR-68).
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.

## Fixes

- [ ] **Keep arcs out of the planet** · XFER-07
  - The build exempts every crossing that starts or ends in an orbit of the primary (`check_periapsis` in `src/sim/transfer/legs.rs`), so a High → Low arc to a station on the far side of the planet can pass through it. Replace the exemption with the Low band limit of the new XFER-07, and drop the F-148 comment.
  - The check also covers the coast after an Intercept: the conic the ship is left on (the turned one after an Intercept through Low, XFER-13) must not have its periapsis ahead inside the limit.
  - Tests: a far-side arc to a Low band is infeasible with "Path passes too close to *primary*" (PLNR-92); a near-side arc to a Low band, and a departure from one, are feasible. An Intercept in a planet's frame whose coast would dip below the Low band is infeasible. Arriving exactly at the Low radius must pass, so allow for rounding.

- [ ] **Drop `double_click_window_ms`**
  - The spec's tunables table no longer lists it: the outline selects on a single click at once (INS-05), and double-clicks use the picking backend's click count. Delete the field, its default and its `#[allow(dead_code)]` from `view/tunables.rs`. No behaviour changes.

## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Planner: whole-day Depart, and settings the solver rejects** · PLNR-13, PLNR-21, PLNR-23, PLNR-24, PLNR-29, PLNR-30, PLNR-33 to PLNR-35, PLNR-40, PLNR-50, PLNR-80 to PLNR-82, PLNR-87, PLNR-92, PHAS-16, XFER-02
  - Meaning changed, so their ✓ is cleared: PLNR-13, 21, 23, 24, 29, 30, 40, 50, 80, 81, 82, 87 and 92, and XFER-02. PLNR-33 to PLNR-35 and PHAS-16 are new.
  - PLNR-80 also now says what the build already does for a leg that ends on a whole band of its primary (the 180-day range, no Next window). Only the wording changed there.
  - Depart is in whole days, from Soonest, and the grid's columns are keyed by absolute day and kept as the clock runs (PLNR-82, PLNR-87), so a day's passing adds a column rather than redoing the search. Soonest is solved again once a game day, and again when Schedule is pressed (PLNR-50).
  - Every leg has a Depart slider, so the player can always delay (PLNR-40, PLNR-80). A phasing leg's track is one colour. A leg whose windows come round more than once a day leaves at its window (PHAS-16). That is a search at `window_wait_samples` starts for each grid cell, so keep the grid quick, for example by solving a column's window once and reusing it across the travel times when the answer allows.
  - The preset the sliders sit on is drawn selected (PLNR-29), so the default is visible, and moving a slider clears it.
  - A thumb can't rest on a rejected setting (PLNR-33). Over-budget settings stay selectable. The empty track colour and its tooltip (PLNR-35) show where the gaps are.
  - Band to band round one primary starts its Travel range at the Hohmann time (PLNR-81), and an Intercept through the primary's own Low band isn't offered (PLNR-30). Only whole bands are covered by PLNR-81: a leg to a station, a ship or a body keeps the old range, and PLNR-33 handles its rejections. Say in a finding if one of those looks wrong in play.
  - Tests: positions keep their day across a clock run; the grid isn't searched again on a day tick beyond the new columns; a drag across a rejected stretch lands on an accepted setting; Schedule from Soonest solves from the clock.
- [ ] **Planner tooltips** · PLNR-94 to PLNR-106
  - Tooltips for the planner's buttons and sliders, in plain words (PLNR-95). PLNR-94 changed (Rendezvous has a longer text), so its ✓ is cleared.
  - They sit on labels, not tracks, because a rejected stretch of a track has its own tooltip (PLNR-35). They use `tooltip_delay_ms`.
  - Tests: each text shows after the delay on its control; the variants show in their cases; no text contains a word PLNR-90 bans.

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

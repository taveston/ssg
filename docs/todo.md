# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Drop velocity leaders from the system view** · SYM-11, PLNR-68
  - SYM-11 is withdrawn: delete the leader's code and tests, and the four `leader_*` tunables. PLNR-68 no longer mentions the leader; only the wording changed, so its ✓ is cleared just so the build is checked against it.
  - Why: the drawn trajectory shows what the leader did, and the leader measured velocity relative to the star, so in a planet's frame it pointed along the planet's motion (was F-208, F-243). A combat view may want a leader later; the deleted code is in git history.

## Fixes

- [ ] **Orbit membership follows the ship** · F-242, F-201 · RAIL-09, SCL-07, HOV-04, INS-05, MRK-09, HOV-02
  - The spec already says it: a ship on a Parked segment is a member of its named orbit and nothing else (RAIL-09), wherever that orbit is. The build fixes membership at spawn, so a ship that arrives in an orbit isn't one of its members: after a transfer to another body its symbol and outline entry stay with the old body (F-242), and its new ring draws no tail and doesn't reach it on hover or click (F-201).
  - Keep each orbit's members, ring strokes, ring targets, identity mark, outline rows and fold parent up to date as ships leave and arrive, including an orbit that had no members before and one left empty.
  - Leads from the clean-up: `index_members` in `shared_orbit.rs` runs only at Startup, so `OrbitMembers` never changes (F-201). Arrival already moves the ship's `ChildOf` to the new body (`sim/transfer/plan.rs`), but `fill_outline` refills only when the selection or the expanded rows change, which may be why the outline lags (F-242). Check both.
  - From the clean-up's review, in the same code:
    - `rings/marks.rs` `sync_ring_marks` now removes a band's identity mark as soon as its last ship leaves, since `rings.owners()` lists an orbit only while it has members. A ship coming back then fades the mark in from 0 instead of resuming. Decide with the membership rework (keep the mark while the orbit exists), and fix the doc comment that says an empty ring keeps its mark.
    - `multi_markers/gather.rs` falls back to a thing's own `orbit.radius_km` when its named orbit isn't found; `together_sets` assumes one radius per orbit. Take the radius from the orbit, or skip the slot.
    - `overlap_clusters` relies on two unchecked invariants from `units()`: each thing is in at most one spot a frame, and each spot's lead is unique and one of its own things. Add a `debug_assert`.
    - `link.rs` `linking_many_things_does_not_test_every_pair` has a 3 s timing bound in a debug test (about 0.1 s now). Count comparisons instead of timing, so it can't flake.
  - Tests: Earth Low to Moon Low moves the ship's symbol, fold parent and outline entry to the Moon; a ship arriving in an empty orbit gets a ring with its tail, and hover and click on that ring reach it; the orbit it left loses it.
  - Delete F-242 and F-201 when done.

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

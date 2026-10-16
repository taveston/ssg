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
- [ ] **Moons fold into a ring on the planet's marker** · GRP-05, GRP-10, GRP-11, MUL-02, HOV-06, HOV-07, INS-06
  - A parent whose folded satellites are all natural bodies draws a second ring behind its marker and a dim "+N" on its label's second line, in place of a fold marker (GRP-10, GRP-11). A parent with any folded ship, group or station keeps the fold marker, which holds the folded bodies too. GRP-05, MUL-02, HOV-06, HOV-07 and INS-06 changed meaning, so their ✓ is cleared.
  - Delete the all-bodies look of the multi-marker (plain neutral circles) and its tests (MUL-02). Add the tunables `moon_ring_offset_px` (3), `moon_count_px` (9) and `moon_count_alpha` (0.7).
  - Only the parent's own folded satellites decide which form it takes, as they do for the count (MUL-04). A moon whose own station is folded into it is still one body.
  - Tests: a parent with only moons folded shows the ring and "+N" and no fold marker; folding a ship or station in switches it to the fold marker and removes the ring and the count line; the ring's box and hit area count in the marker's box (HOV-07) and a click on it selects the parent and opens no tray; a selected folded moon puts the mark round the marker and the ring; the "+N" line is the first thing dropped when its label has no room (LBL-03). Mock-ups 20 to 22 show the look.

## Fixes

- [ ] **Orbit membership follows the ship** · F-242, F-201 · RAIL-09, SCL-07, HOV-04, INS-05, MRK-09, HOV-02
  - The spec already says it: a ship on a Parked segment is a member of its named orbit and nothing else (RAIL-09), wherever that orbit is. The build fixes membership at spawn, so a ship that arrives in an orbit isn't one of its members: after a transfer to another body its symbol and outline entry stay with the old body (F-242), and its new ring draws no tail and doesn't reach it on hover or click (F-201).
  - Keep each orbit's members, ring strokes, ring targets, identity mark, outline rows and fold parent up to date as ships leave and arrive, including an orbit that had no members before and one left empty.
  - Leads from the clean-up: `index_members` in `shared_orbit.rs` runs only at Startup, so `OrbitMembers` never changes (F-201). Arrival already moves the ship's `ChildOf` to the new body (`sim/transfer/plan.rs`), but `fill_outline` refills only when the selection or the expanded rows change, which may be why the outline lags (F-242). Check both.
  - Tests: Earth Low to Moon Low moves the ship's symbol, fold parent and outline entry to the Moon; a ship arriving in an empty orbit gets a ring with its tail, and hover and click on that ring reach it; the orbit it left loses it.
  - Delete F-242 and F-201 when done.

## Other

- [ ] **Multi-markers and tray: clean-up after review**
  - Work per frame grows with the square of the ship count (`multi_markers/gather.rs`: `together_sets`, `link_groups`, `overlap_clusters`, `slide_things`, `fold_markers`; `MultiMarkers::get` is a linear search). Link only neighbours in phase, bucket overlap candidates into a grid of `cluster_split_px` cells, and index markers by key.
  - `place_multi_markers` writes every pool node's `Node` each frame. Write only on change.
  - The tray rebuilds every row on each arrow step. Leave `selected` out of the rebuild check and recolour the rows instead.
  - The tray's rows repeat the outline's row code (`tray/draw.rs` `spawn_row`, `inset/outline.rs`), with slightly different indent and toggle width. Share one row builder.
  - The marker box (HOV-07) is computed in both `tray/contents.rs` and `markers/brackets.rs`. Keep one function in `multi.rs`.
  - The four `MultiLook` builds could share constructors in `look.rs`.
  - Split the long functions: `spawn_tray` and `spawn_row` (about 85 and 90 lines), and `TrayInputs::line` (about 50).
  - Headless test helpers: `the_window` is in both `headless_tests/camera.rs` and `tray.rs`, and `camera.rs` reads the `HoverMap` inline where `tray.rs` has `hovered()`. Share them in `headless_tests.rs`.
  - `overlap_clusters` now latches pairs of things, docked things included, so two overlapping stations with many ships docked give |a|·|b| pairs a frame. Fold this into the rework above, for example by latching lead things plus each unit's previous lead.
  - `rings/marks.rs` `sync_ring_marks` compares a lone body's mark (`MarkShows::Body`) by entity only, so a change in that body's look (its group face, its record) isn't redrawn. Compare its look inputs too.
  - `sync_ring_marks` has no branch for a ring whose owner leaves `rings.owners()`, so its mark would stay. Harmless while named orbits never go away; remove it when they can.

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

## Parked

Specced but not built, because nothing needs it yet. Don't pick these for a coding session. Each says what brings it back. Their requirements still count for the coverage check in `docs/README.md`.

- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - **Revisit when:** a system reads ideology (a faction, settlement or character that carries a profile, or an effect that uses the scores) and the UI for showing it is specced. Until then the code would have no caller.
  - Decide the open point in `docs/spec/ideology.md` first (whether characters share the structure), and say where the option values are loaded from (IDEO-31).
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

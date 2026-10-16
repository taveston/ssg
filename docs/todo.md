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
- [ ] **Planets: a neutral moon ring and an object pip that pops out** · F-229, F-230, F-239 · GRP-05, GRP-07, GRP-10 to GRP-13, LBL-01, MUL-02, MUL-04, MUL-06, MUL-08, HOV-06, HOV-07, INS-06
  - Moons and objects are separate. Folded natural bodies draw a neutral second ring behind the parent's marker and a dim "+N" counting only them (GRP-10, GRP-11). Folded ships, groups and stations draw a 6 px pip down and to the right of the parent, with a back pip when there are two or more (GRP-12). The fold marker is no longer drawn at rest.
  - When the pointer is within `pop_reach_px`, or the parent or something it holds is selected, or the tray is open, the pip grows into the fold marker and slides out so its box touches the parent's (GRP-13, MUL-06). Use the nearest parent, and `pop_release_px` as hysteresis, so it doesn't flicker in the crowded inner system.
  - The fold marker holds only objects, so its tray and count leave out folded moons (MUL-04). Its tray opens right, else below or above, never left over the parent (MUL-08). The parent's label tries left first (GRP-12, LBL-01).
  - Delete the fold marker at rest, the all-bodies look of the multi-marker (MUL-02) and the fold-marker-beside-parent placement with their tests. Add the tunables `object_pip_px` (6), `object_pip_gap_px` (2), `pip_back_offset_px` (3), `pop_reach_px` (24), `pop_release_px` (32), `pop_anim_ms` (120); `moon_ring_offset_px`, `moon_count_px` and `moon_count_alpha` as before.
  - Things docked at a station inside a larger marker count for the frames and the pip's colours but not for the count (MUL-02, MUL-04; F-230).
  - The pip's front and back colours follow MUL-02. Only the parent's own folded satellites decide, as they do for the count. A moon whose station is folded into it is still one body.
  - Tests: moons only shows the neutral ring and "+N", no pip; a folded ship adds the pip and leaves ring and "+N" unchanged; allied plus own shows an allied front pip and an own back pip; the pointer near pops it out and away releases it; selecting the parent pops it with no pointer; a selected folded ship puts the mark round the popped marker; the tray never covers the parent; the label doesn't move when the marker pops; the ring's box and hit area count in the parent's marker. Mock-ups 20 to 22, 26 and 27 show the look; mock-up 26's popped state sits below the planet and is superseded by GRP-13.

- [ ] **A shared orbit with one station shows the station's marker** · F-231 · MOT-14
  - MOT-14 now says a shared orbit whose only thing is a station shows that station's marker, with its docked count, as the build already does. Clear the check on the build's behaviour only if it differs; add a test for Iver's High band.

- [ ] **A joined marker draws one tail in dot mode** · F-232 · MUL-05
  - In dot mode a together or overlap marker draws the tail of its first tray row's thing, behind its centre, so a convoy or cluster no longer looks still on a running clock. Joined things still draw no tail of their own, and a fold marker has none. Ring mode is unchanged.
  - Tests: the TEST-12 convoy shows one tail in dot mode; an overlap cluster shows its lead's tail; the tail follows the lead when the tray order changes; no tail in ring mode beyond the exposure ring.

- [ ] **Two stations are never together** · F-233 · MUL-03
  - Reject content that puts two stations together (PHAS-02) with a loader error naming both. Remove the build's lead-and-docked handling of two stations at one point. Two overlapping station markers join as two things (MUL-05), each shown in the tray (MUL-09) with its own docked things under it.
  - Tests: the loader rejects two stations at one orbit and phase; two overlapping stations form a marker counting 2 with neither docked.

- [ ] **Name the shared orbit's mark in the planner click rule** · F-235 · MUL-12
  - MUL-12 now says what the build already does: with the planner open, a click on a shared orbit's identity mark picks the orbit's band and opens the tray. Add a test for it.

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

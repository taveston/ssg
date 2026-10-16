# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

## Fixes

- [ ] **Planner: review fixes** · PLNR-13, PLNR-33, PLNR-50
  - Found in review of the whole-day Depart item.
  - Add the missing tests: a Rendezvous or Intercept change that leaves the thumbs on a rejected setting moves them (PLNR-13, PLNR-33); Schedule from Soonest, when the leg solved again is rejected or over budget, makes no order and shows the reason (PLNR-50).
  - `debug_shot.rs` `fly_order`: if the Soonest solve fails at Schedule, the shot waits for its timeout. Exit at once with the planner's reason, as CLAUDE.md says.
  - At a clock rate where a game day passes every frame, Soonest is searched in full every frame (36 travel times × 24 starts for a window leg). Check for stutter; if there is, use the old Soonest's window phase as the hint for the new one.
- [ ] **Tray: review fixes** · HOV-02, MUL-10
  - Found in review of the tray item.
  - The HOV-02 test (`headless_tests/tray.rs`) has no ring under the tray. Put one there and check a click hits the row.
  - The row toggle (`tray/draw.rs`) skips `ClickCheck::selects`, so a press and move over it still opens or closes the row.

## Other

- [ ] **Planner: clean-up after review**
  - `legs.rs` `lambert_burns` repeats the ends and Lambert solve of `lambert_crossing`: share one helper.
  - Rename `Search.windows` (window timings) to `window_starts`, since `Sliders.windows` means something else.
  - `Search::soonest` has an unreachable `_ =>` arm, kept only by a presets test that builds a `Range` Depart slider. Change the test to `Days` and drop the arm.
  - `presets.rs` `refine` solves about 9 depart days for phasing legs, where every day costs the same. Refine on Soonest only.
  - Split the long functions: `leave_at_window` (about 70 lines), `move_thumb` and `depart_model` (about 50 each).
  - `tooltip.rs` `show_tooltips` writes the tooltip box's `Node` every frame. Write it only on change.
- [ ] **Multi-markers and tray: clean-up after review**
  - Work per frame grows with the square of the ship count (`multi_markers/gather.rs`: `together_sets`, `link_groups`, `overlap_clusters`, `slide_things`, `fold_markers`; `MultiMarkers::get` is a linear search). Link only neighbours in phase, bucket overlap candidates into a grid of `cluster_split_px` cells, and index markers by key.
  - `place_multi_markers` writes every pool node's `Node` each frame. Write only on change.
  - The tray rebuilds every row on each arrow step. Leave `selected` out of the rebuild check and recolour the rows instead.
  - The tray's rows repeat the outline's row code (`tray/draw.rs` `spawn_row`, `inset/outline.rs`), with slightly different indent and toggle width. Share one row builder.
  - The marker box (HOV-07) is computed in both `tray/contents.rs` and `markers/brackets.rs`. Keep one function in `multi.rs`.
  - The four `MultiLook` builds could share constructors in `look.rs`.
  - Split the long functions: `spawn_tray` and `spawn_row` (about 85 and 90 lines), and `TrayInputs::line` (about 50).
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

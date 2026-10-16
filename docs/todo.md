# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **A joined marker draws one tail in dot mode** · F-232 · MUL-05
  - In dot mode a together or overlap marker draws the tail of its first tray row's thing, behind its centre, so a convoy or cluster no longer looks still on a running clock. Joined things still draw no tail of their own, and a fold marker has none. Ring mode is unchanged.
  - Tests: the TEST-12 convoy shows one tail in dot mode; an overlap cluster shows its lead's tail; the tail follows the lead when the tray order changes; no tail in ring mode beyond the exposure ring.

- [ ] **Two stations are never together** · F-233 · MUL-03
  - Reject content that puts two stations together (PHAS-02) with a loader error naming both. Remove the build's lead-and-docked handling of two stations at one point. Two overlapping station markers join as two things (MUL-05), each shown in the tray (MUL-09) with its own docked things under it.
  - Tests: the loader rejects two stations at one orbit and phase; two overlapping stations form a marker counting 2 with neither docked.

- [ ] **Shared orbit click in dot mode, and the identity mark's tray header** · F-238 · HOV-05, MUL-08
  - A click on a shared ring selects the parent and expands that orbit's row in the outline in both modes, and opens the identity mark's tray only in exposure-ring mode; in dot mode it opens no tray. The identity mark's tray header reads "HIGH ORBIT · N THINGS", with the orbit's name.
  - Tests: dot mode, a click on a shared ring opens no tray and closes an open one; ring mode, a click on the mark opens it with the orbit's header.

- [ ] **The tray's fill is opaque** · F-240 · MUL-08
  - Draw the tray's fill at full alpha, so labels and the belt don't show through it. The inset keeps the panel fill as it is. Test: the tray's background colour has alpha 1.

- [ ] **A tray follows the marker that keeps most of its things** · F-241 · MUL-10
  - Identify a tray's marker by the set of things on its rows, not by its first thing. When things leave or join, the tray moves to the marker holding most of them, a tie going to the one holding the first row's thing, and rebuilds its rows; it closes only when no marker holds any of them.
  - Tests: a convoy's lead leaves and the tray stays open on the rest; a new thing joins ahead of the first and the tray stays; everything parts and the tray closes with the selection kept.

- [ ] **Belts become a list** · SCL-08, MRK-05, MRK-18, SOL-07, TEST-01
  - A system holds any number of named belts. Replace the single `Belt` component (`src/sim/body.rs`) with one belt record per belt: a name, inner and outer radius, and optionally a mass and a rocky, metallic and icy split. `BeltBlueprint` becomes a list in the system blueprint. Corvane's belt is named "Corvane belt" and Sol's "Main belt".
  - MRK-05, MRK-18, SOL-07 and TEST-01 changed meaning (more than one belt, belt names), so their ✓ is cleared. SOL-11's count of one belt still holds.
  - Tests: two belts draw two bands, each with its own grain seed; a system with no belt draws none; the belt counts and names of Corvane and Sol.
- [ ] **Sector generator** · SGEN-01 to SGEN-92, ARCH-22
  - Build `src/content/generator/` as plain data, one module per stage, with its own seeded random generator. Ask before adding a crate. Add `GeneratorTunables` (ARCH-22) holding every tunable in the spec's table, documented field by field.
  - The name data file is `assets/names/names.txt` (SGEN-80): author at least 120 stems, 40 endings and the reserved list. Placeholder syllables are fine.
  - Tests first: the invariants of SGEN-91 over many seeds are the main check, plus one test per requirement, named for what it checks (ARCH-30). Build the stages in order (volume and stars, summaries, gate choice, links, failure) and keep each stage's tests green before the next.
  - ARCH-22 changed meaning (a third tunables struct), so its ✓ is cleared.
  - Where the spec leaves a choice open, pick something sensible and add a finding.
- [ ] **System generator** · SYSG-01 to SYSG-71, INS-08 · needs "Belts become a list" and "Sector generator"
  - `generate_system` builds the records Corvane and Sol use (SCL-03). Add planet facts (SYSG-22) and the gate state (SYSG-52) as simulation components.
  - INS-08 changed meaning (planet facts and gate state in the inset), so its ✓ is cleared.
  - Tests: the invariants of SYSG-70 over generated systems; a generated system spawns in a headless app and its named orbits resolve (SYSG-05).
- [ ] **Sector view** · SECT-01 to SECT-44, CAM-25, SOL-22, UNIT-05 · needs "System generator"
  - A new view module with its plugin (`src/view/sector/`), a layer state both views read (SECT-01), and the light-year formatting in the shared module (UNIT-05).
  - CAM-25 changed (the first entry is "Sector" and it works) and SOL-22 changed (`--sector` and `--seed` are now named), so their ✓ is cleared. Rename "Cluster" to "Sector" in `src/view/topbar/breadcrumb.rs` and its comments. Leave `cluster_join_px`, `overlap_clusters` and the other marker-grouping names alone: they mean overlapping markers.
  - Add the keys `sector`, `seed` and `enter` to the `OW_SHOT` list in `CLAUDE.md` (SECT-44).
  - The looks are first drafts: judge them on screen and add findings.

## Fixes

- [ ] **Planet marker: review fixes** · GRP-10, GRP-11, GRP-12, GRP-13, INS-06
  - Found in review of the moon ring and object pip item.
  - The moon ring's box and its drawing differ while it fades: `ring_offset` scales the marker's `BackFrame` (box, hit area, label box) by the fade, but the ring node is drawn at the full `moon_ring_offset_px` from the start (`markers/moon_ring.rs`, `MarkerPaint::paint`). Make them agree.
  - Missing tests: the pip fades in over `fold_unfold_anim_ms` and with its parent's fade (MOT-11); hovering the pip or fold marker, or its open tray, keeps it popped (headless, not only `wants_out`); the INS-06 hand-over for a moon as its ring fades; GRP-11's "+N" is dropped first when crowded; a label test with a real label size and a neighbour nearby.
- [ ] **A double-click lands on what its first click hit** · F-247 · ZOOM-16
  - With nothing selected, the first click selects and opens the inset, which narrows the map and moves the object before the second click (INS-07), so the double-click misses. Make the second click of a double-click act on what the first click hit, wherever the pointer now is over the map.
  - Test: with nothing selected, a double-click on a planet frames it, though the inset opened between the clicks.

## Other

- [ ] **Planet marker: clean-up after review**
  - `MarkerPaint::paint` collects each disc marker's ring nodes into a new Vec every frame, and `lay_out_holders` clones `Node` twice per pool node every frame. Cache them or skip them while nothing changes.
  - `MarkersPlugin` also calls `init_resource::<FoldSummary>()`, which `GroupingPlugin` already does; grouping is its one writer (ARCH-03). Drop it, or say it is there only for test apps.
- [ ] **Text sizes: clean-up after review**
  - Found in review of the map text sizes item.
  - `changing_a_text_size_tunable_changes_every_text_it_names` doesn't cover an edge tab's name: add a gate tab and check its label follows `map_label_px`.
  - `apply_text_sizes` is registered in `TunablesPlugin`; give it a small plugin of its own in `text_size.rs`, so the tunables module doesn't depend on a view system.

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

## Parked

Specced but not built, because nothing needs it yet. Don't pick these for a coding session. Each says what brings it back. Their requirements still count for the coverage check in `docs/README.md`.

- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - **Revisit when:** a system reads ideology (a faction, settlement or character that carries a profile, or an effect that uses the scores) and the UI for showing it is specced. Until then the code would have no caller.
  - Decide the open point in `docs/spec/ideology.md` first (whether characters share the structure), and say where the option values are loaded from (IDEO-31).
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

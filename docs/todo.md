# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Planets: a neutral moon ring and an object pip that pops out** · F-229, F-230, F-239 · GRP-05, GRP-07, GRP-10 to GRP-13, LBL-01, MUL-02, MUL-04, MUL-06, MUL-08, HOV-06, HOV-07, INS-06
  - Moons and objects are separate. Folded natural bodies draw a neutral second ring behind the parent's marker and a dim "+N" counting only them (GRP-10, GRP-11). Folded ships, groups and stations draw a 6 px pip down and to the right of the parent, with a back pip when there are two or more (GRP-12). The fold marker is no longer drawn at rest.
  - When the pointer is within `pop_reach_px`, or the parent or something it holds is selected, or the tray is open, the pip grows into the fold marker and slides out so its box touches the parent's (GRP-13, MUL-06). Use the nearest parent, and `pop_release_px` as hysteresis, so it doesn't flicker in the crowded inner system.
  - The fold marker holds only objects, so its tray and count leave out folded moons (MUL-04). Its tray opens right, else below or above, never left over the parent (MUL-08). The parent's label tries left first (GRP-12, LBL-01).
  - Delete the fold marker at rest, the all-bodies look of the multi-marker (MUL-02) and the fold-marker-beside-parent placement with their tests. Add the tunables `object_pip_px` (6), `object_pip_gap_px` (2), `pip_back_offset_px` (3), `pop_reach_px` (24), `pop_release_px` (32), `pop_anim_ms` (120); `moon_ring_offset_px`, `moon_count_px` and `moon_count_alpha` as before.
  - Things docked at a station inside a larger marker count for the frames and the pip's colours but not for the count (MUL-02, MUL-04; F-230).
  - The pip's front and back colours follow MUL-02. Only the parent's own folded satellites decide, as they do for the count. A moon whose station is folded into it is still one body.
  - Tests: moons only shows the neutral ring and "+N", no pip; a folded ship adds the pip and leaves ring and "+N" unchanged; allied plus own shows an allied front pip and an own back pip; the pointer near pops it out and away releases it; selecting the parent pops it with no pointer; a selected folded ship puts the mark round the popped marker; the tray never covers the parent; the label doesn't move when the marker pops; the ring's box and hit area count in the parent's marker. Mock-ups 20 to 22, 26 and 27 show the look; mock-up 26's popped state sits below the planet and is superseded by GRP-13.

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

- [ ] **Map text sizes in one place, and the vertical axis clips at the map edge** · F-216, F-220 · MRK-19, REF-06
  - Add `map_label_px` (12), `map_second_line_px` (10) and `bar_text_px` (14) to the `Tunables` resource and read every map and bar text size from them, so an exposure ring's name is 12 px like a marker's. Remove the sizes each module sets for itself.
  - Clip the vertical axis's upper half at the map area's edge, as the arms are, so its arrowhead never runs under the top bar.
  - Tests: a ring's name and a marker's name share one size; changing a tunable changes every text it names; at tilt 75 with the frame body high on screen, the axis ends at the map's edge.

- [ ] **Belts become a list** · SCL-08, MRK-05, MRK-18, SOL-07, TEST-01
  - A system holds any number of named belts. Replace the single `Belt` component (`src/sim/body.rs`) with one belt record per belt: a name, inner and outer radius, and optionally a mass and a rocky, metallic and icy split. `BeltBlueprint` becomes a list in the system blueprint. Corvane's belt is named "Corvane belt" and Sol's "Main belt".
  - MRK-05, MRK-18, SOL-07 and TEST-01 changed meaning (more than one belt, belt names), so their ✓ is cleared. SOL-11's count of one belt still holds.
  - Tests: two belts draw two bands, each with its own grain seed; a system with no belt draws none; the belt counts and names of Corvane and Sol.
- [ ] **Sky in the system view** · SKY-01, SKY-03 to SKY-22, MRK-12
  - A new module `src/view/sky/` with its plugin. The sky is made by one plain function from a seed and `Tunables` (SKY-01), with no Bevy types; the view makes it at start-up from `sky_default_seed`, because the sector seed doesn't exist yet (SKY-02 comes with the sector item below). Draw it as the bottom layer in the map area only, at infinity: through the camera's heading, tilt and field of view, with no position (SKY-16, SKY-18, SKY-20). Never a hover or click target (SKY-19). Always on, with no toggle (SKY-22).
  - MRK-12 changed meaning (the sky is the new bottom layer), so its ✓ is cleared.
  - Add the `sky_*` tunables of `docs/spec/sky.md` to the `Tunables` resource, documented field by field. `sector_sky_factor` waits for the sector item.
  - How to draw it is open (spec, open points): tens of thousands of soft points as one instanced draw, and the glow as a cheap field over the map area, are the obvious start. Mock-up 28 is the look. Judge it on screen and add findings.
  - Tests: the same seed gives the same sky and two seeds differ (SKY-01, SKY-03); the star counts and the magnitude ratios per step, over many seeds (SKY-04, SKY-05); the band's latitude spread and the core being denser (SKY-06); over 1,000 seeds the band's tilt is in range and it passes within the margin of the standard view's line of sight, with the fallback when the range is out of reach (SKY-07, SKY-08); the brightest and faintest opacity (SKY-09); a star's deposited light stays within the tolerance for sub-pixel offsets (SKY-11); no colour above the saturation cap (SKY-12); the glow's falloff, lane and mottle bounds, and the dither's size and zero mean (SKY-13 to SKY-15); a star's projected position doesn't change under pan, zoom or a change of frame, and turns by the heading change (SKY-16); nothing changes with the clock (SKY-17); no star is drawn outside the map area, also while the inset slides (SKY-18); a click through the sky hits what is beneath (SKY-19); the sky is below every other layer (SKY-20); no star is an outline (SKY-21).

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
- [ ] **Sky in the sector view and from the sector seed** · SKY-02, SKY-23, SKY-24, SECT-05 · needs "Sector view" and "Sector generator"
  - Derive the sky's seed from the sector seed and the name "sky" with the generator's own derivation (SGEN-03, SKY-02); `sky_default_seed` stays for Corvane and Sol. The sector view draws the same sky through its camera, scaled by `sector_sky_factor` (0.5), and below the links (SKY-23, SECT-05). Add `sector_sky_factor` to `Tunables`.
  - Tests: the derived seed depends on the sector seed and a different sector gives a different sky; Corvane and Sol use the default; the sector view's sky equals the system view's at the same heading and tilt, with every opacity multiplied by the factor; the default tunables satisfy SKY-24; the sky is below the links.

## Fixes

- [ ] **A double-click lands on what its first click hit** · F-247 · ZOOM-16
  - With nothing selected, the first click selects and opens the inset, which narrows the map and moves the object before the second click (INS-07), so the double-click misses. Make the second click of a double-click act on what the first click hit, wherever the pointer now is over the map.
  - Test: with nothing selected, a double-click on a planet frames it, though the inset opened between the clicks.

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

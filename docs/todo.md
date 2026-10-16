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

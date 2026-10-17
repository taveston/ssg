# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **System generator** · SYSG-01 to SYSG-71, INS-08
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

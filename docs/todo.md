# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Planet facts for Corvane and Sol** · SURF-60 to SURF-63
  - Author the facts of tables 1 and 2 in `content/` (`planet_facts` is `None` on every hand-made body today), the stars' type, luminosity and age, a surface seed per body, and Earth's look override. Gravity and Teq come from the SGEN-38 and SGEN-39 rule functions.
  - Tests: every natural body has facts; Earth's override is set and no other body's; Corvane's star luminosity follows SGEN-22.

- [ ] **Procedural planet surfaces in the inset** · SURF-01 to SURF-05, SURF-10 to SURF-26, SURF-30 to SURF-35, SURF-40 to SURF-42, SURF-45, SURF-46, SURF-70 to SURF-72, INS-04 (changed: the image is the globe; ✓ cleared) · needs "Planet facts for Corvane and Sol"
  - Port the shader of mock-ups 29 and 30 to WGSL under `src/view/surfaces/`, as a bake pass into a cubemap and a per-frame globe pass. Keep the look chooser, the surface temperature rule and the resolution rule as plain functions with unit tests.
  - Start with the inset only. The bake spreads over frames; the inset shows an empty frame until it lands.
  - Tests: the chooser on every row and the Sol bodies (SURF-70); determinism and seed difference by baking a 64 px face headlessly (SURF-71); large forms persist (SURF-72). Mark SURF-13, SURF-15 to SURF-19 *Manual* only if a pixel read-back of a small face can't check their distinguishing features (a glow where hot, rays only when dead, no craters when molten).

- [ ] **The globe in the system view** · SURF-50 to SURF-52, SURF-73, MRK-11 (changed: the picture replaces the fill once large enough; ✓ cleared) · needs "Procedural planet surfaces in the inset"
  - Replace MRK-11's fill and rim with the globe above `globe_min_px` with hysteresis `globe_release_px`; the gap, marker, label and selection mark stay on top. Same light as the inset.
  - Tests: globe-or-disc from the projected radius with hysteresis, headless.

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

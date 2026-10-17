# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **The globe in the system view** · SURF-34, SURF-50 to SURF-52, SURF-73, MRK-11 (changed: the picture replaces the fill once large enough; ✓ cleared)
  - Replace MRK-11's fill and rim with the globe above `globe_min_px` with hysteresis `globe_release_px`; the gap, marker, label and selection mark stay on top. Same light as the inset.
  - Tests: globe-or-disc from the projected radius with hysteresis, headless.
  - SURF-34: the rule (`needs_rebake`, `rebake_face_px`) and the fine-scale bake (`describe_surface(.., fine_scales = true)`) exist in `view/surfaces/`; the trigger from the projected radius and drawing the re-baked face remain.
  - From review: `SurfaceBakes::evict` (`view/surfaces/bake.rs`) ties on `last_used`, so if more than `bake_cache_count` globes are drawn in one frame the evicted ones restart every frame and never finish. Never evict an entry used this frame, or let the cache grow to the number drawn.

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

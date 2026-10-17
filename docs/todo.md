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
- [ ] **Halden IV moves in, and hand-made stars list age and luminosity** · TEST-01, TEST-02, SURF-60, SOL-02
  - Halden IV at 0.75 AU, radius 7,400 km (0.82 g with its authored mass), no authored "Surface" fact. Iver Yard named in Iver's High band (as built).
  - Corvane's and Sol's stars show Class, Age and Luminosity.
- [ ] **Surface looks: Earth's override, bake size, dense before volcanic, a star look** · SURF-13, SURF-16, SURF-27 (new), SURF-32, SURF-33, SURF-45, SURF-61, SURF-63, SURF-70
  - `earth_override_t_k` 1,000 K; `bake_face_px` 512 and `bake_cache_count` 3; the chooser tries dense before volcanic; a star's inset image is a limb-darkened disc (`star_limb_darkening`).
- [ ] **The sky: a glow factor for the sector view, and a trial of a stronger band** · SKY-23
  - New `sector_glow_factor` 1.0: the glow keeps its system-view strength in the sector view.
  - Experimental: in the system view, try modestly higher `sky_glow_alpha` and `sky_lane_depth` so the band and its lanes read in the game window. Compare shots against mock-up 28, and report the values tried with shots. Change the defaults only if a value clearly reads better, and say so in the summary.
- [ ] **The outline scrolls only for its own selections** · INS-05
  - A selection from the map, a tray or a key no longer scrolls the outline. A selected body that moves still opens and scrolls to its new path (as built).
- [ ] **Fold and multi-marker refinements** · GRP-12, GRP-13, GRP-14 (new), MUL-01, MUL-04, MUL-05, MUL-10, INS-06
  - The pip overlaps the parent's marker (`object_pip_rim_offset_px` replaces `object_pip_gap_px`); a single thing pops with no back frame; only parents with an object marker count for "nearest"; label slots centred on the parent's own box.
  - Things folded into a folded moon pass up to the nearest unfolded ancestor and are listed under the moon's heading in its tray (GRP-14).
  - A fold marker's tray follows only its own marker; the tally counts only rows shown; tails follow symbols out as well as in.
- [ ] **Edge arrows for gate wrecks** · EDGE-01
- [ ] **Whole numbers on the scale bar** · UNIT-05
- [ ] **Planner: drives, rejected colour, low-thrust turns, Depart span** · PLNR-01, PLNR-28, PLNR-65, PLNR-80, XFER-13
  - Transfer only for a ship with a drive; the rejected colour is the neutral mixed into the ground at `track_empty_mix`; no turn for a low-thrust leg; `depart_min_span_days` 7.
- [ ] **Generator: sub-Neptune radii, gate rounds, small bodies** · SGEN-36, SGEN-57, SYSG-22, SYSG-23, SYSG-41
  - Sub-Neptune exponent 0.59; a short round adds all missing systems at once; belt bodies are "small body".
- [ ] **Sector inset and readout** · SECT-06, SECT-08, SECT-22, SECT-23
  - The truth readout is a label obstacle; no Reason fact; orbits always in AU.

## Fixes

- [ ] **Panels let what's beneath show through** (MRK-01, INS-07, CLK-02)
  - The top bar and sidebar use `PANEL` at alpha 0.97, but blended in linear light, so links and orbit rings show at about 20% under the top bar. Make the panels opaque.
- [ ] **Planner paths may lose pieces to the near clip** (PLNR-60, PLNR-69)
  - Sector links lost pieces with `depth_bias` −1 against the 0.001 near plane, fixed by using no bias. Planner paths (`paths/strokes.rs`, `IN_FRONT = -1.0`) use the same bias. Check them and drop the bias, or use a smaller one.

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

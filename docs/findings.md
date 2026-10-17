# Findings

Observations from building and playing the prototype, in two sections. **To triage** holds findings waiting for a decision at the next design session. **Parked** holds findings that can't be decided until a later feature is designed, each with a line saying what brings it back. Once a finding is settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings go under To triage and take the next free number from F-317.

---

## To triage

### F-311 · The close-in re-bake never triggers for the frame body
- **Observation:** SURF-34 re-bakes once the projected radius passes `bake_face_px` × `rebake_zoom_factor` = 512 px. ZOOM-14's inner limit caps the frame body's diameter at `body_max_screen_fraction` (0.25) of the map's height, so its radius is about 105 px at most. The finer face only shows for a body that isn't the frame, seen large from a close neighbour's frame (a giant from an inner moon), or through `OW_SHOT`'s `distance`. In practice it never shows.
- **Affects:** SURF-34, ZOOM-14 · **Type:** spec wrong
- **Disposition:** open. Lower `rebake_zoom_factor` (about 0.08 gives about 80 px), or park SURF-34 for the planetary view.

### F-312 · The close face has no hysteresis
- **Observation:** The close face's threshold is a hard 512 px (`needs_rebake`), and the face in view switches with no margin. A body whose radius hovers near the threshold, or a view centre dragged across a cube-face edge, drops the close face and starts it again each time: a new 4096² strip (128 MB, 256 MB for a glowing body) and all 16 bands from the start. The finer face may never land. Found in review; not fixed, since SURF-34 says nothing about either.
- **Affects:** SURF-34, F-311 · **Type:** spec gap
- **Disposition:** open. Add a release threshold like `globe_release_px` (a new tunable), an angular margin before switching faces, or keep the old close face until the new one is ready.

### F-313 · The limb glow on the map shows only inside the gap
- **Observation:** On the map, the globe's node covers the disc and its `body_gap_px` gap. The gap is painted ground colour plus the atmosphere's limb glow (SURF-24), and the node is transparent beyond it, so the glow is cut off at the gap's outer edge, 2 px out. On a large gas giant that reads as a crisp 2 px bright rim on the sunward side. The build keeps it: the gap stays ground-coloured where there is no glow, so overlapping discs stay distinct.
- **Affects:** MRK-11, SURF-24, SURF-50 · **Type:** spec gap
- **Disposition:** open. Keep it, drop the glow on the map (ground-only gap), or let the glow fade out over the sky past the gap.

### F-314 · Close-face rules the spec leaves open
- **Observation:** For SURF-34 the build chose: the face in view is the one under the middle of the disc as seen; a body holds one close face, dropped the first frame it isn't drawn (4096 px Rgba16Float takes 128 MiB per row, 256 MiB for a glowing body, see F-307); it is baked in `rebake_scale`² row bands, so a frame bakes no more than one ordinary face; bodies in the telescope frame get no close face, since the finest craters would show detail the knowledge state hides.
- **Affects:** SURF-34, SURF-25, SURF-31 · **Type:** spec gap
- **Disposition:** open. Copy into SURF-34 if fine.

### F-315 · A fading body draws its disc, not its globe
- **Observation:** The globe pass has no fade. A body marker drawn at less than full strength (folding, joining a multi-marker, or fading into its exposure ring) draws MRK-11's disc, which fades with it, rather than the globe. Rare at globe sizes, but the picture swaps to the disc as the fade starts.
- **Affects:** SURF-50, GRP-03, MOT-11 · **Type:** spec gap
- **Disposition:** open. Keep it, or add an alpha to the globe uniform so the picture fades.

### F-316 · The hazy cloud deck has straight-edged steps close in
- **Observation:** Close in on Halden IV (radius about 480 px, cube bake only), the hazy cloud pattern shows short straight-edged steps in many places, and the same with the close face. They look like a property of the deck's noise rather than face seams, but they read as artefacts at map scale. Pell's airless look shows nothing similar.
- **Affects:** SURF-16, SURF-17 · **Type:** bug
- **Disposition:** open. Check the dense and hazy deck's warp in `bake.wesl`.

## Parked


### F-145 · An intercept needs a standoff distance
- **Observation:** Raised in a combat design session. XFER-10 ends a ship-as-destination crossing at the target's position on the crossing end day, so an Intercept is a collision course with zero miss distance. A real engagement has a chosen miss distance, and it sets how long each side is in weapon range and which weapons can act. The order has no way to carry one. An Intercept at a station is trimmed to meet it (PHAS-15), so it too has zero miss distance.
- **Affects:** XFER-08, XFER-09, XFER-10, PHAS-15 · **Type:** spec gap
- **Disposition:** parked. Option: a `standoff_km` on an Intercept whose destination is a ship, so the crossing ends offset from the target's rail rather than on it. A Rendezvous keeps zero. What the standoff should default to, and how the resolver reads it, are for the conflict doc to settle.
- **Revisit when:** the combat design is specced. The default is an open question in `docs/ideas/mechanics/conflict.md`.

### F-158 · Departure and capture costs add up to far more than a real trip
- **Observation:** Raised while sizing Marlin for Sol (SOL-15). BAND-03 and XFER-04 charge a ship the cost of escaping its band to rest at infinity, then the burn that sets its speed round the primary, and the same again at the far end. A real burn deep in a gravity well does both at once and is much cheaper (the Oberth effect). From Earth's Low band to a Low band at the target, with a Hohmann crossing, the spec's cost against a patched-conic figure for the same bands: Venus 11.3 against 6.7 km/s, Mars 10.1 against 5.6, Ceres 14.4 against 9.5, Mercury 21.4 against 13.2. The spec is 1.5 to 1.8 times too high. The low-thrust crossing (XFER-06) adds the same escape cost, which for a spiral is too low: a spiral escape costs the full circular speed, not √2 − 1 of it.
- **Affects:** BAND-03, XFER-04, XFER-06, XFER-12, SOL-15 · **Type:** spec wrong
- **Disposition:** parked. The model needs fixing, and the fix is a significant piece of work, so the spec and the build stay as they are for now (decided 2026-10-16). Option: for an impulsive drive, replace departure cost + burn 1 (and burn 2 + capture cost) with one burn from the band's circular orbit to a hyperbola with the crossing's excess speed, √(*v∞*² + 2μ/*r*) − √(μ/*r*). The excess speed is already computed for BAND-06, XFER-09 and PHAS-03. Open: how burn days (XFER-05) and approach days (BAND-04) fit, what the spiral should cost, and new figures for the XFER-12 worked checks. Marlin's delta-v would then be set again.
- **Revisit when:** the transfer-cost rework is scheduled. `docs/ideas/mechanics/transfers.md` (section 2.1) holds the lean.

### F-159 · Groups are only loosely defined
- **Observation:** SYM-09 draws a group, but nothing says what selecting one shows. The inset (INS-02) has no form for a group. The outline (INS-05) lists a group by name and count (GRP-09), with no way to list its members or reach one by one, for their Delta-v and Drive (PLNR-05). "Commander" means a person in SYM-09 and in mock-up 7 ("Cdr Ilse Marr"), but "command" means the faction giving orders in SYM-03. How groups form, merge and split is not specced either: no order creates a group, and merge and detach orders are not specced (PHAS-14). So groups exist only as test content (TEST-06) for now. The planner already refuses them (PLNR-01, PLNR-11). The combat ideas (candidate) detach and rejoin ships and call every group a force, which would be the first source of groups in play.
- **Affects:** SYM-09, SYM-10, INS-02, INS-05, GRP-09, PLNR-01, PLNR-05 · **Type:** spec gap
- **Disposition:** parked. Safe to defer: the player can't form groups yet. Specify the group inset, member access and the meaning of "commander" together with the orders that form and split groups.
- **Revisit when:** the orders that form and split groups are designed, or a group first appears in play.

### F-163 · Vestige's burn time is longer than the orbit it leaves
- **Observation:** Vestige's first burn from Earth's Low band to its High band (SOL-17) is about 3.0 km/s at 20 milli-g, so XFER-05 gives it 4.2 h of burn days. Against the 52 h Hohmann crossing that is a small share. The rule keeps the ship Parked in Low for those 4.2 h, then applies the whole burn as one impulse at the crossing start, and solves the crossing again from that later start. So the burn only delays the trip: no thrust arc is drawn, no gravity loss is charged, and the time the ship would spend burning is spent waiting. Yet 4.2 h is about 2.7 laps of Low (94.5 min), so a real burn that long could not be treated as an impulse at one point. Marlin's burn (0.1 g, about 51 min) is about half a lap, which is borderline too.
- **Affects:** XFER-05, SOL-16, SOL-17 · **Type:** spec gap
- **Disposition:** parked. Accept the delay as a stand-in, or say how a burn that lasts a large share of an orbit is modelled (bears on F-158).
- **Revisit when:** the transfer-cost rework (F-158) is scheduled; how a long burn is modelled goes with it.

### F-173 · Groups are together, but nothing forms them yet
- **Observation:** PHAS-14 needs the ships of a group to be together. A group is now one body with one orbit and phase, and its ships have no place of their own (F-176), so they are together by construction. Only content makes groups (TEST-06); nothing in play forms or merges one (F-159). `SystemTree::all_together` says whether separate ships could become one group (all parked in one orbit, every two together); whatever forms or merges groups should call it.
- **Affects:** PHAS-14 · **Type:** spec gap
- **Disposition:** parked. Use the check when group orders are specced.
- **Revisit when:** the orders that form and split groups are designed (F-159).

### F-197 · Orison can't reach Iver, so its tracks are all orange
- **Observation:** Seen on screen. A Rendezvous from Halden IV's High band to Iver's needs at least 13.2 km/s, more than Orison's 9.9, so both tracks are the over-budget colour from end to end and no window shows as a bright band. An Intercept of Iver, or Halden IVa, is within reach. The empty colour (the inset's hairline colour) is light grey, close to the dimmest blue, so a rejected setting is hard to tell from a dear one.
- **Affects:** PLNR-28, TEST-30, F-158 · **Type:** spec wrong
- **Disposition:** parked. Give Orison more delta-v or cheaper bands (F-158).
- **Revisit when:** the transfer-cost rework (F-158) sets Orison's delta-v and the band costs. The empty colour is already a spec change.

### F-202 · Dashes crawl along a path in flight
- **Observation:** Dashes and dots are laid on screen from the start of the dashed part, so their length stays `path_dash_px` at any zoom (PLNR-61). In flight that start is the ship, which moves, so the dashes slide along the path as the ship flies and as the camera moves. Laying them from the path's end would hold them still while the ship flies, but not while the camera zooms.
- **Affects:** PLNR-61 · **Type:** idea
- **Disposition:** parked. Watch in play; anchor the dashes to the arrival end if the crawl distracts.
- **Revisit when:** the crawl distracts in play.

### F-259 · Same sector on every machine depends on the platform's maths library
- **Observation:** SGEN-02 asks for the same sector "on every run and machine". The random numbers are integer arithmetic, identical everywhere, and a test pins the published xoshiro256** and SplitMix64 values. But the generator also uses `powf`, `ln`, `exp`, `cbrt`, `sin` and `cos`, which Rust takes from the platform's maths library, and their last bit may differ between operating systems. Range and Hill-spacing branches can turn that into a different sector. The same build target gives the same sector every run. The sky (SKY-01) uses the same generator and functions, so the same holds for it.
- **Affects:** SGEN-02, SGEN-06, SKY-01 · **Type:** spec gap
- **Disposition:** parked. Accept "same target, same sector", or write the few functions in the repo (SGEN-06 rules out a crate such as `libm`), or allow such a crate.
- **Revisit when:** saves, shared seeds or multiplayer need the same sector across machines. Until then "same build target, same sector" stands.

### F-267 · Gate choice details, and `core_share` barely matters
- **Observation:** The build chose: the quota is ceil(`reason_quota` × N), and each reason ranks all eligible systems, zeros included; a junction's "nearest" is nearest to its nearest chosen system; the "shortest chain" is fewest relays, then least length; of equal scores the higher id is dropped first; range growth from connectors carries into stage 5; if no system can be dropped, every quota drops by one and the stage restarts; with fewer eligible systems than N, all are gated. Gates and links draw no random numbers, so their stage seeds go unused. The rounds refill by overall score, so `core_share` 0.6 gave the same gated set as 0.8.
- **Affects:** SGEN-55, SGEN-56, SGEN-57 · **Type:** spec gap
- **Disposition:** parked. Copy the choices into the spec, and say whether `core_share` should matter (for example, fill past the core with connectors' neighbours rather than by score).
- **Revisit when:** gate choice is revisited (with F-272): whether connectors should be common enough for relays to act as chokepoints. Lean: a tighter `connect_range_factor` for SGEN-56 only, swept for the share of junctions and relays.

### F-272 · Connectors almost never happen at the default range
- **Observation:** Over the 200-sector sweep there were 6 junctions and no relays: at two mean spacings the top scorers are already joined, so SGEN-56 is barely exercised and the gated set is in effect the top N by score. Found in review.
- **Affects:** SGEN-56, SGEN-60 · **Type:** spec gap
- **Disposition:** parked. Say whether that is intended, or tune the range.
- **Revisit when:** gate choice is revisited (with F-267).

### F-287 · A large sector at its fit is crowded and mostly unlabelled
- **Observation:** At the fit of a 200-system sector, the 14 px star markers crowd together and most gated labels give way. Only names round the rim show.
- **Affects:** SECT-03, SECT-06 · **Type:** spec wrong
- **Disposition:** parked. Smaller nodes when zoomed out, labels only above some zoom, or accept it.
- **Revisit when:** the sector view has been played.

### F-295 · `OW_SHOT` `distance` is in km in the sector view
- **Observation:** The shot's `distance` key keeps its km unit in the sector view, where 10 ly is 9.46e13 km, awkward to type and read. SECT-44 names only `sector`, `seed`, `select`, `hover` and `enter`. Found in review.
- **Affects:** SECT-44 · **Type:** spec gap
- **Disposition:** parked. Accept a light-year figure in the sector view (for example `distance=12ly`), or leave it as documented in `CLAUDE.md`.
- **Revisit when:** sector-view shots need distances often enough that km gets in the way.

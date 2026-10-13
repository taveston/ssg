# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Console buttons** · BTN-01, BTN-02, CLK-02, INS-02
  - Was F-117: Close was orange, Focus and Pause blue. `button_stroke_px` is new.
- [ ] **Partly transparent exposure rings; thinner normal rings** · MOT-01, MOT-02, MRK-05, HOV-03
  - The tail's head uses the same alpha as the exposure ring, so the closing stays seamless (MOT-11).
  - The normal width is hard-coded as `NORMAL_RING_WIDTH_PX` in `view/rings.rs`; move it and the highlight width to `Tunables`.
  - HOV-03 now also draws the selected ring at `orbit_selected_alpha` and `orbit_highlight_width_px` drops to 1 px, so the selected orbit stops dominating the view (was F-116).
- [ ] **Opaque symbol insides** · SYM-03
  - Was F-123. The fill tunables are now mix fractions into the ground colour, not alpha.
- [ ] **Sharp symbols** · SYM-16, SYM-17
  - Was F-124: symbols looked blurred beside the rings. `symbol_stroke_px` default drops from 1.6 to 1.5 px.
- [ ] **Body markers as outlines** · MRK-03, MRK-04, MRK-07, MRK-11, MRK-12, MRK-13
  - Was F-125, and replaces the item "Outlined body discs, a size smaller". New sizes 16/12/8 px; `body_fill_alpha` is renamed `body_fill_mix`; `star_tick_count`, `star_tick_px` and `planet_dot_px` are new. Rings now draw below everything (MRK-12).
- [ ] **Edge arrows as tabs** · EDGE-02, EDGE-03
  - Was F-119: a label was clipped to "Gat" and one arrow had no label. Do it with the "Gate edge labels" fix below.
- [ ] **Morrow IX's trader at about 9 d** · TEST-09
  - The build already uses 9 d (was F-113); check and mark ✓.
- [ ] **Remove the hover tooltip** · HOV-01, HOV-02
- [ ] **Ease tail length on rate changes** · MOT-13
- [ ] **Inset as a sidebar; "In orbit" outline** · INS-05, INS-07
  - `inset_dock_margin_px` is withdrawn; `sidebar_slide_ms` is new.
- [ ] **Inset panel anatomy** · INS-09
  - Was F-120. Build after the sidebar.
- [ ] **Label placement** · LBL-01 to LBL-04
  - Was F-115: labels overprinted each other round Halden IV. `label_gap_px` and `label_fade_ms` are new. The hardest item here; keep placement in its own view module.
- [ ] **Ship groups and condition** · SYM-09, SYM-10, SYM-12, GRP-09, TEST-06
  - The symbols are built; groups add to them (`view/symbols.rs`, `symbol_draw.rs`).
- [ ] **Velocity leaders** · SYM-11
  - Blocked: ships in transit are not specced yet.
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.

## Fixes

- [ ] **Gate edge labels show the destination alone** · EDGE-01
  - Label reads "Ostra", not "Gate Ostra".

## Other

- [ ] **Screenshot debug option**
  - A built-in way to capture the game window without editing `main.rs`: for example, an environment variable naming a view (frame body, distance, tilt, time rate) and an output path. The app sets that view, waits for it to settle, saves a screenshot and exits.
  - Keep it in a view debug module, read through the bindings or a debug resource, not in `main.rs`. Then replace the "temporarily edit `main.rs`" steps in CLAUDE.md ("Seeing the game").

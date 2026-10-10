# Validation: System view

Checks for every requirement in `docs/spec/system-view.md`. Run them at the end of each iteration and record the result in that iteration's column: ✓ pass, ✗ fail (with the finding ID, e.g. "✗ F3-04"), – not built yet. A failure always gets a finding in `docs/findings/`.

**Auto** marks checks that should become `cargo test` cases named after the ID (for example `fn grp_01_hysteresis()`); until they exist, check them by hand.

## Setup

- Run the app (`cargo run`). Default state: system frame, 1 d/s, nothing selected.
- Keys (from CAM and CLK-05): Space pause · `.` / `,` rate · 1 d/s and 5 d/s buttons · wheel or + / − zoom · left or middle drag pan · WASD / arrows pan · right drag rotate · Q / E heading · Page Up / Down tilt · N level and north up · F focus · Home · Ctrl+1–4 save bookmark · 1–4 recall · Esc close. Iteration 3 and earlier used + / − for rate, right drag for tilt only and middle drag for pan.
- **Scene A, system frame:** start state.
- **Scene B, Halden IV selected:** click Halden IV at the system frame.
- **Scene C, Halden IV satellite frame:** double-click Halden IV.
- **Scene D, following Halden IV:** scene C (from CAM-06, focus also follows).
- Reference images: `docs/ideas/presentation/mockups/` (board 1 for scene A, board 6 for B and C). Look and layout only; the spec wins on behaviour.

## Checks

| ID | How to check | Auto | it3 |
|---|---|---|---|
| SCL-01 | Code read: body data in km/kg/days as `f64`; one conversion function in `scale.rs`. | | |
| SCL-02 | Scene C: moon orbit radii look proportionally right next to each other and to Halden IV's orbit when zooming out. | | |
| SCL-03 | Code read: Corvane is built only from `system_data.rs` records. | | |
| SCL-04 | Computed periods match Kepler for each body (and TEST-02 values). | Auto | |
| CLK-01 | At 1 d/s the date advances one day per second. | | |
| CLK-02 | Top bar shows breadcrumb, date, Pause, 1 d/s, 5 d/s; active rate highlighted. | | |
| ~~CLK-03~~ | Withdrawn; see CLK-05. | | |
| CLK-05 | Space pauses and resumes; `.` and `,` double and halve; `,` stops at 1/16. + and − no longer change the rate. | | – |
| CLK-04 | While paused, nothing moves and hover, click, zoom all work. | | |
| MRK-01 | Compare colours with board 1: own blue, foreign orange, white selection ring. | | |
| MRK-02 | Labels in Barlow, date and figures in JetBrains Mono. | | |
| MRK-03 | Markers keep the same pixel size at every zoom step. | | |
| MRK-04 | Scene C: disc for moons, hollow square for Arden Highport, triangle for the parked own ship, diamond for K-3, X for salvage sites. | | |
| MRK-05 | Orbits are faint rings; the belt is a shaded band. | | |
| MRK-06 | At every stop, each planet keeps its own marker; no pill ever counts another planet. | | |
| EDGE-01 | Scene A: each gate shows as an edge arrow labelled with its destination; panning moves the arrow round the edge. | | |
| HOV-01 | Hover Halden IV: white ring and the exact tooltip text. | | |
| INS-01 | Click a body: inset opens with a leader line to the marker; clock keeps running. | | |
| INS-02 | Inset has header, Focus, Pin, Close, image, key facts and a disabled "Open planetary view". | | |
| INS-03 | Pin Halden IVc, then select Iver: both insets visible. Esc closes the unpinned one. | | |
| INS-04 | Halden IV shows SURVEYED with date; IVc TELESCOPE FRAME · UNSURVEYED in orange with ranges; Lantern LAST IMAGED with date. | | |
| ZOOM-01 | One wheel notch = one ×2 step, animated with ease-out; a fast spin still moves one step per notch. | | |
| ZOOM-02 | Scroll twice quickly: the second retargets smoothly with no stutter or queued delay. | | |
| ~~ZOOM-03~~ | Withdrawn; see ZOOM-13. | | |
| ZOOM-04 | Stops land at system frame, region frame and Halden IV satellite frame, each framing its contents with a margin. | Auto (stop radii from orbits) | |
| ZOOM-05 | The step landing on a stop is visibly slightly slower and ends exactly on it. | | |
| ZOOM-06 | Scroll out past the system frame and in past the satellite frame: nothing happens; no layer change. | | |
| ZOOM-07 | Pinch on a trackpad; release near a stop: it settles on the stop. | | – F3-01 |
| ZOOM-08 | Double-click Halden IV at system frame: one ~400 ms pan-and-zoom to scene C, inspected. F on a selection does the same. | | |
| ZOOM-09 | Double-click Halden IV again: nothing visible happens; hook named in `camera.rs`. | | |
| ZOOM-10 | From scene C, Home returns to the system frame with an animated move. | | |
| ZOOM-11 | Ctrl+1 at scene C, Home, then 1: returns to scene C, following Halden IV. | | |
| ~~ZOOM-12~~ | Withdrawn; see CAM-10 to CAM-13. | | |
| ZOOM-13 | Nothing selected or followed, tilted to ~45°: the plane point under the cursor stays under it while wheel-zooming. With a selection: zoom anchors on it. Scene D, cursor elsewhere and IVc selected: zoom anchors on Halden IV. | | – |
| CAM-01 | Right drag fully up and down: tilt stops at the limit, never passes the plane. Heading turns past 360° without stopping. | Auto (clamp) | – |
| CAM-02 | Fresh start: straight down, north up. | | – |
| CAM-03 | Tilt and rotate, then N: ~400 ms move to straight down, north up; centre and scale unchanged. | | – |
| CAM-04 | Tilt and rotate at scene C, then Home: one move to the system frame, level and north up. | | – |
| CAM-05 | Scene D at 5 d/s: Halden IV stays fixed on screen; moons orbit around it. | Auto (pivot = position + offset) | – |
| CAM-06 | Focus via double-click, F and the inset's Focus button: each starts following. Ctrl+1 in scene D after panning and tilting, Home, then 1: following resumes with the same offset, heading and tilt. | | – |
| CAM-07 | Scene D: pan away from Halden IV, then run at 5 d/s: Halden IV keeps its new screen position. | | – |
| CAM-08 | Scene D: select IVc, zoom, rotate: still following. Then each of Home, double-click Iver, a bookmark saved without following, and the chip's ×: following ends (or moves to Iver). | | – |
| CAM-09 | Scene D: chip under the breadcrumb reads "Following Halden IV"; × ends following and the chip disappears. | | – |
| CAM-10 | Left drag on empty space and from a marker: the map moves exactly with the cursor, also when tilted; release stops dead. A tiny jitter on click still selects. Shift+left drag does nothing. | Auto (threshold) | – |
| CAM-11 | Middle drag behaves as CAM-10. | | – |
| CAM-12 | Right drag: horizontal turns, vertical tilts, about the screen centre (or the followed object). | | – |
| CAM-13 | Right-click a marker and empty space: nothing visible; hook named in the camera module. A right drag never triggers it. | | – |
| CAM-14 | WASD and arrows at scene A and scene C, level and tilted: the same apparent speed; short ease in and out; Shift is about 3× faster. Up always moves towards the top of the screen. | | – |
| CAM-15 | Q / E turn; Page Up tilts towards the horizon, Page Down back; tilt stops at the limits. | | – |
| CAM-16 | + / = / numpad + and − / numpad − step zoom exactly like the wheel, including stops, limits and retargeting. Nothing selected: anchored on the screen centre. | | – |
| CAM-17 | Default: cursor at the window edge does nothing. With `edge_scroll_enabled` = true: it pans at keyboard speed. | | – |
| CAM-18 | Code read: all camera and clock keys come from one bindings resource. | | – |
| CAM-19 | Pan hard in each direction at scene A, and pan far while following: the view stops at the bound. Home always recovers. | Auto (clamp) | – |
| CAM-20 | Double-click Halden IV and drag during the move: the camera takes over smoothly with no snap, and the move does not resume. | | – |
| GRP-01 | Step zoom in and out across a moon's threshold, and hold just inside the 20 to 24 px band: no flicker. | Auto (hysteresis) | |
| GRP-02 | Zooming in from scene A, IVd and IVc unfold before IVa, IVb and the station (board 6, middle panel). | | |
| GRP-03 | Satellites slide out of and back into Halden IV during zoom animations. | | |
| GRP-04 | Scene A: no moon, station or parked ship drawn anywhere; only pills. | | |
| GRP-05 | Halden IV shows a pill beside its marker, not overlapping it, with the right count. | | |
| GRP-06 | Halden IV's pill has an orange outline (K-3) and a blue dot (parked own ship). | | |
| GRP-07 | Hover the pill: list of folded satellites. Click it: same as clicking Halden IV. | | |
| GRP-08 | No transit ships yet; check again when they exist. | | – |
| MAG-01 | Scene B: magnifier and inset open together. Clicking the pill does the same. | | |
| MAG-02 | Dashed circle ~80 px, "×N" label, map dimmed; it stays centred on Halden IV as it moves (run at 5 d/s). | | |
| MAG-03 | Phases in the magnifier match the real positions seen in scene C at the same moment (pause, compare). | Auto (scale factor) | |
| MAG-04 | Station, parked ship and K-3 always present; only moons of 200 km or more radius. | Auto | |
| MAG-05 | With the inner-orbit test triggered (Morrow's moons), "+N outer" appears on the frame. | Auto | |
| MAG-06 | Hover a satellite in the magnifier: tooltip. Click: inset switches to it. Double-click: frames it. | | |
| MAG-07 | Magnifier closes on selecting another body, on Esc, and when zooming in unfolds the satellites (no jump at hand-over). | | |
| MOT-01 | Scene C at 1 d/s: IVc and IVd are dots with ~30° fading trails. | | |
| MOT-02 | Scene C at 1 d/s: Arden Highport, IVa and IVb are bright, thicker rings with labels fixed on the ring, no dot. | | |
| MOT-03 | Switch 1 d/s ↔ 5 d/s: IVc (31 s → 6.2 s) changes mode with a cross-fade; nothing flickers near a threshold. | Auto (mode state machine) | |
| MOT-04 | Pause: every satellite becomes a dot at its true position. | | |
| MOT-05 | Hover and click an exposure ring: targets that satellite. | | |
| MOT-06 | With the magnifier open, run at 1 d/s and 5 d/s: same modes inside the magnifier. Pell also follows the rule at high rates. | | |
| MOT-07 | Select IVa while it is in ring mode: ring stays highlighted; inset says position "shown when paused". | | suspect F3-04 |
| TEST-01 | All listed bodies, sites and gates present. | | |
| TEST-02 | Inspect or log periods: Highport ≈0.1 d, IVa ≈1.5, IVb ≈4.2, IVc ≈31, IVd ≈68. | Auto | |
| TEST-03 | K-3 (diamond, slow orbit) and one own parked ship (triangle) orbit Halden IV. | | |
| TEST-04 | See INS-04. | | |

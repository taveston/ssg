# Spec: System view

*Scope:* the strategic vector display of one star system, as built in the proof-of-concept iterations. *Source ideas:* `docs/ideas/presentation/presentation.md` (section 2), `docs/ideas/presentation/presentation-sketches.md` (boards 1, 2, 5, 6), mock-ups in `docs/ideas/presentation/mockups/`. Where a mock-up and this spec disagree, this spec wins.

*How to read it:* every requirement has a stable ID (`ZOOM-03`). IDs are never reused or renumbered; a withdrawn item stays in place marked ~~struck~~ with a note. *Since* says which iteration first built it. Numbers in `code` are tunables (table at the end); they live in `src/tunables.rs` and must never be hard-coded. Checks for each item are in `docs/validation/system-view.md`.

---

## 1. Data and scale

| ID | Requirement | Since |
|---|---|---|
| SCL-01 | Simulation data is stored in real units (km, kg, days) as `f64`, and converted to render space in one place. | it1 |
| SCL-02 | Positions and orbits are drawn at true scale, with one linear scale for planetary and satellite orbits alike. Only markers are stylised, never geometry. | it2 |
| SCL-03 | The system is data (a tree of body records) that a generator can replace without changing rendering code. | it2 |
| SCL-04 | Orbits are circular and coplanar, with periods from Kepler's third law using the parent's mass. | it1 |

## 2. Clock and top bar

| ID | Requirement | Since |
|---|---|---|
| CLK-01 | The game clock runs continuously at a default of 1 game day per real second. | it1 |
| CLK-02 | The top bar shows the breadcrumb ("Cluster / Corvane system"), the calendar date, and Pause, 1 d/s and 5 d/s buttons; the active rate is highlighted. | it2 |
| CLK-03 | Space toggles pause; + doubles and − halves the rate (minimum 1/16 d/s). | it1 |
| CLK-04 | Pausing stops all orbital motion; every view stays fully interactive while paused. | it1 |

## 3. Markers and palette

| ID | Requirement | Since |
|---|---|---|
| MRK-01 | Palette: ground #070A0E; neutral bodies and text #DCE6EC; orbits and secondary text #9FB6C4; own assets #5AA9E6 (blue); foreign contacts and warnings #F0A040 (orange); selection and hover as a white ring. | it2 |
| MRK-02 | Type: Barlow for labels and UI; JetBrains Mono for dates, figures and status readouts. | it2 |
| MRK-03 | Each object is a fixed-size screen marker by class, projected from its 3D position each frame; marker size never depends on physical size. | it2 |
| MRK-04 | Marker shapes: filled disc = body; hollow square = station; filled triangle = own ship; filled diamond = foreign contact; X = salvage site. | it2 |
| MRK-05 | Orbits are faint rings; the belt is a translucent shaded band. | it2 |
| MRK-06 | Separate planets are never merged, however close on screen. | it3 |
| EDGE-01 | A gate outside the viewport is shown as an arrow pinned to the screen edge in its direction, labelled with its destination. | it2 |

## 4. Hover and inspection

| ID | Requirement | Since |
|---|---|---|
| HOV-01 | Hovering a marker draws a white ring and a tooltip: name · ownership · contents summary — "Click: inspect · Double-click: frame its moons · Double-click again: planetary view". | it2, text it3 |
| INS-01 | Clicking a marker selects it and opens an inspection inset anchored to the marker by a leader line; the clock keeps running. | it2 |
| INS-02 | The inset shows: header (name, class and ownership line), Focus, Pin and Close buttons; an image for the knowledge state (INS-04); key facts; and an "Open planetary view" button, disabled until the planetary view exists. | it2 |
| INS-03 | Pin keeps an inset open when the selection changes, so two objects can be compared; Esc closes the unpinned inset. | it2 |
| INS-04 | The inset image follows knowledge: **surveyed** = clean render, status "SURVEYED · *date*"; **unsurveyed** = grainy telescope frame, "TELESCOPE FRAME · UNSURVEYED" in orange, estimates as ranges; **stale** = sepia image, "LAST IMAGED · *date*". | it2 |

## 5. Navigation and zoom

| ID | Requirement | Since |
|---|---|---|
| ZOOM-01 | Each wheel notch multiplies the view scale by `zoom_step_factor`, animated over `zoom_anim_ms` with ease-out. Raw wheel deltas are never used. | it3 |
| ZOOM-02 | New input during a zoom animation retargets from the current scale; it is not queued. | it3 |
| ZOOM-03 | Zoom is anchored on the cursor, or on the selected object when there is one. | it3 |
| ZOOM-04 | Stops are computed per system from its real orbits: **system frame** (all planetary orbits fit; gates stay as edge indicators), **region frame** (the inner system, or around the selected planet's orbit), **satellite frame** (the selected planet's satellites fit). | it3 |
| ZOOM-05 | A zoom step that lands on a stop takes longer by `zoom_stop_resistance`, then settles exactly on the stop. | it3 |
| ZOOM-06 | Hard limits: outer = system frame; inner = satellite frame of the selected body, or of the body nearest the cursor when nothing is selected. Scrolling past a limit does nothing and never changes layer. | it3 |
| ZOOM-07 | Trackpad pinch is continuous, and on release settles to the nearest stop if within `pinch_settle_steps`. | not built |
| ZOOM-08 | The first double-click on a body inspects it and frames its satellites in one pan-and-zoom of `focus_jump_ms`; F does the same for the current selection. | it3 |
| ZOOM-09 | A double-click on an already-inspected body is the planetary-view trigger. Until that view exists it calls a clearly named hook and does nothing visible. | it3 |
| ZOOM-10 | Home returns to the system frame with the same animated move. | it3 |
| ZOOM-11 | Ctrl+1 to Ctrl+4 save a bookmark (scale and centre, or the followed object); 1 to 4 recall it with the same animated move. | it3 |
| ZOOM-12 | Right drag tilts the camera; middle drag pans. | it1 |

## 6. Satellite grouping

A **satellite** is anything orbiting a body other than the star: moons, stations, ships parked in orbit. Its **parent** is the body it orbits. Moons of moons follow the same rules. Ships in transit are not satellites.

| ID | Requirement | Since |
|---|---|---|
| GRP-01 | A satellite is drawn individually only while its orbit's on-screen radius is at least `unfold_at_px`; below `fold_below_px` it folds into its parent and its marker, label, ring and trail are hidden. Between the two it keeps its current state. | it3 |
| GRP-02 | The test is per satellite and re-evaluated as zoom changes, so outer satellites unfold before inner ones. | it3 |
| GRP-03 | Folding and unfolding animate the satellite into or out of its parent over `fold_unfold_anim_ms`, including during zoom animations. | it3 |
| GRP-04 | At the system frame every moon and planetary station is folded. | it3 |
| GRP-05 | A parent with folded satellites shows a "+N" pill beside its marker without overlapping it. | it3 |
| GRP-06 | The pill has an orange outline if any folded satellite is foreign, and a small blue dot if own ships are parked there. | it3 |
| GRP-07 | Hovering the pill lists the folded satellites; clicking it does exactly what clicking the parent does. | it3 |
| GRP-08 | Ships in transit are never folded. | it3 (rule only; no transit ships yet) |

## 7. Fan-out magnifier

| ID | Requirement | Since |
|---|---|---|
| MAG-01 | Selecting a body with folded satellites (marker or pill) opens the magnifier together with the inspection inset, as one selection. | it3 |
| MAG-02 | Form: a circle of `magnifier_radius_px` with a dashed frame, centred on the parent and following it as it orbits, over the dimmed map; a label shows the magnification (for example "×40"). | it3 |
| MAG-03 | Geometry inside is true: real phases and real relative radii, scaled so the outermost included orbit fills the frame. Radii are never compressed. | it3 |
| MAG-04 | Contents: all stations and parked ships, and moons with radius at least `magnifier_moon_radius_threshold_km`. | it3 |
| MAG-05 | If the innermost included orbit would fall within `magnifier_inner_min_px` of the centre, the outermost small moons are dropped and counted as "+N outer" on the frame. | it3 |
| MAG-06 | Satellites inside are interactive: hover shows the tooltip; click inspects that satellite (the inset switches); double-click follows ZOOM-08. | it3 |
| MAG-07 | The magnifier closes when the selection changes, on Esc, or when zooming unfolds the satellites normally; in the last case the hand-over has no visual jump. | it3 |

## 8. Motion rule

The **apparent period** of an orbit is its orbital period divided by the time rate, in real seconds; infinite when paused.

| ID | Requirement | Since |
|---|---|---|
| MOT-01 | Apparent period at or above `t_dot_secs`: the object is a dot at its true position with a fading trail of `trail_arc_degrees` of arc. | it3 |
| MOT-02 | Apparent period below `t_ring_secs`: exposure-ring mode. No dot; the orbit ring is brighter and `exposure_ring_width_px` wide; the label sits on the ring at a fixed spot. | it3 |
| MOT-03 | Between the two thresholds the current mode is kept; switching cross-fades over `motion_crossfade_ms`. | it3 |
| MOT-04 | There is no special case for pause: an infinite apparent period gives dot mode. | it3 |
| MOT-05 | Hover and click on an exposure ring target its object. | it3 |
| MOT-06 | The rule applies to every orbiting object (moons, stations, parked ships, close-in planets), inside the magnifier too; ships in transit are ignored. | it3 |
| MOT-07 | A selected object in ring mode keeps its ring highlighted, and the inset gives its position as "shown when paused". | it3 |

## 9. Test content: Corvane

The prototype system, kept as data (SCL-03). It exists to exercise the rules above, so its values serve testing first. Its orbits do not yet match the reference layout in `docs/ideas/mechanics/drives-and-transit.md` section 2 (see `docs/findings/iteration-3.md`).

| ID | Requirement | Since |
|---|---|---|
| TEST-01 | Corvane contains at least: the star; planets Pell, Iver (with Iver Yard station), Halden IV (capital, own) and Morrow (outer giant with many moons); a belt with salvage sites Wreck Field and Lantern; and off-screen gates. | it2 |
| TEST-02 | Halden IV's satellites have periods of about: Arden Highport 0.1 d, IVa 1.5 d, IVb 4.2 d, IVc 31 d, IVd 68 d, so both motion modes show at 1 d/s. Radii follow Kepler from Halden IV's mass. | it3 |
| TEST-03 | Halden IV also has a foreign contact K-3 (diamond, slow orbit) and one own ship parked in orbit (triangle), as static orbiting objects. | it3 |
| TEST-04 | The inspection data covers all three knowledge states: Halden IV surveyed, Halden IVc unsurveyed, Lantern stale. | it2 |

## Tunables

All in the `Tunables` resource (`src/tunables.rs`; `src/view/tunables.rs` from iteration 4). Defaults are starting values; changing one is a spec change only if the default is being settled.

| Tunable | Default | Used by |
|---|---|---|
| `unfold_at_px` (R_show) | 24 px | GRP-01 |
| `fold_below_px` | 20 px | GRP-01 |
| `fold_unfold_anim_ms` | 200 | GRP-03 |
| `t_dot_secs` (T_dot) | 25 s | MOT-01 |
| `t_ring_secs` (T_ring) | 20 s | MOT-02 |
| `motion_crossfade_ms` | 200 | MOT-03 |
| `trail_arc_degrees` | 30 | MOT-01 |
| `exposure_ring_width_px` | 3 | MOT-02 |
| `magnifier_radius_px` | 80 | MAG-02 |
| `magnifier_inner_min_px` | 12 | MAG-05 |
| `magnifier_moon_radius_threshold_km` | 200 | MAG-04 |
| `zoom_step_factor` | ×2 | ZOOM-01 |
| `zoom_anim_ms` | 200 (range 150–250) | ZOOM-01 |
| `zoom_stop_resistance` | 0.35 | ZOOM-05 |
| `pinch_settle_steps` | 1 | ZOOM-07 |
| `focus_jump_ms` | 400 | ZOOM-08, ZOOM-10, ZOOM-11 |
| `zoom_fit_margin` | 0.8 | ZOOM-04 (added in it3; not in the original ideas) |
| `double_click_window_ms` | 350 | ZOOM-08 (fallback only) |

## Not yet specced

From the ideas, not to be built until specced: layer toggles, alerts, legend, Administration button, inspection fidelity for foreign contacts (thermal track), ships in transit and trajectories, uncertainty ellipses, the planetary view and its transition, the cluster view, gate distance labels, and every open question in `docs/ideas/presentation/presentation.md` section 6.

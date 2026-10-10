# Orphan Worlds

Hard sci-fi space strategy game for PC. The design lives in `docs/`; read the relevant parts before making design-level decisions.

- `docs/gdd.md`: main design document (vision, pillars, systems, open questions)
- `docs/presentation.md`: view layers and visual style. Section 2 ("Navigation and zoom" and "Satellites: grouping, fan-out, and motion") is the spec for the current milestone.
- `docs/presentation-sketches.md`: the reference sketches described in words: palette, markers, example content, and boards 1 to 6.
- `docs/mockups/`: the same sketches as PNG images, one per board; open them with your image reader. See `docs/mockups/README.md`. Where an image and `docs/presentation.md` disagree, the doc wins.
- `docs/world-generator.md`: how maps will be generated (not needed yet; useful background for how systems are structured)
- `docs/faction-history.md`: faction history options and ideology rules
- `docs/components.md`: ship component catalogue
- `docs/ideology-triangle.svg`: diagram of the ideology triangle

Status tags in the docs: **Decided** = settled, **Direction** = settled in principle, **Candidate** = under consideration. Don't treat Candidate items as requirements unless this file lists them for the current milestone.

## Stack

- Rust (stable) and Bevy, pinned in `Cargo.toml`. Bevy's API changes between versions, so check the docs and examples for the pinned version rather than relying on memory.
- Keep dependencies minimal. Ask before adding crates beyond Bevy. Fonts are assets in `assets/fonts/` (Barlow, JetBrains Mono, OFL).

## Done

- **Iteration 1:** coarse 3D system with circular orbits, an orbit/zoom/pan camera and a game clock.
- **Iteration 2:** the Corvane system as data at true scale, fixed-size UI markers projected from 3D positions, a top bar with calendar date, hover tooltips, inspect in place with pinning, fidelity states, and edge indicators for off-screen gates.

## Current milestone: iteration 3, fix grouping, zoom and motion

Iteration 2 got three things wrong, and the spec has since been tightened to fix them: all moons are drawn individually at whole-system zoom, moons orbit too fast to read at 1 day/s, and the fully continuous zoom feels odd. The rules are in `docs/presentation.md` section 2; board 6 (`docs/mockups/6-satellite-grouping-and-motion.png`) shows the intended result. Implement those subsections as written. Summary of what to change:

1. **Tunables first.** Put every number marked *tunable* in the spec in one config resource (one file, documented field by field): R_show and fold thresholds (24 / 20 px), T_dot and T_ring (25 / 20 s), magnifier radius (80 px) and innermost-orbit minimum (12 px), moon size threshold for the magnifier, zoom step factor (×2), zoom animation time (150 to 250 ms), stop resistance, pinch settle distance, focus-jump time (about 400 ms), fold and cross-fade times (about 200 ms). Several are open design questions, so they must be easy to change without hunting through the code.
2. **Satellite grouping** (replaces the current overlap-based pill logic in `markers.rs`). A satellite is anything orbiting a body other than the star. Draw it only if its orbit's on-screen radius is at least R_show, with hysteresis; otherwise fold it into its parent, with no marker, label, ring or trail. Evaluate per satellite, so outer moons unfold first. Animate fold and unfold from and to the parent. The parent's "+N" pill sits beside its marker; give it an orange outline if a folded satellite is foreign and a blue dot if own ships are parked there; hovering lists the contents; clicking it equals clicking the parent. Never merge separate planets. At the default system zoom every moon and planetary station must be folded.
3. **Fan-out magnifier** (replaces the current fan-out). Opens with the inspection inset when a body with folded satellites is selected. A circular frame of about 80 px radius with a dashed border and a magnification label ("×40"), centred on and following the parent, over the dimmed map. Geometry inside is true: real phases and real relative radii, scaled so the outermost included orbit fills the frame. Include stations, parked ships, and moons above the size threshold; if the innermost orbit would fall within 12 px of the centre, drop the outermost small moons and show "+N outer" on the frame. Satellites inside are interactive. It closes on selection change, Esc, or when zooming unfolds the satellites normally, handing over without a visual jump.
4. **Motion rule.** Apparent period = orbital period ÷ time rate (infinite when paused). At or above T_dot: a dot at the true position with a fading trail of about 30° of arc (replace the current period-fraction trail). Below T_ring: exposure-ring mode, with no dot, a brighter and slightly thicker ring, the label fixed on the ring, and hover or click on the ring targeting the satellite. In between, keep the current mode; cross-fade on switching. No special case for pause. It applies to every orbiting object (moons, stations, parked ships, close-in planets), inside the magnifier too. A selected satellite in ring mode keeps its ring highlighted and the inset says its position is "shown when paused". Ring-mode hit testing is new, so make sure hover and click work on the ring itself.
5. **Zoom and navigation** (replaces `handle_zoom_input` in `camera.rs`).
   - Each wheel notch multiplies scale by the step factor, animated with ease-out; never use raw wheel deltas. New input mid-animation retargets from the current scale.
   - Zoom towards the cursor, or towards the selected object if there is one.
   - Stops computed per system from its real orbits: system frame (all planetary orbits fit; gates stay as edge indicators), region frame (inner system, or around the selected planet's orbit), satellite frame (the selected planet's satellites fit). Add slight resistance at each stop, then settle exactly on it.
   - Hard limits: outer = system frame; inner = satellite frame of the selected body, or of the body nearest the cursor when nothing is selected. Scrolling past a limit does nothing more.
   - Trackpad pinch stays continuous but settles to the nearest stop on release if within one step.
   - First double-click on a body inspects it and frames its satellites in one pan-and-zoom of about 400 ms; the focus key (F) does the same for the selection. A double-click on an already-inspected body is the planetary-view trigger: the view doesn't exist yet, so leave a clear hook and keep the inset button disabled.
   - Home returns to the system frame. Bookmarks: Ctrl+1 to Ctrl+4 save the current scale and centre (or followed object), and 1 to 4 recall them with the same animated move.
   - Folding and unfolding happen during zoom animations, so satellites slide in or out as the view arrives.
   - Keep the existing camera tilt (right drag) and pan (middle drag).
6. **Test content.** The motion rule needs both modes visible at 1 day/s. Adjust Halden IV's satellites so their periods match board 6: Arden Highport about 0.1 days, IVa about 1.5, IVb about 4.2, IVc about 31, IVd about 68. Change orbit radii or Halden IV's mass to get there, and keep Kepler consistent. Add two placeholder satellites of Halden IV so the pill indicators can be seen: foreign contact K-3 (a diamond, in a slow orbit) and one own ship parked in orbit (a triangle). Both are static orbiting objects for now, with no ship simulation behind them.
7. **Hover tooltip text.** Change to "Click: inspect · Double-click: frame its moons · Double-click again: planetary view", as in board 1.

Out of scope: ships in transit and trajectories (but keep the rule that transit ships never fold, and make the motion rule ignore them), the planetary view and transition, layer toggles, alerts, the administration and cluster views, and the open questions listed at the end of `presentation.md` section 6. Don't hard-code an answer to those; use the config values.

## Guidelines

- Keep simulation data separate from rendering. Store everything in real units (km, days) using `f64` in simulation data, and convert to render space in one place. Marker size is a presentation choice in screen pixels, never derived from physical size.
- Keep the system as data, so a generator can replace it later (see `docs/world-generator.md`).
- Organise code with Bevy plugins, roughly one per area. Grouping, the magnifier and the motion rule are separate concerns; consider a module each rather than growing `markers.rs` and `hover_inspect.rs` further.
- Work in steps and keep the app runnable after each one; a sensible order is tunables → zoom steps and stops → grouping → motion rule → magnifier → double-click, focus, Home and bookmarks → test content.
- Run `cargo check` and `cargo clippy` before calling work done. I run the app myself to judge the visuals, so describe what I should see and how to try each feature, including how to check the motion rule (for example, pause and switch between 1 d/s and 5 d/s while the magnifier is open).
- Compare your result with the mock-ups for look and layout, but follow `docs/presentation.md` for behaviour.
- If something in the docs is ambiguous or doesn't work on screen, make a sensible choice, note it in your summary, and suggest what to change in the docs.

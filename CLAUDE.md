# Orphan Worlds

Hard sci-fi space strategy game for PC. The design lives in `docs/`; read the relevant parts before making design-level decisions.

- `docs/gdd.md`: main design document (vision, pillars, systems, open questions)
- `docs/presentation.md`: view layers and visual style (the main reference for the current milestone)
- `docs/presentation-sketches.md`: reference sketches described in words: palette, markers, example content, hover, inspect, and planetary view boards. It links to a design canvas you can't open; the doc is the full description.
- `docs/faction-history.md`: faction history options and ideology rules
- `docs/components.md`: ship component catalogue
- `docs/ideology-triangle.svg`: diagram of the ideology triangle

Status tags in the docs: **Decided** = settled, **Direction** = settled in principle, **Candidate** = under consideration. Don't treat Candidate items as requirements unless this file lists them for the current milestone.

## Stack

- Rust (stable) and Bevy, pinned in `Cargo.toml`. Bevy's API changes between versions, so check the docs and examples for the pinned version rather than relying on memory.
- Keep dependencies minimal. Ask before adding crates beyond Bevy. Font files are assets, not dependencies (see below).

## Done: iteration 1

A coarse 3D system: placeholder "Veska" system as data (`system_data.rs`), circular orbits, gizmo orbit rings, sphere meshes with a compressed size scale (`scale.rs`), an orbit/zoom/pan camera with click-to-follow, and a game clock with pause and speed keys.

## Current milestone: iteration 2, the system view as a vector display

Goal: turn the system view into the vector plotting display from `docs/presentation.md` section 2, using the Corvane example content from `docs/presentation-sketches.md`. Still no simulation: nothing here needs ships, economy, or factions to work.

In scope:

1. **Corvane system as data.** Replace Veska with Corvane (K2V star): Pell, Iver with the dormant Iver Yard station, an inner asteroid belt with an unsurveyed wreck field, Halden IV (capital world, moons IVa to IVd, Arden Highport station), Morrow (outer giant with 11 moons), and two or three gates at the system edge, one of them to Ostra. Invent plausible orbits and periods (use Kepler's third law with the star's mass), and put gates far enough out that they are off-screen when the inner system fills the view. Each object carries the data the hover and inspect views need: class, ownership (own / foreign / neutral), a one-line description, key facts, and knowledge state (surveyed with an imaged date, unsurveyed, or stale with a last-imaged date). Use the sketch's figures for Halden IV and IVc; the "Lantern" station wreck in the belt can be the stale example.
2. **True-scale geometry, fixed-size markers.** Positions and orbits at true scale; replace the spheres with markers drawn at a fixed screen size by class: filled disc for bodies (size by class), hollow square for stations, a gate marker, X for salvage sites. Keep the 3D camera, but default to a steep, near top-down angle; markers face the camera. Remove the compressed body-size scale. The zoom range must span a moon system up to the gates, so make zoom smooth and logarithmic.
3. **Clutter handling.** Moons and stations that would overlap their planet's marker on screen collapse into a pill with a count ("+5"), and expand into individual markers as you zoom in. The belt is density shading (a faint band, not individual rocks), with the wreck field promoted to its own X marker.
4. **Visual language from the sketches.** Near-black ground, faint grey-blue orbit lines, and the colour roles: off-white neutral, blue own, orange foreign or warning, white ring for hover and selection. Star with a soft glow and a label. Labels next to markers. Fading trails behind fast bodies (Pell, inner moons). A 1 AU scale bar that adapts to zoom. Use Barlow for labels and UI and JetBrains Mono for dates and figures; both are OFL fonts from Google Fonts. Put them in `assets/fonts/` with their licence files.
5. **Top bar.** Breadcrumb ("Cluster / Corvane system", not clickable yet); the date as a calendar starting 14 Mar 2341 (e.g. "14 Mar 2341"); pause, 1 d/s and 5 d/s buttons, with the existing keyboard shortcuts still working. Replace the debug readout.
6. **Hover and inspect in place.**
   - Hover: white ring plus a tooltip ("Halden IV · own · 4 moons, 1 station — Click: inspect").
   - Click: the rest of the map dims, the planet's moons and stations fan out in a small local bubble around its marker, and an inspection inset opens next to the marker, joined by a leader line. The clock keeps running.
   - The inset has a header (name, class line, Pin and Close), a placeholder image, key facts, "In orbit" chips that inspect that moon or station, and a disabled "Open planetary view" button.
   - Pin keeps an inset open (with a dashed leader line) so two objects can be compared. Esc or clicking empty space closes the unpinned inset.
   - The camera only refocuses on an object if you choose that from the inset (a "Focus" action); click is for inspecting now, not following.
7. **Fidelity follows knowledge.** The inset image reflects knowledge state, generated in code (no art assets yet): surveyed is a clean shaded gradient globe with a blue "SURVEYED · 02 FEB 2341" status line; unsurveyed is a blurred, grainy frame with corner brackets, an orange "TELESCOPE FRAME · UNSURVEYED" label, and facts shown as ranges or "unknown"; stale is desaturated and sepia-tinted with "LAST IMAGED · 2 Y 9 M AGO" worked out from the current date.
8. **Edge-of-screen indicators.** Gates that are off-screen get an arrow at the screen edge with a label ("Gate → Ostra · off-screen").

Out of scope for now: ships, convoys and contacts (and their uncertainty ellipses), transfer arcs, the planetary view and the transition into it, the administration view, the cluster view, layer toggles, alerts, double-click actions, focus bookmarks, real textures or renders, and anything from the economy, characters, or factions. Leave clean seams for ship and contact markers, since they come next.

## Guidelines

- Keep simulation data separate from rendering. Store everything in real units (AU or km, days, km for body radius) using `f64` in simulation data, and convert to render space in one place. Marker size is a presentation choice in screen pixels, never derived from physical size.
- Keep the system as data (Rust structs, or a RON file in `assets/` if that becomes easier), so a generator can replace it later.
- Organise code with Bevy plugins, roughly one per area (system data, orbits, markers and labels, camera, time, UI, inspection).
- Work in steps and keep the app runnable after each one; a sensible order is data → true scale and markers → clutter → visual style → top bar → hover and inspect → fidelity → edge indicators.
- Run `cargo check` and `cargo clippy` before calling work done. I run the app myself to judge the visuals, so describe what I should see and how to try each feature.
- If something in the docs is ambiguous or doesn't work on screen, make a sensible choice, note it in your summary, and suggest what to change in the docs.

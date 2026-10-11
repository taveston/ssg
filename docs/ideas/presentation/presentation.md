# Orphan Worlds: Presentation & Visual Style

*Status:* Draft. Companion to the design document. Status tags as in the design document: **Decided**, **Direction**, **Candidate**. Reference sketches: [Inspect & Planetary Transition](https://claude.ai/artifact/E8U8j7d2HdB5gGd1S1W5WR) (design canvas: system view, inspect in place, transition storyboard, planetary view, inspection fidelity states), described in words in *Reference Sketches*.

---

## 1. View Layers (decided)

| Layer | Purpose | Style |
|---|---|---|
| Cluster | Gate network, systems, inter-system movement | Vector display (section 2) |
| System | Bodies, orbits, ships, stations, gates within one system | Vector display (section 2) |
| Planetary | Settlement administration and ground combat on one body | See section 3 |
| Faction administration | High-level management, collated reports, character interactions | Fully 2D UI (direction; section 4) |

- The first three are spatial layers, nested from cluster down to planet. Faction administration runs in parallel to them rather than being a level of zoom.

## 2. Spatial Display

- **The problem (context):** at true scale, planets are far smaller than a pixel in a whole-system view, and ships are invisible at any zoom that shows more than one of them. Engagements span thousands of kilometres, so a close-up of any single ship tells the player little about what is happening.
- **Strategic display (decided):** the system and cluster views are a vector plotting display: orbits as faint lines, bodies and ships as solid markers, trajectories as lines. System, cluster, and combat views share one visual language; combat is the same display with the clock slowed (design document, 4.6). The look reads as the government's own instruments, which suits a head of state rather than a pilot.
- **Scale (direction):** positions and orbits are drawn at true scale; markers are drawn at a fixed screen size by class, never at physical size. Only the marker is stylised, never the geometry. (Specced for the system view: SCL-02, MRK-03, MRK-04.)
  - Marker shape and weight encode type: planet, moon, station, gate, ship, salvage site.
  - Ship symbology (candidate): NATO-style frames for affiliation, icons for ship type, and amplifiers for state, groups and identification. See *Symbology & UI Style* (`symbology.md`).
  - Navigation by stepped zoom with stops, focus jumps, and bookmarks (specced: `docs/spec/system-view.md`, ZOOM); edge-of-screen indicators for important off-screen objects (candidate; specced for gates as EDGE-01).
  - How gate distance is handled on screen is deliberately left open until gate placement is tuned (section 6).
- **Inspection view (decided):** highlighting any object expands it into a semi-photorealistic view. This applies to everything: bodies, stations, gates, sites, and ships. Ships appear as blips because of the distances involved, not because they are kept abstract.
  - **Presentation (candidate):** the view expands in place as an inset anchored to the marker, with the vector display still visible around it. Insets can be pinned to compare objects side by side. (Specced: INS-01 to INS-03.)
  - **Fidelity follows knowledge (candidate):** the view shows the best image the state holds. An unsurveyed body is a grainy telescope frame; a surveyed one is a clean render with overlays (ice, ruins, settlements); an old observation shows the last image with its date. Own ships and stations render fully; foreign contacts only as well as they have been observed. Ties the visuals to the uncertainty pillar and to partly known salvage sites (design document, 4.5). (Specced for bodies and sites: INS-04; foreign contacts not yet.)
  - **Renders from designs (candidate):** ship and station renders are assembled from their component designs, so two bids for the same requirements look visibly different, and as-built variants and refits show.
- **Information model (decided):** the player has a semi-omniscient view. Signal propagation delay is not modelled: what the state knows, the player sees at once, in every system.
- **Abstract uncertainty (candidate):** limits on knowledge are modelled in abstract ways rather than through physics:
  - Undetected contacts are not shown. Thermal detection (see *Component Catalogue*, sensors) decides what appears.
  - Tracked contacts carry an uncertainty region reflecting tracking quality: sensor coverage, the contact's emission control, time since a good detection.
  - Salvage contents, foreign designs, and unsurveyed bodies are known only as well as they have been observed (see the inspection view above).
- **Clutter (candidate):**
  - Moons, stations, and ships in orbit around a body fold into one count marker on that body (specced: `docs/spec/system-view.md`, GRP).
  - Asteroid belts and debris fields appear as density shading; surveyed, claimed, or active sites are promoted to individual markers.
  - Information layers can be switched on and off: ownership, resources, salvage, traffic, sensor coverage.
- **Motion (candidate):** ship paths are drawn as their actual arcs, including burns. Transfer planning shows a fan of alternative paths, each labelled with transit time and delta-v (design document, 4.2). How fast-orbiting objects are drawn is set by the motion rule (specced: `docs/spec/system-view.md`, MOT).
- **Cluster view (decided):** same style as the system view: systems as nodes, gates as links. Gate throughput, reliability, and ownership as line weight, line style, and colour (candidate).

### Navigation, satellites and motion

> **Specced** in `docs/spec/system-view.md`: navigation and zoom as ZOOM-01 to ZOOM-11, satellite grouping as GRP-01 to GRP-08, the motion rule as MOT-01 to MOT-08, with the tunables table. The rules moved there on 2026-10-10; this section keeps only the reasoning and the parts not yet specced.

- **Why (settled 2026-10-09, after iteration 2):** the proof of concept drew every moon at whole-system zoom, moons orbited too fast to read at 1 day/s, and fully continuous zoom felt odd. Hence logarithmic stepped zoom with stops computed from each system's real orbits; folding satellites into a "+N" count marker on their parent below a screen-radius threshold; a true-geometry magnifier for a selected body's folded satellites, instead of compressing radii (**rejected** 2026-10-10, see Camera); and a motion rule that shows fast orbits as bright "exposure rings" rather than smeared dots, based on apparent period (orbital period ÷ time rate), so pausing or slowing time brings dots back with no special case.
- **Not yet specced (direction):**
  - Changing layer, to the cluster view or a planetary view, is always deliberate: breadcrumb, double-click on the focused body, or the inset button. Scrolling never changes layer.
  - The motion rule also applies to the orbital layer of the planetary view.
  - A trajectory whose target is in exposure-ring mode ends on the ring; with a dot, it ends at the predicted arrival position.
  - The cluster view uses the same navigation rules where they fit.

### Camera

> **Specced** in `docs/spec/system-view.md` as CAM-01 to CAM-22, with ZOOM-13 to ZOOM-17 (anchor, limits, stops, double-click), and CLK-05 (time-rate keys). Settled 2026-10-10. Optimised for keyboard and mouse; gamepad and touch are kept possible but not specified.

- **Camera model (decided): a map camera with constrained tilt.** The camera always looks at a pivot on the orbital plane. Rotation about the plane's normal is free; tilt runs from straight down to a limit well short of the horizon, and the camera never goes below the plane. The default is top-down, north up. *Why:* orbits are coplanar and markers are fixed-size, so a free 3D camera adds little except ways to get lost, while some tilt gives depth and matches the planetary-view transition. A pure top-down view was considered and rejected for that reason.
- **Rotation (decided): free, with a one-key reset** to level and north up; Home resets it too. A fixed north would help spatial memory, but free rotation lets the player look across the system from any side, and the reset gives the memory aid back on demand.
- **Selection and focus (decided 2026-10-10): two states, not three.** An earlier draft had selection, focus (a one-off framing move) and follow (persistent tracking). Focus and follow were always coupled, "double-click an already-inspected body" was ambiguous, and selection still moved the camera via the zoom anchor, so a selected but untracked planet could be zoomed into and immediately fly off screen. Now:
  - **Selection** is the object whose details are shown (inset, white ring). Click sets it, Esc clears it. It never moves the camera.
  - **Focus** is the object the camera is locked onto. Player-facing word: "Focus" (matches F and the inset button; "follow" is not used). Set by double-click, F (focuses the selection), the inset's Focus button, a bookmark, or **automatically by zooming in** past the region frame towards a body's satellites. Cleared by Home, **zooming out to the system frame** (at that scale orbital drift is readable), the chip's ×, or focusing something else. Panning moves an offset relative to the focused body instead of clearing it. Shown by a "Focus: *name*" chip and corner brackets on the marker, never the selection ring.
  - Zoom anchors on the focus, else the cursor; the inner zoom limit is the focused body, else the body nearest the cursor. Double-click on the focused body opens the planetary view: "look at it", then "go there".
  - The two zoom rules mirror each other (zoom in locks on, zoom out releases), so most of the time the player never manages focus at all. Selection and focus can still differ, which is useful: stay on Halden IV while reading about IVc or K-3 without the camera jumping to a fast moon.
  - Rejected: selection also focuses (Stellaris-like). Clicking a fast moon would swing the camera onto it, and you lose reading about one thing while watching another.
- **Magnifier (rejected 2026-10-10):** a dashed circle that fanned out a selected planet's folded satellites in true geometry. It looked bad on screen, and once double-click focused and framed a planet in one move it only showed the same moons at a second scale. Replaced by the inset's "In orbit" list (INS-05), which shows what is in orbit without moving the camera; a selected folded satellite rings its parent's pill (INS-06). Specced as INS-05, INS-06 and MOT-08; MAG-01 to MAG-08 withdrawn.
- **Mouse (decided):** left drag and middle drag pan 1:1 under the cursor with no inertia; right drag rotates (horizontal = heading, vertical = tilt); **right-click is reserved** for context menus and orders, told apart from right drag by a small movement threshold. Shift+left drag is kept free for a possible box-select.
- **Keyboard (decided):** WASD and arrows pan at a speed relative to the screen, so it feels the same at every zoom; Q/E rotate; Page Up/Down tilt; + and − zoom one step. **The time rate moves to `.` (faster) and `,` (slower)** so + and − can zoom (common in strategy games). Edge scrolling exists but is off by default. All bindings live in one table so they can be rebound later.
- **Bounds (decided):** the pivot cannot leave the system's neighbourhood, so the player can't get lost in empty space; Home is always the recovery.
- **Per-system camera memory (direction):** leaving a system for the cluster or a planetary view and coming back restores the camera, including the focused object. Not specced until there is more than one system.
- **Rotating frame (candidate):** while focused, optionally rotate the camera with the focused body's orbit so the star stays in one direction; useful for planning transfers.
- **Selection cycling (candidate):** Tab / Shift+Tab step the selection through bodies in orbit order. Useful on the keyboard and the main way to navigate with a gamepad.
- **Cluster view camera (direction):** the cluster map is 3D, but shallow in depth. The same camera model applies, with the pivot on the cluster's mean plane; tilt there carries real information rather than flourish, so it may want a wider range. Because depth is shallow, drop lines from each system to the mean plane (or a faint plane grid) could make height readable at a glance; candidate.
- **Gamepad and touch (candidate):** keep every camera action reachable without a mouse. Gamepad: left stick pans, right stick rotates and tilts, triggers step zoom, shoulders cycle selection, a virtual cursor for picking. Touch: one-finger drag pans, pinch zooms (ZOOM-07), two-finger twist rotates, two-finger vertical drag tilts, double-tap stands in for double-click. Nothing should be reachable only by hover: HOV-01's tooltip content also needs a home in the inset.

## 3. Planetary View

- **Purpose (decided):** where settlements are administered and ground combat takes place.
- **Entry (direction):** reached from the system view by the inset's "Open planetary view" button or by double-clicking the focused body; never by scrolling. The inspection view is a look at a body, the planetary view is where the player acts on it.
- **Style (candidate):** a globe using the body's photoreal render as its surface, with vector overlays for settlements, infrastructure, extraction sites, ground formations, landing sites, and orbital assets overhead. This joins the two visual styles in one view.
- **Ground combat (candidate):** delegated and lightly simulated like space combat (design document, 4.6): the player sets objectives and posture, and watches formations move on the overlay.
- **Coverage (candidate):** a full planetary view only for bodies with settlements, surface sites, or fighting; small moons and asteroids use the inspection view alone.

## 4. Faction Administration View

- **Purpose (decided):** the faction managed at a high level: policy, budget, appointments, organisations, collated reports, and character interactions.
- **Style (direction):** a fully 2D UI, separate from the spatial layers.
- **Links to the map (candidate):** reports, requests, and alerts link to the place they concern, and spatial objects link back to their owners, staff, and records.
- **Characters (candidate):** shown with what the state knows about them, keeping "average" and "unknown" visibly distinct (design document, 4.3).

## 5. Art Scope (direction)

- The vector layer is cheap to produce. Photoreal work is needed for generated body types and surfaces, stations, gates, ruins, and ship components. The administration view needs portraits and 2D interface art. Keeps art cost in line with the scope risk (design document, section 7).

## 6. Open Questions

- How gate distance appears on screen once placement is settled (true scale, compressed, or an optional overview scaling).
- Which limits on knowledge to model abstractly, and how; how sensor coverage is shown.
- Inspection view in place or as a panel; how much survey detail is needed for each fidelity level.
- Colour palette and faction colours (a candidate for ships in `symbology.md`).
- Zoom: step factor, animation timing, how strong the resistance at stops is, and whether a region frame is needed in small systems.
- Camera: final tilt limit and pan and rotate speeds after testing; the tilt range for the cluster view.
- Motion rule: switch abruptly between dot and exposure ring, or fade gradually across a band of speeds; final values for R_show, T_dot, T_ring after testing.
- Planetary view as a globe or flat map, and how much surface detail is modelled.
- How the administration view is organised (screens, panels, an inbox of reports), and how the player moves between it and the spatial layers.

# Orphan Worlds: Presentation & Visual Style

**In short:** the game has three nested spatial layers (cluster, system, planet) and a separate 2D administration view. The cluster and system views are a vector plotting display at true scale, with fixed-size markers; combat uses the same display with the clock slowed. Any object can be inspected in a semi-photoreal inset whose fidelity follows what the state knows. The camera is a map camera with limited tilt; selection (what you read about) and focus (what the camera tracks) are separate. Much of section 2 is now specced in `docs/spec/system-view.md`; this doc keeps the reasoning and what isn't specced yet.

*Status:* draft. Companion to the overview; status tags as there (**Decided**, **Direction**, **Candidate**). Reference sketches: [Inspect & Planetary Transition](https://claude.ai/artifact/E8U8j7d2HdB5gGd1S1W5WR), a design canvas with the system view, inspect in place, transition storyboard, planetary view and inspection fidelity states. *Reference Sketches* describes it in words.

---

## 1. View layers (decided)

| Layer | Purpose | Style |
|---|---|---|
| Cluster | Gate network, systems, movement between systems | Vector display (section 2) |
| System | Bodies, orbits, ships, stations, gates within one system | Vector display (section 2) |
| Planetary | Settlement administration and ground combat on one body | See section 3 |
| Faction administration | High-level management, collated reports, character interactions | Fully 2D UI (direction; section 4) |

The first three are spatial layers, nested from cluster down to planet. Faction administration runs alongside them; it isn't a level of zoom.

## 2. Spatial display

- **The problem (context):** at true scale, planets are far smaller than a pixel in a whole-system view, and ships are invisible at any zoom that shows more than one of them. Engagements span thousands of kilometres, so a close-up of one ship tells the player little.
- **Strategic display (decided):** the system and cluster views are a vector plotting display. Orbits are faint lines, bodies and ships are solid markers, trajectories are lines. System, cluster and combat views share one visual language; combat is the same display with the clock slowed (*Combat, Raiders & Diplomacy*). It reads as the government's own instruments, which suits a head of state rather than a pilot.
- **Scale (direction):** positions and orbits are drawn at true scale. Markers are a fixed screen size by class, never physical size. Only the marker is stylised, never the geometry. Zoomed in close enough, a body is drawn as its true disc instead. (Specced for the system view: SCL-02, MRK-03, MRK-04, MRK-07.)
  - Marker shape and weight show the type: planet, moon, station, gate, ship, salvage site.
  - Ship and station symbology (decided, specced as SYM): NATO-style frames for affiliation, icons for ship type, and amplifiers for state, groups and identification. See *Symbology* (`symbology.md`).
  - Navigation is by stepped zoom with stops, focus jumps and bookmarks (specced: ZOOM). Important off-screen objects get edge indicators (candidate; specced for gates as EDGE-01).
  - How gate distance is shown on screen is left open until gate placement is tuned (section 6).
- **Inspection view (decided):** highlighting any object expands it into a semi-photoreal view. This applies to everything: bodies, stations, gates, sites and ships. Ships appear as blips because of the distances involved, not because they are kept abstract.
  - **Presentation (decided 2026-10-12): one inset, docked at the right of the map.** The inset no longer follows its marker: text that moves across the screen, even slowly, is awkward to read. The white selection ring, on the marker and its orbit, shows what is selected, so there is no leader line. There is no pinning: comparing objects side by side belongs in tables in the administration view, and a fixed dock with several insets would need its own layout rules. (Specced: INS-01 to INS-03, INS-07.)
  - **Fidelity follows knowledge (candidate):** the view shows the best image the state holds. An unsurveyed body is a grainy telescope frame. A surveyed one is a clean render with overlays (ice, ruins, settlements). An old observation shows the last image with its date. Own ships and stations render fully; foreign contacts only as well as they have been observed. This ties the visuals to the uncertainty pillar and to partly known salvage sites (*Salvage & Research*). (Only placeholder surveyed and unsurveyed images are specced, INS-04: how information is gathered isn't designed yet, so there is no stale state, status text or date. Foreign contacts not yet.)
  - **Renders from designs (candidate):** ship and station renders are assembled from their component designs. So two bids for the same requirements look visibly different, and as-built variants and refits show.
- **Information model (decided):** the player has a semi-omniscient view. Signal delay isn't modelled: what the state knows, the player sees at once, in every system.
- **Abstract uncertainty (candidate):** limits on knowledge are modelled abstractly rather than through physics.
  - Undetected contacts aren't shown. Thermal detection (see *Component Catalogue*, sensors) decides what appears.
  - Tracked contacts carry an uncertainty region showing tracking quality: sensor coverage, the contact's emission control, time since a good detection.
  - Salvage contents, foreign designs and unsurveyed bodies are known only as well as they have been observed (see the inspection view above).
- **Clutter (candidate):**
  - Moons, stations and ships in orbit around a body fold into one count marker on that body (specced: GRP).
  - Asteroid belts and debris fields appear as density shading. Surveyed, claimed or active sites get individual markers.
  - Information layers can be switched on and off: ownership, resources, salvage, traffic, sensor coverage.
- **Motion (candidate):** ship paths are drawn as their actual arcs, including burns. Transfer planning shows a fan of alternative paths, each labelled with transit time and delta-v (*Drives & Transit*). How fast-orbiting objects are drawn is set by the motion rule (specced: MOT).
- **Cluster view (decided):** the same style as the system view, with systems as nodes and gates as links. Gate throughput, reliability and ownership could show as line weight, line style and colour (candidate).

### Navigation, satellites and motion

> **Specced** in `docs/spec/system-view.md`: navigation and zoom as ZOOM-01 to ZOOM-11, satellite grouping as GRP-01 to GRP-08, the motion rule as MOT-01 to MOT-12, with the tunables table. This section keeps the reasoning and the parts not yet specced.

- **Why:** the first prototype drew every moon at whole-system zoom, moons orbited too fast to read at 1 day/s, and fully continuous zoom felt odd. Hence:
  - logarithmic stepped zoom, with stops computed from each system's real orbits;
  - satellites fold into a "+N" count marker on their parent below a screen-radius threshold;
  - a motion rule based on apparent period (orbital period ÷ time rate) that shows fast orbits as bright "exposure rings" rather than smeared dots. Pausing or slowing time brings the dots back.
  - **Tails on the ring (decided 2026-10-12).** The first build drew dot-mode trails as a chain of sampled points, which looked bad. Instead the orbit ring itself carries the tail as a brightness gradient behind the dot, so it is one smooth stroke and reinforces the ring as the click target. The tail covers the arc swept in the last few real seconds, so it lengthens as time speeds up; when it reaches a full circle it simply is the exposure ring. One continuous rule replaces the two thresholds and the cross-fade between them. 
    Tails stay visible when paused (decided 2026-10-12), since that's when the map is studied: tail length uses the selected rate, which pausing keeps, and while paused each tail is capped just short of a full circle so ring-mode bodies drop back to dots. The bright head end still shows direction. (Specced: MOT-01, MOT-02, MOT-04, MOT-10, MOT-11, MOT-12, `tail_secs`, `dot_fade_from_degrees`.)
- **Not yet specced (direction):**
  - Changing layer, to the cluster view or a planetary view, is always deliberate: the breadcrumb, a double-click on the focused body, or the inset button. Scrolling never changes layer.
  - The motion rule also applies to the orbital layer of the planetary view.
  - A trajectory whose target is in exposure-ring mode ends on the ring; with a dot, it ends at the predicted arrival position.
  - The cluster view uses the same navigation rules where they fit.

### Camera

> **Specced** in `docs/spec/system-view.md` as CAM-01 to CAM-24, with ZOOM-13 to ZOOM-17 (anchor, limits, stops, double-click) and CLK-05 (time-rate keys). Optimised for keyboard and mouse; gamepad and touch are kept possible but not specified.

- **Camera model (decided): a map camera with limited tilt.** The camera always looks at a pivot on the orbital plane. Rotation about the plane's normal is free. Tilt runs from straight down to a limit well short of the horizon, and the camera never goes below the plane. The **standard orientation** is north up with a slight tilt, not flat top-down.
  - *Why:* orbits are coplanar and markers are fixed-size, so a free 3D camera adds little except ways to get lost. Some tilt gives depth and matches the planetary-view transition, which is why a pure top-down view was rejected.
- **Rotation (decided): free, with a one-key reset** to the standard orientation; Home resets it too. Neither key is needed in normal play: the mouse alone should always get you back to a clean view (see zoom-out below). A fixed north would help spatial memory, but free rotation lets the player look across the system from any side, and the reset gives the memory aid back on demand.
- **Selection and focus (decided): two separate states, and focus is always deliberate.**
  - **Selection** is the object whose details are shown (inset, white ring). A click sets it and Esc clears it. It never moves the camera.
  - **Focus** is the object the camera is locked onto: the frame of reference the camera sits in. With nothing focused, the camera sits in the star's frame. The player-facing word is "Focus", matching F and the inset button.
    - *Why it exists:* at true scale with the clock running, a planet crosses the screen in seconds once you're zoomed in. At the system frame, drift is slow enough to read. So focus matters exactly when you're close.
    - Set only by double-click, F (focuses the selection), the inset's Focus button, or a bookmark. **Zooming never sets focus.**
    - Cleared by: Home, zooming out to the system frame, the chip's ×, or focusing something else. Panning moves an offset from the focused body, clamped so the body stays on screen.
    - Shown by a "Focus: *name*" chip and corner brackets on the marker, never the selection ring.
  - **Select by orbit ring (decided 2026-10-12).** Every drawn orbit is a click target for its object, and highlights on hover and selection. The ring stands still while the body moves, so selection no longer means chasing a dot. That lets bodies stay dots at higher speeds, so the ring-mode threshold came down. Near the star and round a busy parent, rings crowd: the nearest ring wins, and the hover highlight is what tells the player which one they'll get. (Specced: HOV-02, HOV-03; the threshold is now `tail_secs`.)
  - **Why selection and focus stay separate with a single inset (decided 2026-10-12).** Merging them would make every click move the camera. And a body in ring mode can be selected by its ring but can't be focused, so the two states must differ.
  - **Fast bodies can't be focused.** A body in exposure-ring mode has no readable position, so double-click or F only selects it. If the focused body enters ring mode (the clock speeds up), focus moves to its parent without a jump, and never comes back on its own: a change of clock rate shouldn't move the camera twice.
  - **Zoom has two stops, both meaningful:** the system frame (everything) and the focused body's satellite frame (its neighbourhood). Every other notch is a plain step. Zoom anchors on the focus, else the cursor.
  - **Inner limit:** with a body focused, just before its true disc would fill a large part of the screen. Close enough, the body is drawn at its true size instead of as a marker. With nothing focused, a fixed minimum view radius: you can zoom in on a point and watch bodies drift past, and double-click is the way to get closer to anything.
  - **Zooming out to the system frame resets the view:** the pivot eases back to the star and the tilt to standard, as part of the same zoom, keeping the heading. The wheel alone always brings the whole system back.
  - Double-click on the focused body opens the planetary view: first "look at it", then "go there".
  - Keeping selection and focus apart is useful: you can stay on Halden IV while reading about IVc or K-3, without the camera jumping to a fast moon.
  - *Why two states:* an earlier draft had three (selection; focus as a one-off framing move; follow as persistent tracking). Focus and follow were always coupled, "double-click an already-inspected body" was ambiguous, and selection still moved the camera through the zoom anchor, so a selected but untracked planet could be zoomed into and immediately fly off screen.
  - *Rejected:* selection also focuses, as in Stellaris. Clicking a fast moon would swing the camera onto it, and you could no longer read about one thing while watching another.
  - *Rejected:* zooming in sets focus. It changed what the camera followed as a side effect of scrolling, and which body you got depended on the cursor. Playing it felt unpredictable.
  - *Rejected:* a "region frame" stop between the system and satellite frames (the inner system, or a frame round the focused planet's orbit). It was vague and arbitrary; plain zoom steps do its job.
- **Magnifier (rejected):** a dashed circle that fanned out a selected planet's folded satellites in true geometry. It looked bad on screen, and once double-click focused and framed a planet in one move, it only showed the same moons at a second scale. The inset's "In orbit" list (INS-05) replaces it: it shows what is in orbit without moving the camera. A selected folded satellite rings its parent's pill (INS-06).
- **Mouse (decided):** left drag and middle drag pan 1:1 under the cursor, with no inertia. Right drag rotates (horizontal = heading, vertical = tilt). **Right-click is reserved** for context menus and orders; a small movement threshold tells it apart from right drag. Shift+left drag is kept free for a possible box select.
- **Keyboard (decided):** WASD and the arrows pan at a speed relative to the screen, so it feels the same at every zoom. Q/E rotate, Page Up/Down tilt, + and − zoom one step. **The time rate uses `.` (faster) and `,` (slower)**, so + and − can zoom, as is common in strategy games. Edge scrolling exists but is off by default. All bindings live in one table so they can be rebound later.
- **Bounds (decided):** the pivot can't leave the system's neighbourhood, so the player can't get lost in empty space. Home always recovers.
- **Per-system camera memory (direction):** leaving a system for the cluster or a planetary view and coming back restores the camera, including the focused object. Not specced until there is more than one system.
- **Rotating frame (candidate):** while focused, the camera could optionally rotate with the focused body's orbit, so the star stays in one direction. Useful for planning transfers.
- **Selection cycling (candidate):** Tab and Shift+Tab step the selection through bodies in orbit order. Useful on the keyboard, and the main way to navigate with a gamepad.
- **Cluster view camera (direction):** the cluster map is 3D, but shallow. The same camera model applies, with the pivot on the cluster's mean plane. Tilt there carries real information, so it may want a wider range. Because depth is shallow, drop lines from each system to the mean plane (or a faint plane grid) could make height readable at a glance (candidate).
- **Gamepad and touch (candidate):** every camera action should be reachable without a mouse.
  - Gamepad: left stick pans, right stick rotates and tilts, triggers step zoom, shoulders cycle selection, and a virtual cursor picks.
  - Touch: one-finger drag pans, pinch zooms, two-finger twist rotates, two-finger vertical drag tilts, double-tap stands in for double-click.
  - Nothing should be reachable only by hover: HOV-01's tooltip content also needs a home in the inset.

## 3. Planetary view

- **Purpose (decided):** where settlements are administered and ground combat takes place.
- **Entry (direction):** from the system view, by the inset's "Open planetary view" button or by double-clicking the focused body; never by scrolling. The inspection view is a look at a body; the planetary view is where the player acts on it.
- **Style (candidate):** a globe that uses the body's photoreal render as its surface, with vector overlays for settlements, infrastructure, extraction sites, ground formations, landing sites and orbital assets overhead. This joins the two visual styles in one view.
- **Ground combat (candidate):** delegated and lightly simulated, like space combat (*Combat, Raiders & Diplomacy*). The player sets objectives and posture, and watches formations move on the overlay.
- **Coverage (candidate):** a full planetary view only for bodies with settlements, surface sites or fighting. Small moons and asteroids use the inspection view alone.

## 4. Faction administration view

- **Purpose (decided):** managing the faction at a high level: policy, budget, appointments, organisations, collated reports and character interactions.
- **Style (direction):** a fully 2D UI, separate from the spatial layers. Layout, panels, message traffic, reports, requests and dossiers: *Interface Style* (`ui-style.md`, candidate).
- **Links to the map (candidate):** reports, requests and alerts link to the place they concern, and spatial objects link back to their owners, staff and records.
- **Characters (candidate):** shown with what the state knows about them, keeping "average" and "unknown" visibly distinct (*Characters, Organisations & Events*).

## 5. Art scope (direction)

The vector layer is cheap to produce. Photoreal work is needed for generated body types and surfaces, stations, gates, ruins and ship components. The administration view needs portraits and 2D interface art. This keeps art cost in line with the scope risk (design document, section 7).

## 6. Open questions

- How gate distance appears on screen once placement is settled: true scale, compressed, or an optional overview scaling.
- Which limits on knowledge to model abstractly, and how; how sensor coverage is shown.
- Whether the inspection view is in place or a panel; how much survey detail each fidelity level needs.
- Faction colours beyond the affiliation colours in `symbology.md`.
- Zoom: step factor, animation timing, strength of the resistance at stops, the minimum view radius, and the standard tilt.
- Camera: the final tilt limit and pan and rotate speeds after testing; the tilt range for the cluster view.
- Motion rule: final values for R_show, `tail_secs` and `dot_fade_from_degrees` after testing; whether the label should glide from the dot to its fixed spot on the ring as the tail closes, rather than jump.
- Planetary view as a globe or a flat map, and how much surface detail is modelled.
- How the administration view is organised, and how the player moves between it and the spatial layers: candidate in `ui-style.md`.

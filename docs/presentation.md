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
- **Scale (direction):** positions and orbits are drawn at true scale; markers are drawn at a fixed screen size by class, never at physical size. Only the marker is stylised, never the geometry.
  - Marker shape and weight encode type: planet, moon, station, gate, ship, salvage site.
  - Navigation by stepped zoom with stops, focus jumps, and bookmarks (see *Navigation and zoom* below); edge-of-screen indicators for important off-screen objects (candidate).
  - How gate distance is handled on screen is deliberately left open until gate placement is tuned (section 6).
- **Inspection view (decided):** highlighting any object expands it into a semi-photorealistic view. This applies to everything: bodies, stations, gates, sites, and ships. Ships appear as blips because of the distances involved, not because they are kept abstract.
  - **Presentation (candidate):** the view expands in place as an inset anchored to the marker, with the vector display still visible around it. Insets can be pinned to compare objects side by side.
  - **Fidelity follows knowledge (candidate):** the view shows the best image the state holds. An unsurveyed body is a grainy telescope frame; a surveyed one is a clean render with overlays (ice, ruins, settlements); an old observation shows the last image with its date. Own ships and stations render fully; foreign contacts only as well as they have been observed. Ties the visuals to the uncertainty pillar and to partly known salvage sites (design document, 4.5).
  - **Renders from designs (candidate):** ship and station renders are assembled from their component designs, so two bids for the same requirements look visibly different, and as-built variants and refits show.
- **Information model (decided):** the player has a semi-omniscient view. Signal propagation delay is not modelled: what the state knows, the player sees at once, in every system.
- **Abstract uncertainty (candidate):** limits on knowledge are modelled in abstract ways rather than through physics:
  - Undetected contacts are not shown. Thermal detection (see *Component Catalogue*, sensors) decides what appears.
  - Tracked contacts carry an uncertainty region reflecting tracking quality: sensor coverage, the contact's emission control, time since a good detection.
  - Salvage contents, foreign designs, and unsurveyed bodies are known only as well as they have been observed (see the inspection view above).
- **Clutter (candidate):**
  - Moons, stations, and ships in orbit around a body fold into one count marker on that body (see *Satellites* below).
  - Asteroid belts and debris fields appear as density shading; surveyed, claimed, or active sites are promoted to individual markers.
  - Information layers can be switched on and off: ownership, resources, salvage, traffic, sensor coverage.
- **Motion (candidate):** ship paths are drawn as their actual arcs, including burns. Transfer planning shows a fan of alternative paths, each labelled with transit time and delta-v (design document, 4.2). How fast-orbiting objects are drawn is set by the motion rule (see *Satellites* below).
- **Cluster view (decided):** same style as the system view: systems as nodes, gates as links. Gate throughput, reliability, and ownership as line weight, line style, and colour (candidate).

### Navigation and zoom (direction)

Applies to the system view; the cluster view uses the same rules where they fit. Numbers marked *tunable* are starting values.

- **Logarithmic steps.** Each scroll notch multiplies the scale by a fixed factor (tunable: ×2), animated over 150–250 ms with an ease-out. Never scale linearly or follow raw wheel deltas. New input during an animation retargets it from the current scale rather than queuing.
- **Zoom anchor.** Zoom towards the cursor; when an object is selected, zoom towards the selected object instead.
- **Stops.** Zoom resists slightly at stops that frame something meaningful, then settles exactly on them:
  - **System frame:** all planetary orbits fit on screen. Gates stay as edge-of-screen indicators until gate distance is settled.
  - **Region frame:** the inner system, or the area around the selected planet's orbit.
  - **Satellite frame:** the selected planet's satellites fit on screen. The fan-out magnifier hands over to the real view here (see *Satellites*).
  - Stops are computed per system from its actual orbits, not fixed scales.
- **Trackpad pinch** stays continuous, but on release it settles to the nearest stop if within one zoom step of it (tunable).
- **Double-click (direction):** the first double-click on a body inspects it and frames its satellite system in one animated move (pan and zoom together, about 400 ms); the focus key does the same. A double-click on a body that is already inspected opens its planetary view, when it has one. The Home key returns to the system frame. Bookmarks are saved stops (scale and centre or followed object).
- **Hard ends.** Zoom stops at the system frame (outer limit) and the satellite frame of the selected body, or of the body nearest the cursor when nothing is selected (inner limit). Changing layer, to the cluster view or a planetary view, is always deliberate: breadcrumb, double-click on an already-inspected body, or the inset button. Scrolling past an end does not change layer.
- **Interaction with grouping.** Satellites fold and unfold during the zoom animation, so they slide in or out of their count marker as the view arrives at a stop.

### Satellites: grouping, fan-out, and motion (direction)

Written as implementation rules. Numbers marked *tunable* are starting values for the proof of concept.

**Terms**

- A **satellite** is anything orbiting a body other than the star: moons, stations, and ships parked in orbit. Its **parent** is the body it orbits. Moons of moons follow the same rules recursively.
- Ships in transit between bodies are **not** satellites and are never folded.
- The **apparent period** of an orbit is its orbital period divided by the current time rate. A 1.8-day moon at 1 day/s has an apparent period of 1.8 real seconds. When paused, the apparent period is infinite.

**Grouping (direction)**

- A satellite is drawn individually only when its orbit's radius on screen is at least *R_show* (tunable: 24 px). Otherwise it is **folded** into its parent. Its marker, label, orbit ring, and trail are not drawn.
- The test is per satellite and re-evaluated as the zoom changes, so outer moons can unfold before inner ones. Use hysteresis to stop flicker: fold below 20 px, unfold at 24 px or more (tunable). Unfolding animates the satellite out from its parent over about 200 ms.
- **At the default whole-system zoom, moons and planetary stations are folded.** The whole-system view shows the star, planets, belts, gates, sites, and ships in transit, with one count marker per planet. Drawing every moon individually at this zoom is wrong.
- **Count marker:** a rounded pill reading "+N" (N = folded satellites), placed beside the parent marker without overlapping it.
  - If any folded satellite is a foreign contact, the pill gets an orange outline; if own ships are parked there, a small blue dot.
  - Hovering the pill lists the folded satellites. Clicking it does the same as clicking the parent.
- Grouping is only between a parent and its satellites. Separate planets are never merged, even when close on screen; label placement handles overlaps.

**Fan-out (direction)**

- **Opens** when the player selects a body that has folded satellites, by clicking its marker or its count pill. The inspection inset opens with it; both are one selection.
- **Closes** when the selection changes, on Esc, or when the player zooms in far enough that the satellites unfold normally. In that last case the fan-out hands over to the normal view without a jump.
- **Form:** a circular magnifier centred on the parent, with a dashed frame, radius about 80 px (tunable), drawn over the dimmed map. A small label shows the magnification (for example "×40"). The magnifier follows the parent as it orbits.
- **Geometry inside is true:** true orbital phase and true relative radii, magnified so the outermost included orbit fills the frame. Radii are not compressed.
- **Contents:** all stations and parked ships, and moons above a size threshold (tunable). If true scale would put the innermost orbit closer than 12 px to the centre, leave out the outermost small moons and list them as "+N outer" on the frame.
- Satellites in the magnifier are interactive: hover shows a tooltip; click inspects that satellite (the inset switches to it); double-click follows the double-click rule in *Navigation and zoom*.
- The motion rule below applies inside the magnifier as it does everywhere else.

**Motion rule (direction)**

- **Apparent period at or above *T_dot* (tunable: 25 s):** draw the satellite as a dot at its true position with a short fading trail (about 30° of arc).
- **Apparent period below *T_ring* (tunable: 20 s):** **exposure-ring mode.** No dot. The orbit ring is drawn brighter and slightly thicker, and the satellite's label sits on the ring at a fixed spot. Hovering or clicking the ring interacts with the satellite.
- Between the two thresholds, keep the current mode (hysteresis). Switching cross-fades over about 200 ms.
- **Paused:** the apparent period is infinite, so everything is drawn as a dot at its true position. No special case is needed; slowing time for combat or a close approach brings dots back the same way.
- Applies to everything that orbits: moons, stations, parked ships, close-in planets, and the orbital layer of the planetary view. Ships in transit are not affected.
- A trajectory whose target is in exposure-ring mode ends on the ring; with a dot, it ends at the predicted arrival position.
- A selected satellite in exposure-ring mode keeps its ring highlighted; the inset shows its position as "shown when paused".

## 3. Planetary View

- **Purpose (decided):** where settlements are administered and ground combat takes place.
- **Entry (direction):** reached from the system view by the inset's "Open planetary view" button or by double-clicking an already-inspected body; never by scrolling. The inspection view is a look at a body, the planetary view is where the player acts on it.
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
- Colour palette and faction colours.
- Zoom: step factor, animation timing, how strong the resistance at stops is, and whether a region frame is needed in small systems.
- Motion rule: switch abruptly between dot and exposure ring, or fade gradually across a band of speeds; final values for R_show, T_dot, T_ring, and the magnifier size after testing.
- Planetary view as a globe or flat map, and how much surface detail is modelled.
- How the administration view is organised (screens, panels, an inbox of reports), and how the player moves between it and the spatial layers.

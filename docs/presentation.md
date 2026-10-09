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
  - Navigation by smooth zoom and focus bookmarks; edge-of-screen indicators for important off-screen objects (candidate).
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
  - Moons and stations around a planet group into one marker with a count ("Jovian + 6"), expanding on zoom.
  - Asteroid belts and debris fields appear as density shading; surveyed, claimed, or active sites are promoted to individual markers.
  - Information layers can be switched on and off: ownership, resources, salvage, traffic, sensor coverage.
- **Motion (candidate):** at 1 day per second inner moons visibly orbit while outer planets barely move, which teaches orbital mechanics without explanation. Fast bodies get fading trails rather than jumping dots. Ship paths are drawn as their actual arcs, including burns. Transfer planning shows a fan of alternative paths, each labelled with transit time and delta-v (design document, 4.2).
- **Cluster view (decided):** same style as the system view: systems as nodes, gates as links. Gate throughput, reliability, and ownership as line weight, line style, and colour (candidate).

## 3. Planetary View

- **Purpose (decided):** where settlements are administered and ground combat takes place.
- **Entry (candidate):** reached from the system view by selecting or zooming into a body; the inspection view is a look at a body, the planetary view is where the player acts on it.
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
- Planetary view as a globe or flat map, and how much surface detail is modelled.
- How the administration view is organised (screens, panels, an inbox of reports), and how the player moves between it and the spatial layers.

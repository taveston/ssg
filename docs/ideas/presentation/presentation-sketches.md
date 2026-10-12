# Orphan Worlds: Reference Sketches (Inspect & Planetary Transition)

**In short:** a description in words of the six boards on the design canvas [Inspect & Planetary Transition](https://claude.ai/artifact/E8U8j7d2HdB5gGd1S1W5WR), so it can be used as context without opening it. The boards cover the system view, inspecting in place, the transition to the planetary view, the planetary view, inspection fidelity, and satellite grouping with the motion rule. PNG exports are in `mockups/`.

Everything shown is **candidate** unless *Presentation & Visual Style* marks it otherwise. The parts that have been built are specced in `docs/spec/system-view.md`; where a sketch and the spec disagree, the spec wins. Two things the sketches show have since been rejected: the fan-out bubble and magnifier (boards 2 and 6), replaced by focus and the inset's "In orbit" list; and the anchored, pinnable inset (boards 1 and 2), replaced by one inset docked at the right of the map.

---

## Shared visual language

- **Ground:** near-black (#070A0E). Panels are slightly lighter and translucent (rgba(12,17,23,0.97)), with thin pale-blue borders.
- **Colour roles:**
  - off-white (#DCE6EC): neutral bodies and text
  - pale grey-blue (#9FB6C4): orbits and secondary text
  - **blue (#5AA9E6): own assets**
  - **orange (#F0A040): foreign contacts and warnings**
  - white ring: selection or hover

  Blue and orange were chosen so own and foreign stay distinct for colour-blind players.
- **Type:** Barlow for labels and UI; JetBrains Mono for dates, figures and status readouts.
- **Markers:** filled disc = body (fixed size by class); rounded pill with a count ("+5") = grouped moons and stations; hollow square = station; filled triangle = own ship or convoy; filled diamond = foreign contact; X = salvage site; dashed ellipse = uncertainty region.
- **Planet images:** placeholder gradients standing in for real renders.

## Example content used

- **System:** Corvane (K2V star). Bodies:
  - Pell, an inner planet with a fading trail
  - Iver, with the dormant Iver Yard station
  - an inner asteroid belt (density shading) with an unsurveyed wreck field
  - Halden IV, the capital world, with 4 moons (IVa–IVd) and Arden Highport station
  - Morrow, an outer giant (+11 grouped)
  - a gate to Ostra, off-screen
- **Traffic:** own Convoy C-12 on a transfer arc from Iver to Halden IV, arriving in 19 days. Foreign Contact K-3 near Halden IV, with a weak track and an uncertainty ellipse.
- **Halden IV:** settlements Port Arden (capital, 141k), Kessel Deep (58k) and Tarn outpost; a polar ice deposit; an ice quarry; an ore pit run by Varga Mining; the 1st Arden Guard ground formation; a shuttle field; Governor Anneke Voss.
- **Date shown:** 14 Mar 2341, running at 1 day/s.

## Board 1: System view (hover)

- A full-screen vector display at true scale: orbits as faint circles, faint range rings, the belt as a shaded band, the star with a glow and label.
- **Top bar:** breadcrumb (Cluster / Corvane system); date with pause, 1 day/s (active) and 5 d/s; layer toggles (Ownership active, Resources, Salvage, Sensors); an Administration button that switches to the 2D administration view.
- **Legend** top left, explaining marker shapes. **Alerts** top right: an orange-bordered "New contact near Halden IV" and a neutral "Bid received: survey tender".
- **Off-screen gate:** edge-of-screen arrow labelled "Gate → Ostra · off-screen · 41 d from Halden".
- **No moons are drawn individually at this zoom.** Every planet's moons, stations and parked ships are folded into its count marker (Halden IV "+5", Morrow "+11"), following the grouping rule.
- **Hover on Halden IV:** a white ring on the marker, the "+5" group pill, and a tooltip: "Halden IV · own · 4 moons, 1 station — Click: inspect · Double-click: frame its moons · Double-click again: planetary view".
- 1 AU scale bar bottom-left.

## Board 2: Inspect in place, with a pinned comparison

- Clicking Halden IV **dims the rest of the map** (Halden's own orbit stays bright) and **fans out its moons and station** in a small local bubble around the marker, each on its own labelled orbit ring. The clock keeps running. *Rejected:* this bubble became the fan-out magnifier, which was dropped; the inset's "In orbit" list replaces it.
- **Inspection inset**, anchored to the marker by a short leader line, to its right:
  - Header: name, "Cold terrestrial · own · capital world", Pin and Close buttons.
  - Image: lit globe render with surface overlays (settlement squares, labelled Port Arden; an extraction triangle) and a status line "SURVEYED · imaged 02 Feb 2341".
  - Key facts: population 212k, 3 settlements, surface (0.82 g, thin N₂, −41 °C), governor A. Voss.
  - "In orbit" chips: Arden Highport, IVa, IVb, IVc (pinned), IVd.
  - Actions: **Open planetary view** (primary) and Records.
- **Pinned inset** bottom left for Halden IVc, joined to its marker by a dashed line: a grainy, blurred telescope frame labelled "TELESCOPE FRAME · unsurveyed"; water ice "likely · 2–9 Gt"; surface sites "unknown"; a "Commission survey…" button. It shows how pinning lets players compare two objects side by side.

## Board 3: Transition storyboard (inspect → planetary view)

About 600 ms. The clock keeps running; Esc or the breadcrumb plays it in reverse.

- **A · 0 ms, Trigger:** "Open planetary view" in the inset, or double-clicking a body that is already inspected (now: the focused body, ZOOM-17). Scrolling never switches view.
- **B · 200 ms, Detach:** the inset frame and text fade. The planet image lifts out of the inset and slides to the centre, while the vector map zooms on Halden and fades back.
- **C · 400 ms, Anchor:** the camera tilts. The moon and station orbits persist as rings around the globe, so the player keeps their bearings. Surface overlays begin to draw on.
- **D · 600 ms, Settle:** the globe moves left, and the settlement panel and layer toggles slide in. The breadcrumb gains "Halden IV". The last-selected settlement opens by default.

Principle: the image itself carries the player through the transition, rather than a cut to a separate screen.

## Board 4: Planetary view

- **Globe** (photoreal render) filling the left two-thirds, with a faint graticule and **vector overlays:**
  - Settlements as blue-outlined squares (Port Arden selected with a white ring; Kessel Deep; Tarn outpost hollow).
  - Polar ice deposit as a dashed blue region labelled "Polar ice · surveyed".
  - Extraction sites as triangles (Ice quarry; Ore pit · Varga Mining).
  - Dotted supply routes between settlements and sites.
  - Own ground formation as a NATO-style infantry box (1st Arden Guard).
  - Shuttle field as a crosshair circle.
- **Orbital layer:** tilted orbit rings drawn behind and in front of the globe, with Arden Highport station and moons IVa and IVb on their rings, and an edge cue for the inbound convoy ("Convoy C-12 · arrives in 19 d").
- **Top bar:** back button and breadcrumb (Cluster / Corvane system / Halden IV); date and speed; Administration button.
- **Body switcher chips** top left: Halden IV (active), Arden Highport, IVa, IVb, "IVc · inspect only". Bodies without a full planetary view open the inspection view instead.
- **Layer toggles** bottom left: Settlements, Resources, Military, Ownership, Orbital.
- **Settlement panel** on the right (Port Arden):
  - "Capital · pressurised habitat · state-owned"; population 141k; stability Steady; local ideology Paternalist.
  - Governor card: portrait placeholder, name, known traits (Cautious, Principled) plus a dashed "? not yet observed" trait, skills as ranges ("Administration 6–8 · Industry ~4"), and a link to the dossier.
  - Monthly flows: water ice +3.2 kt (quarry), metals +410 t (Varga Mining), components −18 units in orange (import shortfall).
  - Request card (orange border): Voss asks for funding for a second habitat dome, with actions "Review in Administration" and "Later". Requests link into the administration view rather than being resolved here.

## Board 5: Inspection fidelity follows knowledge

The same inset in four states of knowledge:

1. **Unsurveyed** (Halden IVc): a blurred, grainy telescope frame with corner brackets; "TELESCOPE FRAME · UNSURVEYED" in orange; estimates as ranges; overlays withheld.
2. **Surveyed** (Halden IV): a clean render with settlement and ice overlays; "SURVEYED · 02 FEB 2341" in blue; exact figures.
3. **Stale** (Station Wreck "Lantern", a salvage site): a desaturated, sepia-tinted image with the date stamp "12 JUN 2338"; "LAST IMAGED · 2 Y 9 M AGO"; recoverable "then: 4–6 kt", since then "unknown".
4. **Foreign contact** (Contact K-3): a coarse, pixelated thermal blob; "THERMAL TRACK ONLY"; class "medium hauler?"; low confidence. Ships are imaged only as well as they have been observed.

## Board 6: Satellite grouping and motion rule

Drawn because the first prototype showed every moon at whole-system zoom, orbiting too fast to read.

- **Top row, grouping by zoom** (three panels):
  1. Whole-system zoom: Halden IV with a "+5" pill. The pill has an orange outline because a foreign contact is among the folded satellites, and a blue dot because own ships are parked there. No moons or moon orbits are drawn.
  2. Mid zoom: the outer moons IVc and IVd have passed 24 px and unfold on their rings. IVa, IVb and the station stay folded as "+3".
  3. Close zoom: everything is unfolded on true orbits, including the foreign contact K-3.
- **Bottom row, the motion rule**, shown inside a magnifier labelled "×40" with Halden IV selected. The magnifier is rejected; the motion rule stands.
  - Running at 1 day/s: Arden Highport (apparent period 0.1 s), IVa (1.5 s) and IVb (4.2 s) are bright exposure rings with labels on the ring. IVc (31 s) and IVd (68 s) are dots with short trails. A table lists each object, its apparent period and how it is drawn.
  - Paused: every satellite is a dot at its true position on faint rings. A convoy's transfer line ends at IVc's predicted arrival point.

## UX choices proposed by the sketches (candidate)

- Hover gives a tooltip, and click inspects in place. A double-click frames the body's satellites; a second double-click enters the planetary view. (Now decided by focus rather than inspection: ZOOM-16, ZOOM-17.)
- The inspection inset is anchored by a leader line and can be pinned for side-by-side comparison.
- The transition reuses the inset image and keeps moon orbits as anchoring rings. It is reversible and never pauses the clock.
- Bodies without a full planetary view still appear in the body switcher but open the inspection view.
- Requests surfaced in spatial views route the player to the administration view for the decision.

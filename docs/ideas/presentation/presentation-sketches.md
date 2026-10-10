# Orphan Worlds: Reference Sketches (Inspect & Planetary Transition)

*Status:* Reference sketches, 2026-10-09. Everything shown is **candidate** unless *Presentation & Visual Style* marks it otherwise. Canvas: [Inspect & Planetary Transition](https://claude.ai/artifact/E8U8j7d2HdB5gGd1S1W5WR). This document describes the canvas in words so it can be used as context without opening it. The parts that have been built are specced in `docs/spec/system-view.md`; where a sketch and the spec disagree, the spec wins. PNG exports are in `mockups/`.

---

## Shared visual language

- **Ground:** near-black (#070A0E), panels slightly lighter and translucent (rgba(12,17,23,0.97)) with thin pale-blue borders.
- **Colour roles:** off-white (#DCE6EC) for neutral bodies and text; pale grey-blue (#9FB6C4) for orbits and secondary text; **blue (#5AA9E6) = own assets**; **orange (#F0A040) = foreign contacts and warnings**. Blue/orange chosen so own vs. foreign stays distinct for colour-blind players. White ring = selection or hover.
- **Type:** Barlow for labels and UI, JetBrains Mono for dates, figures, and status readouts.
- **Markers:** filled disc = body (fixed size by class); rounded pill with a count ("+5") = grouped moons and stations; hollow square = station; filled triangle = own ship or convoy; filled diamond = foreign contact; X = salvage site; dashed ellipse = uncertainty region.
- **Planet images:** placeholder gradients standing in for real renders.

## Example content used

- **System:** Corvane (K2V star). Bodies: Pell (inner planet with fading trail), Iver (with dormant Iver Yard station), inner asteroid belt (density shading) with an unsurveyed wreck field, Halden IV (capital world, 4 moons IVa–IVd plus Arden Highport station), Morrow (outer giant, +11 grouped). Gate to Ostra off-screen.
- **Traffic:** own Convoy C-12 on a transfer arc from Iver to Halden IV, arriving in 19 days; foreign Contact K-3 near Halden IV with a weak track and an uncertainty ellipse.
- **Halden IV:** settlements Port Arden (capital, 141k), Kessel Deep (58k), Tarn outpost; polar ice deposit; ice quarry; ore pit run by Varga Mining; 1st Arden Guard ground formation; shuttle field; Governor Anneke Voss.
- **Date shown:** 14 Mar 2341, running at 1 day/s.

## Board 1: System view (hover)

- Full-screen vector display at true scale: orbits as faint circles, faint range rings, belt as a shaded band, star with glow and label.
- **Top bar:** breadcrumb (Cluster / Corvane system); date with pause, 1 day/s (active), 5 d/s; layer toggles (Ownership active, Resources, Salvage, Sensors); an Administration button that switches to the 2D administration view.
- **Legend** top-left explaining marker shapes. **Alerts** top-right (orange-bordered "New contact near Halden IV"; neutral "Bid received: survey tender").
- **Off-screen gate:** edge-of-screen arrow labelled "Gate → Ostra · off-screen · 41 d from Halden".
- **No moons are drawn individually at this zoom.** Every planet's moons, stations and parked ships are folded into its count marker (Halden IV "+5", Morrow "+11"), following the grouping rule in *Presentation & Visual Style* (Satellites).
- **Hover on Halden IV:** white ring on the marker, "+5" group pill, and a tooltip: "Halden IV · own · 4 moons, 1 station — Click: inspect · Double-click: frame its moons · Double-click again: planetary view".
- 1 AU scale bar bottom-left.

## Board 2: Inspect in place, with a pinned comparison

- Clicking Halden IV **dims the rest of the map** (Halden's own orbit stays bright) and **fans out its moons and station** in a small local bubble around the marker, each on its own orbit ring and labelled (IVa–IVd, station square). The clock keeps running. The bubble became the fan-out magnifier, later rejected (2026-10-10, see *Presentation & Visual Style*, Camera); the inset's "In orbit" list replaces it.
- **Inspection inset** anchored to the marker by a short leader line, to its right:
  - Header: name, "Cold terrestrial · own · capital world", Pin and Close buttons.
  - Image: lit globe render with surface overlays (settlement squares, labelled Port Arden; an extraction triangle) and a status line "SURVEYED · imaged 02 Feb 2341".
  - Key facts: population 212k, 3 settlements, surface (0.82 g, thin N₂, −41 °C), governor A. Voss.
  - "In orbit" chips: Arden Highport, IVa, IVb, IVc (pinned), IVd.
  - Actions: **Open planetary view** (primary) and Records.
- **Pinned inset** bottom-left for Halden IVc, joined to its marker by a dashed line: grainy, blurred telescope frame labelled "TELESCOPE FRAME · unsurveyed"; water ice "likely · 2–9 Gt"; surface sites "unknown"; button "Commission survey…". Shows how pinning lets players compare two objects side by side.

## Board 3: Transition storyboard (inspect → planetary view)

About 600 ms; the clock keeps running; Esc or the breadcrumb plays it in reverse.

- **A · 0 ms, Trigger:** "Open planetary view" in the inset, or double-clicking a body that is already inspected. Scrolling never switches view.
- **B · 200 ms, Detach:** the inset frame and text fade; the planet image lifts out of the inset and slides to the centre while the vector map zooms on Halden and fades back.
- **C · 400 ms, Anchor:** the camera tilts; the moon and station orbits from the fan-out persist as rings around the globe, so the player keeps their bearings; surface overlays begin to draw on.
- **D · 600 ms, Settle:** the globe moves left; the settlement panel and layer toggles slide in; the breadcrumb gains "Halden IV"; the last-selected settlement opens by default.

Principle: the image itself carries the player through the transition, rather than a cut to a separate screen.

## Board 4: Planetary view

- **Globe** (photoreal render) occupying the left two-thirds, with a faint graticule and **vector overlays:**
  - Settlements as blue-outlined squares (Port Arden selected with a white ring; Kessel Deep; Tarn outpost hollow).
  - Polar ice deposit as a dashed blue region labelled "Polar ice · surveyed".
  - Extraction sites as triangles (Ice quarry; Ore pit · Varga Mining).
  - Dotted supply routes between settlements and sites.
  - Own ground formation as a NATO-style infantry box (1st Arden Guard).
  - Shuttle field as a crosshair circle.
- **Orbital layer:** tilted orbit rings drawn behind and in front of the globe; Arden Highport station, moons IVa and IVb on their rings; an edge cue for the inbound convoy ("Convoy C-12 · arrives in 19 d").
- **Top bar:** back button and breadcrumb (Cluster / Corvane system / Halden IV); date and speed; Administration button.
- **Body switcher chips** top-left: Halden IV (active), Arden Highport, IVa, IVb, "IVc · inspect only" (bodies without a full planetary view open the inspection view instead).
- **Layer toggles** bottom-left: Settlements, Resources, Military, Ownership, Orbital.
- **Settlement panel** on the right (Port Arden):
  - "Capital · pressurised habitat · state-owned"; population 141k; stability Steady; local ideology Paternalist.
  - Governor card: portrait placeholder, name, known traits (Cautious, Principled) plus a dashed "? not yet observed" trait; skills as ranges ("Administration 6–8 · Industry ~4"); link to dossier.
  - Monthly flows: water ice +3.2 kt (quarry), metals +410 t (Varga Mining), components −18 units in orange (import shortfall).
  - Request card (orange border): Voss asks for funding for a second habitat dome; actions "Review in Administration" and "Later". Requests link into the administration view rather than being resolved here.

## Board 5: Inspection fidelity follows knowledge

The same inset in four states of knowledge:

1. **Unsurveyed** (Halden IVc): blurred, grainy telescope frame with corner brackets; label "TELESCOPE FRAME · UNSURVEYED" in orange; estimates as ranges; overlays withheld.
2. **Surveyed** (Halden IV): clean render with settlement and ice overlays; "SURVEYED · 02 FEB 2341" in blue; exact figures.
3. **Stale** (Station Wreck "Lantern", salvage site): desaturated, sepia-tinted image with a date stamp "12 JUN 2338"; "LAST IMAGED · 2 Y 9 M AGO"; recoverable "then: 4–6 kt", since then "unknown".
4. **Foreign contact** (Contact K-3): coarse, pixelated thermal blob; "THERMAL TRACK ONLY"; class "medium hauler?"; confidence low. Ships are imaged only as well as they have been observed.

## Board 6: Satellite grouping and motion rule

Added after the proof of concept showed moons drawn individually at whole-system zoom, orbiting too fast to read.

- **Top row, grouping by zoom** (three panels):
  1. Whole-system zoom: Halden IV with a "+5" pill; orange pill outline because a foreign contact is among the folded satellites, blue dot because own ships are parked there. No moons or moon orbits drawn.
  2. Mid zoom: the outer moons IVc and IVd have passed 24 px and unfold on their rings; IVa, IVb and the station remain folded as "+3".
  3. Close zoom: everything unfolded on true orbits, including the foreign contact K-3 in orbit; no magnifier needed.
- **Bottom row, motion rule inside the magnifier** (magnifier since rejected; the motion rule itself stands) (Halden IV selected, magnifier labelled "×40"):
  - Running at 1 day/s: Arden Highport (apparent period 0.1 s), IVa (1.5 s) and IVb (4.2 s) drawn as bright exposure rings with labels on the ring; IVc (31 s) and IVd (68 s) as dots with short trails. A table lists object, apparent period, and how it is drawn.
  - Paused: every satellite is a dot at its true position on faint rings; a convoy's transfer line ends at IVc's predicted arrival point.

## UX choices proposed by the sketches (candidate)

- Hover gives a tooltip; click inspects in place; the first double-click frames the body's satellites; a double-click on an already-inspected body enters the planetary view (direction).
- Inspecting fans out a planet's grouped moons and stations locally rather than requiring a zoom (now specified as direction in *Presentation & Visual Style*, together with grouping and the motion rule).
- The inspection inset is anchored by a leader line and can be pinned for side-by-side comparison.
- The transition reuses the inset image and keeps moon orbits as anchoring rings; it is reversible and never pauses the clock.
- Bodies without a full planetary view still appear in the body switcher but open the inspection view.
- Requests surfaced in spatial views route the player to the administration view for the decision.

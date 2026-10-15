# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-146.

---

### F-140 · What "on-screen radius" means for a ring the camera is near
- **Observation:** MRK-14 fades a ring by its on-screen radius, but a ring seen in perspective has no single radius. Measured at the ring's centre, Halden IV's own orbit in its frame comes out about 1,100 px and would stay fully drawn, though it crosses the view as a near-straight line. The build measures the radius at the scale of the ring's point nearest the camera, so a ring the camera sits close to counts as huge and fades out. Seen from far off, both measures agree.
- **Affects:** MRK-14 · **Type:** spec gap
- **Disposition:** open. Accept, or say how the radius is measured.

### F-141 · The parent tab is a tab while gate arrows are not yet
- **Observation:** MRK-15 styles the parent's tab "as EDGE-01 to EDGE-03". EDGE-02 and EDGE-03 aren't built, so gates still show a bare arrow and a label. The build gives the parent tab the EDGE-03 look now (ground fill, neutral outline `button_stroke_px` wide, kept whole inside the map area), so for now the parent and the gates look different. The "Edge arrows as tabs" item will bring the gates into line.
- **Affects:** MRK-15, EDGE-02, EDGE-03 · **Type:** spec gap
- **Disposition:** open. Accept for now; recheck when the edge-tab item is built.

### F-142 · Large orbits fade out too soon
- **Observation:** In play, the MRK-14 fade feels too aggressive: rings that still read as rings fade or vanish. The defaults are `orbit_fade_from` 1.5 and `orbit_fade_to` 3 times half the map area's diagonal. Note F-140 too: the build measures a ring's radius at its point nearest the camera, which makes rings near the camera count as larger than they look.
- **Affects:** MRK-14 · **Type:** spec wrong
- **Disposition:** open. Raise the two tunables, change how the radius is measured (F-140), or both.

### F-143 · The parent tab's chevron sits awkwardly inside the box
- **Observation:** MRK-15 puts the chevron inside the tab, beside the parent's marker. On screen it reads poorly. It should either sit outside the box, on its edge-facing side, or be merged with the marker (for example, the marker with a pointer on its rim towards the parent).
- **Affects:** MRK-15, EDGE-03 · **Type:** spec wrong
- **Disposition:** open. Choose the chevron outside the tab or merged with the marker; the gate tabs (EDGE-03) should follow the same choice.

### F-144 · Ships in one band are not in range of each other
- **Observation:** Raised in a combat design session. BAND-05 treats ships in one named orbit as co-located. They share a speed but not a place: a High band can be hundreds of thousands of km round, and the catalogue's longest sensor range is 50,000 km. Ships at different phases are mostly out of range of each other, so "same band, so engaged" is wrong for combat. Closing the gap is a phasing transfer: cheap in delta-v, slow in days. The spec has none. XFER-10 reaches a ship in the same band only as an ordinary arc, and XFER-03 allows zero revolutions, which a phasing arc round a ring can exceed. `transfers.md` sections 4.3 and 7 assume no such transfer is ever needed.
- **Affects:** BAND-05, BAND-06, XFER-01, XFER-03, XFER-10 · **Type:** spec gap
- **Disposition:** open. Decide before the "Transfer simulation" to-do item is built. Options: (a) narrow BAND-05 to selection, departure and grouping, let combat read phase, and add a phasing transfer to a ship or phase in the same band (multi-revolution arcs in XFER-03, or a simpler cost and days from the phase gap); (b) drop BAND-05 and make phase real for every rule. Lean: (a). How combat uses phase is for the conflict doc to settle.

### F-145 · An intercept needs a standoff distance
- **Observation:** Raised in a combat design session. XFER-10 ends a ship-as-destination crossing at the target's position on the crossing end day, so a Pass intercept is a collision course with zero miss distance. A real engagement has a chosen miss distance, and it sets how long each side is in weapon range and which weapons can act. The order has no way to carry one.
- **Affects:** XFER-08, XFER-09, XFER-10 · **Type:** spec gap
- **Disposition:** open. Option: a `standoff_km` on a Pass whose destination is a ship, so the crossing ends offset from the target's rail rather than on it. A Stop (rendezvous) keeps zero. What the standoff should default to, and how the resolver reads it, are for the conflict doc to settle.

# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-144.

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

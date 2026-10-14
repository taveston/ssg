# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-150.

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

### F-144 · Morrow's two innermost moons can't have a High band
- **Observation:** TEST-20 says every star, planet and moon in Corvane carries its two bands. BAND-01 caps High at `high_band_hill_fraction` of the Hill radius, and for a moon the Hill radius scales with its own radius: at 143,000 km from a Jupiter-mass giant, a rocky moon's Hill radius is about 1.9 times its radius, so half of it never clears Low (radius + 500 km). Morrow I and II have a Low band only, whatever plausible mass they are given. The moons now carry masses from their radii at rock density (3,000 kg/m³); every other moon gets both bands.
- **Affects:** TEST-20, BAND-01 · **Type:** spec wrong
- **Disposition:** open. Accept that some small moons have no High band (the build does this), or move Morrow I and II out.

### F-145 · Halden IV's High band makes every parked ship an exposure ring at 1 d/s
- **Observation:** TEST-20 makes Corvane's "High" orbit the BAND-01 band: 10 radii = 67,000 km, a period of about 1.9 d (it was 12 d). TEST-05 and TEST-10 put every parked ship there, and TEST-05 says they sit in "slow orbits (dot mode at 1 d/s)". At 1 d/s the band is now an exposure ring with a "+14" pill (MOT-14), and the symbols only show when paused or below 0.24 d/s. The build follows TEST-20; the headless identity-mark test now runs at 0.2 d/s.
- **Affects:** TEST-05, TEST-10, TEST-20, BAND-01 · **Type:** conflict
- **Disposition:** open. Raise `high_band_radii` (about 34 radii gives 12 d at Halden IV), accept the ring, or state that the parked ships sit elsewhere.

### F-146 · XFER-07 would forbid every trip to a Low band
- **Observation:** XFER-07 makes a crossing infeasible if its periapsis is inside `min_periapsis_radii` (2) × the primary's radius and is reached during the crossing. A Low band is at radius + 500 km, inside that limit for any body over 500 km, and a transfer between two bands has the body as its primary (XFER-01) and reaches periapsis at one end: High → Low arrives at periapsis, Low → High leaves from it. So no band-to-band trip touching Low could be flown. The build exempts a crossing that starts or ends in an orbit of the primary itself, and applies the rule to every other crossing (a fall towards the star from a planet, say).
- **Affects:** XFER-07, BAND-01 · **Type:** spec wrong
- **Disposition:** open. Accept the exemption, or lower the limit below the Low altitude.

### F-147 · Choices the transfer spec leaves open
- **Observation:** The build had to settle these: (a) the end state of a band of the crossing's primary (band-to-band trips, a station at the primary as the start) is the band's circle aimed half a turn ahead of the start, so a Hohmann travel time gives the Hohmann burns; (b) XFER-06's spiral spans the whole crossing, whose length includes the departure and capture costs, so the spiral winds at `|v0 − v1| ÷ travel_days` and lands at the end's angle on the last day; (c) a place whose body is the crossing's primary charges nothing (XFER-01's band-to-band rule, extended to a station or a gate round the primary), and a station, gate or site is aimed at by its own orbit's state, not its parent's; (d) a parked ship as a destination is aimed at as its orbit's body (BAND-05), and the closing speed of a Pass at a ship is relative to that body's velocity; (e) Hold on an InFlight plan leaves that plan on the ship as **Holding** with its rail cut, rather than replacing it with a Done Hold plan, so PLAN-12's origin and destination survive; (f) a ship's position in a named orbit is its orbit's at its own phase, and a captured ship's phase is set so its angle at the arrival day is its arrival direction (BAND-06).
- **Affects:** XFER-01, XFER-06, XFER-09, XFER-10, PLAN-13, BAND-06 · **Type:** spec gap
- **Disposition:** open. Copy the choices that are fine into the spec.

### F-148 · `double_click_window_ms` has no reader
- **Observation:** The tunable existed so a single click on the flat "In orbit" list could wait out the double-click window before the inset was rebuilt under the pointer. The outline (INS-05) stays in place, so a click selects at once and double-clicks come from the picking backend's click count. Nothing reads the tunable now; it is kept in `Tunables` only because the spec's table lists it.
- **Affects:** INS-05, tunables table · **Type:** spec gap
- **Disposition:** open. Drop the tunable from the table, and the field with it.

### F-149 · How a shared orbit's row is named in the outline
- **Observation:** INS-05 names the row "by the parent and the orbit (for example "Halden IV · orbit · 3")". The build reads the example as parent, orbit name, member count: "Halden IV · High · 14". The star's root row has no toggle text other than its glyph, and gates and salvage sites, which orbit the star, are listed under it after the planets by orbit radius.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** open. Confirm the format, or say what the example meant.

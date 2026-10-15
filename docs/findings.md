# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-158.

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

### F-145 · An intercept needs a standoff distance
- **Observation:** Raised in a combat design session. XFER-10 ends a ship-as-destination crossing at the target's position on the crossing end day, so a Pass intercept is a collision course with zero miss distance. A real engagement has a chosen miss distance, and it sets how long each side is in weapon range and which weapons can act. The order has no way to carry one. A Pass at a station is trimmed to meet it (PHAS-15), so it too has zero miss distance.
- **Affects:** XFER-08, XFER-09, XFER-10, PHAS-15 · **Type:** spec gap
- **Disposition:** open. Option: a `standoff_km` on a Pass whose destination is a ship, so the crossing ends offset from the target's rail rather than on it. A Stop (rendezvous) keeps zero. What the standoff should default to, and how the resolver reads it, are for the conflict doc to settle.

### F-146 · Morrow's two innermost moons can't have a High band
- **Observation:** TEST-20 says every star, planet and moon in Corvane carries its two bands. BAND-01 caps High at `high_band_hill_fraction` of the Hill radius, and for a moon the Hill radius scales with its own radius: at 143,000 km from a Jupiter-mass giant, a rocky moon's Hill radius is about 1.9 times its radius, so half of it never clears Low (radius + 500 km). Morrow I and II have a Low band only, whatever plausible mass they are given. The moons now carry masses from their radii at rock density (3,000 kg/m³); every other moon gets both bands.
- **Affects:** TEST-20, BAND-01 · **Type:** spec wrong
- **Disposition:** open. Accept that some small moons have no High band (the build does this), or move Morrow I and II out.

### F-147 · Halden IV's High band makes every parked ship an exposure ring at 1 d/s
- **Observation:** TEST-20 makes Corvane's "High" orbit the BAND-01 band: 10 radii = 67,000 km, a period of about 1.9 d (it was 12 d). TEST-05 and TEST-10 put every parked ship there, and TEST-05 says they sit in "slow orbits (dot mode at 1 d/s)". At 1 d/s the band is now an exposure ring with a "+14" pill (MOT-14), and the symbols only show when paused or below 0.24 d/s. The build follows TEST-20; the headless identity-mark test now runs at 0.2 d/s.
- **Affects:** TEST-05, TEST-10, TEST-20, BAND-01 · **Type:** conflict
- **Disposition:** open. Raise `high_band_radii` (about 34 radii gives 12 d at Halden IV), accept the ring, or state that the parked ships sit elsewhere.

### F-149 · Choices the transfer spec leaves open
- **Observation:** The build had to settle these: (a) the end state of a band of the crossing's primary (band-to-band trips, a station at the primary as the start) is the band's circle aimed half a turn ahead of the start, so a Hohmann travel time gives the Hohmann burns; (b) XFER-06's spiral spans the whole crossing, whose length includes the departure and capture costs, so the spiral winds at `|v0 − v1| ÷ travel_days` and lands at the end's angle on the last day; (c) a place whose body is the crossing's primary charges nothing (XFER-01's band-to-band rule, extended to a station or a gate round the primary), and a gate or site is aimed at by its own orbit's state, not its parent's; (e) Hold on an InFlight plan leaves that plan on the ship as **Holding** with its rail cut, rather than replacing it with a Done Hold plan, so PLAN-12's origin and destination survive.
- **Affects:** XFER-01, XFER-06, PLAN-13 · **Type:** spec gap
- **Disposition:** open. Copy the choices that are fine into the spec.

### F-150 · `double_click_window_ms` has no reader
- **Observation:** The tunable existed so a single click on the flat "In orbit" list could wait out the double-click window before the inset was rebuilt under the pointer. The outline (INS-05) stays in place, so a click selects at once and double-clicks come from the picking backend's click count. Nothing reads the tunable now; it is kept in `Tunables` only because the spec's table lists it.
- **Affects:** INS-05, tunables table · **Type:** spec gap
- **Disposition:** open. Drop the tunable from the table, and the field with it.

### F-151 · How a shared orbit's row is named in the outline
- **Observation:** INS-05 names the row "by the parent and the orbit (for example "Halden IV · orbit · 3")". The build reads the example as parent, orbit name, member count: "Halden IV · High · 14". The star's root row has no toggle text other than its glyph, and gates and salvage sites, which orbit the star, are listed under it after the planets by orbit radius.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** open. Confirm the format, or say what the example meant.

### F-153 · The shared-orbit row's name should be the orbit alone
- **Observation:** The outline names a shared orbit's row "Halden IV · High · 14" (F-151's reading of INS-05). In play the body name is redundant under the body's own entry, and "High" on its own doesn't say what it refers to. The row should read "High orbit" or "Low orbit" (perhaps with the count).
- **Affects:** INS-05 · **Type:** spec wrong
- **Disposition:** open. Decide the row's text; F-151 can be closed with it.

### F-154 · Orbit rows should appear even with one member, and perhaps when empty
- **Observation:** The outline shows a named orbit's row only when it holds two or more members; a lone member is listed as a plain entry, and an empty band (Halden IV's Low, every other body's bands) is not shown at all, so no "Low" row appears anywhere. In play the rows should be there with a single member too, and it is worth deciding whether an empty band shows as a row (which would list two bands under every star, planet and moon).
- **Affects:** INS-05, HOV-04 · **Type:** spec wrong
- **Disposition:** open. Decided in play: empty bands never show; a named orbit with one member should still get its row. (Arden Highport now sits in Halden IV's Low band, TEST-10, so that band is not empty.)

### F-156 · BAND-04 does not say what happens when a band lies outside the Hill sphere
- **Observation:** BAND-04 gives (Hill radius − *r*) ÷ escape speed, which goes negative when the orbit lies outside the Hill sphere. The build's `approach_days` returns 0 there, and also when the body has no Hill sphere or no gravity. The spec covers only "no massive body". In Corvane it matters: Morrow I's Low band (850 km) lies outside its Hill sphere (about 652 km at rock density 3,000 kg/m³).
- **Affects:** BAND-04, BAND-01 · **Type:** spec gap
- **Disposition:** open. Copy the clamp into BAND-04, or decide that a Low band outside the Hill sphere should not exist (which bears on F-146).

### F-157 · What `--system` does with other arguments
- **Observation:** SOL-22 says how `--system <name>` picks the system but not what happens to other arguments. The build ignores them, so `cargo run -- --verbose` still starts on Corvane. Only an unknown or missing name after `--system` stops the program. Names are matched exactly, so `--system Sol` is unknown.
- **Affects:** SOL-22 · **Type:** spec gap
- **Disposition:** open. Accept, or say whether unknown arguments and other capitalisations should be errors.

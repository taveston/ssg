# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-162.

---

### F-141 · The parent tab is a tab while gate arrows are not yet
- **Observation:** MRK-15 styles the parent's tab "as EDGE-01 to EDGE-03". EDGE-02 and EDGE-03 aren't built, so gates still show a bare arrow and a label. The build gives the parent tab the EDGE-03 look now (ground fill, neutral outline `button_stroke_px` wide, kept whole inside the map area), so for now the parent and the gates look different. The "Edge arrows as tabs" item will bring the gates into line.
- **Affects:** MRK-15, EDGE-02, EDGE-03 · **Type:** spec gap
- **Disposition:** open. Accept for now; recheck when the edge-tab item is built.

### F-145 · An intercept needs a standoff distance
- **Observation:** Raised in a combat design session. XFER-10 ends a ship-as-destination crossing at the target's position on the crossing end day, so an Intercept is a collision course with zero miss distance. A real engagement has a chosen miss distance, and it sets how long each side is in weapon range and which weapons can act. The order has no way to carry one. An Intercept at a station is trimmed to meet it (PHAS-15), so it too has zero miss distance.
- **Affects:** XFER-08, XFER-09, XFER-10, PHAS-15 · **Type:** spec gap
- **Disposition:** open. Option: a `standoff_km` on an Intercept whose destination is a ship, so the crossing ends offset from the target's rail rather than on it. A Rendezvous keeps zero. What the standoff should default to, and how the resolver reads it, are for the conflict doc to settle.

### F-147 · Halden IV's High band makes every parked ship an exposure ring at 1 d/s
- **Observation:** TEST-20 makes Corvane's "High" orbit the BAND-01 band: 10 radii = 67,000 km, a period of about 1.9 d (it was 12 d). TEST-05 and TEST-10 put every parked ship there, and TEST-05 says they sit in "slow orbits (dot mode at 1 d/s)". At 1 d/s the band is now an exposure ring with a "+14" pill (MOT-14), and the symbols only show when paused or below 0.24 d/s. The build follows TEST-20; the headless identity-mark test now runs at 0.2 d/s.
- **Affects:** TEST-05, TEST-10, TEST-20, BAND-01 · **Type:** conflict
- **Disposition:** open. Raise `high_band_radii` (about 34 radii gives 12 d at Halden IV), accept the ring, or state that the parked ships sit elsewhere.

### F-149 · Choices the transfer spec leaves open
- **Observation:** The build had to settle these: (a) the end state of a band of the crossing's primary (band-to-band trips, a station at the primary as the start) is the band's circle aimed half a turn ahead of the start, so a Hohmann travel time gives the Hohmann burns; (b) XFER-06's spiral spans the whole crossing, whose length includes the departure and capture costs, so the spiral winds at `|v0 − v1| ÷ travel_days` and lands at the end's angle on the last day; (c) a place whose body is the crossing's primary charges nothing (XFER-01's band-to-band rule, extended to a station or a gate round the primary), and a gate or site is aimed at by its own orbit's state, not its parent's; (e) Hold on an InFlight plan leaves that plan on the ship as **Holding** with its rail cut, rather than replacing it with a Done Hold plan, so PLAN-12's origin and destination survive.
- **Affects:** XFER-01, XFER-06, PLAN-13 · **Type:** spec gap
- **Disposition:** open. Copy the choices that are fine into the spec.

### F-156 · BAND-04 does not say what happens when a band lies outside the Hill sphere
- **Observation:** BAND-04 gives (Hill radius − *r*) ÷ escape speed, which goes negative when the orbit lies outside the Hill sphere. The build's `approach_days` returns 0 there, and also when the body has no Hill sphere or no gravity. The spec covers only "no massive body". In Corvane it matters: Morrow I's Low band (850 km) lies outside its Hill sphere (about 652 km at rock density 3,000 kg/m³).
- **Affects:** BAND-04, BAND-01 · **Type:** spec gap
- **Disposition:** open. Copy the clamp into BAND-04, or decide that a Low band outside the Hill sphere should not exist (which bears on F-146).

### F-158 · Departure and capture costs add up to far more than a real trip
- **Observation:** Raised while sizing Marlin for Sol (SOL-15). BAND-03 and XFER-04 charge a ship the cost of escaping its band to rest at infinity, then the burn that sets its speed round the primary, and the same again at the far end. A real burn deep in a gravity well does both at once and is much cheaper (the Oberth effect). From Earth's Low band to a Low band at the target, with a Hohmann crossing, the spec's cost against a patched-conic figure for the same bands: Venus 11.3 against 6.7 km/s, Mars 10.1 against 5.6, Ceres 14.4 against 9.5, Mercury 21.4 against 13.2. The spec is 1.5 to 1.8 times too high. The low-thrust crossing (XFER-06) adds the same escape cost, which for a spiral is too low: a spiral escape costs the full circular speed, not √2 − 1 of it.
- **Affects:** BAND-03, XFER-04, XFER-06, XFER-12, SOL-15 · **Type:** spec wrong
- **Disposition:** open. The model needs fixing, and the fix is a significant piece of work, so the spec and the build stay as they are for now (decided 2026-10-16). Option: for an impulsive drive, replace departure cost + burn 1 (and burn 2 + capture cost) with one burn from the band's circular orbit to a hyperbola with the crossing's excess speed, √(*v∞*² + 2μ/*r*) − √(μ/*r*). The excess speed is already computed for BAND-06, XFER-09 and PHAS-03. Open: how burn days (XFER-05) and approach days (BAND-04) fit, what the spiral should cost, and new figures for the XFER-12 worked checks. Marlin's delta-v would then be set again.

### F-159 · Groups are only loosely defined
- **Observation:** SYM-09 draws a group, but nothing says what selecting one shows. The inset (INS-02) has no form for a group. The outline (INS-05) lists a group by name and count (GRP-09), with no way to list its members or reach one by one, for their Delta-v and Drive (PLNR-05). "Commander" means a person in SYM-09 and in mock-up 7 ("Cdr Ilse Marr"), but "command" means the faction giving orders in SYM-03. How groups form, merge and split is not specced either: no order creates a group, and merge and detach orders are not specced (PHAS-14). So groups exist only as test content (TEST-06) for now. The planner already refuses them (PLNR-01, PLNR-11). The combat ideas (candidate) detach and rejoin ships and call every group a force, which would be the first source of groups in play.
- **Affects:** SYM-09, SYM-10, INS-02, INS-05, GRP-09, PLNR-01, PLNR-05 · **Type:** spec gap
- **Disposition:** open. Safe to defer: the player can't form groups yet. Specify the group inset, member access and the meaning of "commander" together with the orders that form and split groups.

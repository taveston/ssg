# Findings

Observations from building and playing the prototype, in two sections. **To triage** holds findings waiting for a decision at the next design session. **Parked** holds findings that can't be decided until a later feature is designed, each with a line saying what brings it back. Once a finding is settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings go under To triage and take the next free number from F-192.

---

## To triage

### F-162 · Which band Iver Yard sits in
- **Observation:** SCL-07 now puts every station in a band, so the build drops a station's own orbit. TEST-02 and TEST-09 name the band for Arden Highport and Tern Relay, but nothing names one for Iver Yard, which had its own 45,000 km orbit. The build puts it in Iver's High band (197,200 km at 34 radii, period about 10.6 d). That band was nearest its old orbit when High was 10 radii; now the Low band (6,300 km) is nearer. This moves Iver's satellite frame out to 197,200 km.
- **Affects:** SCL-07, TEST-01 · **Type:** spec gap
- **Disposition:** open. Accept, or name Iver Yard's band in TEST-01.

### F-163 · Vestige's burn time is longer than the orbit it leaves
- **Observation:** Vestige's first burn from Earth's Low band to its High band (SOL-17) is about 3.0 km/s at 20 milli-g, so XFER-05 gives it 4.2 h of burn days. Against the 52 h Hohmann crossing that is a small share. The rule keeps the ship Parked in Low for those 4.2 h, then applies the whole burn as one impulse at the crossing start, and solves the crossing again from that later start. So the burn only delays the trip: no thrust arc is drawn, no gravity loss is charged, and the time the ship would spend burning is spent waiting. Yet 4.2 h is about 2.7 laps of Low (94.5 min), so a real burn that long could not be treated as an impulse at one point. Marlin's burn (0.1 g, about 51 min) is about half a lap, which is borderline too.
- **Affects:** XFER-05, SOL-16, SOL-17 · **Type:** spec gap
- **Disposition:** open. Accept the delay as a stand-in, or say how a burn that lasts a large share of an orbit is modelled (bears on F-158).

### F-168 · Capture and closest approach inside the primary's frame
- **Observation:** BAND-06 and PHAS-11 read the point from the hyperbola a ship arrives on from outside a body's frame. A crossing round the body itself (band to band, or to a gate round the star) has no such hyperbola: its arc ends at a real place. The build lands a Rendezvous where the arc ends, and gives an Intercept there the angle of the arc's end as its closest-approach phase. The old rule (the angle of the arrival velocity) put the ship somewhere it never flew.
- **Affects:** BAND-06, PHAS-11 · **Type:** spec gap
- **Disposition:** open. Accept, and say so in BAND-06.

### F-169 · "Already there" is a reason to the planner and done at once to an order
- **Observation:** PHAS-04 says the plan is Done at once, and PLNR-42 and PLNR-92 make "Already there" a reason the solver gives. The build does both: the solver (`preview`) returns `AlreadyThere`, and `schedule` accepts the order and changes nothing, as Hold does for a parked ship. No Done plan is kept on the ship.
- **Affects:** PHAS-04, PLNR-42 · **Type:** spec gap
- **Disposition:** open. Accept, or say whether a Done plan should be recorded.

### F-170 · How a low-thrust Intercept is trimmed
- **Observation:** PHAS-15 trims an Intercept by lengthening `travel_days`, but a low-thrust drive's travel time is fixed by the drive (XFER-06). The build adds the trim to the spiral's length: same delta-v, gentler thrust, ending at the body a little later. Separately, the trim can fail on an impulsive leg when a small change of travel time moves the exit point past the ship, so the departure wait (PHAS-03) jumps by a period; the leg is then "Can't line up".
- **Affects:** PHAS-15, XFER-06, PHAS-03 · **Type:** spec gap
- **Disposition:** open. Accept, make a low-thrust Intercept at a station infeasible, or trim it by waiting before departure.

### F-171 · Which band an Intercept at a station passes through
- **Observation:** XFER-10 says an Intercept at a station or parked ship takes the target's band, but the order also carries `Intercept { band }`. The build uses the target's band and ignores the order's, so an Intercept at a station in a Low band (Arden Highport) passes through Low and is turned (XFER-13). Its speed (XFER-09) is taken at the target's band; for a station round the crossing's primary it is the plain relative speed (XFER-10).
- **Affects:** XFER-10, XFER-08, XFER-09 · **Type:** spec gap
- **Disposition:** open. Accept, or say what the order's band means for a station.

### F-172 · The phasing rule of thumb holds only near half a turn
- **Observation:** PHAS-13's rule of thumb (delta-v × meeting time within 3% of ⅔ × r × angle, for k ≥ 2) holds for the half-turn gap the check gives. For other gaps it can miss: at a gap of 0.8 of a turn, k = 2, inner orbit, it is 3.1% off. The test checks the half-turn case only.
- **Affects:** PHAS-13 · **Type:** spec wrong
- **Disposition:** open. Limit the rule to the half-turn case, or widen the tolerance.

### F-173 · Groups are together, but nothing forms them yet
- **Observation:** PHAS-14 needs the ships of a group to be together. A group is now one body with one orbit and phase, and its ships have no place of their own (F-176), so they are together by construction. Only content makes groups (TEST-06); nothing in play forms or merges one (F-159). `SystemTree::all_together` says whether separate ships could become one group (all parked in one orbit, every two together); whatever forms or merges groups should call it.
- **Affects:** PHAS-14 · **Type:** spec gap
- **Disposition:** open. Use the check when group orders are specced.

### F-174 · A pass through a moon's Low band from beyond its planet
- **Observation:** A crossing from another planet to a moon goes round the star and aims at the moon's planet (XFER-01). An Intercept through the moon's Low band then turns by the moon's gravity, but at the planet's position and with the velocity relative to the planet, which is what the crossing ends on. The closest-approach phase (PHAS-11) already reads the same way. The build accepts this.
- **Affects:** XFER-13, XFER-01 · **Type:** spec gap
- **Disposition:** open. Accept, or say how a pass through a moon's band is aimed from outside its planet's frame.

### F-175 · A low-thrust pass through Low reads as half a turn
- **Observation:** A low-thrust crossing ends on the body's circle at almost the body's own velocity (XFER-06), so it arrives with almost no excess speed. XFER-13 then gives a turn of nearly 180°, though turning an excess velocity that small changes almost nothing. The build applies the rule as written, so the planner's label (PLNR-65) would read "turn 180°".
- **Affects:** XFER-13, XFER-06, PLNR-65 · **Type:** spec gap
- **Disposition:** open. Accept, show no turn for a low-thrust leg, or make a low-thrust Intercept through Low infeasible.

### F-176 · How a group is held in the simulation
- **Observation:** SYM-01 says a group holds its members, but not how. The build makes a group one body (class Ship, with a `ShipGroup` component) in its named orbit, at its own phase. Its ships are member entities holding a name and a ship record only: no orbit, phase, class marker or drive. So the view draws and folds the group as one body and never sees the members (SYM-09, GRP-09). The group's ship record is derived from the members' and kept in step each frame. The spec names class, armed status, load and condition; the build also chose: the track is the weakest member's (stale before weak, the older stale date), command is shared, and a group is requisitioned only if every member is. A group has no drive, so it can't be ordered anyway.
- **Affects:** SYM-01, SYM-09, PHAS-14, TEST-10 · **Type:** spec gap
- **Disposition:** open. Confirm, or say how members should be held once groups form and split in play.

### F-177 · The words of a group's member count
- **Observation:** SYM-13 gives "4 warships" as the only example. The build names the classes "warship", "troop transport", "supply ship", "cargo ship" and "unknown ship" (plural with "s"), lists them most significant first, and joins them with ", " inside the one " · " part: "2 warships, 1 supply ship". The outline (GRP-09) reads "TF-1 · 4", like an orbit row; its small symbol carries no count or bracket. The inset of a group still reads "Ship · own" with the group's description (F-159).
- **Affects:** SYM-13, GRP-09 · **Type:** spec gap
- **Disposition:** open. Confirm the words, or give the class names.

### F-179 · Which ends of a crossing take the Low band limit
- **Observation:** XFER-07 gives the Low band limit to a crossing or coast that starts or ends "in a place round the primary itself (a band, or a station in one)". It doesn't say whether a ship coasting round the primary counts: one setting out from a coast, or a ship target on one. The build gives the Low band limit to every end round the primary, coasts included, as the old exemption did, and also a gate's or site's own orbit round it. A path solved to touch the Low band lands a hair either side of it, so a periapsis within a millionth of the limit (about 6 mm at a Low band) counts as at it.
- **Affects:** XFER-07 · **Type:** spec gap
- **Disposition:** open. Accept, or say which ends count as round the primary.

### F-180 · An Intercept through its primary's own Low band needs the Hohmann time
- **Observation:** Under XFER-07, an arc from a planet's High band to its own Low band touches the band only at its periapsis. Faster than the Hohmann time, it passes its periapsis under the band before it arrives, so even a Rendezvous is refused. Slower, it arrives still heading down, so a Rendezvous is flown but an Intercept's coast would dip under the band. So an Intercept through the primary's own Low band is flown only at the Hohmann time, and only when the target sits half a turn ahead then. In the reference layout, no departure phase gave a feasible Intercept at the capital's Low band station. The same holds for leaving a Low band: an arc faster than the Hohmann time first dips under the band.
- **Affects:** XFER-07, XFER-08, PLNR-30 · **Type:** spec gap
- **Disposition:** open. Accept, or offer only Rendezvous for a Low band round the ship's own primary.

### F-181 · Sol's drives have no display names
- **Observation:** PLNR-05 gives every drive a display name, and TEST-30 names Corvane's two. Sol's Marlin and Vestige (SOL-15, SOL-16) have drives too, but no names. The build calls Marlin's "hydrogen NTR", like Orison's, and Vestige's "fusion torch".
- **Affects:** PLNR-05, TEST-30, SOL-15, SOL-16 · **Type:** spec gap
- **Disposition:** open. Accept, or name them in SOL-15 and SOL-16.

### F-182 · A band of the primary itself has no window
- **Observation:** PLNR-80 sets the Depart range from the periods of the start and end states. A trip to a band of the crossing's own primary (Halden IV High to Halden IV Low) ends on the band's ring, half a turn ahead of the start (XFER-01), so any point will do and the cost doesn't change with the depart day. Read literally, the two bands' periods would give a Depart range of about an hour and a Next window. The build treats a whole ring of the primary as having no period: the range is `depart_range_days` and there is no Next window. A station or ship in that band has a phase, so it keeps its window.
- **Affects:** PLNR-80, PLNR-85 · **Type:** spec gap
- **Disposition:** open. Accept, and say so in PLNR-80.

### F-183 · Words the spec doesn't give
- **Observation:** PLNR-92 words nine of the solver's reasons. The others needed sentences too, and the build chose: no destination found "*target* can't be found"; no High band "*target* has no High orbit"; Hold "Holding is not a trip"; no path "No path gets there in that time"; a low-thrust drive and a coasting path "This drive can't match a coasting path"; a target in flight "*target* is under way, so it can't be met"; the ship in flight "The ship is under way"; already ordered "The ship already has an order"; no drive "The ship has no drive"; more than one leg "Only one leg can be ordered"; an Intercept in the ship's own orbit "*target* is in this ship's orbit, so it can only rendezvous". The leave readout (PLNR-21) reads "now" when the leave day is under half a minute away, and the laps readout (PLNR-40) says "1 lap" for one. A delta-v that rounds up to 1 km/s reads "1.0 km/s", not "1.00" (UNIT-04).
- **Affects:** PLNR-92, PLNR-21, PLNR-40, UNIT-04 · **Type:** spec gap
- **Disposition:** open. Accept, or give the words.

### F-184 · How the search fills the gaps in its rules
- **Observation:** The build chose these where PLNR-80 to PLNR-87 are silent. A slider's settings are its start plus whole steps, and its end. Refining a preset (PLNR-86) solves those settings within one grid spacing of the sample; if none qualifies (no affordable one for Fastest), the setting nearest the sample is kept. Cheapest is refined by least delta-v alone, as PLNR-86 says, without PLNR-84's tie rule. A phasing leg's meetings are already all on the grid, so nothing is refined. A low-thrust drive's fixed travel time is the one its leg takes when leaving today. A phasing leg's meetings are the *k* whose own orbit fits (PHAS-08), solved with no budget, so a meeting the ship can't afford is still a position, coloured over budget. When nothing on the grid can be planned, the reason shown is the one the solver gave most often. The headless test that searches twice in Corvane, app start included, runs in about 30 ms.
- **Affects:** PLNR-23, PLNR-82, PLNR-84, PLNR-86, PLNR-40, PLNR-12 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-145 · An intercept needs a standoff distance
- **Observation:** Raised in a combat design session. XFER-10 ends a ship-as-destination crossing at the target's position on the crossing end day, so an Intercept is a collision course with zero miss distance. A real engagement has a chosen miss distance, and it sets how long each side is in weapon range and which weapons can act. The order has no way to carry one. An Intercept at a station is trimmed to meet it (PHAS-15), so it too has zero miss distance.
- **Affects:** XFER-08, XFER-09, XFER-10, PHAS-15 · **Type:** spec gap
- **Disposition:** parked. Option: a `standoff_km` on an Intercept whose destination is a ship, so the crossing ends offset from the target's rail rather than on it. A Rendezvous keeps zero. What the standoff should default to, and how the resolver reads it, are for the conflict doc to settle.
- **Revisit when:** the combat design is specced. The default is an open question in `docs/ideas/mechanics/conflict.md`.

### F-158 · Departure and capture costs add up to far more than a real trip
- **Observation:** Raised while sizing Marlin for Sol (SOL-15). BAND-03 and XFER-04 charge a ship the cost of escaping its band to rest at infinity, then the burn that sets its speed round the primary, and the same again at the far end. A real burn deep in a gravity well does both at once and is much cheaper (the Oberth effect). From Earth's Low band to a Low band at the target, with a Hohmann crossing, the spec's cost against a patched-conic figure for the same bands: Venus 11.3 against 6.7 km/s, Mars 10.1 against 5.6, Ceres 14.4 against 9.5, Mercury 21.4 against 13.2. The spec is 1.5 to 1.8 times too high. The low-thrust crossing (XFER-06) adds the same escape cost, which for a spiral is too low: a spiral escape costs the full circular speed, not √2 − 1 of it.
- **Affects:** BAND-03, XFER-04, XFER-06, XFER-12, SOL-15 · **Type:** spec wrong
- **Disposition:** parked. The model needs fixing, and the fix is a significant piece of work, so the spec and the build stay as they are for now (decided 2026-10-16). Option: for an impulsive drive, replace departure cost + burn 1 (and burn 2 + capture cost) with one burn from the band's circular orbit to a hyperbola with the crossing's excess speed, √(*v∞*² + 2μ/*r*) − √(μ/*r*). The excess speed is already computed for BAND-06, XFER-09 and PHAS-03. Open: how burn days (XFER-05) and approach days (BAND-04) fit, what the spiral should cost, and new figures for the XFER-12 worked checks. Marlin's delta-v would then be set again.
- **Revisit when:** the transfer-cost rework is scheduled. `docs/ideas/mechanics/transfers.md` (section 2.1) holds the lean.

### F-159 · Groups are only loosely defined
- **Observation:** SYM-09 draws a group, but nothing says what selecting one shows. The inset (INS-02) has no form for a group. The outline (INS-05) lists a group by name and count (GRP-09), with no way to list its members or reach one by one, for their Delta-v and Drive (PLNR-05). "Commander" means a person in SYM-09 and in mock-up 7 ("Cdr Ilse Marr"), but "command" means the faction giving orders in SYM-03. How groups form, merge and split is not specced either: no order creates a group, and merge and detach orders are not specced (PHAS-14). So groups exist only as test content (TEST-06) for now. The planner already refuses them (PLNR-01, PLNR-11). The combat ideas (candidate) detach and rejoin ships and call every group a force, which would be the first source of groups in play.
- **Affects:** SYM-09, SYM-10, INS-02, INS-05, GRP-09, PLNR-01, PLNR-05 · **Type:** spec gap
- **Disposition:** parked. Safe to defer: the player can't form groups yet. Specify the group inset, member access and the meaning of "commander" together with the orders that form and split groups.
- **Revisit when:** the orders that form and split groups are designed, or a group first appears in play.

# Findings

Observations from building and playing the prototype, in two sections. **To triage** holds findings waiting for a decision at the next design session. **Parked** holds findings that can't be decided until a later feature is designed, each with a line saying what brings it back. Once a finding is settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings go under To triage and take the next free number from F-198.

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

### F-164 · How the selection mark moves while a satellite folds
- **Observation:** HOV-06 says a marker part-way folded or faded has a mark faded by the same alpha, and INS-06 says the mark moves from the pill to the marker without a jump. The build follows INS-06 for the fold: the mark keeps its full strength and its box slides between the pill's box and the marker's as the satellite slides (`fold_unfold_anim_ms`). The fade by the marker's alpha is applied only to the dot fading into its exposure ring (MOT-11). When a parent's last folded satellite unfolds, its pill hides at once, so that mark starts from the parent's marker instead of the pill.
- **Affects:** HOV-06, INS-06, GRP-03 · **Type:** spec gap
- **Disposition:** open. Accept, or say whether the mark should fade with the fold slide.

### F-165 · What a shared orbit's identity pill brackets for
- **Observation:** HOV-05 brackets a shared orbit's identity pill while the orbit is hovered. HOV-06 puts an object's mark on its exposure ring's identity mark, and a member of a shared orbit has the pill as its mark. The build gives the pill the selection mark while any member is selected, and the hover mark while the orbit or any member is hovered (for example, from the outline).
- **Affects:** HOV-05, HOV-06, MOT-14 · **Type:** spec gap
- **Disposition:** open. Accept, or say which objects the pill stands for.

### F-166 · Labels sit very close to the brackets
- **Observation:** Seen on screen. LBL-01 measures `label_gap_px` (6 px) from the marker's edge, and the bracket box reaches `bracket_margin_px` (4 px) beyond it. A selected or hovered object's label starts 2 px outside its brackets, and at the system fit the right-hand arms nearly touch Halden IV's first letter. The build leaves label placement as it was.
- **Affects:** LBL-01, HOV-06, HOV-07 · **Type:** spec wrong
- **Disposition:** open. Options: measure the label gap from the bracket box (for every marker, so labels don't move on hover), or raise `label_gap_px`.

### F-167 · Where the condition bar sits, and what an empty bar looks like
- **Observation:** SYM-12 puts the bar "under the frame" but gives no gap. Directly under the frame, the selection and hover brackets (HOV-07, 4 px clear of the frame) would cross it. The build starts the bar 1 px below the bracket box, so 5 px below the frame. The spec doesn't say how the unfilled part looks either. The build draws it in the bar's colour mixed 25 % into the ground, so a part-full bar shows its whole length. An own ship with no known condition (Corvane's own-flag civilians) has no bar.
- **Affects:** SYM-12, HOV-07 · **Type:** spec gap
- **Disposition:** open. Accept, or give the gap and the empty part's look.

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

### F-178 · Where a group's count and bracket sit
- **Observation:** SYM-09 puts the count "just outside the frame's upper right" and the bracket above the frame. Drawn hard against the frame, as in mock-up 7, the selection and hover brackets covered both. The build starts them outside the brackets' box (`bracket_margin_px` + 1 px), as the condition bar already is: the count to the right of the box, level with the frame's top, and the bracket's feet just above the box (or above the echelon, if higher). Labels keep `label_gap_px` from this outer edge (SYM-13), and other labels keep off it too. The count's width is estimated from the data face's fixed advance.
- **Affects:** SYM-09, SYM-13, HOV-07 · **Type:** spec gap
- **Disposition:** open. Confirm, or set the offsets as tunables.

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

### F-188 · Bracket arms grow with a close disc
- **Observation:** Seen in play. HOV-06 makes each arm `bracket_arm_fraction` of the box's shorter side. When the box grows with a true-size disc (HOV-07), the arms grow too, and at close zoom the corners dominate the view.
- **Affects:** HOV-06, HOV-07 · **Type:** spec wrong
- **Disposition:** open. Options: cap the arm length in pixels (a new tunable), or fix it in pixels for every box.

### F-189 · A scene of terrain alone is grey and bland
- **Observation:** Seen in play. A view showing only natural bodies, with no ships, stations or gates, is all greyscale: neutral markers, neutral discs and faint rings. Nothing draws the eye.
- **Affects:** MRK-01, MRK-11, MRK-13 · **Type:** spec wrong
- **Disposition:** open. Decide whether natural bodies get any colour (for example a tint per body class or composition), or another accent.

### F-190 · The belt doesn't read as an asteroid belt
- **Observation:** Seen in play. The belt's band has hard edges, and as a whole it doesn't convey a field of rocks.
- **Affects:** MRK-05 · **Type:** spec wrong
- **Disposition:** open. Options: soft edges that fade in and out, a density falling off towards the edges, or a scatter of faint points.

### F-191 · An exposure ring doesn't say what it is
- **Observation:** Seen in play. An exposure ring reads as an orbit ring drawn slightly thicker. Nothing tells the player that the object is moving too fast to show as a dot, so the identity mark sitting still on the ring looks like a parked object, not a stand-in for one that is circling.
- **Affects:** MOT-08 to MOT-14 · **Type:** spec wrong
- **Disposition:** open. Options: a distinct look for the ring (dashes, a gradient or a sweep in the sense of motion), a motion cue on the identity mark (a small arrow or a trail), or a word on hover ("orbiting, period 2 h").

### F-192 · Commanded ships with no drive get a planner they can't use
- **Observation:** PLNR-01 gives every commanded ship parked in a named orbit a Transfer button. In Corvane only Orison and RFA Tolland have drives (TEST-30); Ardent, the transports and Delve have none. The build shows them the button, and the planner opens, but every line reads "The ship has no drive". Commanded stations (Tern Relay) get no button, as PLNR-01 says "ship". PLNR-05's Delta-v and Drive facts show only for a ship with a drive.
- **Affects:** PLNR-01, PLNR-05, TEST-30 · **Type:** spec gap
- **Disposition:** open. Accept, hide the button for a ship with no drive, or give every commanded ship a drive in content.

### F-193 · Where the planner's parts sit in the inset
- **Observation:** The spec orders the tuning rows (PLNR-20) but not the rest. The build chose: the header (name, Focus, Close) and class line stay while the planner is open; the image, description, key facts and planetary-view button give way; the Transfer button hides. The quiet Cancel sits beside Schedule, and under the prompt while choosing. The Gap line follows the Travel time row. An order (PLNR-51) and a coasting ship's place (PLNR-54) sit below the key facts, over a hairline; the PLNR-53 line sits above the header. Fastest shows disabled when nothing is affordable (PLNR-29), though PLNR-83 says it then doesn't exist, which would hide it.
- **Affects:** PLNR-02, PLNR-20, PLNR-29, PLNR-41, PLNR-51, PLNR-53, PLNR-54 · **Type:** spec gap
- **Disposition:** open. Accept, or place them.

### F-194 · What a click does while planning, beyond PLNR-11
- **Observation:** An orbit row in the outline both picks its band and opens, so the ships in it stay within reach. A double-click while planning picks and still moves the camera (ZOOM-16), as camera controls work as usual (PLNR-03). A named orbit that is not a band or a docking orbit (Morrow IX's "Outer") can't be picked, as no place stands for it (BAND-02). An Intercept of a station or ship is ordered through High; the solver takes the target's own band anyway (F-171).
- **Affects:** PLNR-03, PLNR-11, INS-05, BAND-02 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-195 · Changing an order cancels it first
- **Observation:** PLNR-53's Schedule replaces the order. The build sends Cancel and then the new order in one frame, so the old reservation is free for the new one. If the new order were refused there, the old one would already be gone; the panel only schedules a leg it has just solved within the budget, so this should not happen. What the simulation answers (`OrderOutcome`) is not shown anywhere.
- **Affects:** PLNR-50, PLNR-53, PLAN-22 · **Type:** spec gap
- **Disposition:** open. Accept, or ask for a replace in the simulation's order API.

### F-196 · How the sliders behave where the spec is silent
- **Observation:** A phasing leg's meetings sit evenly along the track, one position each, not by time. A slider keeps keyboard focus until a press on the map. The thumb is centred on its setting, so it overhangs the track by half its width at each end. The Depart range's start moves once a game day, with the search (PLNR-87), while a depart day in the past moves to today at once (PLNR-24). Each new day runs a full search (the grid's 2,160 solves and the presets), so at fast clock rates the planner searches several times a second.
- **Affects:** PLNR-23, PLNR-24, PLNR-40, PLNR-87 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-197 · Orison can't reach Iver, so its tracks are all orange
- **Observation:** Seen on screen. A Rendezvous from Halden IV's High band to Iver's needs at least 13.2 km/s, more than Orison's 9.9, so both tracks are the over-budget colour from end to end and no window shows as a bright band. An Intercept of Iver, or Halden IVa, is within reach. The empty colour (the inset's hairline colour) is light grey, close to the dimmest blue, so a rejected setting is hard to tell from a dear one.
- **Affects:** PLNR-28, TEST-30, F-158 · **Type:** spec wrong
- **Disposition:** open. Give Orison more delta-v or cheaper bands (F-158), and pick an empty colour darker than the hairline.

## Parked

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

# Findings

Observations from building and playing the prototype, in two sections. **To triage** holds findings waiting for a decision at the next design session. **Parked** holds findings that can't be decided until a later feature is designed, each with a line saying what brings it back. Once a finding is settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings go under To triage and take the next free number from F-264.

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

### F-181 · Sol's drives have no display names
- **Observation:** PLNR-05 gives every drive a display name, and TEST-30 names Corvane's two. Sol's Marlin and Vestige (SOL-15, SOL-16) have drives too, but no names. The build calls Marlin's "hydrogen NTR", like Orison's, and Vestige's "fusion torch".
- **Affects:** PLNR-05, TEST-30, SOL-15, SOL-16 · **Type:** spec gap
- **Disposition:** open. Accept, or name them in SOL-15 and SOL-16.

### F-183 · Words the spec doesn't give
- **Observation:** PLNR-92 words nine of the solver's reasons. The others needed sentences too, and the build chose: no destination found "*target* can't be found"; no High band "*target* has no High orbit"; Hold "Holding is not a trip"; no path "No path gets there in that time"; a low-thrust drive and a coasting path "This drive can't match a coasting path"; a target in flight "*target* is under way, so it can't be met"; the ship in flight "The ship is under way"; already ordered "The ship already has an order"; no drive "The ship has no drive"; more than one leg "Only one leg can be ordered"; an Intercept in the ship's own orbit "*target* is in this ship's orbit, so it can only rendezvous". The laps readout (PLNR-40) says "1 lap" for one. A delta-v that rounds up to 1 km/s reads "1.0 km/s", not "1.00" (UNIT-04).
- **Affects:** PLNR-92, PLNR-40, UNIT-04 · **Type:** spec gap
- **Disposition:** open. Accept, or give the words.

### F-184 · How the search fills the gaps in its rules
- **Observation:** The build chose these where PLNR-80 to PLNR-87 are silent. A Travel time slider's settings are its start plus whole steps, and its end. Refining a preset (PLNR-86) solves those settings within one grid spacing of the sample; if none qualifies (no affordable one for Fastest), the setting nearest the sample is kept. Cheapest is refined by least delta-v alone, as PLNR-86 says, without PLNR-84's tie rule. A phasing leg's meetings are already all on the grid, so nothing is refined. A low-thrust drive's fixed travel time is the one its leg takes when leaving today. A phasing leg's meetings are the *k* whose own orbit fits (PHAS-08), solved with no budget, so a meeting the ship can't afford is still a position, coloured over budget. When nothing on the grid can be planned, the reason shown is the one the solver gave most often. The headless test that searches twice in Corvane, app start included, runs in about 30 ms.
- **Affects:** PLNR-23, PLNR-82, PLNR-84, PLNR-86, PLNR-40, PLNR-12 · **Type:** spec gap
- **Disposition:** open. Accept.

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
- **Observation:** A phasing leg's meetings sit evenly along the track, one position each, not by time. A slider keeps keyboard focus until a press on the map. The thumb is centred on its setting, so it overhangs the track by half its width at each end.
- **Affects:** PLNR-23, PLNR-40 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-197 · Orison can't reach Iver, so its tracks are all orange
- **Observation:** Seen on screen. A Rendezvous from Halden IV's High band to Iver's needs at least 13.2 km/s, more than Orison's 9.9, so both tracks are the over-budget colour from end to end and no window shows as a bright band. An Intercept of Iver, or Halden IVa, is within reach. The empty colour (the inset's hairline colour) is light grey, close to the dimmest blue, so a rejected setting is hard to tell from a dear one.
- **Affects:** PLNR-28, TEST-30, F-158 · **Type:** spec wrong
- **Disposition:** open. Give Orison more delta-v or cheaper bands (F-158), and pick an empty colour darker than the hairline.

### F-198 · The departure ghost of a leg that starts round its own primary
- **Observation:** PLNR-63's departure ghost is the departure body where the path starts. A leg that starts in an orbit of the crossing's primary has no such body: a phasing leg, or a trip from Halden IV's High band to Halden IVa, goes round Halden IV itself. The build ghosts the ship there instead, a `ship_symbol_px` circle at the path's start. In every other case it ghosts the departure's ancestor directly below the primary (XFER-01), as the path starts from that body's state.
- **Affects:** PLNR-63 · **Type:** spec gap
- **Disposition:** open. Accept, or draw no departure ghost for such a leg.

### F-199 · An order being changed hides its own path
- **Observation:** While Change has the planner open on a Scheduled order (PLNR-53), the map draws only the leg being tuned, not the order still in force. Drawing both would give the ship two paths and two arrival ghosts with near-identical labels.
- **Affects:** PLNR-53, PLNR-62 · **Type:** spec gap
- **Disposition:** open. Accept, or show the old path faintly.


### F-202 · Dashes crawl along a path in flight
- **Observation:** Dashes and dots are laid on screen from the start of the dashed part, so their length stays `path_dash_px` at any zoom (PLNR-61). In flight that start is the ship, which moves, so the dashes slide along the path as the ship flies and as the camera moves. Laying them from the path's end would hold them still while the ship flies, but not while the camera zooms.
- **Affects:** PLNR-61 · **Type:** idea
- **Disposition:** open. Watch in play; anchor the dashes to the arrival end if the crawl distracts.

### F-219 · At heading 0 the vertical axis lies on the north arm
- **Observation:** At heading 0, at any tilt between straight down and edge-on, the north arm and the plane's normal both point straight up the screen. They start at the same gap beyond the frame body's box, with the same dot size and pitch, so the axis's upper half sits dot for dot on the north arm. Only the arrowhead, the ticks and a slightly stronger fade show there are two strokes. A few degrees of heading separates them. The build follows REF-06 as written, with no offset or tint.
- **Affects:** REF-06, REF-01 · **Type:** spec gap
- **Disposition:** open. Check on screen. If it reads as one line: a different dot pitch or size for the axis, dots offset by half a pitch, or a tint (spec open points).

### F-221 · A window leg skips a start whose crossing would graze the Low band
- **Observation:** PHAS-16 says to take the cheapest of the sampled starts. Inside a planet's frame the cheapest arc to a station in the Low band often just touches the band, and the burn days alone tip it under, so the leg would be refused (XFER-07). The build prices each start by its arc, then takes the cheapest start whose crossing, flown after its burn days, clears the primary. It takes the cheapest outright only if none clears.
- **Affects:** PHAS-16, XFER-07 · **Type:** spec gap
- **Disposition:** open. Accept, and add the sentence to PHAS-16.

### F-222 · A window leg's grid looks for each day's window where Soonest found it
- **Observation:** Searching every start for every grid cell took about 360 ms per search for a station in the Low band. A window leg costs the same whenever its two ends stand the same way, which recurs every window interval. So the grid searches Soonest in full, and on other days tries the start nearest that phase and one either side, falling back to a full search when none of them can be flown. In tests under 5% of cells differ from a full search, and none is dearer. The solved leg and Schedule always use the full search.
- **Affects:** PLNR-82, PHAS-16 · **Type:** spec gap
- **Disposition:** open. Accept, or ask for an exact grid at about 4 times the cost.

### F-223 · Presets are kept as the clock runs
- **Observation:** PLNR-87 says the clock doesn't recompute the presets. The build keeps their settings. A preset at Soonest, or on a day that has come, moves to Soonest with its travel time kept, as a thumb does (PLNR-24). Its travel time may then be rejected there.
- **Affects:** PLNR-24, PLNR-29, PLNR-87 · **Type:** spec gap
- **Disposition:** open. Accept, or pick the presets again on the daily step (about 100 more solves).

### F-224 · No Intercept through the Low band of the crossing's primary, wherever the ship starts
- **Observation:** PLNR-30 names "the leg's own primary, the body the ship orbits". The build tests the crossing's primary. So a leg from a moon's band to its planet's Low band (or to a station in it) offers no Intercept either, since XFER-07 restricts it the same way.
- **Affects:** PLNR-30 · **Type:** spec gap
- **Disposition:** open. Accept, and reword PLNR-30.

### F-225 · How PLNR-33 judges a position off the grid
- **Observation:** A Depart position that isn't a grid column, and lies between two rejected columns, counts as rejected without being solved, as the track draws it. Otherwise its Travel time track is solved and kept for the session. A travel time between two rejected track samples counts as rejected too. A key step with no accepted setting in its direction leaves the thumb where it is. On a leg with gappy windows a day that is really accepted could be skipped.
- **Affects:** PLNR-33 · **Type:** spec gap
- **Disposition:** open. Accept, or always solve the track for such a day (slower).

### F-226 · Short window intervals make a two-position Depart slider
- **Observation:** From Halden IV's High band to Halden IVa the window interval is 1.7 days. The Depart range is therefore 1.9 days, and the slider has only Soonest and tomorrow. Any window interval between one and about two days gives a slider this short.
- **Affects:** PLNR-80, PHAS-16 · **Type:** spec wrong
- **Disposition:** open. Raise PHAS-16's one-day threshold, or give the Depart range a minimum span.

### F-227 · A phasing leg's grid has only the Soonest column
- **Observation:** PLNR-82 says the grid holds Soonest and the Depart positions. For a phasing leg every day costs the same, so the build keeps only the Soonest column, and the Depart track is drawn in one colour from it (PLNR-40).
- **Affects:** PLNR-82, PLNR-40 · **Type:** spec gap
- **Disposition:** open. Accept, and add the sentence to PLNR-82.

### F-228 · The Depart tooltip when a day has no trip
- **Observation:** PLNR-106 doesn't say what a depart day with no trip at all counts as, or what an empty grid shows. The build counts such a day as outside the margin, so the Depart label shows the full PLNR-96 text, which explains blank. Since Soonest is solved again as the clock runs, the label can switch between its two texts while the planner is open.
- **Affects:** PLNR-106, PLNR-96 · **Type:** spec gap
- **Disposition:** open. Accept, and add the sentence to PLNR-106.

### F-245 · The outline follows a selected ship that moves
- **Observation:** INS-05 opens the path to the selection only when the selection changes. A selected ship that arrives at another body would then drop out of the outline, because its new parent and orbit row are still collapsed. The build also opens the new path (the parent entry and the named orbit's row) when the selected body's parent or named orbit changes, and scrolls it into view; other entries keep their state. That scroll can override where the player had scrolled the outline, on departure and on arrival.
- **Affects:** INS-05, RAIL-09 · **Type:** spec gap
- **Disposition:** open. Accept and say so in INS-05, or open the path without scrolling.

### F-246 · The object pip sits too far from its parent
- **Observation:** In play, the pip reads as slightly detached from its planet. GRP-12 centres it `h + object_pip_gap_px` right of and below the parent's centre, so it clears the parent's marker by the gap.
- **Affects:** GRP-12 · **Type:** spec wrong
- **Disposition:** open. Seen by the user in play (build 776cc34), who says it is minor. Bring the pip closer; letting it overlap the parent's marker would be fine and makes the link clearer.

### F-248 · A popped marker holding one thing looks like a group
- **Observation:** In play, when the pip pops out into the fold marker, it shows a back frame (MUL-01) even when the parent holds only one thing, so a single ship reads as a group. The builder followed MUL-01: a single thing has no back pip at rest, and as it pops its back frame slides out to `back_frame_offset_px`.
- **Affects:** MUL-01, GRP-12, GRP-13 · **Type:** spec wrong
- **Disposition:** open. Seen by the user in play (build 776cc34). A popped marker holding one thing should show no group indicator. Mock-up 27 shows it that way.

### F-249 · How long the axis's lower half is once the upper half is clipped
- **Observation:** REF-07 makes the lower half `ref_down_fraction` of the upper half's length, but doesn't say which length once REF-06 clips the upper half at the map edge. The build uses the unclipped length, so with the frame body high on the map the lower half keeps its full length and can be longer than the clipped upper half. The lower half then doesn't jump as the body nears the edge.
- **Affects:** REF-06, REF-07 · **Type:** spec gap
- **Disposition:** open. Accept, and say so in REF-07.

### F-250 · Does the object marker fade out, or vanish at once?
- **Observation:** GRP-12 says the pip "appears and disappears over `fold_unfold_anim_ms`". INS-06 says a multi-marker whose last thing leaves "hides at once". The build fades the pip in, but drops it at once when its last ship, group or station starts to unfold; the unfolding thing's own slide covers the change.
- **Affects:** GRP-12, INS-06 · **Type:** spec gap
- **Disposition:** open. Pick one rule for a fold marker's last thing: drop "and disappears" from GRP-12, or say how it fades out with nothing left in it.

### F-251 · Which parents count for "nearest" when popping, and a tray above
- **Observation:** GRP-13 pops a marker only when the pointer is nearer its parent than "any other parent's". The build counts every body that has anything folded into it, moons included and on screen or not, so a moons-only planet close by can stop a neighbour's pip from popping. MUL-08 puts a fold marker's tray "above it" when it doesn't fit below; just above the marker it would cover the parent, so the build places it above the parent's box too.
- **Affects:** GRP-13, MUL-08 · **Type:** spec gap
- **Disposition:** open. Say which parents count (all with folds, only those with an object marker, only those on screen), and accept the tray rule into MUL-08.

### F-252 · Where a parent's label is centred beside its popped fold marker
- **Observation:** GRP-12 has the parent's label try left, above, below, then right, "measured from the popped fold marker's box". The build places it beside the union of the parent's box and the popped marker's box, centred along that union's side. The popped marker hangs down and to the right, so a left label sits about 11 px below the planet's centre. In the overview that left slot hits Lantern's marker, so Halden IV's label goes above; centred on the planet it would fit on the left.
- **Affects:** GRP-12, LBL-01 · **Type:** spec gap
- **Disposition:** open. Say whether each slot keeps the union box's edge but is centred on the parent's own box, or accept the union centring.

### F-253 · Ships folded into a folded moon have no marker
- **Observation:** Folded ships, groups and stations go to an object marker, and natural bodies to the moon ring. A satellite of a satellite that has itself folded is counted nowhere. Before, the moon sat in the planet's fold marker and carried its own folds. Now, at the system fit, Morrow IX's Tern Relay (an own station) and the Kessel trader (TEST-09) are on no marker: Morrow shows "+11" and no pip. When one of them is selected, its mark goes round Morrow's marker and ring.
- **Affects:** GRP-11, GRP-12, MUL-04, INS-06, TEST-09 · **Type:** spec gap
- **Disposition:** open. Say whether things folded into a folded moon pass up to the nearest drawn parent's object marker (and how its tray lists them), or stay unshown.

### F-254 · A star's age is drawn up to its lifetime, and giants may pass the maximum
- **Observation:** SGEN-23 says age is drawn from `age_min_gyr` to `age_max_gyr` "and capped at" the main-sequence lifetime. Drawing then capping would put most F and A stars at exactly their lifetime (59% of 1.4 M☉ stars at 4.3 Gyr). The build draws uniformly from `age_min_gyr` to the lesser of `age_max_gyr` and the lifetime. A giant's age (its progenitor's lifetime × 1.0 to 1.1) can pass `age_max_gyr`: a 1 M☉ giant is 10 to 11 Gyr old. The build follows the spec there.
- **Affects:** SGEN-23 · **Type:** spec gap
- **Disposition:** open. Copy the draw into SGEN-23, and say whether giants may pass `age_max_gyr`.

### F-255 · Template bounds the spec leaves open
- **Observation:** SGEN-33 leaves some ranges and mixes open. The build, all in `template_rules`: a hot giant's 0 to 2 outer planets go from 0.5 to 2 snow lines, with the solar-like kinds (0.7 rocky, 0.3 super-Earth); a debris disc's planets go from 0.05 to 2 snow lines, with the compact kinds; a white-dwarf remnant's kinds are gas giant, ice giant and super-Earth, equally likely. Planet counts within a template's range are uniform. Only the Hill rule (SGEN-34) redraws; a planet outside the template's range or beyond the companion limit is dropped at once.
- **Affects:** SGEN-33, SGEN-34 · **Type:** spec gap
- **Disposition:** open. Copy the bounds and mixes into the spec, or give others.

### F-256 · Belts also keep inside the companion limit
- **Observation:** SGEN-30 limits planets to `companion_truncation_fraction` × the nearest companion's separation, but says nothing of belts. A belt at 6 snow lines (16 AU round a G star) could sit outside a companion at 5 AU. The build keeps a belt's outer edge inside the same limit; a belt that finds no room in 20 tries is dropped.
- **Affects:** SGEN-30, SGEN-37 · **Type:** spec gap
- **Disposition:** open. Add belts to SGEN-30, or drop the limit for belts.

### F-257 · Companion mass and separation limits
- **Observation:** SGEN-25 limits a companion's class but not its mass, so a G companion of a 0.85 M☉ G primary could be heavier than it. The build caps a companion's mass at the primary's. SGEN-26 says separations are "limited to" `companion_min_au` and `wide_pair_min_au`; clamping would pile about a fifth of non-M companions at exactly 1,000 AU, so the build redraws an out-of-range separation up to 20 times, then clamps.
- **Affects:** SGEN-25, SGEN-26 · **Type:** spec gap
- **Disposition:** open. Copy both into the spec, or choose otherwise.

### F-258 · Spectral types of white dwarfs and giants
- **Observation:** SGEN-22 builds a spectral type from "its class letter, a digit and V ('D' for a white dwarf, 'III' for a giant)", but neither has a class letter. The build writes a white dwarf as "D" and the digit ("D4"). A giant takes its letter from its temperature (M below 3,700 K, K below 5,200, G below 6,000, F below 7,500, else A), then the digit and "III" ("K2III"). Drawing a giant's radius and luminosity independently could give 1,900 to 10,000 K, so the build draws both at the same place in their log ranges. Every giant is then about 4,300 to 4,600 K, a K giant, with its digit set by mass.
- **Affects:** SGEN-22 · **Type:** spec gap
- **Disposition:** open. Copy into SGEN-22, or give another notation.

### F-259 · Same sector on every machine depends on the platform's maths library
- **Observation:** SGEN-02 asks for the same sector "on every run and machine". The random numbers are integer arithmetic, identical everywhere, and a test pins the published xoshiro256** and SplitMix64 values. But the generator also uses `powf`, `ln`, `exp`, `cbrt`, `sin` and `cos`, which Rust takes from the platform's maths library, and their last bit may differ between operating systems. Range and Hill-spacing branches can turn that into a different sector. The same build target gives the same sector every run.
- **Affects:** SGEN-02, SGEN-06 · **Type:** spec gap
- **Disposition:** open. Accept "same target, same sector", or write the few functions in the repo (SGEN-06 rules out a crate such as `libm`), or allow such a crate.

### F-260 · Every sub-Neptune has the same surface gravity
- **Observation:** The sub-Neptune curve in SGEN-36, R = 2.4 × (M ÷ 6)^0.5, makes M ÷ R² a constant, so every sub-Neptune has a gravity of 1.04 g, whatever its mass. Envelopes aren't scored for settlement, so nothing depends on it yet, but the inspection inset will show the same figure for every one.
- **Affects:** SGEN-36, SGEN-38 · **Type:** spec wrong
- **Disposition:** open. Use another exponent, or accept it.

### F-261 · A moved planet's facts need draws that aren't stored
- **Observation:** SGEN-59 says a moved planet has its facts "worked out again". But the atmosphere depends on a random volatile endowment (SGEN-40) and water on two random draws (SGEN-41), and none of these is kept on the planet's summary, so the facts can't be worked out again consistently. Found in review of part 1; part 2 builds SGEN-59.
- **Affects:** SGEN-59, SGEN-40, SGEN-41, SYSG-03 · **Type:** spec gap
- **Disposition:** open. Suggested: store the endowment on the planet and keep the water class unless the new temperature rules it out, as SYSG-03 (detail never re-rolls) points to. Part 2 builds that, and you can change it.

### F-262 · Should a fold marker's tray follow its things when they unfold?
- **Observation:** MUL-10's rule (the tray follows the marker holding most of its rows' things) lets a fold marker's tray leave it. When framing Iver unfolds Iver Yard, the yard and its two docked ships make up the station's own marker, so the tray moves there: it jumps from below the fold marker to beside the station, its header changes from "IVER · 1 IN ORBIT" to "IVER YARD · 2 DOCKED", and Iver stays selected with no row highlighted. Part-way through an unfold, a together group that has unfolded can outnumber what is still folded, and the tray jumps away from a fold marker that is still there. MUL-10's "because they have parted or unfolded, the tray closes" suggests unfolding was meant to close it. The build follows, as the rule says.
- **Affects:** MUL-10, MUL-07 · **Type:** spec gap
- **Disposition:** open. Keep following, or say a fold marker's tray follows only its own fold marker and closes when it goes.

### F-263 · Do docked things count for a tray's following when their row is closed?
- **Observation:** In a larger marker, docked things sit under their station's row and are listed only when that row is open (MUL-09), and MUL-04 says they aren't counted there. For MUL-10's tally the build counts them either way, so a station with two docked ships outweighs two other things.
- **Affects:** MUL-10, MUL-04, MUL-09 · **Type:** spec gap
- **Disposition:** open. Accept, or count only the rows shown, or the station as one.

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

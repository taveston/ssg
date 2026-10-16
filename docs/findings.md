# Findings

Observations from building and playing the prototype, in two sections. **To triage** holds findings waiting for a decision at the next design session. **Parked** holds findings that can't be decided until a later feature is designed, each with a line saying what brings it back. Once a finding is settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings go under To triage and take the next free number from F-246.

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

### F-200 · How the coast and traffic paths are drawn where the spec is silent
- **Observation:** The coast of a traffic path (PLNR-62) is drawn at `traffic_path_alpha` too, at `consequence_width_px`. Every drawn path has its start dot, traffic included. The coast and its label are drawn only while their event is still ahead; once a coasting ship has passed it, only the crossing it flew is drawn, as PLNR-88 gives no later event. A hovered place's preview (PLNR-12) is a Rendezvous, so it has no coast.
- **Affects:** PLNR-61, PLNR-62, PLNR-67, PLNR-88 · **Type:** spec gap
- **Disposition:** open. Accept, or say what a coast shows after its event.

### F-202 · Dashes crawl along a path in flight
- **Observation:** Dashes and dots are laid on screen from the start of the dashed part, so their length stays `path_dash_px` at any zoom (PLNR-61). In flight that start is the ship, which moves, so the dashes slide along the path as the ship flies and as the camera moves. Laying them from the path's end would hold them still while the ship flies, but not while the camera zooms.
- **Affects:** PLNR-61 · **Type:** idea
- **Disposition:** open. Watch in play; anchor the dashes to the arrival end if the crawl distracts.

### F-203 · Small choices on the map marks
- **Observation:** The ghost's and the coast's labels are in the data face at 12 px, a marker name's size. The coast's label sits beside its event's point as if beside a marker of no size. A ghost's dashes are `ghost_dash_px` long and as far apart, with their count rounded so they meet evenly round the circle. Paths are world lines like the orbit rings, so they pass under a true-size disc rather than over it (MRK-12 draws near arcs over discs only for rings).
- **Affects:** PLNR-63, PLNR-65, PLNR-67, PLNR-69 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-205 · Where the direction chevron sits exactly
- **Observation:** MOT-14 doesn't say how "`chevron_gap_px` beyond the mark's edge" is measured on a curved ring. The build walks round the ring to the point whose straight-line distance from the mark's point is the mark's reach that way, plus the gap, plus half the arrowhead's length; the arrowhead has a right-angled tip, so it is half as long as it is wide. It turns along the ring there. A ring too small on screen to reach that far shows no chevron. The label moves to the mark's left only when the chevron's box would overlap the label's box on the right. On a selected moon the chevron sits close to the brackets.
- **Affects:** MOT-14 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-206 · How an exposure ring's label is placed
- **Observation:** F-204 is fixed: a ring's label is now placed by LBL-01 to LBL-04 like any other label, so it keeps clear of other labels and pills and drops its period line first (LBL-03). Its gap is measured from the mark's box (HOV-07), and the chevron counts as part of the mark (MOT-14). Three choices the spec doesn't make. The whole label (name and period) is centred beside the mark, as a symbol's two-line label is; before, the name alone was centred on the ring point. On the mark's left the lines are left-aligned, as other labels are. A shared orbit's label ranks as its highest-ranked member, or as hovered while the orbit is hovered (LBL-02).
- **Affects:** LBL-01, LBL-02, MOT-02, MOT-14 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-207 · Labels keep clear of other markers' boxes
- **Observation:** LBL-01 says labels never overlap any marker. The build keeps every label clear of each marker's box (HOV-07, `bracket_margin_px` round it), not just the drawn marker, so the brackets round a hovered or selected marker never cross another object's label. Symbols already worked this way (SYM-13). In a crowded view a few more labels drop their second line or hide.
- **Affects:** LBL-01, HOV-07 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-210 · Ticks crowd into a bar near the horizon
- **Observation:** REF-05 puts a tick at every multiple of the scale bar's distance. Towards the plane's horizon, seen near edge-on, they crowd without end. The build stops an arm's ticks at the first pair closer than 4 px, and at 64 per arm. At tilt 85° the last few still read as a short solid bar before the dots run on to the "N".
- **Affects:** REF-05 · **Type:** spec gap
- **Disposition:** open. Accept, or stop the ticks sooner (for example once two are closer than `ref_tick_px`).

### F-211 · Small choices on the frame cue
- **Observation:** The spec leaves these open, so the build chose: the "N" is in the data face at 10 px, at full strength rather than faded with the arm, and shows only while the north arm is longer than `ref_n_inset_px`. A tick is 1 px wide. The arrowhead is 0.75 of its length wide. The vertical axis is drawn at the north arm's strength. The upper half's length is measured from its start (the pole or the gap), not from the centre. An arm's gap is a distance from the centre, not from the box's edge along the arm. The vertical axis is not cut at the map area's edge.
- **Affects:** REF-03, REF-04, REF-05, REF-06 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-212 · The cue lies over the rings, and labels don't avoid it
- **Observation:** The orbit rings, tails, exposure rings and the belt are world lines drawn under the whole overlay, so the cue's arms and lower half, though under every body, lie over them. Labels don't keep clear of the cue: the label halo breaks it where they cross, as for rings. A frame body's label can sit across the upper half of the vertical axis, as Morrow X's does when the label goes above it.
- **Affects:** REF-09, MRK-12, LBL-01 · **Type:** spec gap
- **Disposition:** open. Accept.

### F-215 · The paths' marks and the frame cue shared layers
- **Observation:** The frame cue's arms and the paths' start dots were both drawn one layer below the bodies, and the cue's "N" and the ghosts both just below them, so which drew on top was undefined. The spec puts both the cue and the paths "above the rings" without ordering them. The build now draws, from the bottom: the cue's arms and lower axis, its "N", the paths' dots, the ghosts, then the bodies. Every layer is now named in `view/layers.rs`.
- **Affects:** MRK-12, REF-09, PLNR-69 · **Type:** spec gap
- **Disposition:** open. Accept, or put the cue over the paths.

### F-216 · Map text sizes are set in many places
- **Observation:** The spec names the type roles (MRK-02) but no sizes. Each module sets its own: a marker's name 12 px and its second line 10 px, an exposure ring's label 11 px and its period line 10 px, an edge tab's name 12 px, a path's label 12 px, the scale bar 10 px, the cue's "N" 10 px, the breadcrumb and date 14 px, buttons 13 px. A ring's label is 1 px smaller than a marker's name for no stated reason.
- **Affects:** MRK-02, MOT-02, LBL-01 · **Type:** spec gap
- **Disposition:** open. Options: add a size column to the type roles table (map label, map second line, panel text, bar text) and read the sizes from one place in `view/fonts.rs`; or accept as is.

### F-219 · At heading 0 the vertical axis lies on the north arm
- **Observation:** At heading 0, at any tilt between straight down and edge-on, the north arm and the plane's normal both point straight up the screen. They start at the same gap beyond the frame body's box, with the same dot size and pitch, so the axis's upper half sits dot for dot on the north arm. Only the arrowhead, the ticks and a slightly stronger fade show there are two strokes. A few degrees of heading separates them. The build follows REF-06 as written, with no offset or tint.
- **Affects:** REF-06, REF-01 · **Type:** spec gap
- **Disposition:** open. Check on screen. If it reads as one line: a different dot pitch or size for the axis, dots offset by half a pitch, or a tint (spec open points).

### F-220 · The vertical axis can run past the top of the map
- **Observation:** At `ref_up_fraction` 0.45 of the map height, the axis's upper half can run past the top of the map area when the frame body sits high on screen at a steep tilt. With Halden IV mid-screen at tilt 75 the arrowhead already reaches just under the top bar. The arms are clipped at the map edge or the horizon (REF-03), but REF-06 sets no limit for the axis, so the build doesn't clip it.
- **Affects:** REF-06, REF-03 · **Type:** spec gap
- **Disposition:** open. Clip the upper half at the map's edge as the arms are, or accept.

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

### F-229 · Where the fold marker sits
- **Observation:** GRP-05 and MUL-06 say only that the fold marker sits beside its parent. The build puts its front frame level with the parent's centre, to its right, with the two boxes just touching, as mock-up 18 does. Labels take the right slot first, so planet labels now sit on the left. In Halden IV's own frame its label had no free slot and was hidden. The old pill sat lower right and left the right slot free.
- **Affects:** GRP-05, MUL-06, LBL-01 · **Type:** spec gap
- **Disposition:** open. Keep it level and to the right; move it below right like the pill (about 27 px down to clear a one-line label); or put it on the left.

### F-230 · Do docked things colour a larger marker's frames?
- **Observation:** MUL-04 says things docked at a station inside a larger marker sit under its row and aren't counted. MUL-02 doesn't say whether they count for the frames. The build leaves them out of the frames too, for fold markers, overlap clusters and shared-orbit identity marks. So Iver's fold marker reads as an own "1", not a neutral one. A hostile ship docked inside a larger marker then shows only in the tray.
- **Affects:** MUL-02, MUL-04 · **Type:** spec gap
- **Disposition:** open. Accept, or count docked things for the frames, at the cost of a "1" in another thing's colour.

### F-231 · The identity mark of an orbit holding one station with ships docked
- **Observation:** MOT-14 gives a shared orbit a multi-marker. If the orbit's only thing is a station with ships docked at it (Iver's High band: Iver Yard, Corvid and the Pellam trader), MUL-04 would make that a plain multi-marker counting 1. The build draws the station's own station marker instead: its frame and bar, the docked count and a back frame.
- **Affects:** MOT-14, MUL-03, MUL-04 · **Type:** spec gap
- **Disposition:** open. Accept, and say so in MOT-14.

### F-232 · Things joined into a marker have no tail
- **Observation:** MUL-05 says a joined thing draws no tail of its own and leaves its orbit ring unchanged. In dot mode the build drops joined things' tails; in ring mode they still light the exposure ring. So in dot mode a together or overlap marker (the TEST-12 convoy, a cluster in the High band) has no tail, and on a running clock it reads as standing still.
- **Affects:** MUL-05, MOT-01 · **Type:** spec wrong
- **Disposition:** open. Give a together or overlap marker one tail of its own, from its centre.

### F-233 · Two stations together
- **Observation:** MUL-03 defines docking only for ships and groups. When two stations are together (PHAS-02), the build makes the first the lead and treats the other as docked at it: it is counted as docked and its own symbol is hidden. No content has this case yet.
- **Affects:** MUL-03, MUL-05 · **Type:** spec gap
- **Disposition:** open. Say what two stations together show.

### F-234 · Fold markers never fade
- **Observation:** A fold marker is always drawn at full strength, even when its parent's dot has faded into the exposure ring (MOT-11). Other multi-markers fade with the strongest thing in them. The old pill behaved the same.
- **Affects:** GRP-05, MOT-11 · **Type:** spec gap
- **Disposition:** open. Accept, or fade the fold marker with its parent.

### F-235 · A shared orbit's identity mark while the planner is open
- **Observation:** MUL-12 says a click on any multi-marker other than a fold marker picks nothing. MOT-14 says the identity mark targets what its ring targets, and PLNR-11 says a shared ring picks that orbit's band. The build follows MOT-14 and PLNR-11: the mark picks the band and opens its tray.
- **Affects:** MUL-12, MOT-14, PLNR-11, HOV-05 · **Type:** spec wrong
- **Disposition:** open. Keep this and name the shared orbit's mark beside the fold marker in MUL-12, or make the mark pick nothing.

### F-236 · Rows for a group's members
- **Observation:** A member has no inset of its own, so a member row, clicked or stepped onto, selects its group, and the tray highlights the member row. With the planner open, member rows are dimmed, since a member is wherever its group is and a group can't be picked. With the planner open, Up and Down move only the highlight; picking takes a click. With no row selected, Up or Down selects the first row. Left on a member closes its group and selects the group.
- **Affects:** MUL-09, MUL-10, MUL-12 · **Type:** spec gap
- **Disposition:** open. Copy into the spec if fine.

### F-237 · Esc with both a tray and the planner open
- **Observation:** INS-03 doesn't say which Esc closes first. The build closes the tray first, then ends the planner, then clears the selection.
- **Affects:** INS-03 · **Type:** spec gap
- **Disposition:** open. Accept, and add the order to INS-03.

### F-238 · Shared orbit trays: dot mode and header
- **Observation:** HOV-05 opens the identity mark's tray, but in dot mode there is no identity mark, so a click on a shared ring opens no tray; it only closes any tray that is open. The identity mark's tray uses the "N OBJECTS" header meant for any other marker, rather than naming the orbit.
- **Affects:** HOV-05, MUL-08 · **Type:** spec gap
- **Disposition:** open. Say what a shared ring's click opens in dot mode. A header such as "HIGH ORBIT · N THINGS" may read better.

### F-239 · A fold marker's tray opening left covers its parent
- **Observation:** The fold marker sits to the right of its parent. When its tray opens to the left (for example Morrow with the inset open), the tray covers the parent's marker and label.
- **Affects:** MUL-08, GRP-05 · **Type:** spec wrong
- **Disposition:** open. Keep it, or for a fold marker open the tray above or below instead of to the left. Bears on F-229.

### F-240 · The panel fill shows the map through the tray
- **Observation:** The panel fill is 0.97 alpha, and blending happens in linear light, so bright labels and the belt under the tray show faintly through it (about 15% on a white label). The inset uses the same fill, but it seldom sits over the map.
- **Affects:** MUL-08, INS-09 · **Type:** idea
- **Disposition:** open. Make the tray's fill opaque.

### F-241 · When a changing cluster stops existing
- **Observation:** MUL-10 closes the tray when its marker stops existing. Together and overlap markers are known by their first thing, so if that thing leaves while the others stay, or a new thing joins ahead of it, the build treats the marker as gone and closes the tray, though most of its things are still on one marker.
- **Affects:** MUL-10, MUL-05 · **Type:** spec gap
- **Disposition:** open. Accept, or say the tray follows the marker that keeps most of its things.

### F-244 · At system scale the fold marker dwarfs its planet
- **Observation:** In play, the multi-marker looks fine on its own, but in the system overview a planet's fold marker looks massive next to the planet. The fold marker is a ship symbol's frame (`ship_symbol_px`, 22 px) with a back frame offset 4 px, so about 26 px across, sitting beside a planet marker of `planet_marker_px` (12 px). Every planet with satellites (Halden IV, Iver, Morrow) then reads as a small dot with a big badge, and the badge, not the planet, draws the eye. A fold marker holding only natural bodies (Morrow's "11") is the same size as one holding ships.
- **Affects:** MUL-01, GRP-05, MRK-03 · **Type:** spec wrong
- **Disposition:** open. Seen by the user in play. Options: a fold marker of its own size (a tunable near the planet marker's), or a smaller one only when it holds natural bodies alone. Bears on F-229 (where it sits).

### F-245 · The outline follows a selected ship that moves
- **Observation:** INS-05 opens the path to the selection only when the selection changes. A selected ship that arrives at another body would then drop out of the outline, because its new parent and orbit row are still collapsed. The build also opens the new path (the parent entry and the named orbit's row) when the selected body's parent or named orbit changes, and scrolls it into view; other entries keep their state. That scroll can override where the player had scrolled the outline, on departure and on arrival.
- **Affects:** INS-05, RAIL-09 · **Type:** spec gap
- **Disposition:** open. Accept and say so in INS-05, or open the path without scrolling.

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

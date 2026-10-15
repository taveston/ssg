# Orphan Worlds: Orbital Transfers

**In short:** ships fly on rails along trajectories computed once, when an order is given. The model is patched conics with the planet-centred legs collapsed to a cost and a short delay, so the only geometry is the arc round the primary. High-thrust drives get a two-burn arc (a Lambert solution); low-thrust drives get an estimated spiral. The player plans a transfer with two sliders, depart and travel time, and sees the path, the dates and the delta-v change as they move. A destination is always a place (a body, station, gate, site or ship), never a bare orbit; round a body there are two bands, low and high, plus the orbits of its satellites. Ships sharing a band are one marker on the map and a list in the inset, but they are not at one place: phase round a band is real, and meeting a ship in your own band is a phasing manoeuvre that trades delta-v for time. A scheduled transfer is a visible, cancellable order on the ship. A ship in flight can be re-tasked at any moment, because the planner only ever needs where the ship is, how fast it's going and what it has left. Tactical positioning reduces to which place, which band, and whether the ship rendezvouses or intercepts.

*Status:* draft. **Direction** unless marked otherwise. The simulation of sections 2, 4 and 8 is specced in `docs/spec/transfers.md`. The single-ship planner (3.1 to 3.3, 3.5 and what the player sees in 4.4) is specced in `docs/spec/planner.md`; the return reserve, multi-leg plans, other people's ships, groups, re-tasking and gravity assists are not. Mock-ups 9 to 12 in `docs/ideas/presentation/mockups/` show the planner, trajectory states, the return reserve and a refuel plan.

*Companion documents:* *Drives & Transit* (drive performance, transit times, gate placement, entry tolerance), *Gates*, *Combat, Raiders & Diplomacy* (what tactical positioning feeds), *Component Stats & Ship Assembly* (delta-v and acceleration per design), *Presentation & Visual Style* (how paths are drawn).

---

## 1. Principles

- **Comprehensible first.** The player shouldn't need to know what a Hohmann transfer or a synodic period is. The interface speaks in dates, days, and delta-v as a fuel gauge. The mechanics are real underneath; the vocabulary isn't.
- **A strategy game with real constraints, not a flight simulator.** Abstraction is fine wherever it keeps the trade-offs (time versus delta-v versus waiting) and drops the bookkeeping.
- **One planner, one kind of input.** Everything the planner needs is a *state* (position and velocity at a time, relative to a primary) and a delta-v budget. A parked ship's state is its body's. A ship in flight reads its state off its rail. So planning, scheduling and re-tasking are one code path.
- **Deterministic and cheap.** Orbits are deterministic, so a trajectory is fixed the moment it's scheduled and evaluated analytically at any time. No per-frame integration, no drift, and save games store orders, not positions.

## 2. The model **(Direction)**

### 2.1 Patched conics, with the planet legs collapsed

A transfer between two places has three parts: leave the departure body's orbit, cross to the target, enter the target's orbit. Only the middle part is drawn and simulated as geometry.

- **The crossing** is a conic arc round the primary (the star for planets, the planet for moons and stations). This is the trajectory the player sees and tunes.
- **Departure and capture** are not simulated. Each body has a *departure cost* (delta-v, from its mass and the parking orbit it's reached from) and a short *approach time* (days). The planner adds the cost to the burn and the time to the trip. Leaving Halden's low orbit costs about 2.8 km/s; leaving Arden Highport much less. Ships based at highports and depots are cheaper to send, which is the reason to build them (*Drives & Transit* 2).
- **Why not simulate the hyperbolic legs:** at system scale they're a few hours inside a marker, and they'd add a sphere-of-influence hand-over that is the classic source of bugs and confusion. The cost and the delay are what matter strategically.
- **Bodies never bend a coasting ship.** The same choice, stated the other way round: gravity acts only through the departure and capture costs. A ship coasting on a free conic that crosses a planet's sphere of influence keeps its conic. No capture by accident, no hand-over, and no gravity assist unless the player asks for one (2.6). The crossing is an *event* (a "passing Halden IV" alert, a detection, an engagement if hostile), never a change of path. This is the one place the model is knowingly unphysical, and it is worth it: every trajectory stays a single analytic arc from the moment it's planned.
- **Decided (2026-10-14): one order, cost plus delay.** Staging through a highport is a choice the player makes by basing ships there, not by issuing two orders a trip. Revisit only if deep-well launches feel free in play.
- **Nested primaries.** Moon-to-moon and station-to-station trips round the same planet use the planet as the primary and the same model. A trip from a moon to another planet is a moon-departure cost plus the star-centred crossing. Within a planet's frame nothing changes except the mass.

### 2.2 Ships fly on rails

A ship's trajectory is a list of segments, each with a start day, an end day and a way to evaluate position and velocity at any day in between:

| Segment | Position and velocity from | Used for |
|---|---|---|
| Parked | the body it orbits (its own circular orbit round it) | ships at rest |
| Conic | orbital elements about a primary (Kepler's equation) | high-thrust crossings, coasts |
| Spiral | a parametric curve from the Edelbaum estimate (section 2.4) | low-thrust crossings |

A burn is a step in velocity at the boundary between two segments, charged to the ship's delta-v at that moment. Flying a ship is reading its segment for today's date. This is the same as what bodies already do (`sim/orbit.rs` evaluates a circular orbit from the clock alone), extended to ellipses, hyperbolae and spirals.

### 2.3 High-thrust transfers: two burns

For chemical, NTR, pulsed fission and fusion drives the crossing is a two-burn arc: given the departure state at day *t₀* and the target body's position at day *t₀ + T*, the Lambert problem gives the unique short-way arc, and the two burns are the difference between the arc's end velocities and the departure and arrival velocities. Delta-v = burn 1 + burn 2 + departure cost + capture cost.

- **Two parameters, two sliders.** Departure day and travel time fully determine the arc, which is exactly why the planner can be two sliders with live feedback (section 3).
- **Burn time (decided 2026-10-14).** Whether a drive is impulsive or low-thrust is a flag on the drive in the catalogue, not a computed threshold: NEP, solar thermal and sails are low-thrust (2.4); everything else is impulsive. An impulsive drive's burn time (delta-v ÷ acceleration) is added to the trip and the arc is drawn from the end of the burn, so a pulsed-fusion ship's month-long burn costs a month. Two code paths, both needed anyway, and no third.
- **Mid-course burns** fall out of re-tasking (section 5): a new plan from a state that is already in flight is a coast followed by a burn.

### 2.4 Low-thrust transfers: an estimated spiral

NEP, solar thermal and sails can't be planned as two burns, and solving true low-thrust trajectories in-game is both hard and unnecessary. Use the Edelbaum estimate: for a coplanar circular-to-circular transfer, delta-v ≈ the difference in orbital speeds, and the time is delta-v ÷ acceleration. Phasing is handled by adding a wait or by stretching the spiral. The rail is a smooth parametric curve that interpolates radius and angle over the trip, so it looks like a spiral and arrives where the target will be.

- **This is the physics, in-game.** The estimate under-reads reality by about 10% (*Drives & Transit* 9), but the game's own rules define its truth. What matters is that low-thrust is slow, cheap per tonne, and can leave on any day, and the estimate preserves all three.
- **Low-thrust ships don't wait for windows.** Their planner has one meaningful slider: departure day. Travel time follows from acceleration. That is the "queue-free bulk hauler" of *Drives & Transit* 5, and the planner should say so plainly ("Travel time is fixed by this drive").

### 2.5 Orbit complexity: keep bodies circular and coplanar **(Decided for now)**

Bodies stay on circular, coplanar orbits (SCL-04). Ship trajectories are not circular: they are ellipses, hyperbolae and spirals, and that is where all the visual variety comes from.

- **Why.** The planner is unchanged by body eccentricity (Lambert takes two points, however the bodies move), so there is no simplification to be had on the maths side, but there is a lot of comprehensibility to lose. Concentric rings read at a glance; windows recur on a fixed period per pair of bodies, so the player learns them. Inclination is worse: a tilted orbit lies on a top-down map, and plane-change costs are the least intuitive part of orbital mechanics.
- **Candidate for later:** mild eccentricity on a few bodies for flavour (captured moons, comets, salvage on stretched orbits), drawn as the ellipse they are, with the same Kepler evaluation the ships use. Never inclination for planets. Stations and ships in parking orbits are always circular.
- **What the player feels instead.** The time–delta-v–waiting trade, the synodic rhythm of windows, and the difference between a ship that must wait and one that can leave now. That is enough orbital strategy for a game about running a state.

- **Specced** (2026-10-14) as RAIL-01 to RAIL-09 and XFER-01 to XFER-12 in `docs/spec/transfers.md`: segments and their evaluation, collapsed planet legs with costs and approach days, two-burn Lambert crossings with burn time, Edelbaum spirals, bodies never bending a coast. Two choices the spec makes that this section left open: the departure and capture cost of a band is the escape cost from a circular orbit there, √(μ/r)(√2 − 1), with no Oberth credit, so the all-in figure for a trip between high orbits is a little above the tables in *Drives & Transit* 5; and the approach days of a band are the coast from the band to the Hill radius at escape speed.

### 2.6 Gravity assists **(Candidate)**

Many players have heard of a gravity assist, so a simple version is worth having, as long as it stays an order the player gives rather than something that happens to them.

- **Turn sizes at Halden IV** (0.82 g), arriving at 2, 5 and 10 km/s relative: Low (500 km up) 132°, 78°, 35°; High at 5 radii 89°, 31°, 10°; High at 10 radii 65°, 18°, 5°; High at 20 radii 43°, 10°, 3°. A Low pass is a hard turn; a High pass at 10 radii or more is a small one. Hence the rule in section 7: an Intercept through Low always takes the turn, an Intercept through High never does.
- **The rule.** A flyby is a *waypoint* in a transfer ("Halden IV via Morrow"). At the waypoint the ship's velocity relative to the body may turn by up to a maximum angle set by the body's mass, the ship's arrival speed and the low band radius (the closest it may pass). The turn is free; its size is the assist. One formula, no extra physics, and the arc shows as a kink at the body labelled with the delta-v it saved.
- **How much it's worth.** For a ship arriving at a typical NTR relative speed of about 5 km/s: Morrow, Jupiter-class, about 8 km/s, close to a whole NTR tank; Halden IV about 3 km/s; Iver and the inner planets about 1 km/s or less. So only a giant is worth routing past, and only when it sits between places ships go. In the recommended Corvane layout (Morrow at 4 AU, gates at 2 AU) nothing does, so the rule is dormant there. It comes alive in systems where the generator puts a giant inside the gate ring or between two inhabited worlds, which is a good source of variety: a system with a cheap route and a system without one.
- **Cost.** A via-transfer is two arcs joined at a moving body with a matching condition between them, so the planner needs one more search dimension (when to arrive at the waypoint) and a constraint. That is a real step up from the two-slider planner, which is why this is a candidate and not part of the first build. The player's view stays the same two sliders, with the waypoint chosen from the destination step ("via" on a body's context menu).
- **Rejected:** a flat delta-v credit for flying past a body. It is simpler, but a flyby that helps whichever way you're going is the kind of abstraction that teaches the wrong thing.

- **Specced** (2026-10-16) as XFER-13 to XFER-15, for an Intercept through Low only: the turn is applied at the end of the Intercept, so the ship leaves on a conic bent by 2 arcsin(1/*e*) in the prograde sense, at no cost. Choices the spec makes: the Intercept goes round the body prograde, as a capture does, which also gives the closing speed at a station (the speed at the closest point less the band's circular speed); the turn is not an extra search dimension, because a one-leg plan has no later leg to join it to; and the assist as a waypoint in a multi-leg plan ("via") is still a candidate. Separately, XFER-07 now refuses any plan whose crossing, or whose coast after an Intercept, would hit its primary.

## 3. Planning a transfer **(Direction)**

The flow is: select a ship, pick a destination, tune two sliders while the map shows the result, click Schedule.

### 3.1 Steps

1. **Select the ship** (or group). The inset shows its delta-v remaining and its drive's kind.
2. **Choose a destination:** a Transfer button on the inset, then click a body, station, gate or orbit ring on the map. Hovering a candidate previews the fastest affordable trip.
3. **Tune.** Two sliders in the inset, with the trajectory, dates and delta-v updating as they move:
   - **Depart:** from the earliest possible day (now plus any preparation) to one synodic period of the pair ahead plus a margin, so the next window is always on the slider (decided 2026-10-14); for a low-thrust ship, which has no windows, a fixed `depart_range_days` (180). The slider track is coloured by the delta-v the cheapest trip on that day needs, so windows show as bright bands without being named: a good day looks good.
   - **Travel time:** from the fastest the budget allows to a slow, cheap trip. Same colouring.
   - Readouts, always visible: **Departs** (date, and "in N days"), **Arrives** (date, and "in N days"), **Delta-v** as a bar: used of available, with the remainder after the trip. Over-budget settings are reachable on the slider, shown in the warning colour, and Schedule is disabled with a one-line reason ("Needs 11.2 km/s, has 9.9").
   - **Presets:** *Fastest*, *Cheapest*, *Next window*. Most orders will be one click on one of these.
4. **Schedule.** The order is stored as (ship, destination, departure day, travel time); the trajectory is computed and attached. The ship is unchanged until the departure day.

### 3.2 Vocabulary

The interface says Depart, Travel time, Arrive, Delta-v (with a km/s figure) and Window. It never says Hohmann, synodic, Lambert, phase angle, apoapsis or inclination. Delta-v stays, because it is the one number that bids, designs and refuelling already turn on; the delta-v bar makes it a fuel gauge. Decided (2026-10-14): keep "delta-v", with a tooltip on first use ("Delta-v is how much a ship can change its speed before its tanks are dry. Every trip spends some"). Softer words are wrong in some case; the bar does the explaining.

### 3.3 On the map

- **The preview** is the arc the ship would fly, drawn from the departure point to the arrival point, with the arrival point marked on the target's orbit ring at its arrival-day position (dot mode) or on the ring itself (exposure-ring mode), as *Presentation* already says.
- **The arc is drawn in the geometry of its own dates.** It starts where the ship will be on the departure day and ends where the target will be on arrival, each marked on its ring. The ship's symbol stays where the ship is now; once the order is scheduled, its body carries it round the ring towards the departure mark, so the countdown shows on the map. Drawing from today's positions would be wrong in a visible way: a two-burn arc only meets the orbit at a tangent from its true departure point.
- **The marks are ghosts.** The arrival mark is the target's own marker drawn faint and dashed at its arrival-day position, labelled with the date ("arrive day 93"); while planning, the departure body gets the same ghost at the departure day. A ghost reads as "Iver, but later" without a legend. Ghosts exist only for the order being planned or the selected order, never for every arc. Their labels take part in label placement (LBL) below the selection, so they give way to the target's own label when the two sit close.
- **Consequences are drawn in the warning colour.** The part of a path the player didn't ask for, such as the coast after an Intercept, is drawn dotted in the warning colour with its consequence labelled ("crosses Halden's orbit in 140 d"), so a planned path reads as two things: what you ordered, and what follows from it.
- **What is drawn without selecting (candidate).** A ship in flight always shows its arc, thin and faint: it is what says the dot in open space is going somewhere. This is the traffic layer (*Presentation* 2), on by default. A scheduled order shows nothing on the map until its ship is selected, only the countdown in the ship's label; the ship hasn't moved and a dashed arc per queued convoy would be clutter. Selecting a ship brightens its arc and adds the marks and ghosts; hovering a ship in flight brightens its arc.
- **One preview, not a fan.** The earlier candidate (a fan of alternative paths, each labelled) is replaced by one live path plus the coloured slider tracks, which carry the same information with far less clutter. *(Proposed change to Presentation, Motion.)*
- **Scheduled transfers** show the planned arc dashed from the ship's departure point, and the ship's label carries "Departs in N d". In flight, the arc is solid behind the ship and dashed ahead, with "Arrives in N d". The velocity leader (SYM-11) applies once ships move.

### 3.4 Other people's ships **(Candidate)**

The state knows a track, not an order. A foreign ship in flight is drawn with a *projection*: its known conic carried forward with no burns assumed, dashed in its affiliation colour, up to where it crosses something ("intercepts Halden IV in 12 d at 9.1 km/s"). Whether it will rendezvous or intercept is not knowable in advance and the map never pretends it is: the player reads intent from speed and class, which is a skill worth having. A weak track (SYM-01) widens into the uncertainty region the presentation already proposes, along the projection. An observed burn snaps the projection to the new conic and raises an alert ("K-3 changed course"). The inset can help without guessing: "Could rendezvous at Halden IV: needs 4.2 km/s" when the class's delta-v is known.

### 3.5 Scheduled orders

- A ship with a scheduled transfer is visually cued (the dashed arc and the label) and the inset shows the order with **Cancel** and **Change** (which reopens the sliders at the stored settings).
- On the departure day the order is re-validated against the ship's actual state (it may have been refuelled, damaged or moved). If it still holds, the ship departs; if not, it doesn't, and an alert says why.
- Delta-v is *reserved* when scheduled and *spent* at each burn, so the inset can show "9.9 available, 7.5 reserved".
- Alerts on departure, on arrival, and when a window the player was shown is about to close without an order. **The planner never pauses the game (decided 2026-10-14).** Whether an alert pauses is the alerts system's setting, not the planner's. Keeping the planner live while the clock runs is cheap: the coloured slider tracks are about a thousand two-burn solves, microseconds each, refreshed once a game day; the preview arc is one solve per slider move.
- **Specced** (2026-10-15) as PLNR-01 to PLNR-88 and PLNR-90 to PLNR-94 in `docs/spec/planner.md`: 3.1 to 3.3 and 3.5 apart from alerts, for one ship. Choices the spec makes that this section left open:
  - The arrival modes are Rendezvous and Intercept, each with a tooltip (PLNR-94). They were Rendezvous and Intercept; the specs and these documents use the new words throughout (2026-10-16).
  - The Travel time slider runs from `travel_min_factor` to `travel_max_factor` times the trip that needs least fuel, not from "the fastest the budget allows", so the track doesn't move when fuel changes. The colours and the Fastest preset show what is affordable.
  - A scheduled order draws its path only while its ship is selected, and the ship's label always carries the countdown. This settles the two readings of 3.3 and 3.5.
  - The Depart readout is the day the ship leaves its orbit, wait included (4.4).
  - Fastest is the earliest arrival that is affordable; Cheapest is the least delta-v; Next window is the first day whose best trip is within `window_margin_kms` of the best of the whole range.
  - Ghost labels rank below every other label.
  - The "In orbit" outline stays in the panel, so a folded moon or a ship in a band can be picked as a destination.
  - Only the state's own ships draw paths. Other people's ships (3.4) wait.
  - A ship in flight can't be picked as a destination, unless it is the state's own and coasting with no order. This keeps the planner from aiming at a path the player can't know, or one an order could change.
  - The planner opens for a ship parked in an orbit. A ship in flight or coasting can't be ordered until re-tasking (6) is specced.
  - The Delta-v tooltip shows on hover; showing it once on first use waits for remembered player settings.
  - Intercept is offered for a station, because a drive-by raid that doesn't spend delta-v matching the station's orbit is what an Intercept is for (section 7). The solver trims the pass to meet the station (PHAS-15) and the planner shows the trim; how a ship avoids hitting what it meets is for Conflict. A raid on a station in a Low band needs an Intercept through Low (2.6), which is not yet specced.

### 3.6 The return reserve **(Direction)**

The planner only ever showed the cost of the next thing, and delta-v is a budget, so a player could send a ship on an intercept and only learn afterwards that it can't come home. The fix is to make "can it get back" part of every preview, computed, never left to arithmetic.

- **Home** is the nearest place a ship can refuel: a friendly station, depot or yard with fuel (*Economy*; the hydrogen depot in *Drives & Transit* 9). Which places count is data the economy owns; the planner only asks for the list.
- **One fuel for now (decided 2026-10-14).** Until fuel logistics are designed, there is one propellant, or every port that refuels holds every kind. So "a port with fuel" is one question, not one per drive.
- **The reserve** is the delta-v the cheapest trip home from the plan's end state needs, allowing a wait of up to `return_window_days` for a good window (one global value for now, decided 2026-10-14; a per-order "willing to wait" field only if people ask). It is computed from the end state, so after an Intercept it starts from a coasting conic, which is exactly when it matters.
- **The delta-v bar shows three things:** this trip, the reserve, and what's left. The reserve is a shaded band at the far end of the bar; a trip that eats into it turns the band the warning colour, and a trip that leaves less than the reserve makes the order **one-way**. The readout says so in words: "Return: 7.1 km/s, waiting up to 60 d" or "One-way: can't reach a friendly port after this".
- **One-way is allowed, but never by accident.** The Schedule button reads "Schedule (one-way)" and asks once. A standing rule per ship class (candidate: civilians and tankers refuse one-way orders without an override; warships don't) keeps haulers out of trouble without nagging the fleet.
- **In flight,** the reserve is recomputed as delta-v is spent and the ship's inset shows it, so "can this fleet still disengage" is a glance, not a sum. Retreat (section 7) is a re-task that the reserve guarantees.

### 3.7 Multi-leg plans **(Candidate)**

A plan is a list of legs; each leg is a transfer planned from the predicted end state of the one before. Nothing new in the planner: the state it starts from is the previous leg's end rather than the ship's present.

- **Most plans are one or two legs:** out, or out and back. A **Round trip** toggle adds the return leg automatically (the cheapest return within `return_window_days`), so an intercept or a flyby is planned as go-and-come-back from the start and the reserve question answers itself.
- **Add leg** appends a destination; the legs show as a short stack in the inset, each with its own sliders when selected. The delta-v bar stacks the legs in order, with the reserve still shaded after the last. On the map the arcs join end to end, with a ghost at each handover.
- **Waypoints** are legs with no stop: an Intercept at a body (section 7), or a via-flyby that takes an assist (2.6). "Halden IV via Morrow" is a two-leg plan whose first leg is an Intercept.
- **Execution:** legs run in order. Each leg is re-validated when it starts, as a scheduled order is (3.5); if the ship's state no longer fits, the plan pauses there with an alert and the ship holds. A re-task in flight drops the rest of the plan.
- **Groups** plan every leg on the weakest ship, and the reserve too.
- **Refuelling stops.** A Rendezvous at a friendly port with fuel refuels the ship by default (a toggle on the leg turns it off). The planner then plans the next leg from a full tank, and adds a planned dwell of `refuel_days` to the leg's start. The dwell is a known number, so the planner stays exact: whether the port actually has fuel on the day is checked when the leg starts, like everything else, and a dry port pauses the plan with an alert and the ship holding. "Rendezvous at Iver Yard, refuel, then Halden IV" is a two-leg plan with nothing special in it. Fuel stock, price and queueing at a port are the economy's to design; the planner only needs a yes or no and a dwell.
- **Keep it short.** A plan is a few legs, not a tour. If people want long patrol routes, that is a standing order for the conflict doc, not a ten-leg plan.

## 4. Destinations and orbits **(Direction)**

### 4.1 A destination is a place, not an orbit

The player never picks an orbit by its radius. They pick a thing: a body, a station, a gate, a salvage site, or another ship. The planner decides the orbit that "being at" that thing means. This keeps the target step to one click, and it means every ship's position has a name ("High orbit, Halden IV", "At Iver Yard", "At the Ostra gate") rather than a number.

- **Bodies** have two bands: **Low** and **High**. Low is just above the surface or atmosphere: the place for shuttles, ground operations, blockade and bombardment. High is the standard parking orbit, cheap to leave and the default arrival. Both radii come from the body (a tunable altitude for low; a multiple of the radius, capped by the Hill sphere, for high). Candidate figure for high: 10 radii or more. That keeps the capture burn small, which is what High is for, and keeps a flyby's turn there under about 20° at normal speeds, small enough to ignore (2.6). The departure cost in 2.1 is the cost from the band the ship is in.
- **Stations** sit in a body's Low or High band, as ships do. **Decided** (2026-10-15): a player-built station can only be put where a ship can go, so it takes one of the same bands; the Station place is the band it occupies. A station is a point on that ring with its own phase, and a ship docked at it sits at that phase (4.4).
- **Lagrange points** as places for ships and stations to site at. **Candidate** (2026-10-15): the points of a planet-star pair (and of a moon-planet pair) would be extra named places beyond the two bands, useful for relays, depots and ambushes. Open: how a ship transfers to one, what holding station there costs, and whether stations built there fall outside the band rule above.
- **Gates** are a place at the gate's orbit, with the entry rule in section 5.
- **Belts and sites** are points on a ring; arriving means that point's orbit.
- **Ships** can be destinations too (section 7): the target is a trajectory rather than a body, and the planner aims at where it will be.
- **No free orbit as a destination.** A ship can't be sent to "2.3 AU, 40° ahead of Halden". But **coasting** is a state a ship can be in: a ship told to **Hold**, or one that has made an *Intercept* (section 7), is on a free conic round the primary with no further burns. Its place is named by what it's doing ("Coasting, Halden IV → Iver, intercepted Halden IV") and the only thing it can be ordered to do is a new transfer from its current state. That is enough for pickets and ambushes without an orbit editor.
- **Coasting ships drift.** A free conic is an ellipse that crosses planet orbits, or a hyperbola that leaves the system. Neither needs simulating beyond the arc: bodies don't bend it (2.1), and a crossing is an event. The planner shows the consequence of an Intercept before it's scheduled: the arc after the flyby, "crosses Iver's orbit in 40 d", or "leaves the system in 310 d" in the warning colour. A ship on a hyperbola with no delta-v to turn back is lost once it's past the last body it could reach, and the game says so while there's still time. **For now (decided 2026-10-14):** a lost ship raises an alert and is removed before it leaves the visible area. Stranding it where a rescue tug could reach it is the intended later behaviour, once there is something to rescue it with.

### 4.2 Why bands, not a free radius

**Decided (2026-10-14): two bands.** A staging band near the Hill sphere would solve a problem the departure-cost model already prices; add one only when a mechanic needs it.

Three bands' worth of choice (low, high, or at a satellite) covers every strategic reason to be at a body: close enough to act on the surface, parked cheaply, or docked. A free radius adds a number the player must understand, a slider that mostly does nothing, and no decision that the bands don't already make. Moving between bands is an in-frame transfer (2.1, nested primaries): a small order with a real cost, so low orbit of a deep well is a deliberate place to be.

### 4.3 Ships that share an orbit

Every ship in high orbit of Halden IV is on the same ring. At true scale that was always going to be the case: at any zoom that shows more than one planet, every orbit round Halden is inside its marker, and the satellite folding rule (GRP) already collapses them into one count. So sharing an orbit is not a new problem for selection; it is the same problem with more ships in the count. The design follows from that:

- **The list groups by band, not by gap.** Selection, folding and the "In orbit" list treat the ships in a band as one set. Where each ship is round the band is real, and it matters for reach, rendezvous, leaving and combat (4.4).
- **Slots are where ships arrived.** A ship keeps the phase it captured at (the capture point, 4.4), so ships that arrived at different times sit apart on the ring without any rule placing them. A ship that docks at a station takes the station's phase. Ships built at a yard or placed by the generator take the yard's phase or a spread.
- **One list, reached three ways.** The body's inset has an "In orbit" section (INS-07) listing everything there, grouped by band, and that is where a ship is picked. Clicking the body marker or its fold count opens it; clicking a band's ring, drawn as a slow ring or as an exposure ring, opens it with that band expanded. When the orbit is slow enough to show dots, or the game is paused and exposure rings drop back to dots (MOT-12), a single ship can also be picked by clicking its dot. Ships that matter (selected, ordered, in a group) can be promoted to their own marker on the fold, the way the label priority order already favours the selection.
- **Where the detail lives:** the planetary view's orbital layer, where the bands unfold into distinct rings with the ships spread along them, is the place to see and pick individual ships round one body. The system view is for the trip; the planetary view is for the parking lot.
- **Sharing is data, not geometry (decided 2026-10-14).** Natural bodies never share an orbit: two bodies at one radius aren't stable outside special co-orbital cases, and the generator keeps bodies apart anyway. Everything artificial sits in a named orbit of its parent (a band, or a station's orbit) and references it. So the view never groups objects by radius; it draws one ring per named orbit and lists its members.
- **Specced** as SCL-07 (named orbits in the data), HOV-04, HOV-05 and MOT-14 (shared rings, their targeting and the identity mark on exposure rings), with the orbit row in INS-05 and TEST-10, on 2026-10-14. These stand on their own: the spec's named orbits are the bands of 4.1 without the band rules yet.
- **Stacking order:** when several things sit on one point (a station and the ships docked at it), the symbol rules for groups apply (SYM-09, GRP-09): one symbol with a count, and the list in the inset.

- **Specced** (2026-10-14, the rest of this section) as BAND-01 to BAND-08 and XFER-08 to XFER-10: two bands per star, planet and moon, places as named orbits, the capture point as a ship's slot, coasting, escaping ships removed at `lost_beyond_au`, Rendezvous and Intercept (High only), a ship as a destination. A moon is a body with its own bands, so "at a moon" is Body { moon, band }; Satellite as a place is for stations.

### 4.4 Phase: where ships are round a body **(Decided 2026-10-15)**

Ships in one band are not at one place. Each has a phase, its angle round the body, and the ships in a band turn together, so the gap between two of them stays the same until one burns. Phase is real wherever a rule needs a place. Only the planet-centred leg is collapsed, and phase there sets a wait. A group is one place. Closing a gap in your own band takes a phasing leg, which trades delta-v for time. Combat reads true positions.

- **Why it matters.** A band at 10 radii round the reference planet (*Drives & Transit* 2) is about 361,000 km round. The longest sensor range in the catalogue is 50,000 km, about 14% of that ring. Two fleets in one band are in reach of each other only if the gap between them is small, and because they turn together the range never closes on its own. So a defender's place round the ring is part of the defence, and an attacker in the same band has to close the gap.
- **Real in the frame, a wait at its edge.** A crossing with the body as its primary (ring to ring, moon to moon, a station or a ship round one planet) flies between real places: the ship leaves from where it is and aims at where the target will be. A crossing that leaves or enters the body's frame replaces the planet-centred leg with a cost and a delay (2.1), which phase doesn't change. Phase only sets a wait:
  - *Leaving waits for the exit point.* The ship has to be at the right point round the band to burn for its exit, and its phase fixes how long that takes. The wait is under one period: at most about two days at the High band of the reference planet, under two hours at Low. Ships in one band at different phases leave at different times, up to a period apart. A low-thrust ship spirals out from anywhere and doesn't wait.
  - *Arriving in a band takes the capture point.* The ship captures at the closest point of its approach path, which its arrival direction and speed fix. Where it lands round a band depends on where it came from.
  - *Arriving at a station or a ship is timed to meet it.* The arrival shifts by up to one period of the band, so the capture point and the target coincide. That is a small trim of the crossing, which costs next to no delta-v, and the ship lands together with its target. A target in a ring can't hide from an arriving ship, and can't be reached early.
- **Meeting a ship in your own band: the phasing leg.** Drop into a smaller orbit and the ship laps ahead; go to a larger one and it falls back and lets the target come round. Burn into the new orbit, fly whole laps, and burn back at the same point, where the target arrives as the ship does. The ship can leave any day, because the gap never changes: there are no windows. The only choice is the number of laps. More laps cost less delta-v and take longer, and the product stays close to constant: about two thirds of the ring's radius times the angle to make up. At the High band of the reference planet, with the target half a turn ahead: about a day for 1.5 km/s, five days for 0.29 km/s, 18 days for 0.08 km/s. At the Low band the same trade runs in hours (2.5 h for 1.5 km/s, 16 h for 0.23 km/s), and a ship there can only go to a larger orbit, because a smaller one dips below the band. The orbit must stay between the Low band and the Hill sphere.
- **Other bands of the same body.** A ship or station in another band is an ordinary in-frame crossing to its true place on the arrival day. Windows come once per synodic period of the two rings, which is hours between Low and High at the reference planet.
- **Stations are points on a ring.** A station has a phase like a ship. A ship within 10 km of it is docked, and sits at its phase.
- **Groups are one place.** A group (a task force or a convoy) has one phase for all its ships, so it leaves, arrives and fights as one. Groups in a band that aren't together are separate, and each leaves at its own time. To move as one they close the gap with a phasing leg, merge, then take the transfer order. Merging needs the groups together. The merge and detach orders are not specced.
- **Intercepting.** An Intercept through a band has its closest approach at a definite point round the band, from the same formula as the capture point, and the leg records it. An Intercept does not read where ships in the band are. Whether a passing ship meets any of them is for combat, which has the point and the speed to work from.
- **Combat.** It reads true positions, so reach between ships in a band follows from the gap (*Combat Simulation* 1).
- **What the player sees (candidate).** In exposure-ring mode phase is invisible, and a High band turns about every two days, so at a day a second its ships blur into the ring. Departs shows the day the ship leaves the band, wait included. The inset should say where a ship is relative to the selection ("12,000 km ahead of K-3", "in sensor reach") instead of relying on dots. The planner for a ship in its own band has no Depart slider, since any day will do, and a Travel time slider that snaps to laps.
- **Specced** (2026-10-15) as PHAS-15: an Intercept at a station or a parked ship in another body's band is trimmed to meet it, because a raid that drives past without touching the station is not worth ordering, and a small closing manoeuvre is the engagement's. This replaces "an Intercept does not read where ships in the band are" (Intercepting, above) for a station or parked ship; an Intercept at a body still doesn't.
- **Specced** (2026-10-15) as BAND-05, BAND-06, XFER-02, XFER-05, XFER-10, PLAN-11 and PHAS-01 to PHAS-14. The spec keeps three limits: captures and intercepts are prograde, a phasing leg is for impulsive drives only, and an Intercept in the ship's own band is not an order.
- **Specced** (2026-10-15), what the player sees, as PLNR-21 and PLNR-40 to PLNR-42: Departs shows the day the ship leaves its orbit, wait included; a ship's own orbit has no Depart slider and Travel time snaps to meetings, with the laps shown; the gap to the target is a line in the panel. The general "relative to the selection" line and "in sensor reach" wait for the combat view.

## 5. Groups and gates **(Candidate)**

- **Groups** plan with the group's lowest delta-v and lowest acceleration, so the whole group makes the trip. The planner says which ship limits it. A group is one place: its ships share a phase (4.4).
- **Gates** are a destination like any other, with one extra readout: arrival speed relative to the gate against the gate's entry tolerance (*Drives & Transit* 6.2, *Gates* 4). A fast, cheap arrival that exceeds the tolerance is over-budget in the same way as a trip that needs too much delta-v, and the planner says which limit was hit.
- **Belts and salvage sites** are points on a ring, so they're ordinary targets.

## 6. Re-tasking in flight **(Direction)**

A ship in flight can be given a new transfer at any time. The planner starts from the ship's state *now* (position and velocity read off its rail) and its remaining delta-v, and everything in section 3 applies unchanged: the Depart slider means "coast for N days, then burn". The new trajectory replaces the old one from the departure day. The old destination is just one of the choices.

- **Why it costs nothing extra:** the two-burn planner never assumed the ship was parked; it assumed a state. A parked ship's state happens to be its body's.
- **Aborts** are a re-task to the ship's origin, or to the nearest body it can still reach. A ship with no affordable destination is drifting, and the game should say so before it happens (the over-budget warnings in 3.1).

## 7. Tactical manoeuvring **(Candidate)**

Combat is simulated automatically and replayed on its own clock (*Conflict*), so there is no moment-to-moment flying. What the player positions is decided before the engagement, and it should all be expressible with the same planner and the same places:

- **Where to be.** A band at a body (low orbit to blockade or bombard, high orbit to defend the yard), at a station, or at a gate. Picket duty is a transfer with a *Hold* at the end.
- **Rendezvous or pass.** An arrival has two modes. **Rendezvous** captures into the destination's orbit and pays the capture burn. **Intercept** skips it: the ship flies through at speed, saving delta-v and time, and keeps going on its conic. Intercept is how a raid, a reconnaissance sweep, a fast gate run (*Drives & Transit* 6.2) or a hit on a convoy works; it also means a fast arrival that can't afford to stop is a legal order, not an error. The planner shows arrival speed for both, and the mode is a toggle next to the sliders.
- **Same orbit: a phasing leg.** A target in the ship's own named orbit is a gap to close, unless it is together with the ship (4.4). The planner opens with a phasing leg: any day, and a number of laps for the travel time. A target in another band of the same body is an in-frame transfer to its true place (2.1), with a window each synodic period of the two rings.
- **Intercept names its closest band.** There is no geometry inside a body's sphere of influence (2.1), so an Intercept is aimed at the body and names the band of closest approach, which sets the relative speed the planner reports (from the body's mass, the band radius and the arrival speed; a 5 km/s approach is over 10 km/s relative at low orbit) and what the ship could act on. The band radius *is* the closest approach, so an Intercept never meets the planet: the ship's aim point on the way in is a free choice the planner makes, which is why any band is reachable from any direction. An Intercept through High leaves on the heliocentric path it arrived on (2.1: bodies never bend a coasting ship), which is a fair rounding at 10 radii or more. An Intercept through Low is always a via-leg (2.6): the turn there is 35° to 130° at normal speeds, too big to ignore, so the planner always applies it and shows the kink. Physics offers no choice, so the player gets none. It does not decide who fights: whether anything in a band engages a passing ship is for *Conflict* (ranges, willingness), and a slow fleet in a wide high band would usually not be in reach of a fast flyby at all. The point round the band where the Intercept goes by is real too, and the leg records it (4.4).
- **A ship in orbit as the destination** is an ordinary order with the ship as destination. From outside the body's frame, a Rendezvous is timed to arrive together with it, for escort or blockade (4.4); an Intercept is aimed at the band, and whether it meets the ship is for combat. From inside the frame it is a crossing to the ship's true place, or a phasing leg in the same band. The same order works for a ship in flight, below.
- **Ship in flight.** A ship or group in flight as the destination. The target's rail gives its position at any arrival day, so the two-slider planner works unchanged; the preview arc ends where the two paths meet, and an *Intercept* is a closing engagement at that speed while a *Rendezvous* matches course (for escort, boarding or capture). Against a contact with a weak track (SYM-01), the order carries the track's uncertainty and may miss.
- **Engagement speed and position both matter.** Relative speed at the meeting point is the tactical number this model produces for free, and *Conflict* notes engagements happen at extreme velocity and range. Position round a band (4.4) decides who is in reach: two fleets in the same band are in reach of each other only if the gap between them is small.
- **Retreat** is a re-task (6). A fleet that has spent its budget can't retreat, which the delta-v bar shows before the fight.
- **Out of scope:** formations, ranges within an engagement, evasive burns. These belong to the combat simulation (*Conflict* 1.8), if anywhere.


## 8. Data shape **(Candidate)**

- `Trajectory { segments: Vec<Segment> }`, each `Segment { start_day, end_day, kind }` with `kind` one of `Parked { body }`, `Conic { primary, elements }`, `Spiral { primary, r0, r1, θ0, θ1 }`. Position and velocity in km and km/day, `f64`, relative to the segment's primary (SCL-01, SCL-06).
- `Plan { ship, legs: Vec<Leg>, status }` and `Leg { destination, arrival: Rendezvous | Intercept { band }, depart_day, travel_days, trajectory }`, with `status` one of Scheduled, InFlight, Holding, Done, Failed. A plain transfer is a one-leg plan; a via-flyby is a leg with `arrival: Intercept` and a free turn at the body. `destination` is a place: `Body { body, band: Low | High }`, `Satellite { body }` (stations, moons), `Gate { gate }`, `Site { site }`, `Ship { ship }` or `Hold`. Orders are the save-game truth; trajectories can be recomputed from them.
- A ship in orbit is a `Parked` segment at its own phase: its position is the orbit's at that phase. A phasing leg is a Parked segment, a Conic round the body, then a Parked segment again at the target's phase (4.4).
- Per body: the low and high band radii (or the altitude and multiple they derive from), and from them the departure cost and approach days per band. Per gate: `entry_tolerance_kms` (*Drives & Transit* 8).
- Per ship: `reserve_kms` (recomputed) and whether its class refuses one-way orders. Per leg: `refuel: bool`. Tunables: `return_window_days`, `refuel_days` (per port later, if ports differ).
- Per drive: `impulsive: bool`, `exhaust_velocity`, `acceleration` (loaded). Per ship: `delta_v_kms`, with reserved and spent.
- A `transfer_solver` module in the simulation layer, pure functions from states and budgets to trajectories, so the planner and the tests share it. The view only draws segments and moves sliders.
- **Specced** (2026-10-14) as RAIL-02, BAND-02, PLAN-01 to PLAN-30 and XFER-11: the trajectory, plan, leg and place shapes, status, delta-v reserved and spent, the drive's flag and acceleration, and the solver module. `reserve_kms`, one-way refusal, `refuel` and the two dwell tunables wait for sections 3.6 and 3.7.

## 9. Open questions

Parked, with the current lean. None blocks the simulation or the planner.

- **Groups:** split when the slow ship holds it up, or always move together? Lean: always together; splitting is an explicit "detach" order.
- **Gravity assists:** build the via-planner once the generator can place a giant in a traffic lane, or earlier as a showcase? Lean: later, no showcase.
- **Refuel dwell:** one `refuel_days` for every port, or a port property? Partial fills? Lean: per port with one default; partial fill allowed, since fuel is a stock. The economy doc's to settle.
- **Intercept and ground targets:** can a passing ship bombard, drop troops or scan, or does acting on a body need a Rendezvous in the right band? Undecided: don't rule out what would plainly work in real life. Revisit with the conflict doc.
- **Intercepting uncertain contacts:** what happens when the contact isn't where the track said? Parked until detection mechanics exist; it is not yet clear that a "weak track" is a meaningful state in this game.
- **Gate arrival on the far side:** does arrival speed carry through, and does the far-side planner start from it? Lean: yes to both; it is why the tolerance rule exists. *Gates*' call.
- **Eccentric flavour bodies:** worth the generator's effort? Lean: not yet; revisit with the world generator.
- **Low-thrust phasing:** spiral out, drift, spiral back. Lean: specify it once a low-thrust ship has a reason to meet something in its own band, such as a tanker meeting a station.
- **Retrograde flybys:** an intercept against the band's motion closes about twice as fast on a ship in the band. Lean: decide with the conflict doc.
- **What an Intercept meets:** the closest-approach point and speed are known, and the encounter with ships in the band is combat's. Lean: combat builds the encounter path from them when a window forms.

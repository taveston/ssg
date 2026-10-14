# Orphan Worlds: Orbital Transfers

**In short:** ships fly on rails along trajectories computed once, when an order is given. The model is patched conics with the planet-centred legs collapsed to a cost and a short delay, so the only geometry is the arc round the primary. High-thrust drives get a two-burn arc (a Lambert solution); low-thrust drives get an estimated spiral. The player plans a transfer with two sliders, depart and travel time, and sees the path, the dates and the delta-v change as they move. A destination is always a place (a body, station, gate, site or ship), never a bare orbit; round a body there are two bands, low and high, plus the orbits of its satellites. Ships sharing a band are one marker on the map and a list in the inset. A scheduled transfer is a visible, cancellable order on the ship. A ship in flight can be re-tasked at any moment, because the planner only ever needs where the ship is, how fast it's going and what it has left. Tactical positioning reduces to which place, which band, and whether the ship stops or passes.

*Status:* draft. **Direction** unless marked otherwise. Nothing here is specced yet.

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
- **Burn time.** The planner treats burns as instant. For drives whose burn would take a noticeable fraction of the trip (pulsed fusion spending 100 km/s at 4 mg burns for about a month), add the burn time to the trip and draw the arc from the end of the burn. Candidate: a drive is *impulsive* if a full-tank burn takes under a tunable fraction of a typical trip; otherwise it's low-thrust (2.4). The thrust classes in *Drives & Transit* 3 are the starting point.
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

### 2.6 Gravity assists **(Candidate)**

Many players have heard of a gravity assist, so a simple version is worth having, as long as it stays an order the player gives rather than something that happens to them.

- **The rule.** A flyby is a *waypoint* in a transfer ("Halden IV via Morrow"). At the waypoint the ship's velocity relative to the body may turn by up to a maximum angle set by the body's mass, the ship's arrival speed and the low band radius (the closest it may pass). The turn is free; its size is the assist. One formula, no extra physics, and the arc shows as a kink at the body labelled with the delta-v it saved.
- **How much it's worth.** For a ship arriving at a typical NTR relative speed of about 5 km/s: Morrow, Jupiter-class, about 8 km/s, close to a whole NTR tank; Halden IV about 3 km/s; Iver and the inner planets about 1 km/s or less. So only a giant is worth routing past, and only when it sits between places ships go. In the recommended Corvane layout (Morrow at 4 AU, gates at 2 AU) nothing does, so the rule is dormant there. It comes alive in systems where the generator puts a giant inside the gate ring or between two inhabited worlds, which is a good source of variety: a system with a cheap route and a system without one.
- **Cost.** A via-transfer is two arcs joined at a moving body with a matching condition between them, so the planner needs one more search dimension (when to arrive at the waypoint) and a constraint. That is a real step up from the two-slider planner, which is why this is a candidate and not part of the first build. The player's view stays the same two sliders, with the waypoint chosen from the destination step ("via" on a body's context menu).
- **Rejected:** a flat delta-v credit for flying past a body. It is simpler, but a flyby that helps whichever way you're going is the kind of abstraction that teaches the wrong thing.

## 3. Planning a transfer **(Direction)**

The flow is: select a ship, pick a destination, tune two sliders while the map shows the result, click Schedule.

### 3.1 Steps

1. **Select the ship** (or group). The inset shows its delta-v remaining and its drive's kind.
2. **Choose a destination:** a Transfer button on the inset, then click a body, station, gate or orbit ring on the map. Hovering a candidate previews the fastest affordable trip.
3. **Tune.** Two sliders in the inset, with the trajectory, dates and delta-v updating as they move:
   - **Depart:** from the earliest possible day (now plus any preparation) to one window period ahead. The slider track is coloured by the delta-v the cheapest trip on that day needs, so windows show as bright bands without being named: a good day looks good.
   - **Travel time:** from the fastest the budget allows to a slow, cheap trip. Same colouring.
   - Readouts, always visible: **Departs** (date, and "in N days"), **Arrives** (date, and "in N days"), **Delta-v** as a bar: used of available, with the remainder after the trip. Over-budget settings are reachable on the slider, shown in the warning colour, and Schedule is disabled with a one-line reason ("Needs 11.2 km/s, has 9.9").
   - **Presets:** *Fastest*, *Cheapest*, *Next window*. Most orders will be one click on one of these.
4. **Schedule.** The order is stored as (ship, destination, departure day, travel time); the trajectory is computed and attached. The ship is unchanged until the departure day.

### 3.2 Vocabulary

The interface says Depart, Travel time, Arrive, Delta-v (with a km/s figure) and Window. It never says Hohmann, synodic, Lambert, phase angle, apoapsis or inclination. Delta-v stays, because it is the one number that bids, designs and refuelling already turn on; the delta-v bar makes it a fuel gauge. Candidate: a tooltip on first use ("Delta-v is how much a ship can change its speed before its tanks are dry. Every trip spends some").

### 3.3 On the map

- **The preview** is the arc the ship would fly, drawn from the departure point to the arrival point, with the arrival point marked on the target's orbit ring at its arrival-day position (dot mode) or on the ring itself (exposure-ring mode), as *Presentation* already says.
- **One preview, not a fan.** The earlier candidate (a fan of alternative paths, each labelled) is replaced by one live path plus the coloured slider tracks, which carry the same information with far less clutter. *(Proposed change to Presentation, Motion.)*
- **Scheduled transfers** show the planned arc dashed from the ship's departure point, and the ship's label carries "Departs in N d". In flight, the arc is solid behind the ship and dashed ahead, with "Arrives in N d". The velocity leader (SYM-11) applies once ships move.

### 3.4 Scheduled orders

- A ship with a scheduled transfer is visually cued (the dashed arc and the label) and the inset shows the order with **Cancel** and **Change** (which reopens the sliders at the stored settings).
- On the departure day the order is re-validated against the ship's actual state (it may have been refuelled, damaged or moved). If it still holds, the ship departs; if not, it doesn't, and an alert says why.
- Delta-v is *reserved* when scheduled and *spent* at each burn, so the inset can show "9.9 available, 7.5 reserved".
- Alerts on departure, on arrival, and when a window the player was shown is about to close without an order. These are the auto-slowdown hooks the overview's risk register asks for.

## 4. Destinations and orbits **(Direction)**

### 4.1 A destination is a place, not an orbit

The player never picks an orbit by its radius. They pick a thing: a body, a station, a gate, a salvage site, or another ship. The planner decides the orbit that "being at" that thing means. This keeps the target step to one click, and it means every ship's position has a name ("High orbit, Halden IV", "At Iver Yard", "At the Ostra gate") rather than a number.

- **Bodies** have two bands: **Low** and **High**. Low is just above the surface or atmosphere: the place for shuttles, ground operations, blockade and bombardment. High is the standard parking orbit, cheap to leave and the default arrival. Both radii come from the body (a tunable altitude for low; a multiple of the radius, capped by the Hill sphere, for high). The departure cost in 2.1 is the cost from the band the ship is in.
- **Satellites and stations** are places of their own at their own fixed orbits. Arriving at Arden Highport means the highport's orbit, co-moving with it.
- **Gates** are a place at the gate's orbit, with the entry rule in section 5.
- **Belts and sites** are points on a ring; arriving means that point's orbit.
- **Ships** can be destinations too (section 7): the target is a trajectory rather than a body, and the planner aims at where it will be.
- **No free orbit as a destination.** A ship can't be sent to "2.3 AU, 40° ahead of Halden". But **coasting** is a state a ship can be in: a ship told to **Hold**, or one that has made a *Pass* (section 7), is on a free conic round the primary with no further burns. Its place is named by what it's doing ("Coasting, Halden IV → Iver, passed Halden IV") and the only thing it can be ordered to do is a new transfer from its current state. That is enough for pickets and ambushes without an orbit editor.
- **Coasting ships drift.** A free conic is an ellipse that crosses planet orbits, or a hyperbola that leaves the system. Neither needs simulating beyond the arc: bodies don't bend it (2.1), and a crossing is an event. The planner shows the consequence of a Pass before it's scheduled: the arc after the flyby, "crosses Iver's orbit in 40 d", or "leaves the system in 310 d" in the warning colour. A ship on a hyperbola with no delta-v to turn back is lost once it's past the last body it could reach, and the game says so while there's still time.

### 4.2 Why bands, not a free radius

Three bands' worth of choice (low, high, or at a satellite) covers every strategic reason to be at a body: close enough to act on the surface, parked cheaply, or docked. A free radius adds a number the player must understand, a slider that mostly does nothing, and no decision that the bands don't already make. Moving between bands is an in-frame transfer (2.1, nested primaries): a small order with a real cost, so low orbit of a deep well is a deliberate place to be.

### 4.3 Ships that share an orbit

Every ship in high orbit of Halden IV is on the same ring. At true scale that was always going to be the case: at any zoom that shows more than one planet, every orbit round Halden is inside its marker, and the satellite folding rule (GRP) already collapses them into one count. So sharing an orbit is not a new problem for selection; it is the same problem with more ships in the count. The design follows from that:

- **Phase along a band doesn't matter.** Ships in a band are treated as co-located for every purpose: selection, departure, combat. The simulation still gives each a phase so they draw as separate dots when unfolded, but no rule reads it. This is the single biggest simplification in the document, and it removes a whole class of "wait for the right moment to leave orbit" bookkeeping that the game doesn't want.
- **Slots are where ships arrived.** A ship keeps the phase it captured at, so ships that arrived at different times sit apart on the ring without any rule placing them. Ships built at a yard or placed by the generator take the yard's phase or a spread.
- **One list, reached three ways.** The body's inset has an "In orbit" section (INS-07) listing everything there, grouped by band, and that is where a ship is picked. Clicking the body marker or its fold count opens it; clicking a band's ring, drawn as a slow ring or as an exposure ring, opens it with that band expanded. When the orbit is slow enough to show dots, or the game is paused and exposure rings drop back to dots (MOT-12), a single ship can also be picked by clicking its dot. Ships that matter (selected, ordered, in a group) can be promoted to their own marker on the fold, the way the label priority order already favours the selection.
- **Where the detail lives:** the planetary view's orbital layer, where the bands unfold into distinct rings with the ships spread along them, is the place to see and pick individual ships round one body. The system view is for the trip; the planetary view is for the parking lot.
- **Specced** as HOV-04, HOV-05 and MOT-14 (shared orbits, their targeting and the identity mark on exposure rings), with the orbit row in INS-05 and TEST-10, on 2026-10-14. These stand on their own and don't depend on bands.
- **Stacking order:** when several things sit on one point (a station and the ships docked at it), the symbol rules for groups apply (SYM-09, GRP-09): one symbol with a count, and the list in the inset.

## 5. Groups and gates **(Candidate)**

- **Groups** plan with the group's lowest delta-v and lowest acceleration, so the whole group makes the trip. The planner says which ship limits it.
- **Gates** are a destination like any other, with one extra readout: arrival speed relative to the gate against the gate's entry tolerance (*Drives & Transit* 6.2, *Gates* 4). A fast, cheap arrival that exceeds the tolerance is over-budget in the same way as a trip that needs too much delta-v, and the planner says which limit was hit.
- **Belts and salvage sites** are points on a ring, so they're ordinary targets.

## 6. Re-tasking in flight **(Direction)**

A ship in flight can be given a new transfer at any time. The planner starts from the ship's state *now* (position and velocity read off its rail) and its remaining delta-v, and everything in section 3 applies unchanged: the Depart slider means "coast for N days, then burn". The new trajectory replaces the old one from the departure day. The old destination is just one of the choices.

- **Why it costs nothing extra:** the two-burn planner never assumed the ship was parked; it assumed a state. A parked ship's state happens to be its body's.
- **Aborts** are a re-task to the ship's origin, or to the nearest body it can still reach. A ship with no affordable destination is drifting, and the game should say so before it happens (the over-budget warnings in 3.1).

## 7. Tactical manoeuvring **(Candidate)**

Combat is auto-resolved on the system view with the clock slowed (*Conflict*), so there is no moment-to-moment flying. What the player positions is decided before the engagement, and it should all be expressible with the same planner and the same places:

- **Where to be.** A band at a body (low orbit to blockade or bombard, high orbit to defend the yard), at a station, or at a gate. Picket duty is a transfer with a *Hold* at the end.
- **Stop or pass.** An arrival has two modes. **Stop** captures into the destination's orbit and pays the capture burn. **Pass** skips it: the ship flies through at speed, saving delta-v and time, and keeps going on its conic. Pass is how a raid, a reconnaissance sweep, a fast gate run (*Drives & Transit* 6.2) or a hit on a convoy works; it also means a fast arrival that can't afford to stop is a legal order, not an error. The planner shows arrival speed for both, and the mode is a toggle next to the sliders.
- **Intercept.** A ship or group as the destination. The target's rail gives its position at any arrival day, so the two-slider planner works unchanged; the preview arc ends where the two paths meet, and a *Pass* intercept is a closing engagement at that speed while *Stop* is a rendezvous (matching course, for escort, boarding or capture). Against a contact with a weak track (SYM-01), the intercept carries the track's uncertainty and may miss.
- **Engagement speed matters, position in orbit doesn't.** Relative speed at the meeting point is the tactical number this model produces for free, and *Conflict* notes engagements happen at extreme velocity and range. Phase round a band (4.3) is deliberately not a factor: two fleets in the same band are engaged, full stop.
- **Retreat** is a re-task (6). A fleet that has spent its budget can't retreat, which the delta-v bar shows before the fight.
- **Out of scope:** formations, ranges within an engagement, evasive burns. These belong to the auto-resolver, if anywhere.


## 8. Data shape **(Candidate)**

- `Trajectory { segments: Vec<Segment> }`, each `Segment { start_day, end_day, kind }` with `kind` one of `Parked { body }`, `Conic { primary, elements }`, `Spiral { primary, r0, r1, θ0, θ1 }`. Position and velocity in km and km/day, `f64`, relative to the segment's primary (SCL-01, SCL-06).
- `TransferOrder { ship, destination, via: Option<body>, arrival: Stop | Pass, depart_day, travel_days, trajectory, status }` with `status` one of Scheduled, InFlight, Done, Failed. `destination` is a place: `Body { body, band: Low | High }`, `Satellite { body }` (stations, moons), `Gate { gate }`, `Site { site }`, `Ship { ship }` or `Hold`. Orders are the save-game truth; trajectories can be recomputed from them.
- Per body: the low and high band radii (or the altitude and multiple they derive from), and from them the departure cost and approach days per band. Per gate: `entry_tolerance_kms` (*Drives & Transit* 8).
- Per drive: `impulsive: bool`, `exhaust_velocity`, `acceleration` (loaded). Per ship: `delta_v_kms`, with reserved and spent.
- A `transfer_solver` module in the simulation layer, pure functions from states and budgets to trajectories, so the planner and the tests share it. The view only draws segments and moves sliders.

## 9. Open questions

- **The planet legs.** Is "departure cost plus a few days" enough, or should a transfer from a deep well visibly stage through a highport (two orders, not one)? The economic reason for highports exists either way.
- **Impulsive versus low-thrust boundary.** Where does pulsed fusion sit? A month-long burn on a two-month trip is neither.
- **Groups:** split a group when the slow ship holds it up, or always move together?
- **Gravity assists:** build the via-planner once the generator can place a giant in a traffic lane, or earlier as a showcase?
- **Lost ships.** A coasting ship with no affordable destination: does it strand (a rescue tug becomes an order), or is it simply gone at the system's edge?
- **Band count.** Are low and high enough, or does a third band (a staging orbit near the Hill sphere, for fleets that must leave fast) earn its place?
- **Pass and ground targets.** Can a passing ship bombard, drop troops or scan, or does anything that acts on a body need Stop in the right band?
- **Intercepting weak tracks.** How far should a miss send the interceptor, and does it get a free re-task when the track firms up?
- **How many windows ahead** should the Depart slider reach? One synodic period covers every option once; further ahead is just waiting.
- **Delta-v wording.** Keep "delta-v" with a first-use explanation, or call it something softer ("burn", "range")?
- **Gate arrival on the far side:** does arrival speed carry through, and does the planner on the far side start from it?
- **Eccentric flavour bodies:** worth the generator's effort, and would a stretched ring on the map read as a mistake?
- **Time acceleration:** at 5 d/s a window closes in seconds. Does the Depart slider's "next window" need to pause the game by default when the day arrives?

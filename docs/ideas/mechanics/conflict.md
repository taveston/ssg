# Orphan Worlds: Combat, Raiders & Diplomacy

**In short:** combat is a small, lightly simulated and essentially auto-resolved part of play, watched on the ordinary system view with the clock slowed, for Defcon-style tension. A fight is mostly decided before the shooting: by who sees whom, the geometry, delta-v reserves and magazines. Defence is mainly interception by a concentrated fleet, set up with standing orders. Raiders arise from simulated conditions, not random spawns. Rival leaders change through elections, coups and death, so relations shift over a long game.

*Status:* early. Status tags as in the overview (**Decided**, **Direction**, **Candidate**). Companion documents: *Overview*, *Presentation & Visual Style* (2, the display; 3, ground combat), *Symbology*, *Economy* (armed haulers, tolls), *Orbital Transfers* (7, tactical manoeuvring), *Component Stats & Ship Assembly* (signatures, weapons).

---

## 1. Combat (direction)

- Engagements happen at extreme velocity and range.
- The player sets a posture, has few levers during the fight, and watches blips merge on a radar-like display.
- The display is the ordinary system view with the clock slowed (*Presentation & Visual Style*).

### 1.1 What realistic combat looks like

A long, quiet approach, then a short exchange.

- Ships coast on known conics for days or weeks. The path is the commitment: a ship can't dodge without another burn, and every burn costs delta-v.
- Contact lasts minutes. Closing speeds are km/s and ranges are thousands of km.
- Most of the outcome is set before anyone fires: detection, geometry, delta-v reserve, magazines and point-defence capacity.
- Wrecks stay on their own conics. A dead ship can still be a hazard, and then it is salvage.

### 1.2 Weapons: no single winner

- **Missiles** do most of the killing. They chase, work at the longest ranges and can saturate defences. Their limits are magazine and cost. A missile out-manoeuvres any ship, so only point defence and decoys stop it.
- **Kinetic launchers** are cheap. A projectile that flies for minutes can be dodged by a small burn, so they suit close range and area denial. The catalogue's launcher fires once a minute, a trickle in a short pass.
- **Lasers** are the best point defence and a heat-limited weapon. One turns about two-thirds of its draw into heat while firing, which loads the radiators. They are tier 3, so mid-future.
- **Near future:** missiles, kinetic launchers and point defence. **Mid future:** lasers become real offence, and a fight between laser ships is a duel of radiator margin.
- Each side needs a mix. The central contest is salvo size against point-defence capacity.

### 1.3 Engagement geometry

- **Dwell time** is the time inside weapon range, roughly range divided by relative speed. It is the central derived number for the resolver. Weapon types respond to it differently: missiles act mostly from launch, point defence and lasers need time on target (capped by heat), and kinetic fire needs it for volume.
- **The objective picks the speed.** Destroying favours a fast Pass: no capture burn and little exposure. Holding (blockade, bombardment, boarding, capture, salvage, escort) needs a Stop. A fast pass commits the ship to its path, and stopping spends the return reserve.
- **Defenders influence speed.** A deep gravity well raises pass speeds.
- **Miss distance is a choice.** An intercept carries a standoff, not a collision course.
- **Ships in one band are not engaged by default.** They share a speed, not a place. A band can be hundreds of thousands of km round, and the longest sensor range in the catalogue is 50,000 km. Closing within a band takes a phasing transfer. This replaces the co-location rule for combat.

### 1.4 Defence is interception

- Fixed assets (stations, yards, gates) outnumber mobile ships and sit on known orbits. A fleet concentrates, so it can't cover them all.
- The defender's problem is **reach**: which threats the fleet can get to before they arrive. It is a delta-v and travel-time question that the planner already answers. The candidate range rings (*Presentation & Visual Style*) would show it.
- Interception is at high relative speed along the approach. It spends delta-v, and the fleet may not afford the return, so a sortie is a gamble.
- A fixed asset still adds sensors, power, radiators and magazines, and anyone who wants to hold it must stop inside that envelope. It is a modifier on the fight, not the main mode.
- At a day per second, a week of warning is seven seconds of play. **Standing orders and alerts** are central: intercept on detection, hold, intercept above a threat level. Detection warning time sets how much room the player has.

### 1.5 Detection and stealth

- A burning drive is visible across the system. At the catalogue's figures an NTR burn (about 900 MW) is seen at about 15 AU. A launch is public. Light delay is minutes and is not modelled.
- Drive choice sets visibility. NTR coasting is about 1.9 MW (seen at about 0.7 AU). A NEP hauler is about 23 MW all the time (about 2.4 AU). An NTR burn is about 900 MW. NEP ships are the visible ones. Emission control adds bounded dimness, limited by how much heat the ship can store.
- A burn gives a bearing, not a fix. A track tightens over days with time and separated sensors. Planets and the sun block lines of sight.
- The current conic of a ship that burned in view is well known. What is uncertain is its **future conic**: a small early burn moves a ship a long way later (100 m/s for an hour is about 360 km), which can ruin an intercept.
- The uncertainty to show is a **reachable envelope** from remaining delta-v, not a measurement ellipse. A weak track is needed only for hidden departures and sensor gaps.

### 1.6 Knowing an enemy's delta-v

The defender gets a range, never a certainty. From weakest to strongest:

1. The drive family sets a ceiling.
2. A burn reveals current mass: thrust comes from the drive and acceleration from the track. Subtract the class's dry mass for the propellant left, and the rocket equation gives remaining delta-v. As-built variance is the error bar.
3. History bounds it: the class maximum minus the delta-v watched spent, unless the ship has refuelled.
4. Intelligence and inspection give exact figures: design records, a yard's bid history, informants, a close lidar pass, a captured sister ship.

A raider can deceive with extra tankage, a throttled drive or a disguised class. The inset could show "Delta-v remaining: 3–7 km/s (est.)", narrowing as burns are seen and intelligence arrives. Scouting then has a concrete payoff.

### 1.7 Worked example: an interception (illustrative)

Raider R-2 is an armed hauler with 8 Dart missiles, one point-defence mount and decoys. It is on a Pass course for Halden IV's High band. TF-1 is 4 warships (assumed 3 Darts and 2 point-defence mounts each) at High. Closing speed is 9 km/s.

| When | What |
|---|---|
| CA−72 h | R-2's correction burn is seen system-wide. Its path is a known conic. |
| CA−60 h | A standing order sends TF-1 out (3 km/s of its 9.9). Both ships are now committed. |
| CA−93 min, 50,000 km | Lidar tracks. Nobody fires. |
| CA−37 min, 20,000 km | Radar range. R-2 fires 8 Darts. TF-1 fires 12 and holds 36 back. |
| CA−17 min, about 9,400 km | The missiles reach point-defence range (300 km), about 18 s before impact. At an assumed 1 shot a second and 0.3 per shot, R-2's one mount expects about 5 kills against 12 missiles and is mission-killed. TF-1's mounts stop all 8. |
| CA, standoff 1,500 km | TF-1 passes the wreck, which keeps its conic and must be diverted, then salvaged. |

The fight was decided at CA−60 h. The shooting took about 20 minutes, with about 18 seconds of point defence.

---

## 2. Raiders (direction)

- They arise from simulated conditions, not random spawns.
- Possible origins: scavengers contesting salvage, failed organisations, state-sponsored privateers (most plausible early on), smugglers avoiding tolls.
- Chokepoints and predictable trajectories invite interdiction.
- Long delays make age-of-sail analogies apt: standing orders, convoys, blockades, chartered companies, privateers.

## 3. Diplomacy (decided)

AI leaders change through elections, coups and death, so relations shift over a long game.

## 4. Open questions

- Pre-battle orders: which standing orders exist and how the player sets them.
- Information limits: how scouting and intelligence narrow a delta-v range.
- After-action reports.
- Ground combat and invasion.
- Making losses matter.
- Suppressing raiders.
- Privateer licences and prize shares.
- The point-defence shot rate. The catalogue gives a chance per shot but no rate.
- Missile range and magazine as ship figures, and the effect of decoys and emission control.
- Dwell time as the resolver's central number, and how each weapon type responds to it.
- The standoff on an intercept order: its default and how the resolver reads it (finding F-145).
- A phasing transfer within a band, replacing co-location for combat (finding F-144; `transfers.md` 4.3 and 7).
- How a raider's reachable envelope is drawn.

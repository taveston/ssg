# Orphan Worlds: Combat, Raiders & Diplomacy

**In short:** combat is a small part of play, simulated in full by the game and never flown by the player. A fight is mostly decided before the shooting: by who sees whom, the geometry, delta-v reserves and magazines. It is simulated at once when it forms, shows on the map as a collapsed engagement while it runs, and is revealed with a watchable replay when it ends. Defence is mainly interception by a concentrated fleet, set up with standing orders. The senior officer present commands, through Command skill and one trait, Bold/Cautious. Raiders arise from simulated conditions, not random spawns. Rival leaders change through elections, coups and death, so relations shift over a long game.

*Status:* early. Status tags as in the overview (**Decided**, **Direction**, **Candidate**). Companion documents: *Overview*, *Presentation & Visual Style* (2, the display; 3, ground combat), *Symbology*, *Economy* (armed haulers, tolls), *Characters, Organisations & Events* (1, skills and traits), *Orbital Transfers* (7, tactical manoeuvring), *Component Stats & Ship Assembly* (signatures, weapons), *Combat Simulation* (`combat-simulation.md`, how a fight is computed), *Combat View* (`../presentation/combat-view.md`, how the replay looks).

---

## 1. Combat (direction)

- Engagements happen at extreme velocity and range.
- The player sets a posture and standing orders beforehand and has no direct control during the fight.
- The player watches a replay in a zoomed combat view. It uses the same visual language as the system view, on its own clock (1.8).

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

- **Dwell time** is the time inside weapon range, roughly range divided by relative speed. It is the central derived number for the simulation. Weapon types respond to it differently: missiles act mostly from launch, point defence and lasers need time on target (capped by heat), and kinetic fire needs it for volume.
- **The objective picks the speed.** Destroying favours a fast Pass: no capture burn and little exposure. Holding (blockade, bombardment, boarding, capture, salvage, escort) needs a Stop. A fast pass commits the ship to its path, and stopping spends the return reserve.
- **Defenders influence speed.** A deep gravity well raises pass speeds.
- **Miss distance is a choice.** An intercept carries a standoff, not a collision course.
- **Ships in one band are not engaged by default.** They share a speed, not a place. A band can be hundreds of thousands of km round, and the longest sensor range in the catalogue is 50,000 km. Closing within a band takes a phasing transfer. This replaces the co-location rule for combat.
- **Evasion is a spend.** Jinking costs delta-v and shows as a burn. A ship thrusting at 0.1 g for 20 minutes spends about 1.2 km/s, so a warship that dodges gives up the reserve it needs to retreat or pursue. A slow hauler gains almost nothing from it.

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

### 1.8 How a fight is run and shown

**Running it**

- An engagement is the cluster of ships whose rails will come within weapon range of each other. Ships already inbound join at their arrival time.
- It is simulated in full, at once, when it forms (t0). The simulation is a point-mass model with sensors, missiles, point defence and commander decisions. It is a pure function of the starting states and standing orders, and the same simulator runs every fight.
- Inputs freeze at t0. Orders given afterwards to ships in the engagement wait until it ends. This holds because no uninvolved ship can reach a fight within hours.
- A window is capped at a few hours. A longer fight splits into phases: survivors return to the main simulation with their damage and magazines, and a new phase forms if they are still in range.
- The main clock keeps running. While the fight runs, its ships appear on the map as one engagement: both sides shown and joined by a link (reusing the folding rules), and not commandable.
- The detail of forming, stepping, sensing, weapons, decisions and orders is in *Combat Simulation*. It proposes that t0 is the first moment opposed ships come within sensor reach, so the window is the few hours around contact, and the decisive order was given long before.

**Revealing it**

- At the end (t1) the results are already known. If the fight involved the player's forces, or lay inside the player's sensor coverage, the main clock pauses and offers Watch or Skip. Whether the pause covers every visible fight or only the player's own is a tunable.
- The replay runs on its own tactical clock in a zoomed combat view. Overlapping fights queue in t1 order. Results appear on the map once the player has watched or skipped.
- A fight outside the player's knowledge produces no recording and no report. The player finds out through fog: a convoy that doesn't arrive, a wreck found later.

**What is stored**

- A fight is recorded only if a player could watch it, decided at t0. Coasting is stored as rail segments. Powered phases (jinking, homing) are sampled keyframes, interpolated on playback. The simulation, not the interpolation, supplies the states handed back to the map. Powered time is bounded by delta-v: a Dart has about 40 s of burn.
- Replays are ephemeral. A recording is available until it is watched or skipped, or for a short window after. Unwatched recordings are kept in saves, up to a cap. The window and the cap are tunables.
- What persists is the outcome:
  - per ship: status (destroyed, disabled, escaped, captured), condition, magazines and delta-v spent, and its new rail;
  - wrecks, as salvage sites;
  - faction knowledge updates: designs better observed, and burns seen;
  - an event-log report: a summary, a short timeline, the commanders' decision tags and a coarse track plot;
  - per commander present: kills, own losses and whether they survived, which feed their experience (1.9).
- One simulator serves all fights, with recording as an optional output. A cheaper model for unobserved fights is deferred until profiling shows a need. If one is used, it must be calibrated against the full simulation, so outcomes don't depend on whether the player is looking.

### 1.9 Commanders

The senior officer present commands the force. A lone ship's captain commands that one-ship force. The same model runs at every scale.

**Autonomy**

- Commanders have no strategic autonomy. Standing orders decide whether and where to fight. Skill and traits act only on how an order is carried out.
- Only traits that move a decision the simulation makes have a combat role. Today that is one: Bold/Cautious. The other pairs in *Characters, Organisations & Events* have no combat role.
- Ruthless/Humane waits for surrender and disabled-ship states, which the simulation doesn't have yet.

**Command skill (direction)**

- Skill sets how well the commander knows the fight and how fast they act. Each channel is linear in the deficit d = (10 - skill) / 10:
  - belief error: how far the commander's picture of the enemy is off;
  - decision latency: the delay before they act on new information;
  - coordination: how much of the force's capacity works together.
- Skill matters mostly before contact. Terminal point defence is automation, crew and numbers, not command.

**Bold/Cautious (direction)**

- One dial between the objective and survival. The simulation derives every commander decision from it.
- Bold commanders press the objective: large salvos, early fire, losses accepted before breaking off.
- Cautious commanders preserve the ship and its reserve: magazines held back, return delta-v kept, early break-off.
- Average sits between. The tactical AI reads this dial and nothing else about temperament.
- The report's decision tags show it, for example "broke off at 40% losses" or "held 60% of magazines". The player learns a commander's trait from these, as *Characters, Organisations & Events* describes.

**Experience (direction)**

- A commander has a base disposition, set at creation. Combat experience adds a bounded hidden offset on top, so traits stay mostly fixed.
- The offset follows a rolling kill-to-loss ratio over the engagements the commander took part in. It is a smoothed log ratio, ln((K + a) / (L + a)), with older fights decayed.
- Many losses and few kills push toward Cautious, as for a sole survivor of a large fight. A run of easy wins pushes toward Bold.
- The effective trait steps between Bold, average and Cautious at thresholds. Hysteresis stops it flickering.
- Command skill grows with experience, with diminishing returns (candidate).
- An order that ends in disaster could leave a grievance against the player (candidate; relationships in *Characters, Organisations & Events*).

**Captains (direction)**

- A captain affects a fight only by being its commander. In a large fight no single captain steers the outcome.
- Captains also serve as flavour and as the pool that fleet commanders are drawn from.
- If the flagship is lost, command passes to the next senior captain after a delay, with that captain's skill and trait (candidate).

---

## 2. Raiders (direction)

- They arise from simulated conditions, not random spawns.
- Possible origins: scavengers contesting salvage, failed organisations, state-sponsored privateers (most plausible early on), smugglers avoiding tolls.
- Chokepoints and predictable trajectories invite interdiction.
- Long delays make age-of-sail analogies apt: standing orders, convoys, blockades, chartered companies, privateers.

## 3. Diplomacy (decided)

AI leaders change through elections, coups and death, so relations shift over a long game.

## 4. Open questions

- Pre-battle orders: which standing orders exist and how the player sets them. A proposal is in *Combat Simulation* 8 and *Combat View* 5.
- Information limits: how scouting and intelligence narrow a delta-v range.
- After-action reports: what the summary and timeline contain (proposed in *Combat View* 6).
- Ground combat and invasion.
- Making losses matter.
- Suppressing raiders.
- Privateer licences and prize shares.
- The point-defence shot rate. The catalogue gives a chance per shot but no rate. *Combat Simulation* 5.3 assumes one a second.
- Missile range and magazine as ship figures. (*Combat Simulation* 4 and 5.1 propose the effect of decoys and emission control.)
- Dwell time as the simulation's central number: how each weapon type responds is proposed in *Combat Simulation* 6.
- The standoff on an intercept order: its default and how the simulation reads it (finding F-145). *Combat Simulation* 1 and 8 propose that the simulation reads rails only, and give a default.
- A phasing transfer within a band, replacing co-location for combat (finding F-144; `transfers.md` 4.3 and 7). *Combat Simulation* 1 proposes that combat reads true rail positions.
- How a raider's reachable envelope is drawn (a candidate overlay in *Combat View* 4).
- The engagement window cap and the length of a phase (placeholder in *Combat Simulation* 11).
- Whether the pause at the end of a fight covers every visible fight or only the player's own (*Combat View* 2 proposes own fights only by default).
- The replay window and the cap on stored recordings.
- The engagement's map symbol (*Combat View* 2 proposes both sides joined by a link).
- Commander decision tags, and the tactical AI that reads the Bold/Cautious dial (proposed in *Combat Simulation* 7).
- Kill-to-loss drift: count or ship value, the smoothing constant, the decay and the thresholds.
- The delay before command passes after a flagship is lost.
- Whether combat experience can leave a grievance against the player.
- A combat role for Ruthless/Humane, once the simulation has surrender and disabled ships.
- A cheaper model for unobserved fights, if profiling shows a need.

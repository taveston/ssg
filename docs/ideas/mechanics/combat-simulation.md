# Orphan Worlds: Combat Simulation

**In short:** one engagement is one pure function. It takes the frozen state of a cluster of ships and their orders, and returns outcomes and, if someone could watch, a recording. It forms when the rails of opposed ships will come within reach, runs for a window of a few hours, and steps coarsely until something comes within point-defence range. Everything a commander sees comes from tracks, not from the true state. The fight is decided by four things the model computes: detection and track quality, dwell time, missile guidance against evasion, and point-defence capacity against salvo size. Damage is per component unit, using the `min_working` counts ship designs already carry. Every commander decision comes from one number, the weight on own losses against the objective, set by Bold/Cautious. The player's only input is the orders given beforehand.

*Status:* all **Candidate**, a worked-out proposal for the points *Combat, Raiders & Diplomacy* leaves open. Section 1.8 there stays the summary of how a fight is run and shown; this doc holds the detail. Companion documents: *Combat, Raiders & Diplomacy* (1.1 to 1.9), *Component Stats & Ship Assembly* (sensors, weapons, signatures), *Orbital Transfers* (rails, reserve, standoff), *Characters, Organisations & Events* (skill, traits), *Combat View* (how it is shown). Figures are the catalogue's; the unmarked tunables in section 11 are placeholders.

---

## 1. Forming an engagement

- **Opposed ships.** Two ships are opposed if their factions are at war, or if one side's orders say to engage and the other qualifies as a target (unknown contacts, with weapons free). Neutral ships passing do not form one.
- **Reach.** For a pair, reach is the range at which either could get a firm track on the other: the best active sensor range, scaled to the other ship's cross-section (section 4). With a lidar it is 50,000 km. Passive thermal detection reaches much further, but it gives only a weak track and cannot support fire, so it does not form an engagement.
- **Forming.** The main simulation tests the committed rails. If two opposed ships will come within reach inside the next `window_cap_h`, an engagement forms. Overlapping clusters merge. A ship is in one engagement at a time.
- **t0** is the first moment any opposed pair is within reach. The state is frozen then: positions, velocities, delta-v, magazines, condition, orders, commanders. Ships whose rails enter the cluster's reach later join at that time, with their rails as frozen.
- **The window** runs from t0 until no opposed pair is within reach and none will be again, capped at `window_cap_h`. In the worked example (1.7) the window opens at CA−93 min, when the lidars reach 50,000 km at 9 km/s closing, and closes 93 minutes after closest approach, about three hours in all. The decisive choice, the intercept order at CA−60 h, was made long before t0 and shows up only as the rails.
- **Phases.** At the cap, survivors go back to the main simulation with damage, magazines and delta-v, and a new phase forms if they are still in reach. Orders given while a phase ran take effect at the next one. Each phase has its own result and its own Watch or Skip.
- **Long standoffs.** A blockade or siege is one engagement phase after another. Fixed assets (stations, gates) take part as ships that cannot move. They add sensors, magazines, radiators and point defence.
- **Shared bands.** The simulation reads true rail positions, so phase round a band counts. Two fleets in one band at a separation inside reach are engaged for as long as the separation holds. Beyond reach they are not, and closing takes a phasing transfer (*Orbital Transfers*, finding F-144).
- **Standoff.** The simulation never reads an order's standoff. It shapes the rail, which ends offset from the target's rail, and the simulation reads positions. That answers how the resolver reads the standoff (finding F-145).

## 2. The step loop

- **Pure function.** Inputs: the frozen state, a seed, the tunables. Outputs: the outcome (1.8), and a recording if requested. No clock, no thread timing, no global state.
- **Random numbers** are counter-based: keyed by the engagement id, the entity ids and an event counter, never by iteration order. The same inputs give the same fight on any machine, with or without recording.
- **Adaptive step.** `step_coarse_s` while nothing is inside `fine_factor` times the longest point-defence range of a defended ship. `step_fine_s` once anything is. Coasting ships and missiles follow their rails exactly between steps, so the step only sets how often sensors, decisions and point defence run.
- **Each step, in order:**
  1. Sensors update tracks (section 4).
  2. Commanders whose decision time has come choose actions (section 7).
  3. Launches, jinks, decoys and sensor changes take effect.
  4. Missiles and projectiles fly (section 5).
  5. Point defence fires (section 5.3).
  6. Hits are resolved and damage applied (section 3).
  7. Ships move: rails, plus any burn.
- **Rails.** Ships coast on their conics. A burn is a piecewise change of velocity that spends delta-v. The simulation uses the same rail code as the main simulation, so survivors hand back a valid rail.
- **Cost.** At two hours with a 10 s step the coarse loop is a few hundred steps. The fine steps cover tens of seconds per missile. Cost is not a concern at the fight sizes in the design; revisit only if fights reach about a hundred ships.

## 3. Ships in the simulation

- **State per ship:** rail, mass, delta-v left, working units of each part group, magazines by weapon, decoys, stored heat, emission state, track picture, commander.
- **Parts.** A design lists parts with `count` and `min_working` (*Component Stats*). The simulation keeps the working count per part group. Below `min_working` the group is degraded: a drive group derates thrust, a radiator group derates the reactor through the heat balance, a point-defence group loses mounts. A sensor group below minimum loses range or tracking.
- **A hit** removes a working unit from a part group picked by weight (share of ship volume, exposed area weighting later). Whether a hit also destroys a neighbouring unit, and how armour changes it, is open (section 12).
- **Armour.** The catalogue's Whipple armour stops 50 MJ per hit. A Dart hit at 17 km/s carries far more, so armour does not stop a missile; it matters against debris and small projectiles. If missiles should be survivable, the warhead or the hit model has to change (section 12).
- **Catastrophic loss.** Some groups can destroy the ship when lost: reactor, propellant tank, magazine. Each has a chance `p_cat` per unit lost.
- **Status, derived:**
  - **Operational:** every group at or above `min_working`.
  - **Degraded:** a group below `min_working`.
  - **Disabled:** cannot manoeuvre, or cannot fight, or no command. This is the mission kill in 1.7.
  - **Destroyed:** catastrophic loss.
  Captured needs boarding and is a later subject (*Combat* 1.9, Ruthless/Humane).
- **Heat.** Ships store a bounded amount of heat. Lasers, active radar and thrusting add heat; radiators remove it. A ship that is full has to stop firing. This is the "duel of radiator margin" in 1.2, and it makes emission control cost something.

## 4. Sensing and tracks

A track is what one side believes about one contact: position, velocity, and an uncertainty radius u.

| Source | Gives | Range | Cost |
|---|---|---|---|
| Passive thermal | Bearing only, weak track | `detect_range` × √signature (MW) | None |
| Radar | Range and bearing, firm track | 20,000 km at 100 m²; scales as cross-section^¼ | The ship radiates and is seen at a range scaling with √(radar signature) |
| Lidar | Firm track, finer | 50,000 km at 100 m²; same scaling | Narrow beam, little tell-tale |
| Datalink | Shares tracks inside a side | Working comms | Lost if the comms or the sharing ship is |

- **Firm track:** u is a small fraction of range, refreshed each step. **Weak track:** u grows with the contact's reachable envelope, ½·a_max·t², between fixes, where a_max comes from the drive family and remaining delta-v (1.6).
- **Cross-section** comes from the hull. It is not in the catalogue yet (section 12). A bigger ship is seen further.
- **Fusion.** Own ships share one picture over datalink. The picture is per side; each commander reads it with their own skill (section 7).
- **Line of sight.** Bodies are spheres that block it. Expected to matter little inside a window; the test is cheap.
- **Going active is a decision.** A firm track costs concealment. The commander goes active when the value of a firm track exceeds the cost of being seen, with the same weight as every other decision (section 7), unless orders set emissions (section 8).
- **Decoys and clutter** do not change tracks. They act on seekers (section 5.1).

## 5. Weapons and defences

### 5.1 Missiles: guidance against evasion

A Dart has 8.0 km/s of delta-v and 20 g of acceleration, so about 40 s of burn in all. The burn is split: **boost** gives closing speed, and a **terminal reserve** corrects the aim. Closing speed is the ships' closing speed plus the boost. Full boost on the 9 km/s pass in 1.7 gives 17 km/s.

The aim error e is how far the missile's predicted intercept point is from where the target will be. It resets to the track's u at each track update and then grows with the target's unseen manoeuvre:

- e = u + ½·a_target·(time since update)², where a_target is what the target is actually doing.
- A missile can null an error if e / time-to-go ≤ its remaining delta-v. It misses by whatever is left over.
- **Midcourse updates.** While the launcher or any ship on the datalink holds a firm track, the missile is updated every `datalink_interval_s`. With 10 s between updates and a 0.1 g jink, e is about 50 m. Dodging does nothing.
- **Without updates** (the track is lost, the launcher is disabled, the target is stealthy) the missile flies on its own seeker, which acquires at `seeker_acquire_km`. At 17 km/s that leaves about 120 s, and a 2 km/s reserve can null about 235 km. A target that has jinked at 0.1 g for about 12 minutes has moved further than that, at a cost of about 0.7 km/s. Then the missile misses.
- **So:** evasion only pays against missiles that lost their updates, and breaking the launcher's track is how a ship defends against a salvo. A missile launched after closest approach cannot catch a target separating at 9 km/s, because the missile has only 8 km/s of delta-v. The launch window closes at closest approach.
- **Boost split** is set by the guidance rule: reserve the delta-v needed to null the largest error the target's remaining delta-v could produce, up to half the total; boost the rest. With working updates the reserve is small and the missile goes fast.
- **Decoys.** A chaff pack gives 8 decoys. When a seeker acquires, it takes a decoy with probability n/(n+1) × (1 − `seeker_discrimination`), where n is the number of live decoys. Decoys last `decoy_life_s`.
- **Salvo timing.** Missiles in a salvo are timed to arrive together by coasting at different speeds. The spread grows as coordination falls (section 7).

### 5.2 Kinetic launchers

- A Lance round leaves at 6 km/s, 5 kg. It is unguided. The chance to hit is the chance the miss distance is inside the target's radius r: p = 1 − exp(−r² / 2σ²), where σ combines the track's u, the target's dodge and the weapon's dispersion.
- For a 10 m radius: at 5,000 km, σ is about 500 m and p is about 2 in 10,000. At 100 km, σ is about 10 m and p is about 0.4. A launcher fires 60 rounds an hour.
- On a fast pass at 1,500 km standoff, a Lance never gets a useful shot. At a slow pass or a stop, it does. That is the "close range and area denial" role in 1.2, and it falls out of the numbers.
- Impact energy is ½·5·(15 km/s)² = 560 MJ at the 9 km/s pass, so a hit always penetrates.

### 5.3 Point defence

- A mount has range 300 km and a 0.3 chance per shot. The catalogue gives no rate: `pd_rate_per_s` is assumed to be 1.
- A mount engages one missile at a time, nearest first, inside its range, and spends the shot whether or not it hits. Each shot is a seeded draw.
- Time in range is range / closing speed: 300 km at 17 km/s is about 18 s. One mount expects 18 × 1 × 0.3 = 5.3 kills in that time. That is the figure in 1.7.
- **Saturation.** A defence stops about (working mounts) × 5.3 missiles from one direction before it runs out of time. Beyond that, missiles get through. Salvo size against mount count is the central contest (1.2).
- **Boost split changes it.** Slower missiles give point defence more time. Cutting boost from 8 to 6 km/s (15 km/s closing) raises the window to 20 s and the kills per mount to about 6.
- Point defence is automation. A commander does not retarget it. Skill reaches it only through coordination: how well the force's mounts cover each other (section 7).

### 5.4 Lasers (tier 3)

- A 20 MW beam draws 60 MWe and adds 40 MWt of heat. Energy on target is power × time on target × coupling, and coupling falls with range as the beam spreads. A ship is disabled when absorbed energy passes its hardness.
- The weapon stops when stored heat is full, so heat sets the shot length, and dwell time sets how much of it can be used (section 6).
- Mostly deferred until lasers are in the catalogue as offence (*Combat* 1.2).

## 6. Dwell time

Dwell time is time inside a weapon's useful range, about range over relative speed. It is computed per pair of ships and per weapon. Each weapon turns it into effect differently.

| Weapon | What dwell sets | On a 9 km/s pass at 1,500 km standoff |
|---|---|---|
| Missile | Launch window, which ends at closest approach (5.1). Salvos are limited by launcher cycle, magazine and the window. | Up to 93 minutes of launch window. Several salvos are possible. |
| Point defence | Time per missile in range: range / closing speed. Kills per mount = rate × time × 0.3. | About 18 s per missile, about 5 kills per mount. |
| Kinetic | Rounds fired inside effective range (100 to 200 km). | Zero: the standoff is outside effective range. |
| Laser | Energy on target, capped by stored heat. | Seconds of dwell at best. |

- A **Stop** makes dwell open-ended. Kinetic and laser weapons then matter, and the fight turns into a heat and magazine contest.
- A **Pass** makes dwell short. Missiles and point defence decide it, and the choice made at the order (speed, standoff) sets the numbers before the window opens.
- **Defenders influence speed.** A deep gravity well raises pass speeds, which shrinks dwell for everyone (1.3).

## 7. Commanders in the simulation

The senior officer present chooses actions for the whole force (1.9). Orders bound the choices; the commander picks within them.

- **Decision times.** Every `decision_interval_s`, and on new information (a track firms up, a launch is detected, a ship is lost). A decision takes effect after the commander's latency.
- **Actions:** go active or stay passive; launch a salvo of n at a target; hold; jink (a burn of given strength and length); deploy decoys; break off. Point defence is not an action.
- **Break off** means stop initiating fire and spend the return reserve to open the range. It works only where geometry and delta-v allow. A ship on a fast pass is already leaving. A ship that stopped to hold has to burn out.
- **One dial.** Each action is scored as V = ΔP(objective) × V_objective − λ × E[own loss] × V_own, using the commander's beliefs. The commander takes the best. λ is the weight on own losses, set by Bold/Cautious: `lambda_bold` below `lambda_avg` below `lambda_cautious`. The same function runs for every commander; only λ and the skill channels differ.
- **What the dial moves, without further rules:** how early to fire (bold accepts a weaker track), how large a salvo (bold launches more of the magazine), when to break off (bold at heavier losses), whether to spend reserve on jinking, and whether to go active. These are the behaviours listed in 1.9.
- **Skill.** With d = (10 − skill) / 10:
  - *Belief error:* the commander's figures for enemy magazines, mounts and delta-v are off by up to ±`belief_err_max` × d, applied once per engagement. The sensors' tracks are not affected.
  - *Decision latency:* `latency_max_s` × d.
  - *Coordination:* the share of the force's capacity that works together is 1 − `coord_loss_max` × d. It widens the salvo arrival spread and reduces the cover point-defence mounts give each other.
- **Experience.** After the fight the outcome updates kills, losses and survival per commander (1.9).
- **Tags.** The simulation records each decision with the action the baseline commander would have chosen: the same state, same skill, average λ. A decision tag is emitted when the choice differs by more than `tag_threshold`, for example "held 60% of magazines" or "broke off at 40% losses". The direction of the deviation is the evidence for the trait, and the tag never names the trait.
- **Flagship lost.** Command passes to the next senior captain after `succession_delay_s`, with that captain's skill and trait. During the delay the force acts on its last orders (candidate, 1.9).

## 8. Orders the player gives

The player sets orders beforehand and has no control during the fight. A force carries one set, set on its inset and read at t0.

| Field | Choices | Notes |
|---|---|---|
| Objective | Destroy, Defend, Hold, Escort, Capture, Salvage | Sets speed (Pass or Stop) and the value of the objective (1.3) |
| Approach | Pass or Stop, standoff km | The standoff shapes the rail. Default: the larger of 1,000 km and 3× the longest known enemy point-defence range. |
| Weapons | Free on hostile and unknown, free on hostile only, tight (fire only if fired on) | Unknown contacts need a rule |
| Reserves | Minimum return delta-v; minimum share of magazines held | The return reserve is the one in *Orbital Transfers* 3.6 |
| Priority | Warships, armed ships, any, transports first | Sets target choice in the salvo |
| Emissions | Active, on contact, silent | Silent keeps passive sensors only |

- **Standing orders** are a trigger and a response: a hostile detected in a named zone, a threat level above a threshold, an own asset attacked. The response is Intercept (with the fields above), Hold or Evade.
- **Threat level** is what the state knows about the contact, so it is as uncertain as the contact is: unknown or unarmed, armed, warship, force. How it is derived is open.
- The commander's trait and skill act inside these. Orders say whether and where to fight; they do not say how bold to be.
- Orders given during a phase wait for the next one (section 1).

## 9. What comes out

- **Result,** as in 1.8: per ship, status, condition, magazines and delta-v spent, and the new rail; wrecks as salvage sites; knowledge updates; the report; per-commander kills, losses and survival.
- **Event log.** One list of timed events is the source for the replay captions and the report: *detected, tracked, went active, launched, track lost, decoyed, intercepted, hit, disabled, destroyed, jinked, broke off, command passed*. Each has a time, the actors, a position and a payload. A decision tag attaches to the decision that caused an event.
- **Recording.** Rails for coasting, keyframes at `recording_hz` for powered phases. A missile is stored as its launch event plus keyframes through the boost and any terminal burn, then its coast as a rail.
- **Fog.** The recording holds the true state. What the replay shows by default is the player's picture, as it was (*Combat View* 3).

## 10. Testing the simulation

The simulation is a pure function, so most of it can be tested without a window.

- **Determinism:** the same inputs and seed give identical results, with or without recording.
- **Conservation:** delta-v, magazines and counts never go up or below zero. Rails handed back are valid.
- **Analytic checks:** one mount against N missiles kills about rate × time × 0.3; a lone missile on a firm track against a jinking ship hits; the same missile with no updates misses once the displacement passes the reserve's reach; missiles launched after closest approach never arrive.
- **Dial monotonicity:** raising λ never makes a commander fire earlier, launch a larger salvo or break off later.
- **Scenario tests:** the worked example in 1.7 as a fixture, with its expected timeline.
- **Calibration** (if a cheaper model is ever used for unobserved fights): its outcomes must match the full simulation on a set of fixtures, so results never depend on whether the player is looking.

## 11. Tunables (placeholders)

| Name | Value | Meaning |
|---|---|---|
| `window_cap_h` | 4 | Longest window before a new phase |
| `step_coarse_s`, `step_fine_s` | 10, 0.5 | Step outside and inside point-defence range |
| `fine_factor` | 3 | Fine step inside this multiple of point-defence range |
| `pd_rate_per_s` | 1 | Shots per mount per second |
| `datalink_interval_s` | 10 | Midcourse update spacing |
| `seeker_acquire_km` | 2,000 | Seeker acquisition range |
| `seeker_discrimination` | 0.5 | Chance a seeker ignores a decoy |
| `decoy_life_s` | 120 | How long a decoy works |
| `decision_interval_s` | 60 | Decision spacing |
| `lambda_bold`, `lambda_avg`, `lambda_cautious` | 0.5, 1, 2 | Weight on own losses |
| `belief_err_max` | 0.4 | Belief error at skill 0 |
| `latency_max_s` | 180 | Decision latency at skill 0 |
| `coord_loss_max` | 0.4 | Coordination loss at skill 0 |
| `tag_threshold` | 25% | Deviation that earns a decision tag |
| `succession_delay_s` | 600 | Time before command passes |
| `recording_hz` | 1 | Keyframe rate for powered phases |
| `p_cat` | per part | Chance a lost part destroys the ship |

## 12. Open questions

- How much damage a hit does: one unit, or more by energy. Whether armour should matter against missiles.
- Ship cross-section for radar and lidar, and the radar signature that gives an active ship away.
- The point-defence rate, and whether a mount can engage more than one missile at a time.
- Whether the missile guidance rule is the right level, or the boost split should be a design choice.
- How threat level is derived, and how standing orders are set and shown (*Combat View* 5).
- Whether the replay shows the player's picture or the truth, and when it flips (*Combat View* 3).
- The window cap, phase length and how a pause at the end of every phase feels in a long standoff.
- Whether weapons-free on unknown should be the default, and how a mistaken engagement is reported.
- Crew and casualties: whether they exist, and whether a disabled ship can still be boarded.
- How a decoy or sensor loss is shown to the player who was not looking.
- Lasers: coupling with range, hardness, and heat as a real constraint.
- Whether fixed assets have their own hit model.
- The default standoff.

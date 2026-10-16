# Orphan Worlds: Combat Simulation

**In short:** one engagement is one pure function. It takes the frozen state of the ships in reach of each other, with their orders, and returns the outcome and, if someone could watch, a recording. It works for any physically plausible design: weapons come in three delivery families, and every mount, sensor and part is read from its figures, never from a catalogue type. Tracks carry a continuous error that feeds every hit chance. Point defence is any mount that can engage a missile, and a launched missile is an uncrewed ship. A hit is a forced fault on a part, picked by presented area, with leftover energy spilling along the ship's spine through its frame. Breaking the frame is a kill, and only energy a part actually stores is ever released. Commanders follow a short list of rules whose thresholds come from their traits.

*Status:* **Direction** for the decisions in this doc (settled in a design session on 2026-10-15); every figure is a placeholder. *Combat, Raiders & Diplomacy* 1.8 is the summary of how a fight is run and shown; this doc holds the detail. The component tables in *Component Stats & Ship Assembly* are illustrative: nothing here depends on a particular design. Companion documents: *Combat, Raiders & Diplomacy*, *Component Stats & Ship Assembly*, *Orbital Transfers*, *Characters, Organisations & Events*, *Combat View*.

**Principle:** where a choice is open, take the simplest rule that the physics allows, so the model can be refined later without being rebuilt. Section 13 lists what was deliberately left out.

---

## 1. Forming an engagement

- **Opposed ships.** Two ships are opposed if their factions are at war, or if one side's orders allow firing on the other (an unknown contact under "hostile + unknown").
- **Reach** between two ships is the larger of:
  - **sensor reach:** any active sensor's range against the other's presented area (section 4);
  - **weapon reach:** how far any of its weapons' projectiles travels in `max_flight_s` at the closing speed plus the projectile's own delta-v. A Dart on a 9 km/s pass: 17 km/s × 3,600 s, about 61,000 km.
- **t0** is the first moment any opposed pair is within reach. Passive detection never forms an engagement: it happens days out and belongs to the main simulation and standing orders.
- **The frozen state.** At t0 the inputs are frozen: rails, delta-v, magazines, working parts, crew, orders, commanders. Ships whose rails enter reach later join at that time.
- **The window** runs until no opposed pair is within reach and none will be, capped at `window_cap_h`. At the cap a new phase forms at once from the survivors if any pair is still in reach. Missiles in flight carry over. Orders given during a phase take effect at the next.
- **Long shots** beyond `max_flight_s` are strategic launches, deferred: a salvo on a rail in the main simulation, like a fleet on an intercept, that forms its own engagement on arrival. Only the missiles and their targets would then be on the tactical map.
- **Bystanders.** A neutral ship within reach at t0 is in the simulation but never a target. It can be detected, can dodge, and breaks off by the civilian rule (section 8).
- **Shared bands.** The simulation reads true rail positions, so position round a band counts (*Orbital Transfers* 4.4). The order's standoff shapes the rail; the simulation never reads it (finding F-145).
- **Fixed assets** (stations, gates) are ships with no drive.

## 2. The step loop

- **A pure function:** frozen state, seed and tunables in; outcome and optional recording out. No clock, threads or global state.
- **Random numbers** are keyed by engagement id, entity ids and an event counter, never by loop order, so a fight is the same on any machine, with or without recording.
- **Adaptive step:** `step_coarse_s` while nothing is within `fine_factor` × the longest point-defence reach of any ship, `step_fine_s` once anything is. Coasting objects follow their rails exactly between steps; the step only sets how often sensing, decisions and point defence run.
- **Each step, in order:**
  1. Sensors update tracks (section 4).
  2. Commanders whose decision is due apply their rules (section 9).
  3. Launches, dodges and sensor changes take effect.
  4. Missiles follow their guidance rule (section 5.3); rounds fly on.
  5. Point defence engages (section 5.4).
  6. Hits are resolved (section 6).
  7. Ships move: rails, plus any burn.

## 3. What the simulation reads from a design

Any design the assembly model accepts can fight. The simulation reads these figures, all derived from the design's parts.

| Figure | Where it comes from |
|---|---|
| Parts | Each part line: design, `count`, `min_working`, mass; a section from its category (section 6.2) |
| Presented area per part | Radiators: tonnes ÷ areal mass. Tanks and cargo: the surface of their volume. Other parts: volume from a typical density for the category, area ∝ volume^⅔ |
| Shielding | One per-hit stopping energy for the ship (0 if none) |
| Stored energy per part | Chemical warheads, chemical motors with their oxidiser aboard; 0 for anything else |
| Mounts | Kind (trained, with a slew rate; or fixed), reaction time, shots or launches per second, magazine |
| Projectiles | Family and figures (section 5.1) |
| Sensors | Active range against a reference area; passive range against a reference signature |
| Crew | The assembly model's crew need, and the crew carried |
| Mass, thrust, delta-v, signature | The assembly model |

Category densities, the shielding figure, stored energy, mount figures and sensor reference values need adding to the component model before this is specced.

## 4. Sensing and tracks

- **One picture per side,** shared over the datalink while comms work.
- **A track** is a position, a velocity and one error radius *u*. Nothing is binary: *u* feeds every hit chance directly. The map's "firm" and "weak" tracks are bands of *u*, for display only.
- **What sets *u*:**
  - Passive thermal sensing gives a bearing with poor range, so *u* is large. It shrinks as more sensors, or more time, separate the bearings. A burning drive or missile motor is bright; a coasting missile is not.
  - Radar and lidar give a small *u*. Their range against a target scales with its presented area^¼, so a missile of a few m² is seen at a fraction of the range of a ship.
  - Between fixes, *u* grows by ½·a·t², where *a* is the most the contact could be accelerating, judged from what is known of its drive and delta-v.
- **Going active** reveals the ship at a range set by its emitted power. It follows the emissions order (section 10).
- **Line of sight:** bodies block it. Expected to matter rarely inside a window.
- **No electronic warfare** beyond decoys. Jamming would later raise the enemy's *u* or block updates within a range.

## 5. Weapons

### 5.1 Three delivery families

| Family | Figures | Examples |
|---|---|---|
| Guided projectile | Delta-v, acceleration, seeker range, mass, warhead (stored energy, or none for kinetic), presented area | Any missile, counter-missiles |
| Unguided projectile | Muzzle velocity, mass, dispersion | Any gun or launcher |
| Beam | Power, spot size at range | Lasers (deferred, section 13) |

Nothing else branches on weapon type. Flak, fragmenting warheads and mines would be new families or warhead options.

### 5.2 Mounts and the engagement test

- A **trained mount** turns at its slew rate; a **fixed mount** (a VLS, a spinal gun) doesn't aim. A guided projectile from a fixed mount turns itself, spending some delta-v off the launch axis. An unguided round from a fixed mount needs the whole ship to point, which the first model doesn't do.
- **A mount can engage any target, ship or missile, if:**
  1. its side holds a track on the target;
  2. a trained mount can slew onto it within its reaction time;
  3. its projectile can reach where the target will be before the target arrives;
  4. the hit chance in the time available is above zero.
- Point defence is not a role. It's any mount passing this test against an inbound missile.

### 5.3 Guided projectiles: missiles are uncrewed ships

- **A launched missile is a ship** with no crew and no commander: a rail, delta-v, acceleration, presented area, signature, seeker and warhead. It is detected, tracked, engaged and hit by the same rules as any ship. With almost no mass and a tiny frame, any hit that gets through kills it.
- **Guidance rule,** in place of a commander:
  - **Boost** for closing speed; keep a **terminal reserve** to correct the aim. The reserve is what's needed to cancel the largest error the target's remaining delta-v could produce, up to half the total.
  - **Midcourse:** while its side holds a track on the target and comms reach the missile, it is updated every `datalink_interval_s`. Aim error resets to the track's *u*.
  - **Orphaned:** with no updates, the error grows by ½·a·t² of the target's unseen manoeuvre until the seeker acquires at its seeker range.
  - **Terminal:** the missile hits if error ÷ time to go is within its remaining delta-v, and misses by the shortfall otherwise.
- **Consequences:**
  - Under live updates, dodging is pointless: with 10 s updates, a 0.1 g dodge is a 50 m error.
  - An orphaned missile can be dodged. A Dart's seeker at 2,000 km leaves about 2 minutes at 17 km/s, in which a 2 km/s reserve corrects about 235 km. A target that dodged at 0.1 g for 12 minutes has moved 254 km, at a cost of 0.7 km/s.
  - A missile can't catch a target separating faster than its own delta-v, so the launch window usually ends at closest approach.
- **Decoys:** when a seeker acquires, it takes one of *n* live decoys with probability n/(n+1) × (1 − `seeker_discrimination`). Decoys last `decoy_life_s`. Seekers never take anything but a decoy or their target.
- **Salvos** are timed to arrive together; the spread grows as the commander's coordination falls. Salvos are stored and drawn as groups.

### 5.4 Unguided projectiles

- Hit chance is 1 − exp(−r² / 2σ²), with *r* the target's size and σ combining track error, the target's dodging and dispersion.
- For a 10 m target: at 5,000 km, σ is about 500 m and p about 2 in 10,000; at 100 km, σ is about 10 m and p about 0.4.
- So guns matter only at close range or against slow targets, and against missiles only if their rounds can get there in time.

### 5.5 Point defence

- Each mount passing the engagement test fires at the inbound missile it has the best chance against.
- **Saturation** follows from time in range. One mount with 1 shot a second at 0.3 per shot, against missiles crossing 300 km at 17 km/s, kills about 5.3 per salvo direction. A force stops about that many per mount from one direction; beyond that, missiles get through. Salvo size against point-defence capacity is the central contest (*Combat, Raiders & Diplomacy* 1.2).
- A counter-missile is a guided projectile aimed at a missile, under the same rule.

## 6. Damage

### 6.1 A hit

- A hit carries energy E: its kinetic energy at the relative speed, plus any warhead's stored energy. A 0.4 t Dart at 17 km/s carries about 58 GJ, about 14 t of TNT.
- **Shielding** stops a hit whose energy is at most the ship's per-hit stopping energy. It matters against small rounds, fragments and debris; nothing a ship can carry stops a whole missile.
- A hit that gets through is a **forced fault**: the same fault the reliability model already uses, repaired later from spares.

### 6.2 Where it lands and how it spreads

- **The pick:** the part hit is a random choice weighted by presented area. Radiators, with large area for little mass, are hit often. Big hydrogen tanks too.
- **Sections** along a spine, assigned from part category: drive · power · tankage · core (cargo and gear) · habitat and command · weapons and sensors. The **frame** is the spine itself.
- **Absorption:** the struck part absorbs up to its mass × `disable_kj_per_kg`; a counted part (one drive, one reactor) then faults. A parametric part (a tank, a radiator) loses the share of its tonnes the energy covers: a tank loses that share of its propellant, a radiator that share of its rejection.
- **Spill:** energy left over crosses the frame, which absorbs `frame_share` of it, then moves to another part in the same section (picked by area), then to the neighbouring sections, nearest first, until it is used up.
- **Radiators sit outboard,** off the spine. A hit there wrecks that share of panel and the rest of the energy passes on into space: no spill and no frame damage.
- **Hit direction** is ignored: every hit uses the whole ship's area.

### 6.3 What it does

- **Re-assembly.** After each hit, the ship's figures are recomputed from its working parts by the assembly model: thrust, delta-v, power, heat balance, sensors, mounts. Losing radiators derates the reactor; losing a drive cuts thrust.
- **Kill:** the frame's toughness is its mass × `disable_kj_per_kg`. The ship is destroyed when frame damage passes `frame_kill_share` of that. A whole missile kills any ship up to a few thousand tonnes in one hit.
- **Stored energy** is released only by parts that store it, immediately, as part of the same spill, and only for the part destroyed (all of a counted part, the destroyed share of a parametric one). A magazine of kinetic Darts is inert; one of chemical-motor missiles can cook off. Choosing a chemical missile is a risk to your own ship.
- **Reactors don't explode.** A reactor hit means loss of power and a **contamination** flag on the ship: harder repair, riskier salvage, crew casualties.
- **Crew** live in the habitat section. Destroying a share of the habitat kills that share of crew; contamination kills more. A ship with less crew than its parts need is disabled. Named characters aboard die with their ship, or at the same chance as any crew member.
- **Status:**
  - *Operational:* every group at or above `min_working`.
  - *Degraded:* a group below `min_working`.
  - *Disabled:* can't manoeuvre, can't fight (no working sensor that can track, or no mount), or too few crew.
  - *Destroyed:* the frame is broken.

## 7. Dwell time

Dwell time, the time inside a weapon's useful range, isn't a separate rule. It's what the geometry gives each family:

| Family | What dwell sets | On a 9 km/s pass at 1,500 km standoff |
|---|---|---|
| Guided | The launch window, ending near closest approach | Up to 93 minutes; several salvos |
| Unguided | Rounds fired inside useful range | None at that standoff |
| Point defence | Time per missile in range | About 18 s, about 5 kills per mount |
| Beam | Energy on target | Deferred |

A Rendezvous makes dwell open-ended, so guns matter. An Intercept makes it short, so missiles and point defence decide.

## 8. Forces and command

- **Who commands:** the senior officer present commands every faction-commanded ship on that side, merging forces for the fight. "Senior" is highest known Command skill, then longest in post, until rank is designed.
- **Orders stay with their force:** each force's own orders still bind its ships. A convoy ordered to evade never fires, even under an escort's commander.
- **Own-flag civilian ships** (not commanded) follow the civilian rule: point defence if fitted, dodge, and open range at once. Requisitioned ships count as commanded.
- **Allies** fight under their own commander with their own picture.
- **A force is one point:** it shares one rail, so point defence is pooled. Every mount can engage a missile aimed at any member, scaled by coordination. Hits land on the ship a missile was aimed at. Formation spacing waits for area-effect weapons.
- **Non-combatants:** intercept and attack orders detach unarmed ships (tankers, transports) in the main simulation before any fight. They wait in place or at a rendezvous and rejoin after. Defend, Hold and Escort keep the force together. One switch on the orders overrides either way. A force caught together in transit fights together.
- **Command passes** after `succession_delay_s` if the commander's ship is destroyed or the commander dies. The force acts on its last decisions meanwhile.

## 9. The tactical AI

### 9.1 Rules

A fixed list of rules, checked in order at each decision (every `decision_interval_s`, and at once on new information). Orders remove actions before the rules run: weapons tight means no Fire until fired on; reserves are floors.

**Force level (the commander):**

| Rule | What it does | Bold | Average | Cautious |
|---|---|---|---|---|
| Break off | Stop firing and open range (9.2) at force losses of … | 60% | 40% | 20% |
| Target the shooter | With missiles inbound, put the ship guiding them first | no | yes | yes |
| Fire | Launch at a target when the expected hit chance per round is at least … | 0.2 | 0.4 | 0.6 |
| Salvo size | Enough to beat their estimated point defence × … | 1.5 | 1.2 | 1.0 |
| Hold back | Keep at least this share of the magazine, or the order's floor if higher | 0% | 25% | 50% |

The salvo is split across ships by who has missiles in reach above their hold-back share. Emissions follow orders.

**Ship level (crew and automation, using the commander's thresholds):**

| Rule | Trigger | Bold | Average | Cautious |
|---|---|---|---|---|
| Point defence | Always: the inbound missile with the best chance | — | — | — |
| Dodge gunfire | Unguided rounds inbound with flight time over `dodge_min_s` | yes | yes | yes |
| Dodge orphans | Missiles inbound whose launcher and its datalink partners have lost us, as far as the commander can tell | delta-v above reserve + 1 km/s | above reserve + 0.5 | above reserve |

### 9.2 Breaking off

- Stop initiating fire; point defence and dodging carry on.
- **Burn** at the force's real thrust (its slowest ship), down to the reserve floor and no further:
  - before closest approach, perpendicular to the relative velocity, widening the miss distance;
  - after it, along the relative velocity, away;
  - at low relative speed this is simply "burn away".
- It can fail. A low-thrust ship barely moves inside a fight, and a force with no delta-v above its reserve can't burn. The report says how it went.
- The enemy may pursue only by its own rules: firing while in reach, burning to chase only if its orders and reserve allow.
- After the window, the main simulation re-plans the route home from the new rail.

### 9.3 Stragglers

- A ship is a straggler if dropping it raises the force's acceleration by more than `straggler_gain`. Disabled ships are always left.
- **Ruthless/Humane** decides:
  - *Ruthless* abandons any straggler;
  - *Average* abandons only if staying together means the break-off fails, as far as the commander can tell;
  - *Humane* never abandons a ship that can still manoeuvre.
- An abandoned ship becomes a one-ship force under its captain.

### 9.4 Skill and tags

- **Skill**, with d = (10 − Command) / 10:
  - *belief error:* the commander's figures for enemy magazines, mounts and delta-v are off by up to ±`belief_err_max` × d, once per fight. This moves salvo size and break-off judgments, not the sensors.
  - *latency:* decisions take effect `latency_max_s` × d late.
  - *coordination:* salvo arrival spread and point-defence pooling worsen by up to `coord_loss_max` × d.
- **Decision tags:** each decision is compared with what the Average column would have done. A difference beyond `tag_threshold` is tagged as behaviour: "held 75% of magazines", "broke off at 20% losses", "left *Wren* behind". Tags never name a trait.

## 10. Orders

| Field | Choices |
|---|---|
| Objective | Destroy, Defend, Hold, Escort (Capture and Salvage later) |
| Approach | Intercept or Rendezvous; standoff in km (shapes the rail) |
| Weapons | Hostile + unknown, hostile only, tight |
| Target first | Armed, transports, any |
| Emissions | Active, on contact, silent |
| Reserves | Return delta-v; share of magazines held back |
| Non-combatants | Detach (default for Destroy) or keep together (default otherwise) |

Standing orders are a trigger and a response (Intercept, Hold, Evade, Alert), checked in order. The commander's traits act within the orders; orders say whether and where to fight.

## 11. What comes out

- **Per ship:** status, condition (working parts), crew, magazines and delta-v spent, contamination, new rail.
- **Wrecks:** a destroyed ship becomes a salvage site on its last rail. Its contents are the parts that survived. A wreck on a collision course is a main-simulation alert; diverting it is an order.
- **Disabled ships** drift on their rails with their surviving crew. Rescue, capture and recovery are later.
- **Missed missiles and rounds** never leave the engagement: they are drawn flying on in the replay and dropped when the window closes. No debris clouds.
- **What each side learns,** only from its own sensors and the event log:
  - seen behaviour: launches by count and type, point-defence mounts seen firing, decoys, burns (thrust, current mass and delta-v left, as in *Combat, Raiders & Diplomacy* 1.6);
  - confirmed armament and class (the identification ladder);
  - damage it could see: a ship that stops manoeuvring or firing reads as "disabled (est.)".
  Magazines beyond what was fired, design details, crew losses and traits are never learned from a fight.
- **Misidentification:** an unknown fired on that proves neutral is a diplomatic incident in the main simulation.
- **Per commander:** kills, losses, survival, tags.
- **Event log:** one list of timed events (detected, launched, track lost, decoyed, intercepted, hit, disabled, destroyed, dodged, broke off, abandoned, command passed). It drives the replay captions, the report and the tags.
- **Recording,** if someone could watch: rails for coasting, keyframes at `recording_hz` for powered flight.

## 12. Testing

- **Determinism:** same inputs and seed, same result, with or without recording.
- **Conservation:** delta-v, magazines, crew and working parts never rise, and never go below zero.
- **Analytic checks:** one mount against N missiles kills about rate × time × chance; a missile under updates hits a dodging ship; an orphaned one misses once the dodge exceeds its reserve's reach; a missile can't catch a target separating faster than its delta-v.
- **Damage checks:** a hit below the shielding does nothing; a radiator hit never reaches the frame; a hit that breaks the frame destroys the ship; an inert magazine never adds energy.
- **AI checks:** each rule fires at its threshold and not before; Bold never holds back more than Cautious; a Humane commander never leaves a manoeuvring ship.
- **Scenario:** the worked example in *Combat, Raiders & Diplomacy* 1.7 as a fixture.

## 13. Left out, for later

- **Hit direction:** ship attitude, presenting the narrowest face, radiators edge-on.
- **Formation spacing:** waits for area-effect warheads (nuclear missiles are tier 3).
- **Heat during a fight:** not presumed. Whether heat sinking is physically plausible needs evaluating first, before choosing a mechanism. Lasers wait for this. Emission control's dimming (*Combat, Raiders & Diplomacy* 1.5) depends on it too.
- **Lasers:** the beam family's hit and damage rules.
- **Electronic warfare:** jamming, spoofing, cutting datalinks.
- **Strategic missile launches** beyond `max_flight_s`.
- **Insensitive munitions:** a release fraction on stored energy.
- **Debris hazards,** stray missiles hitting bystanders, damage control during a fight.
- **Surrender, boarding, capture, rescue.**
- **A designer's own layout** in place of the automatic sections.

## 14. Tunables (placeholders)

| Name | Value | Meaning |
|---|---|---|
| `max_flight_s` | 3,600 | Longest flight counted in weapon reach |
| `window_cap_h` | 4 | Longest window before a new phase |
| `step_coarse_s`, `step_fine_s` | 10, 0.5 | Step outside and inside point-defence reach |
| `fine_factor` | 3 | Fine step inside this multiple of point-defence reach |
| `datalink_interval_s` | 10 | Midcourse update spacing |
| `seeker_discrimination` | 0.5 | Chance a seeker ignores a decoy |
| `decoy_life_s` | 120 | How long a decoy works |
| `disable_kj_per_kg` | 20 | Energy per kg to disable a part or the frame |
| `frame_share` | 0.3 | Share of each spill the frame absorbs |
| `frame_kill_share` | 0.5 | Frame damage that breaks the ship |
| `decision_interval_s` | 60 | Decision spacing |
| `dodge_min_s` | 30 | Shortest flight time worth dodging |
| `straggler_gain` | 25% | Acceleration gain that makes a ship a straggler |
| `belief_err_max` | 0.4 | Belief error at Command 0 |
| `latency_max_s` | 180 | Decision latency at Command 0 |
| `coord_loss_max` | 0.4 | Coordination loss at Command 0 |
| `tag_threshold` | 25% | Difference from Average that earns a tag |
| `succession_delay_s` | 600 | Time before command passes |
| `recording_hz` | 1 | Keyframe rate for powered flight |

The rule tables in 9.1 are tunables too.

## 15. Open questions

- The figures the component model must add: category densities for presented area, shielding, stored energy, mount kind and figures, sensor reference values.
- Calibrating `disable_kj_per_kg`, `frame_share` and `frame_kill_share` so a kinetic round damages a warship without killing it, and a missile kills.
- The point-defence shot rate and chance as design figures.
- How threat level is derived for standing orders.
- Decoy discrimination and how decoys are counted in a design.
- Whether line of sight round bodies is worth the cost.
- Whether fixed assets need anything beyond "a ship with no drive".
- Skill channel values once played.

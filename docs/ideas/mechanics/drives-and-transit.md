# Orphan Worlds: Drives & Transit

*Status:* Draft, 2026-10-10. Everything here is **Candidate** unless marked otherwise. Every figure comes from `scripts/transit/run_all.py` (output in `scripts/transit/results/report.txt` and `transit-table.csv`); low-thrust cases come from `lowthrust_cases.py`. Change `params.py`, not the tables.

> **Repo note (2026-10-10):** `scripts/transit/` is not in this repo yet; the figures below are as last generated. The prototype's Corvane layout (`src/content/corvane.rs`) does not match section 2 yet: see F3-05 in `docs/findings.md`.

*Companion documents:* *Design Document* (4.2 map and gates, 5 technology, 9 open questions on gate distance and delta-v), *Component Catalogue*, *Component Stats & Ship Assembly* (CSA: specific designs and ship assembly; it uses this document's orbits), *Requirements & Bids*, *Reference Sketches* (Corvane).

---

## 1. Summary

- **Drives:** the brief's baseline mostly holds. Two revisions: water in a thermal rocket gives about 3.2 km/s, not 3.5 to 4.5, so water NTR and solar thermal are short-range tugs; and NEP is slower than it looks (0.05 mg loaded for ion/Hall, 0.13 mg for plasma) because the power plant, not the thruster, sets acceleration.
- **Inner system:** hydrogen NTR (about 10 km/s per tank) is the workhorse: Iver → Halden IV in 81 days with good phasing. Delta-v beyond the Hohmann minimum buys speed slowly but buys freedom from waiting for windows quickly; that is the trade the player feels.
- **Windows matter more than flight time.** With a 295-day synodic period, a hydrogen NTR leaving at a random date takes a median 214 days (wait plus flight) for an 81-day trip. Waiting, not flying, is the main cost of low delta-v.
- **Gate distance:** the sketch's "41 d from Halden" is out of reach for everything short of pulsed fusion at 1.6 AU. **Recommended:** place gates by travel time, so the hydrogen NTR reaches them from the main world in about four to six months; in Corvane that is **2.0 AU** (174 days by NTR, 50 to 70 days by pulsed fusion, 22 to 30 days by torch), with Morrow moved out to about 4 AU.
- **Gate entry tolerance is load-bearing.** If a ship must match the gate's orbit exactly, a hydrogen NTR cannot reach a gate at 2 AU or more at all. A rule that gates accept ships arriving at up to about 10 km/s relative speed makes gate runs possible for the workhorse drive.
- **Convoy C-12 "arrives in 19 d" is consistent:** it is 76% through an 81-day hydrogen NTR transfer.

## 2. Corvane reference layout **(Candidate)**

Star: K2V, 0.75 solar masses, 0.30 solar luminosity; snow line about 1.48 AU (2.7 √L). Orbits circular and coplanar for now.

| Body | Radius AU | Period d | Orbital speed km/s | Sunlight (Earth = 1) | Notes |
|---|---|---|---|---|---|
| Pell | 0.25 | 53 | 51.6 | 4.80 | Inner planet |
| Corvane II | 0.36 | 91 | 43.0 | 2.31 | Implied by the numeral "Halden IV"; unnamed in the sketches |
| Iver | 0.50 | 149 | 36.5 | 1.20 | Dormant Iver Yard |
| Inner belt | 0.58 to 0.70 (working 0.64) | 186 to 247 | 33.9 to 30.8 | 0.89 to 0.61 | Wreck field; Varga Mining |
| Halden IV | 0.80 | 302 | 28.8 | 0.47 | Capital world |
| Morrow | 2.20 in the working layout; about 4.0 recommended (section 6) | 1,376 | 17.4 | 0.06 | Outer giant |
| Gate ring | 2.0 recommended (3.0 in the working layout) | 1,193 at 2.0 | 18.2 at 2.0 | | Section 6 |

- **Halden IV checks out:** equilibrium temperature 218 K at albedo 0.2, so the stated −41 °C (232 K) needs only a 14 K greenhouse from a thin N₂ atmosphere. At 0.82 g and Earth-like density it has a 5,753 km radius, 9.6 km/s escape speed and 7.5 km/s to reach low orbit. Leaving low orbit for an interplanetary trip adds 2.8 km/s; the transits below start and end in high parking orbits.
- **Surface access is a separate problem:** a single-stage shuttle to orbit needs a mass ratio of 5.5 on hydrolox and 10.4 on water NTR. Halden is a deep gravity well; most traffic should start from Arden Highport, supplied by shuttles or local ice.
- **Water ice** is stable on airless surfaces beyond about 1.8 AU; inside that, it survives in shadowed craters, deep in asteroids and at Halden's poles (the sketches' polar deposit).

## 3. Drive baseline (revised) **(Candidate)**

Typical ship of each family. "Mass ratio" is wet / (dry + cargo) at departure. Acceleration is for a typical loaded ship; small ships do better.

| Family | Tier | Propellant | v_e km/s | Typical acceleration | Mass ratio | Delta-v km/s | Jet power per kg of ship |
|---|---|---|---|---|---|---|---|
| Chemical (hydrolox from water) | 1 | LH2 + LOX (electrolysed water) | 4.4 | 1 g | 3.0 | 4.8 | 21.6 kW |
| Solar thermal | 1 | water | 3.2 | 3 mg (at Halden) | 3.0 | 3.5 | 47 W |
| NTR solid-core, water | 2 | water | 3.2 | 0.3 g | 4.0 | 4.4 | 4.7 kW |
| NTR solid-core, hydrogen | 2 | LH2 (from water) | 9.0 | 0.2 g (CSA haulers: 0.01 to 0.09 g) | 3.0 | 9.9 | 8.8 kW |
| NEP ion/Hall | 2 | water | 35 | 0.048 mg | 2.0 | 24.3 | 8 W |
| NEP plasma | 3 | water or LH2 | 50 | 0.13 mg | 2.0 | 34.7 | 32 W |
| Pulsed fission | 3 | pulse units | 30 | 0.5 g | 3.0 | 33.0 | 74 kW |
| NTR liquid/gas core | R | LH2 | 20 | 0.1 g | 3.0 | 22.0 | 9.8 kW |
| Pulsed fusion | 3 | D pellets + water | 200 | 4 mg | 2.5 | 183 | 3.9 kW |
| Antimatter-catalysed | R | catalysed pellets + water or LH2 | 150 | 10 mg | 2.72 | 150 | 7.4 kW |
| Continuous fusion torch | R | D + water | 500 | 20 mg | 2.72 | 500 | 49 kW |
| Beamed sail | 2 | none | n/a | 0.007 mg per 100 MW of beam on 10 t | n/a | unlimited near the beam | |

**Changes from the brief's provisional table.**
- **Water in thermal rockets:** 2.9 to 3.0 km/s at 2,500 to 2,800 K from simple nozzle theory, 3.2 km/s with optimistic engineering. Not 3.5 to 4.5. Solid-core hydrogen: 8.2 to 8.6 km/s from theory, 9.0 as the pre-collapse rating (CSA's Kestrel; Tern 8.6).
- **Chemical:** hydrolox made by electrolysing water (4.4 km/s, 1.28 t of water per t of propellant). CSA's methalox Corbel engine needs carbon, which is not in the resource list (section 9).
- **NEP:** the drive is limited by specific mass of the power plant. Built up from reactor, conversion, radiators and thrusters, ion/Hall comes to about 15 kg per kWe (radiators at 700 K are only 1 kg/kWe of it) and plasma to about 5.6 kg/kWe. With a quarter of the ship's mass in power plant, that gives 0.048 and 0.13 mg. The best exhaust velocity for 100 to 200 days of thrusting is 24 to 34 km/s (ion/Hall) and 43 to 61 km/s (plasma), so 35 and 50 are sensible typicals; CSA's Sable Hall cluster at 30 sits in range.
- **NEP propellant:** water, decided here as the default (argon and xenon are not in the resource list). Water-fed RF and Hall thrusters are plausible at lower efficiency; plasma drives may also run on hydrogen.
- **Pulsed fusion:** 200 km/s typical (CSA indicative 300; both are inside the brief's range). A 1,000 t ship burns only 0.5 to 1.4 t of deuterium for its full tank, but at 35 g of deuterium per tonne of water that means processing 13 to 39 kt of water: deuterium supply, not reaction mass, is its logistics.
- **Waste heat sets the ceiling:** radiators reject 2.5 kW/m² at 400 K and 96 kW/m² at 1,000 K (two-sided, emissivity 0.85). Pulsed fusion losing 10% of its power as heat needs about 3,900 m² of radiator at 1,200 K and 130,000 m² at 500 K. High-temperature radiators are what make the fast drives possible.
- **Torch:** at 20 mg and 500 km/s it already makes any inner-system trip a few days to a few weeks. At 0.1 g and 1,000 km/s it would need about 490 kW of jet per kg of ship: keep relic torches rare, low-acceleration or both, or they end orbital strategy.

**Thrust class (for CSA frame rules):** high = chemical, pulsed fission, small NTR ships; moderate = NTR haulers, liquid-core, pulsed fusion, torch; low = NEP, solar thermal, sails.

## 4. Water versus hydrogen **(Candidate)**

The double-duty resource creates a real choice. Propellant for a 100 t ship (dry mass plus payload):

| Delta-v km/s | Water NTR: propellant / wet mass t | Hydrogen NTR: LH2 (tank) / wet mass t | Ice processed for the LH2 t | Electricity MWh | LH2 volume m³ |
|---|---|---|---|---|---|
| 3.4 | 205 / 313 | 49 (7) / 157 | 441 | 3,155 | 694 |
| 5.0 | 444 / 562 | 84 (13) / 196 | 747 | 5,351 | 1,178 |
| 7.5 | 1,512 / 1,672 | 162 (24) / 286 | 1,444 | 10,345 | 2,277 |
| 10.0 | 16,789 / 17,561 | 293 (44) / 437 | 2,623 | 18,783 | 4,133 |
| 12.5 | infeasible | 549 (82) / 731 | 4,905 | 35,130 | 7,731 |

- **Water** is cheap, storable and needs no plant, but is useless much beyond 5 km/s. Ideal for tugs, belt work and shuttles.
- **Hydrogen** needs about 9 t of ice, 52 MWh of electrolysis and 12 MWh of liquefaction per tonne, plus cryogenic tanks (0.12 t per t in CSA's Rime store), and boils off: 0.019% a day with CSA's insulation (7% a year), 0.2% a day with poor tier 1 tanks (52% a year). Electrolysis also yields 8 t of oxygen per t of hydrogen: life support, or chemical oxidiser.
- **Consequence for the economy:** a CSA-sized NTR hauler burns about 2.1 kt of LH2 per loaded trip, about 18.6 kt of ice and 135 GWh of electricity. Halden's quarry (3.2 kt a month in the sketches) fuels one trip every six months. A hydrogen depot (power plant, electrolysis, liquefaction) is a major piece of infrastructure that the economy and station catalogues need.
- **Tankage figures:** this script used conservative family values (0.15 t/t for LH2, 0.04 for water, 0.05% a day boil-off); CSA's specific designs (0.12, 0.03, 0.019%) govern ships. The difference changes the table above by a few per cent.

## 5. Transit times **(Candidate)**

Fastest trip with good phasing, between high parking orbits, using the family's typical delta-v. "Out of range" means the delta-v is below what the trip needs. For pulsed fusion and faster, the second figure is for poor phasing (planets 90° apart).

| Trip (Hohmann delta-v, days; synodic period) | H2 NTR | NEP ion/Hall | NEP plasma | Pulsed fission | Liquid-core NTR | Pulsed fusion | Torch |
|---|---|---|---|---|---|---|---|
| Iver → Halden IV (7.5 km/s, 111 d; 295 d) | 81 | 191 | 114 | 30 | 43 | 25 / 44 | 11 / 20 |
| Iver → belt (4.2 km/s, 91 d; 482 d) | 43 | 127 | 81 | 15 | 21 | 17 / 41 | 8 / 18 |
| Halden IV → belt (3.4 km/s, 129 d; 759 d) | 52 | 170 | 111 | 17 | 25 | 18 / 46 | 8 / 20 |
| Halden IV → Morrow at 2.2 AU (10.8 km/s, 387 d; 387 d) | out of range | 386 | 236 | 131 | 184 | 54 / 76 | 24 / 31 |

- **Short-range drives:** chemical and water NTR reach the belt (76 to 97 days) but not Iver from Halden; solar thermal reaches the belt from Halden only (118 days).
- **NEP spirals** are slow and use more delta-v than impulsive trips (8.9 km/s for Iver → Halden IV against the 7.5 Hohmann minimum), but their large reserves let them leave at any time: the queue-free bulk hauler.
- **Real time:** at 1 day per second, the 81-day NTR trip lasts 81 seconds; an NEP hauler's 191 days about three minutes.
- **Halden IV → Iver with a random start date** (wait plus flight): hydrogen NTR best 81, median 214, worst 360 days; pulsed fission best 30, median 83, worst 175. To the belt the spread is wider (hydrogen NTR median 271, worst 646 days) because the synodic period is 759 days.
- **What extra delta-v buys:** CSA's tangent-departure figures for Iver → Halden IV give 98, 83 and 73 days for 8, 10 and 12 km/s spent: each extra km/s saves about a week. Phasing matters more: from Halden with Iver 120° behind, waiting 40 days gives an 86-day trip for 9.8 km/s or a 104-day trip for 8.1 km/s, and after about 60 days nothing fits a 9.9 km/s budget until the next window. Margin turns a missed window into a short wait instead of a long one.

**HR-1 check:** 10 km/s loaded is sound for Iver → Halden IV: 7.5 km/s minimum plus a margin for phasing. Bids at 9.7 to 10 km/s lose a day or two with good phasing and more with poor. A return leg needs refuelling at Halden.

**Sketch check:** Convoy C-12 "arrives in 19 d" fits an 81-day hydrogen NTR transfer (76% complete) or a 111-day Hohmann (83%). A whole Iver → Halden IV trip of 19 days needs 54 km/s (pulsed fusion or better).

## 6. Gate placement **(Candidate)**

### 6.1 Distance options, from Halden IV

Gate entry tolerance 10 km/s (6.2); best phasing; "rdv" = must match the gate's orbit.

| Gate radius AU | Gate period yr | Hohmann km/s, d | H2 NTR | NEP ion/Hall | NEP plasma | Pulsed fission | Pulsed fusion | Torch |
|---|---|---|---|---|---|---|---|---|
| 1.5 | 2.1 | 7.6, 260 | 108 (rdv 188) | 203 | 135 | 55 | 38 to 61 | 17 to 26 |
| 2.0 | 3.3 | 10.1, 349 | 174 (rdv: out of range) | 276 | 181 | 90 | 50 to 70 | 22 to 30 |
| 3.0 | 6.0 | 12.6, 552 | 304 | 395 | 251 | 158 | 69 to 91 | 30 to 37 |
| 5.0 | 12.9 | 14.5, 1,041 | 605 | 597 | 387 | 288 | 109 to 132 | 44 to 51 |
| 10.0 | 36.5 | 15.4, 2,646 | out of range | at least 770 | at least 596 | 618 | 210 to 237 | 80 to 87 |

- Distance the drives cover in **41 days** from Halden (the sketch's label): hydrogen NTR 1.0 AU, pulsed fission 1.3, liquid-core 1.15, pulsed fusion 1.6, antimatter 2.15, torch 4.55 AU.
- **Waiting:** from Halden to a 3 AU gate, hydrogen NTR best 304, median 459, worst 622 days with a random start (pulsed fission 158, 254, 408). Gate windows recur every synodic period (350 days at 3 AU).
- A 10 to 40 AU "edge" is unplayable for any reproducible drive (the hydrogen NTR cannot get there; pulsed fusion takes 7 months to 2.3 years).

### 6.2 Recommendation

1. **Place gates by travel time, not by a physical edge.** Gate radius chosen so a hydrogen NTR reaches it from the system's main world in about four to six months with good phasing. Corvane: **2.0 AU** (1.35 × snow line). Gate runs then take about 3 minutes of real time early in the game, about one minute with pulsed fusion and 30 seconds with a torch: the propulsion arc changes the strategic map, which is the point.
2. **Gate entry tolerance (new rule, Candidate):** a gate accepts ships arriving at up to about 10 km/s relative to it. Without it the workhorse drive cannot reach any gate beyond about 1.5 AU. The tolerance fits "how gates work is unexplained", and it creates a readable stat: ships must still brake on the far side, so arrival speed, not just travel time, is part of a gate run. Unmaintained gates could lower the tolerance (reduced throughput in the design document).
3. **Keep giants clear of the gate ring:** at least 5 Hill radii. A Jupiter-mass Morrow at 2.2 AU sweeps 1.38 to 3.03 AU, so it rules out a 2 AU gate. Move Morrow to about 4 AU (or make it an ice giant), and let the generator apply the same rule everywhere.
4. **Rejected for now:** 3 AU (keeps Morrow, but 10-month NTR runs and a 459-day median make early gate work glacial); 1.5 AU (3.5-month runs, but the gate sits under twice Halden's distance and loses the "edge" feel).

**Other stars:** the same rule scales. At a home-world sunlight level like Halden's, a red dwarf (M1V) world sits at 0.27 AU and moves at 40 km/s, so even a nearby gate costs 17.6 km/s (Hohmann) and the NTR cannot reach it with 10 km/s of tolerance; a sun-like (G2V) home world sits at 1.46 AU. Red dwarf systems are hard to leave: a useful source of variety for the map generator, which needs per-star gate rules (`report.txt`, star scaling).

## 7. Presentation **(Candidate)**

- **Replace fixed travel labels** such as "41 d from Halden" with figures for the selected ship or fleet: "Gate → Ostra · 2.0 AU · 174 d (NTR) · next good window in 63 d". With nothing selected, show the fastest available ship.
- **Show the window:** a small time–delta-v strip (porkchop-lite) on the transfer order, with "leave now" versus "wait for window" marked. The core trade is wait time versus delta-v, so show both.
- **Auto-slowdown:** windows open and close within seconds of real time; order queues ("depart at next window") and alerts are needed (risk register: time acceleration breaks orbital timing).

## 8. Data shape **(Candidate)**

- `params.py` holds the star, bodies, drive families and gate radius; the prototype will need the same as data: per system `star {mass, luminosity}`, per body `orbit {a, e, i, phase}`, per gate `orbit` plus `entry_tolerance`.
- Transit times in-game should come from a transfer solver (impulsive arcs within a delta-v budget; Edelbaum or a shooting method for low thrust), not from tables; the tables here are for tuning.

## 9. Open questions

- **Carbon:** methalox needs it; either add carbon (carbonaceous asteroids) to the resource list, or make all chemical drives hydrolox from water.
- **Hydrogen depot:** a station and infrastructure type (power, electrolysis, liquefaction), with an output rate the economy can tune; without it NTR haulers cannot run.
- **Gate tolerance:** value, whether it varies by gate condition, and whether arrival speed carries through to the far side.
- **Gate orbits:** circular at a fixed radius, or in the plane at varying phases? Several gates per system at different phases give different window timings per destination.
- **Planetary departure:** include the 2.8 km/s Halden low-orbit escape in ship budgets or treat Arden Highport as the start of all trips?
- **NEP:** whether low-thrust trips are solved in-game or estimated (Edelbaum gives 167 days for Iver → Halden IV against 191 from the optimised trajectory).
- **Torch tuning:** acceleration cap and rarity, so relic drives feel decisive without ending orbital strategy.
- **Non-circular orbits** and inclination: worth the generator's effort, or keep circular and coplanar?

# Orphan Worlds: World Generator

**In short:** the generator builds the sector for a new game: a flattened 3D volume of stars at realistic density and mix, gates only in the systems the old network had a reason to reach, links between them, full detail for gated systems only, and a historical layer of settlements, yards, ruins and salvage. It produces the truth; what each faction knows of it is handled separately (*Presentation & Visual Style*, 2: fidelity follows knowledge). The first five stages, the bodies of a gated system and a minimal view of the sector are specced. History and factions are not.

*Status:* draft. Companion to the overview (2, structure), *Gates*, *Salvage & Research* and *Setting & Back Story*; status tags as there (**Decided**, **Direction**, **Candidate**). All numbers are placeholders for tuning. The astronomical figures were checked against current surveys on 2026-10-16; the spec's sources paragraph says which held and which did not (`docs/spec/sector-generator.md`).

**Specced** in `docs/spec/sector-generator.md` (stages 1 to 5, the failure of links, names), `docs/spec/system-generator.md` (the bodies of a gated system) and `docs/spec/sector-view.md` (a minimal truth view of the sector, entering a system, start-up).

A **sector** is the star map of one game: a slab of star systems. A **system** is a star with its planets, moons and belts.

---

## 1. Principles

- **3D but flat (decided):** the sector is a three-dimensional volume, narrow in z. This is a concession to playability: the map reads well from a tilted top-down view, links rarely hide behind each other, and the player rarely has to rotate the camera. In reality the galactic disc is hundreds of parsecs thick, so a local volume would be roughly spherical. The flattening is deliberate, not a claim about the galaxy.
- **Informed by current science (decided):** star types, multiplicity, planet occurrence and system architectures follow observed distributions, not genre conventions. Where the game needs something rarer than nature provides (enough habitable worlds, say), the reason is the builders' choice of where to settle, not inflated odds.
- **Gates where there was a reason (decided):** not every system has a gate. The old network built them where something justified the cost: worlds to settle, resources, or a link needed to join the network together.
- **Non-visitable systems are lightly modelled (decided):** systems without a gate are never visited, so they need only enough detail to appear on the map and in the astronomical record.
- **Seeded and reproducible (decided 2026-10-16):** one seed reproduces a sector exactly. Each stage draws from its own derived seed, so tuning one stage doesn't reshuffle the others, and a system's detail depends only on the sector seed and the system's id. Why: tuning needs the rest of the map to hold still, and a system must look the same each time it is entered.

## 2. Pipeline (direction)

1. **Volume:** choose the sector size and shape from the game settings.
2. **Stars:** populate the volume with star systems at realistic density and mix (section 3).
3. **Summaries:** give every system a coarse summary (star, rough planet architecture, notable features), cheap enough to run for all of them.
4. **Gates:** score each system's value to the old network's builders and choose which were gated (section 4).
5. **Links:** connect the gated systems into a network and fail most of the links (section 4).
6. **Detail:** generate full bodies and orbits for the gated systems only (section 5).
7. **History:** lay down what the old network built, what the collapse destroyed, and what survives (section 7).
8. **Factions:** place starting factions to match their histories, then run fairness checks (section 8).

**Specced:** stages 1 to 5 in the sector generator; the bodies half of stage 6 in the system generator (the detail of a system is made when it is entered, not up front). Stages 7 and 8 are not specced.

## 3. Sector Volume & Stellar Population

- **Settings (decided 2026-10-16):** the player picks the number of gated systems, N, through a preset: Small 30, Medium 80, Large 200. The generator works up to about 300; values below 30 are a development setting. A seed is random by default and shown on screen. Why: N is what the player plays with; everything else follows from it.
- **Shape (decided 2026-10-16):** a slab, round-ish in plan (slightly elliptical), with a thickness of a quarter of its width by default (a fifth to a third is allowed).
- **Density (decided 2026-10-16):** realistic, about 0.1 star systems per cubic parsec, so nearest neighbours are typically 1 to 2 parsecs (3 to 6 light years) apart. A 10 pc census of 317 systems agrees to within its incompleteness.
- **Map size (decided 2026-10-16):** about three star systems for every gated one, so one in three is gated. The sector holds about 600 systems at Large, in a slab about 31 pc across and 8 pc thick; gated systems average 3.1 pc (10 ly) apart at every size. Why: the overview counts gated, visitable systems (2, structure); a third makes the ungated stars a believable majority without crowding the map.
- **Stellar mix (direction, approximate solar-neighbourhood figures by primary star):**

  | Primary | Share | Notes |
  |---|---|---|
  | M dwarf | ~70–75% | Dim; compact planet systems; flares, tidal locking |
  | K dwarf | ~12% | Long-lived, calm; arguably the best hosts for settlement |
  | G dwarf | ~7% | Sun-like |
  | F | ~3% | Brighter, wider habitable zone, shorter-lived |
  | A and brighter | <1% | Rare; short-lived; little time for settled planets |
  | White dwarf | ~5–8% | Stellar remnants; debris, polluted remnants, odd sites |
  | Giants | very rare | Possible as a notable landmark |

  - **Decided 2026-10-16:** brown dwarfs and giants are in the catalogue only. They get a place, an age and a metallicity, and never planets or a gate. About one brown dwarf for every five star systems (the 20 pc census finds four stars to one).
  - **Decided 2026-10-16:** white dwarfs can be gated, for science, and have their own remnant architecture (5.1).
- **Multiplicity (decided 2026-10-16, figures checked):** the catalogue records every companion with its separation. About a quarter of M dwarfs and about half of Sun-like stars have companions.
  - Wide pairs (over 1,000 AU) are two entries in the catalogue, each its own system with its own summary.
  - Intermediate pairs truncate planet systems: planets lie inside 0.3 of the nearest companion's separation.
  - Close pairs host circumbinary planets; these are not generated yet (section 9).
  - Only single-star systems can be gated. Why: a gate ring and a planet list round one star are the simplest thing to build and draw, and multi-star detail can wait.
- **Age and metallicity (decided 2026-10-16):** each system gets an age (0.2 to 10 billion years, limited by the star's lifetime) and a metallicity spread around solar. Higher metallicity raises the chance of giant planets and rich rocky material, so it feeds directly into resource value. Age governs M-dwarf activity and which stars have become white dwarfs.
- **Free-floating planets (decided 2026-10-16):** left out of the catalogue entirely for now. Surveys suggest planets without a star may be about as common as stars, and a gate at one would make an intriguing node (the game's title makes this hard to resist), but nothing in the first version can reach or use one. They can be added later as catalogue entries.

## 4. Gates & the Network

### 4.1 Why a system was gated (direction)

The old network's builders knew each system well and built gates where the value justified the cost. The generator scores systems by the motives below and gates the most valuable, plus whatever relays the network needs. The motives line up with the faction origins (*Factions & Ideology*, 3), which helps place starting factions.

| Motive | What makes a system attractive | Related origin |
|---|---|---|
| Settlement | Rocky worlds in or near the habitable zone with tolerable gravity and accessible water | Agricultural colony |
| Extraction | Rich belts, icy moons, metal-rich bodies, high metallicity | Frontier mining colony |
| Industry | Water and metals close together, a calm star, good transfer geometry | Shipyard hub |
| Junction | Central position with many neighbours in range; administration and traffic | Administrative centre |
| Science | Unusual objects: white dwarfs, flare stars, close binaries, young systems | Research outpost |
| Strategic | Approaches to valuable systems (a dense mesh has few true chokepoints) | Naval base |
| Relay | Little intrinsic value, but needed to join systems beyond link range | None; gate-maintenance outposts at most |

- **Decided 2026-10-16: how the gated set is chosen.** Exactly N systems are gated. Settlement, extraction, industry and science are intrinsic: a system has them whatever else is gated. Junction, strategic and relay depend on the shape of the network, which does not exist until the core is chosen. So: pick a core by score, with a quota of at least 10% of N reserved for each of the four intrinsic reasons and the rest by overall score; add junctions and then relays to join the core into one network; count all of them inside N (if the total is over N, drop the weakest core system and try again); label approaches to valuable systems strategic once the links exist (a dense mesh has almost no chokepoints, so those show in the surviving network instead). Every gated system keeps its main reason. Why: the circularity is broken in the only order that works, and the quota stops one reason (extraction, with belts) crowding the others out.
- **Decided 2026-10-16: belts weigh heavily in extraction.** Most extraction will be from belts, so belt mass and composition dominate the extraction score; giants add a little for helium-3.
- **Decided 2026-10-16: the core is picked strictly by score.** Why: an ungated star can't be reached, so the player has no reason to search for one, and a rich system left ungated on purpose would be a find nobody could use.
- **Relays (decided):** a dim red dwarf that is gated only because the network had to pass through it makes a natural chokepoint with nothing to live on. That suits raiders and defence. Relays are the last connectors added.
- **Ungated systems:** everything else. Most are dim M dwarfs with nothing the builders needed.
- **Names (decided 2026-10-16):** gated systems get names from a name generator; ungated stars get a catalogue designation. All names are placeholders.

### 4.2 Links

- **Links (decided 2026-10-16):** each gated system links to two to five gated systems within a maximum link range, twice the mean spacing of gated systems (about 6 pc, 20 ly). A minimum spanning tree guarantees the old network is connected; extra links go to the nearest neighbours, shortest first, and don't cross in the top-down projection, for readability. The old network is a dense mesh, as *Gates* (6) says: most systems have four or five links, about 4.3 on average. Why: the repair order below needs the links that have not collapsed to join every system, and with about 55% collapsed that takes about 2.2 links per system. A prototype with 2.9 links per system could not get the collapsed share above about 30%. Relays arise where the mesh is thin.
- **Out of the sector (decided 2026-10-16):** no links lead out of the sector and no gate is dark in the first version (*Gates*, 11).
- **Failed links (decided 2026-10-16):** the generator builds the old network, then fails most of its links. The targets are 30% working, 15% dormant and 55% collapsed, shifted a little for each map; they are targets, not exact, because the rules below come first, and a prototype put the result at about 30%, 18% and 51%. Failure is weighted towards links between busy, valuable systems, plus chance. Collapsed links leave a gate wreck in each of their two systems, as salvage. Dormant links are faulted at one end or both (*Gates*, 6, 7 and 9); whether a fault is damage or a deliberate lock is left to the historical layer. Rules the generator enforces:
  - every gated system keeps at least one gate that is not collapsed, so none is a pure orphan;
  - the largest working region holds at least 40% of the gated systems;
  - every pocket can be reached from the main region by a repair order: a sequence of dormant links, each faulted only at the end already held, so the holder can restart it. Why: a pocket that could never be reached would play no part in the game.
- **Gate position (decided 2026-10-16):** gates sit on a ring round the star, one per link end, at 1.35 times the snow line (the 2.0 AU of Corvane, *Drives & Transit*, 6). The ring slides to the nearest radius at least five Hill radii from every giant planet (outward first, then inward, within a limit); if none is in reach, the offending giant moves instead. A collapsed link's end is a wreck on the ring. Why: the giants' orbits are drawn realistically with no regard for the ring, so the ring adapts to the planets, not the planets to the ring.
- Fairness checks (section 8) run on the surviving network.

## 5. Gated Systems: Full Detail

### 5.1 Approach (direction)

Generate from observed occurrence rates and architecture templates rather than simulating planet formation: cheap, plausible and tunable. A coarse **summary** (a template, planets with their orbit, mass and kind, belts, and per-planet facts) is made for every system, and the detail of a gated system refines it and never re-rolls it.

- **Architecture templates by star type (decided 2026-10-16):**
  - compact multi-planet M-dwarf systems, like TRAPPIST-1
  - Solar-system-like, with an inner rocky zone and outer giants
  - a hot giant that has cleared the inner system
  - debris-disc systems
  - white-dwarf remnants with debris and surviving outer planets
  - Orbits are circular, coplanar and prograde (SCL-04). Eccentric and tilted orbits are left out. Why: the simulation holds only circular orbits, and the first version should not outrun it.
- **Occurrence (direction, figures checked 2026-10-16):**
  - Small planets are nearly everywhere: M dwarfs host about 1.2 planets of 0.5 to 2.5 Earth radii per star at periods up to 10 days, and at least a third of Sun-like stars host several planets of 1 to 4 Earth radii within a year.
  - Sub-Neptunes (between Earth and Neptune in size, absent from our own system) are the most common type.
  - M dwarfs typically host two or three small planets in close orbits.
  - Cold giants: about 14% of stars host one at 2 to 8 AU and 9% at 8 to 32 AU, rising with stellar mass and metallicity; a few percent of M dwarfs.
  - Hot Jupiters occur around 0.5 to 1% of Sun-like stars.
- **Stability (decided 2026-10-16):** planets are spaced at least eight mutual Hill radii apart; moons orbit within half their planet's Hill radius.
- **Key boundaries (decided 2026-10-16):** the habitable zone runs from 0.99 to 1.70 × √(L/L☉) AU (the Sun's conservative limits, checked), the snow line is at 2.7 × √(L/L☉) AU. Water ice is abundant beyond the snow line, so its distance shapes the trade-off between water and solar power. The snow line is a game convention, not a surveyed figure.

### 5.2 Bodies (candidate)

- **Planets:** rocky (airless, thin atmosphere, dense atmosphere, icy, molten), super-Earths, sub-Neptunes, ice giants, gas giants. **Decided 2026-10-16:** every planet carries its kind, gravity, equilibrium temperature, atmosphere class (airless, thin, dense, or an envelope) and water (none, ice or surface), shown in the inspection inset; molten and icy are not separate kinds.
- **Tidal locking (direction, 2026-10-17):** every planet and moon carries a **tidally locked** fact (yes or no), worked out by the generator beside the others. A locked body turns once per orbit and keeps one face to its parent. The rule is the standard locking time, t = 0.4 ω a⁶ m Q ÷ (3 G M² k₂ R³), compared with the system's age: locked if t is less than the age. Here ω is a starting spin (a 12 h day), a the orbit radius, m and R the body's mass and radius, M the parent's mass, and Q ÷ k₂ one number per kind (about 330 for rock, 2 × 10⁴ for a sub-Neptune, 7 × 10³ for an icy moon). It reads only figures the generator already has. Checked against known bodies it puts the Moon, Callisto and Iapetus in lock and Earth out of it. On generated systems every moon of a giant locks, and planets lock inside about 0.3 to 0.5 AU: in compact systems nearly all of them, in solar-like systems almost none. Spin-orbit resonances such as Mercury's 3:2 are not generated, so Mercury would count as locked. **Why:** the view needs it to place a hot world's molten side, and a rotation period and day length (which locked bodies fix at one orbit) are the first things a settlement or economy rule would ask for. **Specced** as SGEN-44, with moons in SYSG-33 and the inset line in INS-08. **Open:** the figures for Q ÷ k₂; whether a rotation period is also shown for the unlocked; whether a locked body's far side is a fact the inset shows.
- **Hot worlds and volcanic worlds (direction, 2026-10-17):** two different causes, told apart by two facts. A **lava world** is hot because of its star. Its surface temperature follows from equilibrium temperature and locking: a locked body with no atmosphere peaks at 4^(1/4) × Teq (about 1.41 ×) at the substellar point and falls with the cosine of the angle from the star to the power of 1/4, while a spinning one is one temperature all over. Rock glows from about 800 K and melts between about 1,400 and 2,200 K (tunable). So a locked airless world has a molten sea on its star-facing side from a Teq of about 1,000 K, and a spinning one needs about 1,500 K and then melts everywhere. About 63% of innermost rocky planets round F stars, 28% round G stars and about 1% round K stars have a Teq over 1,500 K in compact systems; M dwarfs have none. A **volcanic world** (Io, not Venus) is hot inside, which is the geological activity fact below. The two apply independently: a hot locked world can also be active. **Open:** whether a thin atmosphere on a hot world should spread some heat to the night side; rock vapour.
- **Geological activity (direction, 2026-10-17):** every rocky body (planet, moon or belt body) carries a **geological activity** fact, dead, active or extreme, so volcanic worlds have a reason to exist and crater density has a driver. It comes from an interior heat flux with two sources, both from figures the generator already has. **Radiogenic heat** scales with the body's mass and fades with the system's age: about 0.09 W/m² × (mass in Earths)^0.42 × e^(−(age − 4.5 Gyr) ÷ 3 Gyr), which gives Earth 0.09, Mars 0.035 and the Moon 0.014. **Tidal heat** is the standard flexing formula, calibrated so Io comes out at its real 2 W/m², using a **forced eccentricity**: a small, hidden number that stands for the stretching a body's neighbours keep up (Io's 0.004 is held by its resonance with Europa and Ganymede). Orbits stay circular on rails (SCL-04); the number feeds the heat only. Flux below 0.05 W/m² is dead, up to 0.5 active, above that extreme (all tunable). In the view: dead keeps craters and rays; active thins the craters and loses rays where the ground is resurfaced; extreme is the Io look of mock-up 30 (sulphur patches, paterae, plumes, no craters). On a rough sample, about a sixth of giants' moons come out extreme at a forced eccentricity of 0.004, and a similar share of compact-template rocky planets at 0.001. **Why:** without it every rocky body is a cratered ball; with it the generator can say which worlds are alive, from facts it already has, and nothing else needs to leave the generator. **Specced** as SGEN-45, SYSG-33 and INS-08. **Open:** one forced eccentricity per place is blunt; a value that rises when a moon has an inner neighbour near a 2:1 period ratio would tie the heat to a visible fact. Tune the single number first.
- **Moons:** giants get moon systems, often with large icy moons, which are the main water sources. Rocky planets get few or none. **Decided 2026-10-16:** moons are named for their planet and a lowercase letter by distance (Morrow a, Morrow b), because numerals don't suit both planets and moons.
  - **Decided:** the test system (Corvane) is a plausible output of the generator, so its giants and their moons use realistic figures. Its main job is testing, so it may be less typical than the generator's usual output. Why: an implausible test system would hide problems the generator will meet, while an unusual one still exercises them.
  - **Direction, giant mass:** real gas giants run from about a tenth of a Jupiter mass (Saturn is 0.3) to several. Corvane's Morrow is one Jupiter mass.
  - **Decided 2026-10-16, moon spacing:** large regular moons sit from about 2 to 30 planet radii out, with periods of about 1 to 20 days (Io 1.8 d, Callisto 16.7 d). None sit inside 2 radii, where tides tear moons apart. Moons stay within half the planet's Hill radius (about 24 million km for Morrow). Small inner moons and distant captured moons also occur; the generator leaves captured moons out.
  - **Candidate:** the test system leaves out distant captured moons, because the satellite fit (ZOOM-15) fits every satellite and a moon at tens of millions of km would shrink the rest to nothing.
  - **Specced:** Morrow's mass and moon periods, in TEST-07 and TEST-08 (`docs/spec/system-view.md`); generated moons in SYSG-30 to SYSG-35 (`docs/spec/system-generator.md`).
  - **Decided:** a second test system, Sol, is the real solar system inside the simulation's limits: the Sun, Mercury to Mars, Ceres, Jupiter, Saturn, Uranus and Neptune with their main moons and the main asteroid belt. Why: Corvane's figures are invented, so they can't show whether scale, spacing and speed feel right. Sol gives reference points people already know, such as how far Mercury sits from the Sun, how big Jupiter's moon system is and how long Neptune takes to go round.
  - **Direction, what Sol leaves out:** orbits are circular, coplanar and prograde (SCL-04), so Sol uses mean distances, ignores eccentricity and tilt, draws Triton prograde, and draws Uranus's moons in the system plane rather than round its tilted equator. Left out: the smallest regular moons (Miranda among them), Pluto and Charon (eccentric, tilted and locked in a resonance with Neptune, and Charon is an eighth of Pluto's mass, so SCL-04's parent-only period would be about 6% off), distant captured moons (as for Corvane), rings, Trojans and comets. It has no gates or salvage sites.
  - **Decided:** Sol also holds two stations and two ships round Earth, so transfers can be tried against known figures once they are built: Perigee Station, Marlin (a near-future hydrogen nuclear thermal ship, 15 km/s) and Vestige (a relic fusion-torch ship, 500 km/s) in the Low band, with Marlin docked at the station, and Apogee Station in the High band. Why: Corvane's ships are at invented planets, and one band-to-band trip round a planet people know is the smallest test of the transfer feature. Two drives, a workhorse that can explore the inner system and a relic that can go anywhere, show what the feature does at the edge of one ship's reach and with no edge at all.
    - Marlin to Apogee Station is the Low-to-High Hohmann crossing: 4.0 km/s of its 15, 52 h, with a window every 95 min. It can make the crossing three times, so the return and a second trip are possible and a fourth is not. Marlin to Perigee Station is already there (PHAS-04).
    - Vestige is 120° ahead of the station, so Vestige to the station is a phasing leg (PHAS-05); Marlin to Vestige is one too.
    - Vestige's first burn for the same crossing is 4.2 h at 20 milli-g (Marlin's is 51 min at 0.1 g), against a 52 h crossing. With the High band 34 radii out, burn time is a small share of the trip, so it tests the burn-time rule only lightly; a nearer High band would test it harder (`high_band_radii` is a tunable).
    - Marlin's 15 km/s reaches the inner system out to Ceres. Minimum costs from the Low band, rendezvous into a Low band at the target: the Moon 4.5 km/s, Mars 10.1, Venus 11.3, Ceres 14.4. Waiting for a window costs more than these Hohmann figures, so the Ceres trip leaves little spare. Mercury (21.4 km/s) is out of reach except as a flyby (10.7), and Jupiter (35) is far out. Vestige reaches all of them. These costs follow the current departure and capture rules, which overstate a real trip by 1.5 to 1.8 times (F-158): with realistic costs Mercury costs about 13 km/s and Marlin would reach it, so its delta-v will be set again when the rules are fixed. Trips past 6 AU meet the open point below.
  - **Direction, small moons in Sol:** Phobos, Deimos, Amalthea, Mimas, Enceladus and Proteus have no High band, and their Low band (500 km up) lies outside their own Hill spheres, so it is no stable orbit at all (Phobos's Hill sphere is 17 km across, its Low band 511 km out). Sol keeps them on purpose, to show how BAND-01 meets real small moons. **Decided** 2026-10-16: a small moon may have no High band. Corvane's Morrow I and II are the same case (TEST-20). Expect findings, and expect the fix to be BAND-01's, not Sol's.
  - **Candidate:** Sol's day 0 is J2000.0 and the planets start at their real mean longitudes, but the calendar still starts on its own epoch (14 Mar 2341). Give a system its own start date if comparing with the real sky matters.
  - **Open:** Sol now has ships, and `lost_beyond_au` (6 AU) puts Saturn and Neptune beyond the point where a coasting ship is lost (BAND-08), so Vestige can't be sent there. It needs to be per system, or much larger.
  - **Specced:** Sol's content, its test ships and stations, and the start-up choice of system, in SOL-01 to SOL-17 and SOL-22 to SOL-23 (`docs/spec/sol.md`).
- **Small bodies (decided 2026-10-16):** a system has up to three belts, each an annulus with a mass and a rocky, metallic and icy composition: inner belts rocky and metallic, outer belts icy. A belt may hold a few large bodies, which are ordinary planet-class bodies. Trojans, comets and rings are left out of the first version. Belts are drawn as density shading, with notable sites as individual markers (*Presentation & Visual Style*, 2); the belts are a list, not a single belt (SCL-08). Why: most extraction will be from belts, so the belt is what the economy needs to know about; Trojans and comets add nothing the belts don't yet.
- **Orbits (decided 2026-10-16):** circular and on rails, as everywhere in the simulation (SCL-04). Eccentricity, inclination and resonances are left out for now. The wider simulation's detail is still open (*Drives & Transit*, 9).

### 5.3 Worlds that support human life (direction)

Starter worlds support human life in habitats on hostile surfaces (*Economy*), not in the open air.

- **Candidate criteria:** gravity roughly 0.3 to 1.5 g; surface temperature and pressure within what habitats can manage; accessible water; a star that isn't too active. The settlement score of the generator uses these (SGEN-50).
- Halden IV in the reference sketches (cold terrestrial, 0.82 g, thin nitrogen, −41 °C) is the intended flavour.
- Close-in worlds of M dwarfs are often tidally locked and exposed to flares, which makes them harder but not impossible homes.

### 5.4 Resources (candidate)

Each body carries rough deposits of the economy's few resources (*Economy*):

- **Water (ice):** beyond the snow line, icy moons, comets, polar caps of inner worlds.
- **Metals:** rocky and metallic asteroids, rocky planets and moons.
- **Rare metals:** scarce and localised; concentrated in metallic asteroids and more common in high-metallicity systems.
- **Helium-3:** gas giants; present but minor.

**Decided 2026-10-16:** the first version records composition class only (rocky, metallic and icy fractions of a belt; water as none, ice or surface on a planet). Deposit amounts wait for the economy spec. Why: how much of what a body holds depends on how the economy counts resources, which is not designed yet.

## 6. Non-Visitable Systems (direction)

Modelled at catalogue level only:

- Position, star type(s), age, metallicity, and a catalogue designation rather than a name (the old network named only the systems it reached). **Decided 2026-10-16.**
- **Known planets (decided 2026-10-16 for the truth view):** an ungated system carries the same coarse summary as a gated one, and the truth view of the sector shows it. A view filtered by what a faction knows would show a short list as astronomers would know it, biased towards large planets and close orbits, as real detection methods are (candidate).
- **Uses:** landmarks and depth in the sector view; lore and recovered records ("a survey of this system was planned"); candidates for dark-gate destinations later.
- They are **never** visited, so full detail is never generated. If a later feature makes one visitable, the coarse summary seeds its detail on demand.

## 7. Historical Layer (direction)

Laid down after the physical map, in the same order as the setting's timeline (*Setting & Back Story*):

1. **Old network:** settlements, yards, stations and gate-maintenance outposts, placed by each system's gate motive.
2. **Collapse:** destruction, abandonment and decay, varying by system. Most links collapse; some gates are dormant and many survivors are degraded (*Gates*, 6 and 7).
3. **Turmoil and recovery:** surviving populations consolidate into the starting factions on their worlds.
4. **Result at game start:** inhabited worlds, dormant yards, salvage sites with partly known contents, ruins and records. Richer sites lie further out or behind dark gates (*Salvage & Research*).
5. **Authored beats:** placement rules for Earth-mystery clues and external-threat hints. Placement varies by map; the truth stays fixed.

## 8. Factions & Fairness (candidate)

- Starting factions are placed on systems whose gate motive fits their origin: a shipyard hub where the yards are. This would answer the open question in *Factions & Ideology* about matching starts to origins.
- Fairness checks: comparable starter worlds, resources within reach, distance to rivals, and access to gates and salvage. Fairness is in the totals; starts can differ in kind.
- AI factions draw histories weighted by the map position they receive.

## 9. Open Questions

- Whether native life exists anywhere, and in what form.
- How habitable worlds are guaranteed in sufficient numbers without unrealistic odds. The settlement quota (at least 10% of gated systems are chosen for settlement) does not guarantee that any of them holds a habitable world.
- Circumbinary planets, and whether a binary can be gated.
- Whether a dormant gate's fault is damage or a lock, and which; the historical layer decides it.
- How the historical layer is simulated: rules or a short simulated history.
- How the simulation copes with a few hundred systems, if more than the entered one ever runs at once.
- When dark links to systems outside the sector, and free-floating planets, are added.
- What a faction knows of the sector, and how the sector view is filtered by it.

# Orphan Worlds: World Generator

**In short:** the generator builds the cluster for a new game: a flattened 3D volume of stars at realistic density and mix, gates only in the systems the old network had a reason to reach, links between them, full detail for gated systems only, and a historical layer of settlements, yards, ruins and salvage. It produces the truth; what each faction knows of it is handled separately (*Presentation & Visual Style*, 2: fidelity follows knowledge).

*Status:* draft. Companion to the overview (2, structure), *Gates*, *Salvage & Research* and *Setting & Back Story*; status tags as there (**Decided**, **Direction**, **Candidate**). All numbers are placeholders for tuning. The astronomical figures are approximate and should be checked against current surveys before they are fixed.

---

## 1. Principles

- **3D but flat (decided):** the cluster is a three-dimensional volume, narrow in z. This is a concession to playability: the map reads well from a tilted top-down view, links rarely hide behind each other, and the player rarely has to rotate the camera. In reality the galactic disc is hundreds of parsecs thick, so a local cluster would be roughly spherical. The flattening is deliberate, not a claim about the galaxy.
- **Informed by current science (decided):** star types, multiplicity, planet occurrence and system architectures follow observed distributions, not genre conventions. Where the game needs something rarer than nature provides (enough habitable worlds, say), the reason is the builders' choice of where to settle, not inflated odds.
- **Gates where there was a reason (decided):** not every system has a gate. The old network built them where something justified the cost: worlds to settle, resources, or a link needed to join the network together.
- **Non-visitable systems are lightly modelled (decided):** systems without a gate are never visited, so they need only enough detail to appear on the map and in the astronomical record.
- **Seeded and reproducible (candidate):** one seed reproduces a map exactly. Each stage draws from its own derived seed, so tuning one stage doesn't reshuffle the others.

## 2. Pipeline (direction)

1. **Volume:** choose the cluster size and shape from the game settings.
2. **Stars:** populate the volume with star systems at realistic density and mix (section 3).
3. **Summaries:** give every system a coarse summary (star, rough planet architecture, notable features), cheap enough to run for all of them.
4. **Gates:** score each system's value to the old network's builders and choose which were gated (section 4).
5. **Links:** connect the gated systems into a network (section 4).
6. **Detail:** generate full bodies and orbits for the gated systems only (section 5).
7. **History:** lay down what the old network built, what the collapse destroyed, and what survives (section 7).
8. **Factions:** place starting factions to match their histories, then run fairness checks (section 8).

## 3. Cluster Volume & Stellar Population

- **Shape (candidate):** a slab, roughly round or irregular in plan, with a thickness of about a fifth to a third of its width.
- **Density (candidate):** realistic, about 0.1 star systems per cubic parsec, so nearest neighbours are typically 1 to 2 parsecs (3 to 6 light years) apart. For scale, a slab 6 pc thick and about 25 pc across holds roughly 300 systems.
- **Map size (candidate):** the overview's 10 to 100 systems counts gated, visitable systems. If a quarter to a half of all systems are gated, a large map holds a few hundred stars in total.
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

  Brown dwarfs add roughly one for every four to six stars; most are isolated, some are companions.
- **Multiplicity (direction):** roughly a quarter of M dwarfs and about half of Sun-like stars have companions.
  - Wide pairs behave as two nearby systems.
  - Close pairs host circumbinary planets.
  - Intermediate pairs truncate planet systems.
- **Age and metallicity (candidate):** each system gets an age (0 to about 10 billion years) and a metallicity spread around solar. Higher metallicity raises the chance of giant planets and rich rocky material, so it feeds directly into resource value. Age governs M-dwarf activity and which stars have become white dwarfs.
- **Free-floating planets (candidate):** surveys suggest planets without a star may be about as common as stars. Most are invisible, but a few could appear as charted objects, and a gate at one would make an intriguing node. The game's title makes this hard to resist.

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
| Strategic | Chokepoints and approaches to valuable systems | Naval base |
| Relay | Little intrinsic value, but needed to join systems beyond link range | None; gate-maintenance outposts at most |

- **Relays (candidate):** a dim red dwarf that is gated only because the network had to pass through it makes a natural chokepoint with nothing to live on. That suits raiders and defence.
- **Ungated systems:** everything else. Most are dim M dwarfs with nothing the builders needed. A few rich systems were never gated (too remote, or planned and never finished), which can feed lore and regret.

### 4.2 Links (candidate)

- Each gated system links to a small handful (two to five) of gated systems within a maximum link range. The network is connected.
- Links prefer near neighbours, so relays and chokepoints arise naturally rather than by decree.
- A mild preference for links that don't cross in the top-down projection, for readability.
- Some links lead out of the cluster and are dark (*Gates*, 11).
- **Failed links (candidate, speculative):** the generator builds the old network, then fails most of its links. Fairness checks (section 8) run on the surviving network. Collapsed links leave a gate wreck in each of their two systems, as salvage. Some links are dormant rather than collapsed, faulted or locked at one end or both. See *Gates* (6, 7, 9).
- Gates sit at the system edge, one per destination (*Gates*, 2). The gate's distance from the star, and whether it scales with the star's mass or the extent of its planets, is open (section 9).

## 5. Gated Systems: Full Detail

### 5.1 Approach (direction)

Generate from observed occurrence rates and architecture templates rather than simulating planet formation: cheap, plausible and tunable.

- **Architecture templates by star type (candidate):**
  - compact multi-planet M-dwarf systems, like TRAPPIST-1
  - Solar-system-like, with an inner rocky zone and outer giants
  - a hot or eccentric giant that has cleared the inner system
  - debris-disc systems
  - white-dwarf remnants with debris and surviving outer planets
- **Occurrence (direction, approximate):**
  - Small planets are nearly everywhere.
  - Sub-Neptunes (between Earth and Neptune in size, absent from our own system) are the most common type.
  - M dwarfs typically host two or three small planets in close orbits.
  - Cold giants occur around perhaps 10 to 20% of Sun-like stars and only a few percent of M dwarfs.
  - Hot Jupiters occur around about 1% of stars.
- **Stability (candidate):** planets are spaced by a minimum number of mutual Hill radii; moons orbit within a fraction of their planet's Hill sphere.
- **Key boundaries (direction):** the habitable zone is at about 0.95 to 1.7 × √(L/L☉) AU, the snow line at about 2.7 × √(L/L☉) AU. Water ice is abundant beyond the snow line, so its distance shapes the trade-off between water and solar power.

### 5.2 Bodies (candidate)

- **Planets:** rocky (airless, thin atmosphere, dense atmosphere, icy, molten), super-Earths, sub-Neptunes, ice giants, gas giants.
- **Moons:** giants get moon systems, often with large icy moons, which are the main water sources. Rocky planets get few or none.
  - **Decided:** the test system (Corvane) is a plausible output of the generator, so its giants and their moons use realistic figures. Its main job is testing, so it may be less typical than the generator's usual output. Why: an implausible test system would hide problems the generator will meet, while an unusual one still exercises them.
  - **Direction, giant mass:** real gas giants run from about a tenth of a Jupiter mass (Saturn is 0.3) to several. Corvane's Morrow is one Jupiter mass.
  - **Direction, moon spacing:** large regular moons sit from about 2 to 30 planet radii out, with periods of about 1 to 20 days (Io 1.8 d, Callisto 16.7 d). None sit inside about 2 radii, where tides tear moons apart. Moons stay within about half the planet's Hill radius (about 24 million km for Morrow). Small inner moons and distant captured moons also occur.
  - **Candidate:** the test system leaves out distant captured moons, because the satellite fit (ZOOM-15) fits every satellite and a moon at tens of millions of km would shrink the rest to nothing.
  - **Specced:** Morrow's mass and moon periods, in TEST-07 and TEST-08 (`docs/spec/system-view.md`).
  - **Decided:** a second test system, Sol, is the real solar system inside the simulation's limits: the Sun, Mercury to Mars, Ceres, Jupiter, Saturn, Uranus and Neptune with their main moons and the main asteroid belt. Why: Corvane's figures are invented, so they can't show whether scale, spacing and speed feel right. Sol gives reference points people already know, such as how far Mercury sits from the Sun, how big Jupiter's moon system is and how long Neptune takes to go round.
  - **Direction, what Sol leaves out:** orbits are circular, coplanar and prograde (SCL-04), so Sol uses mean distances, ignores eccentricity and tilt, draws Triton prograde, and draws Uranus's moons in the system plane rather than round its tilted equator. Left out: the smallest regular moons (Miranda among them), Pluto and Charon (eccentric, tilted and locked in a resonance with Neptune, and Charon is an eighth of Pluto's mass, so SCL-04's parent-only period would be about 6% off), distant captured moons (as for Corvane), rings, Trojans and comets. It has no gates or salvage sites.
  - **Decided:** Sol also holds two stations and two ships round Earth, so transfers can be tried against known figures once they are built: Perigee Station, Marlin (a near-future hydrolox chemical tug, 4.8 km/s) and Vestige (a relic fusion-torch ship, 500 km/s) in the Low band, with Marlin docked at the station, and Apogee Station in the High band. Why: Corvane's ships are at invented planets, and one band-to-band trip round a planet people know is the smallest test of the transfer feature. Two drives at opposite ends of the arc show what it does when delta-v is scarce and when it is not.
    - Marlin to Apogee Station is the Low-to-High Hohmann crossing: 4.0 km/s of its 4.8, 9.2 h, with a window every 98 min. It can't make a second crossing or the return, which tests an order that is too costly. Marlin to Perigee Station is already there (PHAS-04).
    - Vestige is 120° ahead of the station, so Vestige to the station is a phasing leg (PHAS-05); Marlin to Vestige is one too.
    - Vestige's first burn for the same crossing is 3.7 h at 20 milli-g, against a 9.2 h crossing. Burn time is a large share of a trip this short, so it tests the burn-time rule at small scale. Expect a finding.
    - Marlin can only just reach the Moon (about 4.5 km/s of its 4.8, with Hohmann timing and capture into the Moon's Low band) and can't reach Mars (about 10 km/s). Vestige reaches both. Trips past 6 AU meet the open point below.
  - **Direction, small moons in Sol:** Phobos, Deimos, Amalthea, Mimas, Enceladus and Proteus have no High band, and their Low band (500 km up) lies outside their own Hill spheres, so it is no stable orbit at all (Phobos's Hill sphere is 17 km across, its Low band 511 km out). Sol keeps them on purpose, to show how BAND-01 meets real small moons; F-146 is the same effect. Expect findings, and expect the fix to be BAND-01's, not Sol's.
  - **Candidate:** Sol's day 0 is J2000.0 and the planets start at their real mean longitudes, but the calendar still starts on its own epoch (14 Mar 2341). Give a system its own start date if comparing with the real sky matters.
  - **Open:** Sol now has ships, and `lost_beyond_au` (6 AU) puts Saturn and Neptune beyond the point where a coasting ship is lost (BAND-08), so Vestige can't be sent there. It needs to be per system, or much larger.
  - **Specced:** Sol's content, its test ships and stations, and the start-up choice of system, in SOL-01 to SOL-17 and SOL-22 to SOL-23 (`docs/spec/sol.md`).
- **Small bodies:** inner rocky or metallic belts, outer icy belts, Trojans, comets. Drawn as density shading, with notable sites as individual markers (*Presentation & Visual Style*, 2).
- **Orbits:** Keplerian, on rails. How much more detail is modelled is open (*Drives & Transit*, 9).

### 5.3 Worlds that support human life (direction)

Starter worlds support human life in habitats on hostile surfaces (*Economy*), not in the open air.

- **Candidate criteria:** gravity roughly 0.3 to 1.5 g; surface temperature and pressure within what habitats can manage; accessible water; a star that isn't too active.
- Halden IV in the reference sketches (cold terrestrial, 0.82 g, thin nitrogen, −41 °C) is the intended flavour.
- Close-in worlds of M dwarfs are often tidally locked and exposed to flares, which makes them harder but not impossible homes.

### 5.4 Resources (candidate)

Each body carries rough deposits of the economy's few resources (*Economy*):

- **Water (ice):** beyond the snow line, icy moons, comets, polar caps of inner worlds.
- **Metals:** rocky and metallic asteroids, rocky planets and moons.
- **Rare metals:** scarce and localised; concentrated in metallic asteroids and more common in high-metallicity systems.
- **Helium-3:** gas giants; present but minor.

## 6. Non-Visitable Systems (direction)

Modelled at catalogue level only:

- Position, star type(s), age, metallicity, and a catalogue designation rather than a name (the old network named only the systems it reached).
- **Known planets (candidate):** a short list as astronomers would know it, biased towards large planets and close orbits, as real detection methods are. The player never learns more.
- **Uses:** landmarks and depth in the cluster view; lore and recovered records ("a survey of this system was planned"); candidates for dark-gate destinations later.
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

- Slab proportions, and whether the plan shape is round, irregular or elongated.
- The ratio of gated to ungated systems; maximum link range; links per system; the share of links that fail, and how they're chosen (*Gates*, 11).
- Gate distance from the star: fixed, scaled by stellar mass, or beyond the outermost major planet (*Drives & Transit*, 9).
- Which architecture templates are needed, and how many bodies per system.
- How much orbital detail is modelled (eccentricity, inclination, resonances).
- Whether native life exists anywhere, and in what form.
- How habitable worlds are guaranteed in sufficient numbers without unrealistic odds.
- Whether free-floating planets and white dwarfs get special treatment.
- How the historical layer is simulated: rules or a short simulated history.
- Generation speed and how the simulation copes with a few hundred stars.

# Orphan Worlds: World Generator

*Status:* Draft. Companion to the design document (2, 4.2, 4.5, 5). Status tags as in the design document: **Decided**, **Direction**, **Candidate**. All numbers are placeholders for tuning; the astronomical figures are approximate and should be checked against current surveys before they are fixed.

The generator builds the cluster for a new game: stars, the systems the player can visit, the gate network between them, and the historical layer left by the old network (settlements, yards, ruins, salvage). It produces the truth; what each faction knows of it is handled separately (*Presentation & Visual Style*, 2: fidelity follows knowledge).

---

## 1. Principles

- **3D but flat (decided):** the cluster is a three-dimensional volume with a narrow extent in z. This is a concession for playability: the map reads well from a tilted top-down view, links rarely hide behind each other, and the player rarely has to rotate the camera. In reality the galactic disc is hundreds of parsecs thick, so a local cluster would be roughly spherical; the flattening is deliberate, not a claim about the galaxy.
- **Informed by current science (decided):** star types, multiplicity, planet occurrence, and system architectures follow observed distributions, not genre conventions. Where the game needs something rarer than nature provides (for example enough habitable worlds), the reason is the builders' choice of where to settle, not inflated odds.
- **Gates where there was a reason (decided):** not every system has a gate. The old network built them where something justified the cost: worlds to settle, resources, or a link needed to join the network together.
- **Non-visitable systems are lightly modelled (decided):** systems without a gate are never visited, so they need only enough detail to appear on the map and in the astronomical record.
- **Seeded and reproducible (candidate):** one seed reproduces a map exactly. Each stage draws from its own derived seed, so tuning one stage does not reshuffle the others.

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
- **Map size (candidate):** the design document's 10 to 100 systems counts gated, visitable systems. If a quarter to a half of all systems are gated, a large map holds a few hundred stars in total.
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
- **Multiplicity (direction):** roughly a quarter of M dwarfs and about half of Sun-like stars have companions. Wide pairs behave as two nearby systems; close pairs host circumbinary planets; intermediate pairs truncate planet systems.
- **Age and metallicity (candidate):** each system gets an age (0 to about 10 billion years) and a metallicity spread around solar. Higher metallicity raises the chance of giant planets and rich rocky material, so it feeds directly into resource value. Age governs M-dwarf activity and which stars have become white dwarfs.
- **Free-floating planets (candidate):** surveys suggest planets without a star may be about as common as stars. They are mostly invisible, but a few could appear as charted objects, and a gate at one would make an intriguing node. The game's title makes this hard to resist.

## 4. Gates & the Network

### 4.1 Why a system was gated (direction)

The old network's builders knew each system well and built gates where the value justified the cost. The generator scores systems by the motives below and gates the most valuable, plus whatever relays the network needs. The motives line up with the faction origins (*Faction History Options*, 3), which helps place starting factions.

| Motive | What makes a system attractive | Related origin |
|---|---|---|
| Settlement | Rocky worlds in or near the habitable zone with tolerable gravity and accessible water | Agricultural colony |
| Extraction | Rich belts, icy moons, metal-rich bodies, high metallicity | Frontier mining colony |
| Industry | Water and metals close together, a calm star, good transfer geometry | Shipyard hub |
| Junction | Central position with many neighbours in range; administration and traffic | Administrative centre |
| Science | Unusual objects: white dwarfs, flare stars, close binaries, young systems | Research outpost |
| Strategic | Chokepoints and approaches to valuable systems | Naval base |
| Relay | Little intrinsic value, but needed to join systems beyond link range | None; gate-maintenance outposts at most |

- **Relays (candidate):** a dim red dwarf that exists on the map only because the network had to pass through it gives a natural chokepoint with nothing to live on, which suits raiders and defence.
- **Ungated systems:** everything else. Most are dim M dwarfs with nothing the builders needed, but a few rich systems were never gated (too remote, or planned and never finished), which can feed lore and regret.

### 4.2 Links (candidate)

- Each gated system links to a small handful (two to five) of gated systems within a maximum link range; the network is connected.
- Links prefer near neighbours, so a few relays and chokepoints arise naturally rather than by decree.
- Mild preference for links that do not cross in the top-down projection, for readability.
- Some links lead out of the cluster and are dark (design document, 4.2; parking lot).
- Gates sit at the system edge, one per destination (design document, 4.2). The gate's distance from the star, and whether it scales with the star's mass or the extent of its planets, is open (section 9).

## 5. Gated Systems: Full Detail

### 5.1 Approach (direction)

Generate from observed occurrence rates and architecture templates rather than simulating planet formation: cheap, plausible, and tunable.

- **Architecture templates by star type (candidate):** compact multi-planet M-dwarf systems (TRAPPIST-1-like); Solar-system-like with an inner rocky zone and outer giants; a hot or eccentric giant that has cleared the inner system; debris-disc systems; white-dwarf remnants with debris and surviving outer planets.
- **Occurrence (direction, approximate):** small planets are nearly ubiquitous; sub-Neptunes (between Earth and Neptune in size, absent from our own system) are the most common type; M dwarfs typically host two or three small planets in close orbits; cold giants occur around perhaps 10 to 20% of Sun-like stars and only a few percent of M dwarfs; hot Jupiters around about 1%.
- **Stability (candidate):** planets spaced by a minimum number of mutual Hill radii; moons within a fraction of their planet's Hill sphere.
- **Key boundaries (direction):** habitable zone at about 0.95 to 1.7 × √(L/L☉) AU; snow line at about 2.7 × √(L/L☉) AU. Water ice is abundant beyond the snow line, so its distance shapes the trade-off between water and solar power.

### 5.2 Bodies (candidate)

- **Planets:** rocky (airless, thin atmosphere, dense atmosphere, icy, molten), super-Earths, sub-Neptunes, ice giants, gas giants.
- **Moons:** giants get moon systems, often with large icy moons, the main water sources; rocky planets get few or none.
- **Small bodies:** inner rocky or metallic belts, outer icy belts, Trojans, comets; drawn as density shading with promoted sites (*Presentation & Visual Style*, 2).
- **Orbits:** Keplerian, on rails; how much more detail is modelled is open (design document, 9).

### 5.3 Worlds that support human life (direction)

Starter worlds support human life in habitats on hostile surfaces (design document, 4.1), not in the open air. Candidate criteria: gravity roughly 0.3 to 1.5 g; surface temperature and pressure within what habitats can manage; accessible water; a star that is not too active. Halden IV in the reference sketches (cold terrestrial, 0.82 g, thin nitrogen, −41 °C) is the intended flavour. Close-in worlds of M dwarfs are often tidally locked and flare-exposed, which makes them harder but not impossible homes.

### 5.4 Resources (candidate)

Each body carries rough deposits of the economy's few resources (design document, 4.1):

- **Water (ice):** beyond the snow line, icy moons, comets, polar caps of inner worlds.
- **Metals:** rocky and metallic asteroids, rocky planets and moons.
- **Rare metals:** scarce and localised; concentrated in metallic asteroids and more common in high-metallicity systems.
- **Helium-3:** gas giants; present but minor.

## 6. Non-Visitable Systems (direction)

Modelled at catalogue level only:

- Position, star type(s), age, metallicity, and a catalogue designation rather than a name (the old network named the systems it reached).
- **Known planets (candidate):** a short list as astronomers would know it: biased towards large planets and close orbits, as real detection methods are. The player never learns more.
- **Uses:** landmarks and depth in the cluster view; lore and recovered records ("a survey of this system was planned"); candidates for dark-gate destinations later.
- **Never** visited, so full detail is never generated. If a later feature makes one visitable, the coarse summary is the seed for generating its detail on demand.

## 7. Historical Layer (direction)

Laid down after the physical map, in the same order as the setting's timeline (design document, 5):

1. **Old network:** settlements, yards, stations, and gate-maintenance outposts placed by each system's gate motive.
2. **Collapse:** destruction, abandonment, and decay, varying by system; some gates degraded.
3. **Turmoil and recovery:** surviving populations consolidate into the starting factions on their worlds.
4. **Result at game start:** inhabited worlds, dormant yards, salvage sites with partly known contents, ruins, and records, with richer sites further out or behind dark gates (design document, 4.5).
5. **Authored beats:** placement rules for Earth-mystery clues and external-threat hints, varying by map while the truth stays fixed.

## 8. Factions & Fairness (candidate)

- Starting factions are placed on systems whose gate motive fits their origin (a shipyard hub where the yards are), resolving the open question in *Faction History Options*.
- Fairness checks: comparable starter worlds, resources within reach, distance to rivals, and access to gates and salvage. Fairness is in the totals; starts can differ in kind.
- AI factions draw histories weighted by the map position they receive.

## 9. Open Questions

- Slab proportions and whether the plan shape is round, irregular, or elongated.
- Ratio of gated to ungated systems; maximum link range; links per system.
- Gate distance from the star: fixed, scaled by stellar mass, or beyond the outermost major planet (design document, 9).
- Which architecture templates are needed, and how many bodies per system.
- How much orbital detail is modelled (eccentricity, inclination, resonances).
- Whether native life exists anywhere, and in what form.
- How habitable worlds are guaranteed in sufficient numbers without unrealistic odds.
- Whether free-floating planets and white dwarfs get special treatment.
- How the historical layer is simulated: rules or a short simulated history.
- Generation speed and how the simulation copes with a few hundred stars.

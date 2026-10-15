# Spec: Sol test system

**In short:** a second test system made from the real solar system, so scale, spacing and speed can be judged against figures people know. It holds the Sun, Mercury to Mars, Ceres, Jupiter, Saturn, Uranus and Neptune with their main moons, and the main asteroid belt, with no stations, ships, gates or salvage sites. Orbits stay circular, coplanar and prograde, as the simulation requires, so each body sits at its mean distance. The game starts on Corvane; `--system sol` starts it on Sol.

*Scope:* Sol's content and the choice of system at start-up. *Source ideas:* `docs/ideas/mechanics/world-generator.md` (section 5.2). **Built** is ✓ once a requirement is in the code.

*Sources of the figures:* NASA's planetary and satellite fact sheets (masses, radii, periods, moon orbits); JPL's approximate positions of the major planets (planet orbit radii and mean longitudes at J2000.0); JPL's planetary satellite tables (radii and masses of the small moons, and the orbits of Mars's moons); the Dawn results for Ceres.

*Retired IDs* (never reuse): SOL-20, SOL-21.

---

## 1. What Sol contains

| ID | Requirement | Built |
|---|---|---|
| SOL-01 | Sol is a system in `content/`, built from the same blueprint types as Corvane (SCL-03). No simulation or view code names it. *Checked in review.* | ✓ |
| SOL-02 | The star is named "Sol": class G2V, mass `SOLAR_MASS_KG`, mean radius 695,700 km, surveyed. | ✓ |
| SOL-03 | The planets are the nine bodies of table 1, each round the star with the orbit radius, mean radius and mass the table gives and the key fact "Class" set to the table's class. Ceres is a planet-class record. | ✓ |
| SOL-04 | The moons are the twenty-one bodies of table 2, each a child of its parent with the orbit radius, mean radius and mass the table gives and the key fact "Class: Moon". Triton also has the key fact "Real orbit: retrograde, drawn prograde", and each moon of Uranus "Real orbit: round Uranus's tilted equator, drawn in the system plane". | ✓ |
| SOL-05 | Each body's period, from Kepler's third law with its parent's mass (SCL-04), is within 1% of the real period in the tables. | ✓ |
| SOL-06 | At day 0 each planet is at its phase in table 1: its mean longitude at J2000.0, which Sol takes as day 0 (the calendar keeps its own epoch, CLK-02). Ceres starts at 0°. The *i*th moon of a parent, counting outwards from 0, starts at 1.3 × *i* rad. | ✓ |
| SOL-07 | Sol has one belt, the main asteroid belt, from 2.2 to 3.2 AU, with no salvage sites. | ✓ |
| SOL-08 | Sol has no stations, ships, gates or salvage sites. Every body in it is neutral and surveyed. | ✓ |
| SOL-09 | With the default tunables, every star, planet and moon in Sol has a Low band, and a High band except Phobos, Deimos, Amalthea, Mimas, Enceladus and Proteus: half their Hill radius does not exceed their Low radius (BAND-01). Tethys keeps its High band by about 19 km (High 1,050 km, Low 1,031 km). | ✓ |
| SOL-10 | The system fit (ZOOM-15) is Neptune's orbit, 30.07 AU. | ✓ |
| SOL-11 | Sol spawns in a headless app (ARCH-13) as one star, nine planets (Ceres included), twenty-one moons and one belt, with no gates, and every named orbit resolves. | ✓ |

## 2. Start-up

| ID | Requirement | Built |
|---|---|---|
| SOL-22 | The system to load is chosen at start-up by the argument `--system <name>`: `corvane` (the default, with no argument) or `sol`. An unknown name stops the program with a message listing the known names. The choice is a plain function of the arguments (ARCH-20). | ✓ |
| SOL-23 | The arguments are read in `content/`; `main.rs` only adds the plugin (ARCH-01). *Checked in review.* | ✓ |

## 3. Figures

Orbit radii are mean distances, and every orbit is drawn circular and in one plane. Mean radii follow SCL-05; where a NASA sheet gives three axes the figure is their mean. Real periods are sidereal, in days (SOL-05). Table 1 names each planet's class, which is its "Class" key fact (SOL-03). Earth takes the Earth–Moon barycentre's orbit and longitude.

**Table 1: planets**

| Body | Orbit radius (AU) | Mean radius (km) | Mass (kg) | Real period (d) | Phase at day 0 (°) | Class |
|---|---|---|---|---|---|---|
| Mercury | 0.387099 | 2,439.7 | 3.301 × 10²³ | 87.969 | 252.250 | Terrestrial planet |
| Venus | 0.723336 | 6,051.8 | 4.867 × 10²⁴ | 224.701 | 181.979 | Terrestrial planet |
| Earth | 1.00000 | 6,371.0 | 5.972 × 10²⁴ | 365.256 | 100.465 | Terrestrial planet |
| Mars | 1.52371 | 3,389.5 | 6.417 × 10²³ | 686.980 | 355.447 | Terrestrial planet |
| Ceres | 2.768 | 469.7 | 9.384 × 10²⁰ | 1,680 | 0 | Dwarf planet |
| Jupiter | 5.20289 | 69,911 | 1.898 × 10²⁷ | 4,332.589 | 34.396 | Gas giant |
| Saturn | 9.53668 | 58,232 | 5.683 × 10²⁶ | 10,755.699 | 49.954 | Gas giant |
| Uranus | 19.1892 | 25,362 | 8.681 × 10²⁵ | 30,685.400 | 313.238 | Ice giant |
| Neptune | 30.0699 | 24,622 | 1.024 × 10²⁶ | 60,189.018 | 304.880 | Ice giant |

**Table 2: moons**

| Moon | Parent | Orbit radius (km) | Mean radius (km) | Mass (kg) | Real period (d) |
|---|---|---|---|---|---|
| Moon | Earth | 384,400 | 1,737.4 | 7.346 × 10²² | 27.3217 |
| Phobos | Mars | 9,375 | 11.08 | 1.062 × 10¹⁶ | 0.3187 |
| Deimos | Mars | 23,457 | 6.2 | 1.441 × 10¹⁵ | 1.2625 |
| Amalthea | Jupiter | 181,400 | 83.5 | 2.466 × 10¹⁸ | 0.498179 |
| Io | Jupiter | 421,800 | 1,821.5 | 8.932 × 10²² | 1.769138 |
| Europa | Jupiter | 671,100 | 1,560.8 | 4.800 × 10²² | 3.551181 |
| Ganymede | Jupiter | 1,070,400 | 2,631.2 | 1.482 × 10²³ | 7.154553 |
| Callisto | Jupiter | 1,882,700 | 2,410.3 | 1.076 × 10²³ | 16.689017 |
| Mimas | Saturn | 185,520 | 198.2 | 3.751 × 10¹⁹ | 0.942422 |
| Enceladus | Saturn | 238,020 | 252.0 | 1.08 × 10²⁰ | 1.370218 |
| Tethys | Saturn | 294,660 | 531.1 | 6.175 × 10²⁰ | 1.887802 |
| Dione | Saturn | 377,400 | 561.4 | 1.096 × 10²¹ | 2.736915 |
| Rhea | Saturn | 527,040 | 763.3 | 2.31 × 10²¹ | 4.517500 |
| Titan | Saturn | 1,221,870 | 2,575 | 1.346 × 10²³ | 15.945421 |
| Iapetus | Saturn | 3,560,850 | 734.5 | 1.81 × 10²¹ | 79.330183 |
| Ariel | Uranus | 190,900 | 578.9 | 1.29 × 10²¹ | 2.520379 |
| Umbriel | Uranus | 266,000 | 584.7 | 1.22 × 10²¹ | 4.144176 |
| Titania | Uranus | 436,300 | 788.9 | 3.42 × 10²¹ | 8.705867 |
| Oberon | Uranus | 583,500 | 761.4 | 2.88 × 10²¹ | 13.463234 |
| Proteus | Neptune | 117,647 | 208.0 | 5.0 × 10¹⁹ | 1.122315 |
| Triton | Neptune | 354,760 | 1,353.4 | 2.14 × 10²² | 5.876854 |

The Moon's period comes out about 0.5% long, because SCL-04 uses the parent's mass alone; every other body is within 0.25%.

## Tunables

None.

## Not in Sol

Left out on purpose; see `docs/ideas/mechanics/world-generator.md` (5.2) for why. Not to be built until specced: Pluto and Charon, Miranda and the other small regular moons, distant captured moons, rings, Trojans and comets, eccentric, tilted or retrograde orbits, and any station, ship, gate or salvage site.

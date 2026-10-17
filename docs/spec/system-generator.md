# Spec: System generator

**In short:** the system generator builds the bodies of one gated system on demand, from the sector's record of it (`sector-generator.md`). It returns the same kind of records as the hand-made Corvane and Sol, so nothing in the view changes to draw it. The star and the summary's planets and belts come through unchanged; detail adds names, starting positions, moons for the planets, a few large bodies in the belts, key facts and descriptions, and the gates. The gates sit on a ring round the star: a gate for each working or dormant link and a wreck for each collapsed one, spread round the ring. Everything is a pure function of the sector seed and the system's id, so a system looks the same every time it is entered, in any order.

*Scope:* the bodies of a gated system and its gates. Not here: ungated systems (they have no detail), ships, stations, ownership, knowledge, salvage contents, resource deposits, rings, Trojans, comets, and eccentric or tilted orbits. *Source ideas:* `docs/ideas/mechanics/world-generator.md` (sections 5.2 and 5.3), `docs/ideas/mechanics/gates.md` (sections 3, 6 and 7), `docs/ideas/mechanics/drives-and-transit.md` (section 6, gate placement).

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables, fields of `GeneratorTunables` (SGEN-05; table at the end), never hard-coded. Every built requirement has an automated test (ARCH-36).

*Retired IDs* (never reuse): none.

---

## 1. Contract

| ID | Requirement | Built |
|---|---|---|
| SYSG-01 | `generate_system` takes a sector, a system id and the tunables, and returns the system's records, built from the blueprint types of `content/` that Corvane and Sol use (SCL-03). An id that is not a gated system's returns an error naming the id, not a panic: ungated systems have no detail. | ✓ |
| SYSG-02 | The result is a pure function of the sector (its seed and records), the id and the tunables. Its seed is derived from the sector seed, the word "system" and the id (SGEN-03). Generating any systems in any order, or twice, gives equal results. | ✓ |
| SYSG-03 | Detail refines the summary and never re-rolls it. The star, every planet's orbit radius, mass, kind, radius and facts, and every belt's radii, mass and composition are those of the summary, unchanged. Detail adds only: starting phases, moons, large belt bodies, names, key facts, descriptions and the gates and wrecks of the ring. | ✓ |
| SYSG-04 | Every natural body is neutral and surveyed (SOL-08), until knowledge is modelled. The system has no ships, stations or salvage sites other than the gate wrecks (SYSG-53), and no ownership. | ✓ |
| SYSG-05 | A generated system spawns in a headless app (ARCH-13) with the body, belt and gate counts its records give, and every named orbit resolves. | ✓ |

## 2. The star

| ID | Requirement | Built |
|---|---|---|
| SYSG-10 | The star is named as the system (SGEN-82), has the mass in kg, the mean radius and the class of the summary, and the authored key facts "Class" (the spectral type, SGEN-22), "Age" (in Gyr to one decimal), "Luminosity" (in solar units to 3 significant figures) and "Metallicity" ("[Fe/H]" and the value to 2 decimals). | ✓ |

## 3. Planets

| ID | Requirement | Built |
|---|---|---|
| SYSG-20 | Each planet of the summary is a record round the star at its summary orbit radius, with its summary mass and radius and the name the sector gave it (SGEN-82). It has its own orbit; natural bodies never share one (SCL-07). | ✓ |
| SYSG-21 | Each natural body's starting phase is drawn uniformly from its own derived seed (SYSG-02, from the system's seed and the body's place in its list), so adding or removing one body doesn't move the others. | ✓ |
| SYSG-22 | Each planet, moon and belt body carries **planet facts**: its kind, surface gravity (SGEN-38), equilibrium temperature (SGEN-39), atmosphere (SGEN-40) and water (SGEN-41), as data a rule or a view can read. A planet's kind is the one of SGEN-32, a moon's is "moon" and a belt body's is "rocky". | ✓ |
| SYSG-23 | Each natural body has a one-line description built from its facts: kind, atmosphere and water in plain words. The wording is a placeholder. | ✓ |

## 4. Moons

A planet's moons follow the realistic spacing of the idea doc (`world-generator.md`, 5.2): regular moons start well clear of the planet, are spaced out geometrically, and stay inside part of the planet's Hill sphere.

| ID | Requirement | Built |
|---|---|---|
| SYSG-30 | A planet gets a number of moons drawn from the range for its kind (`moon_counts`): gas giant 3 to 12, ice giant 1 to 6, sub-Neptune 0 to 2, super-Earth 0 to 1, rocky 0, 1 or 2 with probabilities 0.80, 0.15 and 0.05. No planet has more than `moon_max_count` moons. | ✓ |
| SYSG-31 | A planet's moons lie at orbit radii, measured in planet radii, that start between `moon_first_orbit_radii` and grow by a factor between `moon_orbit_ratio` from one moon to the next. A moon is dropped, and so are all those beyond it, if its orbit lies beyond the outer limit, the smaller of `moon_outer_limit_radii` planet radii and `moon_hill_fraction` of the planet's Hill radius. No moon orbits closer than `moon_inner_limit_radii` planet radii. | ✓ |
| SYSG-32 | A moon's mass is its planet's mass times a ratio drawn log-uniformly from `moon_mass_ratio_giant` for a giant and from `moon_mass_ratio_other` for any other planet. Its radius follows from the density `moon_density` (icy if the planet orbits at or beyond the snow line, otherwise rocky): R = (3 m ÷ 4 π ρ)^(1/3). | ✓ |
| SYSG-33 | A moon has planet facts (SYSG-22) by the same rules as a rocky planet (SGEN-38 to SGEN-41), using its own mass and radius and its planet's distance from the star for its equilibrium temperature. | ✓ |
| SYSG-34 | A moon is named for its planet, a space and a lowercase letter by distance from the planet, the nearest "a": for example "Morrow a", "Morrow b". | ✓ |
| SYSG-35 | A moon's period follows from Kepler's third law with its planet's mass (SCL-04). | ✓ |

## 5. Belts and belt bodies

| ID | Requirement | Built |
|---|---|---|
| SYSG-40 | The belts of the summary become belt records in order from the inside out, each with the name "*System* belt" if the system has one belt, "*System* inner belt" and "*System* outer belt" if two, and "inner", "middle" and "outer" if three. | ✓ |
| SYSG-41 | Each belt has a number of **belt bodies** drawn with probabilities `belt_body_count_weights` up to `belt_body_max`: ordinary planet-class bodies with orbits inside the belt's annulus, named from the sector's spare names for that belt (SGEN-82). A belt body's mass is log-uniform in `belt_body_mass_me`, and the bodies' masses sum to at most `belt_body_mass_cap` × the belt's mass: the smallest are dropped until they do. Its radius follows from the density `belt_body_density` (icy if the belt is mostly icy, otherwise rocky). Orbit radii are at least 2% apart and `ring_planet_gap` × the ring's radius from the ring's. A belt body has planet facts (SYSG-22) like a rocky planet. | ✓ |

## 6. The gate ring

Gates sit at the system's edge, one per link end (`gates.md`, section 2). The ring they share was sized and cleared of planets by the sector generator (SGEN-58, SGEN-59), so the detail only places gates on it.

| ID | Requirement | Built |
|---|---|---|
| SYSG-50 | The ring's radius is the one the sector stores for the system (SGEN-58). The detail never moves it or any planet. | ✓ |
| SYSG-51 | The system has one record for each link end it belongs to: a **gate** if the link is working or dormant, and a **wreck** if it is collapsed. All are on the ring: round the star at the ring's radius, with a period from Kepler's third law (SCL-04). | ✓ |
| SYSG-52 | A gate is a record of class Gate, named "*Destination* gate", with the destination system's name (EDGE-01), a gate entry tolerance of `gate_entry_tolerance_km_s` (PLAN-30), and a **gate state**: working; or dormant, with whether this end is faulted. | ✓ |
| SYSG-53 | A wreck is a salvage site, named "*Destination* gate wreck", with the key fact "Wreck of the gate to *Destination*". Its contents are not generated. | ✓ |
| SYSG-54 | The records on the ring are spread in phase. With n of them, the i-th has the phase θ + i × 360° ÷ n + a jitter drawn uniformly within ± `gate_jitter_fraction` of that spacing, θ being drawn once for the ring. So no two are closer than (1 − 2 × `gate_jitter_fraction`) × 360° ÷ n. They are ordered by the link ids. | ✓ |
| SYSG-55 | The inset of a gate lists its state (INS-08). | ✓ |

## 7. Names

| ID | Requirement | Built |
|---|---|---|
| SYSG-60 | The generator makes no names of its own. Systems, planets and belt bodies take theirs from the sector (SGEN-82). Moons, belts, gates and wrecks are named as SYSG-34, SYSG-40, SYSG-52 and SYSG-53 say, from those names. Over a whole sector no two names of systems, planets and belt bodies are equal, ignoring case, and none is reserved (SGEN-81). | ✓ |

## 8. Checks

| ID | Requirement | Built |
|---|---|---|
| SYSG-70 | These are tested over every gated system of at least `invariant_seeds` sectors of each preset: SYSG-02, SYSG-03, SYSG-31, SYSG-32, SYSG-41, SYSG-54, SYSG-60. And in every system: the ring clears every giant's orbit by at least `ring_giant_clearance_hill` Hill radii and every planet's orbit by `ring_planet_gap` × its radius, unless the sector moved the planet (SGEN-59); and the number of gates and wrecks equals the number of the system's links. | ✓ |
| SYSG-71 | Corvane and Sol stay as they are, hand-made test content. The generator doesn't replace them and doesn't read them (SOL-01). | ✓ |

## Tunables

All fields of `GeneratorTunables` (SGEN-05), beside those in `sector-generator.md` (which also holds `belt_body_max`, SYSG-41). Defaults are starting values.

| Tunable | Default | Used by |
|---|---|---|
| `moon_counts` | as SYSG-30 | SYSG-30 |
| `moon_max_count` | 26 | SYSG-30, SYSG-34 |
| `moon_first_orbit_radii` | 2.5 to 8 planet radii | SYSG-31 |
| `moon_orbit_ratio` | 1.4 to 2.2 | SYSG-31 |
| `moon_outer_limit_radii` | 30 planet radii | SYSG-31 |
| `moon_hill_fraction` | 0.5 | SYSG-31 |
| `moon_inner_limit_radii` | 2.0 planet radii | SYSG-31 |
| `moon_mass_ratio_giant`, `moon_mass_ratio_other` | 1e-7 to 2e-4; 1e-6 to 1e-2 | SYSG-32 |
| `moon_density` | icy 1.8 g/cm³, rocky 3.3 g/cm³ | SYSG-32 |
| `belt_body_count_weights` | 0: 0.4, 1: 0.3, 2: 0.2, 3: 0.1 | SYSG-41 |
| `belt_body_mass_me` | 1e-5 to 3e-4 Earth masses | SYSG-41 |
| `belt_body_mass_cap` | 0.4 | SYSG-41 |
| `belt_body_density` | icy 1.5 g/cm³, rocky 2.5 g/cm³ | SYSG-41 |
| `gate_entry_tolerance_km_s` | 10 km/s | SYSG-52 |
| `gate_jitter_fraction` | 0.25 | SYSG-54 |

## Open points

- The look of a gate by state, in the system view, is not specced. The state shows in the inset (SYSG-55) and a wreck is a salvage-site marker (X).
- The facts rules for moons and belt bodies reuse those for rocky planets. A cold icy moon with a thin atmosphere (a Titan) is rare under the retention rule and tends to come out airless.
- Tidal locking, rotation periods, day length and surface features are not generated.
- A system with a moon-heavy giant can have many bodies near the planet, and the satellite fit (ZOOM-15) fits them all. If that is a problem for the view, lower `moon_outer_limit_radii` before changing a rule.
- Large belt bodies could be the sites of the settlements, yards and ruins of the historical layer. That layer places them later (`world-generator.md`, section 7).

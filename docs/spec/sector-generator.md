# Spec: Sector generator

**In short:** the generator builds a **sector**, the star map of one game, from two inputs: N, the number of systems that carry gates, and a seed. It lays out a flat 3D slab at about 0.1 star systems per cubic parsec, holding three star systems for every gated one. It gives every star its class, age and metallicity, and records companions. It gives every system a coarse summary of planets, belts and their facts. It then picks exactly N systems to carry gates, by what the old network's builders would have wanted (settlement, extraction, industry, science), plus junctions and relays to join them. It links them into a dense old network, up to five links each and about four on average, and fails most of the links, aiming at about 55% collapsed, 15% dormant and 30% working, under rules that keep every system and every pocket of the network useful. The same inputs always give the same sector. Full detail for a gated system is made on demand (`system-generator.md`). The sector is seen in `sector-view.md`.

*Scope:* stages 1 to 5 of the world generator (volume, stars, summaries, gate choice, links), the failure of links, and the names of gated systems and their planets. Not here: bodies in detail (`system-generator.md`), the historical layer, factions and fairness (stages 7 and 8), resource deposits (the economy spec, when it exists). *Source ideas:* `docs/ideas/mechanics/world-generator.md` (sections 1 to 5), `docs/ideas/mechanics/gates.md` (sections 6, 7 and 9).

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables (table at the end), fields of the generator's tunables struct (SGEN-05), never hard-coded. Every built requirement has an automated test (ARCH-36); the invariants over many seeds are SGEN-91.

*Retired IDs* (never reuse): none.

---

## The algorithm at a glance

| Stage | What it does | Requirements |
|---|---|---|
| Settings | N and a seed in; one derived seed per stage | SGEN-01 to SGEN-06 |
| 1. Volume | Sizes a flat slab from N; places a site for each system and brown dwarf | SGEN-10 to SGEN-12 |
| 2. Stars | Gives each entry its class, mass, age and metallicity; records companions; splits wide pairs into two entries; marks which entries can carry a gate | SGEN-20 to SGEN-29 |
| 3. Summaries | Gives each system a template, a coarse planet list, 0 to 3 belts and per-planet facts (gravity, temperature, atmosphere, water, tidal locking, geological activity) | SGEN-30 to SGEN-45 |
| 4. Gates | Scores each eligible system for four reasons; picks a core by score with a quota per reason; adds junctions and relays to join it; adjusts until exactly N; tags each system with a main reason; names it; sizes its gate ring | SGEN-50 to SGEN-59 |
| 5. Links | Joins the gated systems with 2 to 5 links each, avoiding crossings; labels strategic systems; then fails links by traffic and chance and enforces the rules that keep the network useful | SGEN-60 to SGEN-76 |
| Names | Gives gated systems, their planets and spare belt bodies unique names; gives the rest catalogue designations | SGEN-80 to SGEN-82 |

The sector is the output (SGEN-90). Stages run in this order, each from its own derived seed (SGEN-03), so tuning one stage doesn't reshuffle the earlier ones.

## 1. Settings and determinism

| ID | Requirement | Built |
|---|---|---|
| SGEN-01 | The generator takes **settings**: N, the number of gated systems, and a seed (a `u64`). The presets are Small `preset_small`, Medium `preset_medium` and Large `preset_large`. N below `min_gated_count` or above `max_gated_count` is refused with an error that names the range, not a panic. Values below the presets are a development and test setting only. | ✓ |
| SGEN-02 | The same settings and tunables give the same sector, field for field, on every run and machine. No stage reads the clock, the environment or an unseeded random source. | ✓ |
| SGEN-03 | Each stage draws from its own seed, derived from the sector seed and the stage's name. A stage's output depends only on its own seed, its own tunables and the outputs of earlier stages. Changing a tunable of one stage leaves the output of every earlier stage identical. | ✓ |
| SGEN-04 | The seed is random when the player gives none, and is part of the sector (SGEN-90) so the view can show it (SECT-08). Seeds read and print as decimal `u64`s. | ✓ |
| SGEN-05 | Every number the spec marks as tunable is a field of one `GeneratorTunables` struct (`src/content/generator/tunables.rs`), documented field by field and passed to the generator. No stage hard-codes a tunable. *Checked in review.* | ✓ |
| SGEN-06 | The generator is plain Rust in `src/content/generator/`, one module per stage, with no Bevy rendering, UI or ECS types, and it returns plain data (SGEN-90). Its random numbers come from a small seeded generator written in the repo. No crate is added. *Checked in review.* | ✓ |

## 2. Stage 1: volume

The sector is a slab: a round-ish disc in plan, thin in z. Thickness is a fixed share of width. A slab this thin reads well from a tilted top-down view (`docs/ideas/mechanics/world-generator.md`, section 1).

| ID | Requirement | Built |
|---|---|---|
| SGEN-10 | The sector holds T = N × `stars_per_gated` star systems (3N at the default, so about one in three carries a gate). Of these, P = round(T × `wide_pair_fraction` ÷ 2) are wide pairs (SGEN-27), so stage 1 places S = T − P sites for systems. It also places B = round(T × `brown_dwarf_per_system`) sites for brown dwarfs (SGEN-28). | ✓ |
| SGEN-11 | The slab holds T systems at `system_density_per_pc3`, so its volume is V = T ÷ density. Its thickness is `slab_thickness_ratio` × W, where W is the diameter of the circle whose area equals the plan area: V = π (W ÷ 2)² × thickness. With the defaults, Small is about 17 pc wide and 4 pc thick, Medium 23 and 6, Large 31 and 8. The mean spacing of gated systems, (V ÷ N)^(1/3), is then about 3.1 pc (10 ly) at every N. | ✓ |
| SGEN-12 | The plan is an ellipse of that area, with semi-axes (W ÷ 2) × √E and (W ÷ 2) ÷ √E, E drawn per sector between 1 and `plan_elongation_max`. Each site is placed uniformly in the slab, at least `min_system_separation_pc` from every earlier site. A site that cannot be placed in 50 tries is placed at the best of them. Positions are stored in km from the slab's centre, as `f64` (SCL-01). | ✓ |

## 3. Stage 2: stars

An **entry** is one place in the catalogue: a star system (one or more bound stars), a brown dwarf, or a giant. Only entries with exactly one star can carry a gate (SGEN-29), but the catalogue records every companion.

| ID | Requirement | Built |
|---|---|---|
| SGEN-20 | Entries get ids 0 to T + B − 1 in a fixed order and a **designation** `<designation_prefix>-<id as four digits>` (for example "OW-0173"); a companion star adds a letter (for example "OW-0173 B"). Every entry has its position (SGEN-12), its stars, its age (SGEN-23) and its metallicity (SGEN-24). | ✓ |
| SGEN-21 | The primary star of each system is drawn from the class mix `star_mix`: M, K, G, F, A, white dwarf and giant. Over many sectors the shares of the classes match the mix within 0.02. | ✓ |
| SGEN-22 | A star's mass is drawn log-uniformly from its class's mass range (`star_mass_ranges`). Its luminosity, radius and effective temperature follow from the mass by `star_mass_luminosity` and `star_mass_radius`; a white dwarf's radius is `white_dwarf_radius_rsun` and its luminosity is drawn log-uniformly from `white_dwarf_luminosity_lsun`; a giant's from `giant_properties`. Temperature is 5,772 K × (L ÷ R²)^(1/4), with L and R in solar units. A star's **spectral type** is its class letter, a digit 0 to 9 from the hot end of the class's mass range (floor of 10 × (M_max − M) ÷ (M_max − M_min)), and "V" ("D" for a white dwarf, "III" for a giant). | ✓ |
| SGEN-23 | Age is drawn uniformly between `age_min_gyr` and `age_max_gyr`, and capped at the star's main-sequence lifetime, 10 Gyr × M^−2.5 with M in solar masses. A white dwarf's age is drawn between `white_dwarf_age_min_gyr` and `age_max_gyr`. A giant's age is its progenitor's lifetime times a factor drawn from 1.0 to 1.1. | ✓ |
| SGEN-24 | Metallicity, [Fe/H], is drawn from a normal distribution with mean `metallicity_mean` and spread `metallicity_sigma`, clamped to ± `metallicity_clamp`. | ✓ |
| SGEN-25 | A system is multiple with probability `multiplicity_by_class` for its primary's class. A multiple has 2 stars with probability 0.75, 3 with 0.18 and 4 with 0.07 (`star_count_weights`). A companion's class is drawn from the mix of M, K, G, F and A, limited to classes no more massive than the primary's (a white dwarf's companions are M); it shares the primary's age and metallicity. | ✓ |
| SGEN-26 | A companion's separation from the primary, in AU, is log-normal: log10 of it has mean and spread `separation_log10_au` (one pair of values for M dwarfs and one for other classes), limited to between `companion_min_au` and `wide_pair_min_au`. Companions are recorded with their separations and affect nothing else but SGEN-29 and SGEN-30. | ✓ |
| SGEN-27 | P of the sites chosen at random each hold a **wide pair**: a second system is placed at a separation drawn log-uniformly between `wide_pair_min_au` and `wide_pair_max_au`, in a random direction, and drawn as any other system. The two are separate entries, each recording the other as its wide partner. | ✓ |
| SGEN-28 | Brown dwarfs and giants are **catalogue-only**: they get a designation, position, age, metallicity and mass (a brown dwarf's from `brown_dwarf_mass_msun`), and no planets, belts, companions or gate. A giant is a system whose primary is of class giant. There are no free-floating planets: nothing stars-less is in the catalogue. | ✓ |
| SGEN-29 | An entry is **eligible** to carry a gate if it is a system with exactly one star and no recorded companion of any kind, and its class is M, K, G, F, A or white dwarf. The two members of a wide pair are eligible separately. | ✓ |

## 4. Stage 3: summaries

A **summary** is a coarse description of a system, cheap enough to make for every one: a template, a planet list, belts, and per-planet facts. It is the skeleton of the system. Detail for a gated system refines it and never re-rolls it (SYSG-03). All orbits are circular, coplanar and prograde (SCL-04). Composition is a class only; amounts of resources in deposits are not generated (SGEN-43).

| ID | Requirement | Built |
|---|---|---|
| SGEN-30 | Every system (not a brown dwarf or a giant) gets a summary. The planets of a system with companions orbit the primary and lie inside `companion_truncation_fraction` × the separation of its nearest companion; a planet drawn farther out is dropped. Planets round a pair, and planets of a companion, are not generated. | ✓ |
| SGEN-31 | A system's **template** is drawn from `template_weights` for its primary's class: compact multi-planet, solar-like, hot giant, debris disc, or, for white dwarfs only, white-dwarf remnant. | ✓ |
| SGEN-32 | A planet has a **kind** and a mass in Earth masses, drawn log-uniformly from the kind's range (`planet_kind_masses`): rocky, super-Earth, sub-Neptune, ice giant or gas giant. Ice and gas giants are the **giants**. | ✓ |
| SGEN-33 | The template sets how many planets, of which kinds, and where, by `template_rules`. Compact multi-planet: 3 to 7 rocky, super-Earth and sub-Neptune planets, the innermost with a period of 0.8 to 6 days and each next period 1.3 to 2.0 times the one before, none beyond half the snow line. Solar-like: 2 to 5 rocky and super-Earth planets between a quarter and 1.2 times the snow line. Hot giant: one gas giant with a period of 1 to 5 days and 0 to 2 rocky or super-Earth planets beyond half the snow line, and no cold giants. Debris disc: 0 to 3 rocky, super-Earth and sub-Neptune planets inside twice the snow line. White-dwarf remnant: 0 to 2 giants and super-Earths between 2 and 30 AU. Orbits within a template are drawn log-uniformly in their range, or by the period rule. A planet whose orbit would lie outside the template's range is dropped. | ✓ |
| SGEN-34 | Neighbouring planets are at least `min_mutual_hill` mutual Hill radii apart, a mutual Hill radius being ((m₁ + m₂) ÷ (3 M★))^(1/3) × (a₁ + a₂) ÷ 2. A planet whose orbit breaks this is redrawn up to 20 times, then dropped. | ✓ |
| SGEN-35 | Independently of the template (except hot giant and white-dwarf remnant), a system gets cold giants with probability `cold_giant_chance` for its class group (FGKA or M), multiplied by 10^(`giant_metallicity_exponent` × [Fe/H]) and capped at `cold_giant_chance_cap`. If it does, it gets 1, 2 or 3 giants (`cold_giant_count_weights`), each a gas giant with probability `cold_giant_gas_share` and otherwise an ice giant. The first lies log-uniformly between `cold_giant_orbit_snow_lines` times the snow line; each next lies 1.6 to 3.0 times farther out. | ✓ |
| SGEN-36 | A planet's radius follows from its mass by the curve of its kind (`mass_radius`), in Earth units. Rocky and super-Earth: R = M^0.29. Sub-Neptune: R = 2.4 × (M ÷ 6)^0.5. Ice giant: R = 3.95 × (M ÷ 16)^0.3. Gas giant: R = 11.21 × (M ÷ 317.8)^0.142 below a Jupiter mass and 11.21 above. The curves give Mars, Venus, Earth, Uranus, Neptune, Saturn and Jupiter within 5% of their real radii. | ✓ |
| SGEN-37 | A system has 0 to 3 **belts**, the count drawn by template (`template_rules`). A belt is an annulus with an inner and an outer radius, outer ÷ inner drawn log-uniformly between `belt_width_ratio`; a mass in kg; and a composition split of rocky, metallic and icy fractions that sum to 1. The belt's centre is drawn log-uniformly between `belt_centre_snow_lines` times the snow line, and no planet's orbit lies inside the annulus, nor do two belts overlap; a belt that cannot be placed in 20 tries is dropped. Inside the snow line the icy fraction is at most 0.10 and the metallic fraction 0.10 to 0.30; beyond it the icy fraction is 0.60 to 0.90 and the metallic at most 0.05; the rocky fraction is what remains (`belt_composition`). Mass is log-uniform in the range for its side of the snow line (`belt_mass_kg`), times `debris_mass_factor` for a debris disc, times 10^[Fe/H]. | ✓ |
| SGEN-38 | A planet's surface **gravity** is G × M ÷ R², reported in units of g₀ = 9.80665 m/s². | ✓ |
| SGEN-39 | A planet's **equilibrium temperature** is 278.6 K × (L)^(1/4) × (a)^(−1/2) × (1 − A)^(1/4), with L in solar units, a in AU and the Bond albedo A fixed at `equilibrium_albedo`. It assumes the heat spreads evenly and ignores any greenhouse effect, so it is not the surface temperature. | ✓ |
| SGEN-40 | A rocky or super-Earth planet's **atmosphere** is one of airless, thin or dense. Its retention index is v² ÷ T, with v the escape velocity in km/s and T the equilibrium temperature in K, times a volatile endowment drawn log-uniformly between `volatile_endowment_range`. Below `atmosphere_thin_index` it is airless, from there to `atmosphere_dense_index` thin, and above that dense. With an endowment of 1 the rule gives Mercury, the Moon, Ganymede and Io airless; Mars and Titan thin; Venus and Earth dense. A sub-Neptune, ice giant or gas giant has an **envelope**. | ✓ |
| SGEN-41 | A rocky or super-Earth planet's **water** is surface, ice or none. Surface water needs a thin or dense atmosphere, an equilibrium temperature inside `surface_water_t_k` for that atmosphere, and a draw under `surface_water_chance`. Otherwise it has ice if its equilibrium temperature is at most `ice_t_eq_max_k` and a draw is under `ice_chance_beyond_snow_line` (at or beyond the snow line) or `ice_chance_inside_snow_line`. Otherwise none. A planet with an envelope has none. | ✓ |
| SGEN-42 | Distances scale with the star's luminosity L in solar units. The habitable zone runs from `hz_inner_au` × √L to `hz_outer_au` × √L AU, and the snow line is `snow_line_au` × √L AU. A white dwarf's snow line is `snow_line_au` itself, taken from its progenitor. | ✓ |
| SGEN-43 | A summary gives composition as classes only. It holds no resource deposit amounts, and no free-floating, Trojan or cometary bodies. | ✓ |
| SGEN-44 | Every planet is **tidally locked** or not. Its locking time is t = 0.4 ω a⁶ m (Q ÷ k₂) ÷ (3 G M² R³), with ω the spin of a `lock_initial_day_h` day, a the orbit radius, m and R the planet's mass and radius, M the parent's mass (the star, for a planet) and Q ÷ k₂ the value of `lock_q_over_k2` for its kind (rocky and super-Earth; sub-Neptune and giants; a moon's value is used by SYSG-33). It is locked if t is less than the system's age (SGEN-23). A locked body turns once per orbit, keeping one face to its parent; spin-orbit resonances are not generated. With the defaults the rule locks the Moon, Callisto and Iapetus to their planets and leaves Earth free of the Sun. | ✓ |
| SGEN-45 | A rocky or super-Earth planet has a **geological activity**: dead, active or extreme, from its interior heat flux in W/m², the sum of two terms. Radiogenic: `radiogenic_flux_w_m2` × (m in Earth masses)^`radiogenic_mass_exponent` × exp((`radiogenic_reference_age_gyr` − age) ÷ `radiogenic_decay_gyr`), with the system's age; the factor exceeds 1 for a system younger than the reference age. Tidal: (21 ÷ 2) × `tidal_response` × G M² R⁵ n e² ÷ a⁶, divided by the surface area 4 π R², with M the parent's mass, R the body's radius, a its orbit radius, n = √(G M ÷ a³) its mean motion and e the `forced_eccentricity` for its place (planet of a star; moon of a planet). The orbit itself stays circular (SCL-04): e is a heat input only. The flux is dead below `activity_active_w_m2`, active from there up to and including `activity_extreme_w_m2`, and extreme above. With the defaults Earth is active, Mars and the Moon dead, Io extreme and Europa active. Only rocky and super-Earth planets, moons and belt bodies carry the fact (SYSG-22, SYSG-33); a body with an envelope has none. | ✓ |

## 5. Stage 4: gate choice

The old network's builders gated systems they had a reason to reach. The generator scores each eligible system for four reasons, picks a core by score, joins it with junctions and relays, and adjusts until exactly N systems are gated. Scores run from 0 to 1.

| ID | Requirement | Built |
|---|---|---|
| SGEN-50 | **Settlement** score: the best habitability of any planet in the summary, times the star's calm factor. A planet's habitability is the product of four fits, each from 0 to 1: gravity (`settle_gravity_g`), equilibrium temperature (`settle_t_eq_k`), water (surface 1, ice 0.8, none 0.2) and atmosphere (thin or dense 1, airless 0.5, envelope 0). A fit that takes four values is 0 below and above the outer two and 1 between the middle two, linear between. The calm factor is 1, except `young_m_calm` for an M dwarf younger than `calm_age_gyr`. | ✓ |
| SGEN-51 | **Extraction** score: 1 − exp(−(B + G)). B is the sum over belts of mass ÷ `belt_value_scale_kg` × (metallic × `value_metal` + rocky × `value_rock` + icy × `value_ice`), and G is `giant_extraction_bonus` per giant planet. Both are multiplied by 10^(`metallicity_value_slope` × [Fe/H]). Belts therefore dominate. | ✓ |
| SGEN-52 | **Industry** score: the star's calm factor (SGEN-50) times the best pair of a water source and a metal source whose orbit radii differ by at most the factor `industry_radius_ratio`, scoring the smaller of the two. Water sources are icy belts and planets with ice or surface water; metal sources are belts with metallic or rocky fractions and rocky or super-Earth planets. A source scores 1 − exp(−its mass index), the index being the belt's value (SGEN-51) or `planet_source_index` for a planet. | ✓ |
| SGEN-53 | **Science** score: the largest of the values in `science_values` that apply: white dwarf, young system (under `young_system_gyr`), flare star (an M dwarf under `flare_age_gyr`), debris-disc template, hot-giant template, and extreme metallicity ([Fe/H] more than `extreme_metallicity` away from zero, either way). | ✓ |
| SGEN-54 | A system's **overall score** is the largest of its four scores. A system's **main reason** is the reason it was picked for, if it was picked for a quota (SGEN-55); otherwise its highest-scoring reason, ties in the order settlement, extraction, industry, science. A junction or relay (SGEN-56) has that as its main reason. | ✓ |
| SGEN-55 | The **core** has round(`core_share` × N) systems. First each of the four reasons has a **quota** of ceil(`reason_quota` × N) systems, filled in turns: in turn, each reason takes its highest-scoring eligible system not yet chosen, until every quota is met. The rest of the core is the unchosen eligible systems with the highest overall scores. Ties go to the lower id. | ✓ |
| SGEN-56 | **Connectors** join the core into one network whose links are no longer than the link range (SGEN-60). The chosen systems form **components**: groups in which every system is joined to another by a pair within link range. While there is more than one component, a **junction** is added if there is an unchosen eligible system within link range of more than one component: the one that touches most components, then has most chosen systems in range, then lies nearest. If there is none, the shortest chain of eligible systems between the two closest components is added as **relays**. If no chain exists, the link range grows by `link_range_growth` and the step repeats. | ✓ |
| SGEN-57 | The gated set has exactly N systems. The generator makes the core (SGEN-55) and its connectors (SGEN-56); if together they exceed N it drops the lowest-scoring core system that is not needed for a quota, and if they fall short it adds the highest-scoring unchosen eligible system, then repeats, for at most `max_gate_rounds`. If the rounds run out the generator removes the lowest-scoring chosen system that is not a quota pick and does not split the network, or adds the highest-scoring unchosen eligible system within link range of it, until the count is N. If the quota picks and their connectors alone exceed N, every reason's quota is lowered by one and the stage restarts. | ✓ |
| SGEN-58 | Each gated system gets a **gate ring**, the circle its gates will sit on (`system-generator.md`, section 6). Its starting radius is `ring_snow_line_factor` × the snow line. The ring is valid at a radius when it lies at least `ring_giant_clearance_hill` Hill radii (r_H = a × (m ÷ 3 M★)^(1/3)) from every giant's orbit, and at least `ring_planet_gap` × its radius from every planet's orbit. If the starting radius is not valid the ring slides to the nearest valid radius outward, if one lies within `ring_slide_limit` of the starting radius (as a fraction of it), and otherwise the nearest valid radius inward within the limit. The ring's radius is part of the sector (SGEN-90). Belts do not constrain it. | ✓ |
| SGEN-59 | If no valid radius lies within the slide limit, the ring stays at its starting radius and each planet that breaks the clearance moves to the nearest orbit that restores it, outward preferred. A planet it then crowds (SGEN-34) is pushed outward by the least amount that restores the spacing, and so on outward. A planet that moves has its facts (SGEN-38 to SGEN-45) worked out again. Scores and main reasons are not worked out again. | ✓ |

## 6. Stage 5: links

The links are the old network, before it failed. They are the same gate pairs that exist at game start, but all working (SGEN-70 fails most of them).

| ID | Requirement | Built |
|---|---|---|
| SGEN-60 | The **link range** is `link_range_factor` × (V ÷ N)^(1/3) (about 6 pc, 20 ly, at the defaults), grown by `link_range_growth` whenever a step cannot be done within it (SGEN-56, SGEN-61). The final range is part of the sector. | ✓ |
| SGEN-61 | Links are made in three steps. (1) A minimum spanning tree over the gated systems, by 3D distance. (2) Links between pairs within range are added shortest first while both ends have fewer than `links_max` links and the link crosses no existing link in the top-down projection. (3) Any system still with fewer than `links_min` links gets links to the nearest systems within range, not already linked and under `links_max` links, avoiding a crossing if one can be avoided; if none is within range, the range grows. The old network is therefore a dense mesh: every system links to as many near neighbours as the range and the no-crossing rule allow. | ✓ |
| SGEN-62 | Every gated system has between `links_min` and `links_max` links, the old network is connected, and no link is longer than the final link range. For the presets the mean number of links per system is at least `mean_links_min`. | ✓ |
| SGEN-63 | No link added in step 2 of SGEN-61 crosses another link in the top-down projection. | ✓ |
| SGEN-64 | After the links exist, a gated system is labelled **strategic** if it is not itself among the `strategic_top_share` of gated systems with the highest overall scores and at least `strategic_min_valuable` of the systems it links to are. It is an approach to valuable systems. The label is separate from the main reason. | ✓ |
| SGEN-65 | Each link records its two ends, its length, and its **origin** (spanning, extra or second, SGEN-61). Each end belongs to one gated system and becomes one gate or one wreck there (`system-generator.md`, section 6). | ✓ |

## 7. Failure of links

Most of the old network has failed (`docs/ideas/mechanics/gates.md`, section 6). A link is **working**, **dormant** (intact but too narrow for ships, faulted or locked at one end or both) or **collapsed** (both mouths gone). The shares below are targets. The rules after them take precedence, which is why the collapsed share ends slightly lower.

| ID | Requirement | Built |
|---|---|---|
| SGEN-70 | The **draw** uses the shares `draw_working_share` and `draw_dormant_share`, and the rest collapsed, with the working and dormant shares each shifted by a random amount within ± `state_share_shift` for the map. The rules that follow (SGEN-72 to SGEN-74) move some links towards less failure, so the **result** differs from the draw: its targets are `working_share`, `dormant_share` and `collapsed_share`, and the draw shares are set so the result lands near them. Over many sectors of each preset the mean share of each state in the result is within 0.06 of its target, and no single sector is more than 0.15 away. | ✓ |
| SGEN-71 | Each link gets a failure weight β × t + (1 − β) × u, with β = `failure_traffic_weight`, t the mean of its two systems' overall scores divided by the largest such mean, and u a random number in [0, 1). With L links, round(d × L) + round(c × L) of them fail, d and c being the dormant and collapsed draw shares after the shift of SGEN-70: the ones with the highest weights. Of those, the dormant count are chosen at random and the rest collapse. | ✓ |
| SGEN-72 | The largest group of systems joined by working links, the **main region**, holds at least `main_region_min_share` × N systems. If the draw leaves it smaller, the failed link of lowest failure weight that joins the main region to another group is made working, repeatedly, until it holds enough. | ✓ |
| SGEN-73 | Every gated system keeps at least one link that is not collapsed. A system whose links are all collapsed has the collapsed link of lowest failure weight made dormant. | ✓ |
| SGEN-74 | Every group outside the main region can be brought in by a **repair order**. A **group** is a set of systems joined by working links. The **held region** starts as the main region. A group is reached when a dormant link joins it to the held region; it then joins the held region, and the repair continues until every group is in. The link used to reach a group is the shortest dormant link between it and the held region, and it is faulted only at its end in the held region, so the holder can restart it. If a group has no dormant link to the held region, its collapsed link of lowest failure weight to the held region is made dormant first. Every other dormant link is faulted at one end or both, chosen at random. | ✓ |
| SGEN-75 | The rules are applied in the order SGEN-72, SGEN-73, SGEN-74. Each only moves a link towards less failure, so a later rule never undoes an earlier one. | ✓ |
| SGEN-76 | There are no links out of the sector. A dormant link records which ends are faulted but not why; whether a fault is damage or a deliberate lock is left to the historical layer. | ✓ |

## 8. Names

All names are placeholders, made from authored syllables; no name is final.

| ID | Requirement | Built |
|---|---|---|
| SGEN-80 | A name is a stem followed by an ending, from a plain-text data file `assets/names/names.txt` with sections `[stems]` (at least 120), `[endings]` (at least 40) and `[reserved]`. A name is capitalised. | ✓ |
| SGEN-81 | Every name made for a sector is unique in it, ignoring case, and none is in the reserved list (which starts with Corvane and Sol). If a draw repeats a name `max_name_retries` times, the generator makes a three-part name (stem, stem, ending). | ✓ |
| SGEN-82 | For each gated system the sector stores a name for the system, a name for each planet in its summary, and `belt_body_max` spare names for each belt in its summary (for large belt bodies, `system-generator.md`). Ungated entries have only their designation. Names are drawn from the stage's own seed in id order. Moons and belts are not given names here: a moon is named from its planet and a belt from its system (`system-generator.md`, section 7). | ✓ |

## 9. Output and checks

| ID | Requirement | Built |
|---|---|---|
| SGEN-90 | The result is one `Sector` value holding: the settings and the stage seeds; the slab's dimensions; every entry (id, designation, position, stars with spectral types, age, metallicity, companions and wide partner, summary, eligibility); for each gated system its name, main reason, strategic label, ring radius, planet names and spare belt-body names; the final link range; and every link (SGEN-65) with its state and its faulted ends. | ✓ |
| SGEN-91 | The invariants of this spec are tested over at least `invariant_seeds` seeds for each preset and for N = `min_gated_count`: SGEN-10, SGEN-11, SGEN-12, SGEN-29, SGEN-34, SGEN-37, SGEN-55, SGEN-57, SGEN-58, SGEN-62, SGEN-63, SGEN-64, SGEN-70, SGEN-72, SGEN-73, SGEN-74 and SGEN-81. The same seed twice gives equal sectors (SGEN-02). Changing a tunable of one stage leaves the earlier stages' outputs equal (SGEN-03). | ✓ |
| SGEN-92 | A sector with N = `max_gated_count` generates in a debug test build in under `generation_test_cap_s` seconds, which guards against work growing with the square of the entry count. | ✓ |

## Sources of the figures

Checked against published surveys on 2026-10-16; the paper or table is named, not quoted.

- *Stellar mix and density.* The RECONS 10 pc census (317 systems, 378 stars: 75% M, 12% K, 5% G, 2% F, 1% A, 6% white dwarfs; about 0.08 systems per pc³ in an incomplete sample) agrees with the idea doc's mix and 0.1 per pc³ within the sample's scatter. The white-dwarf and A shares are the idea doc's.
- *Brown dwarfs.* The 20 pc census (Kirkpatrick et al.) finds about four stars for every brown dwarf. `brown_dwarf_per_system` is 0.2.
- *Multiplicity.* M dwarfs: 26.8% multiple, companion separations peaking at 4 to 20 AU (Winters et al. 2019). Sun-like stars: 56% single, 33% double, 8% triple, 3% higher, period peak near 300 years (Raghavan et al. 2010). The K, F, A and white-dwarf fractions, the log-normal widths and the wide-pair share are my interpolations.
- *Small planets.* Mid-type M dwarfs host about 1.2 planets per star of 0.5 to 2.5 Earth radii with periods of 0.5 to 10 days (Hardegree-Ullman et al. 2019). At least a third of Sun-like stars host several planets of 1 to 4 Earth radii with periods under a year (Zhu et al. 2018, as reviewed by Winn and Petigura).
- *Giants.* 14.1% of stars host a giant at 2 to 8 AU and 8.9% at 8 to 32 AU, rising with stellar mass and metallicity (Fulton et al. 2021). Hot Jupiters: 0.5 to 1% of Sun-like stars, the Doppler surveys near 1% and Kepler near 0.6% (Winn and Petigura). `cold_giant_chance` and the template weights follow these. The metallicity exponent is the commonly quoted rule (giant occurrence rising about as 10^(2 [Fe/H])); I did not re-check it.
- *Habitable zone.* For the Sun the conservative limits are 0.99 and 1.70 AU (Kopparapu et al. 2013). Scaling by √L alone ignores the star's temperature, a simplification.
- *Not checked against a survey:* the snow line at 2.7 √L AU (a game convention taken from the Sun's early disc; real snow lines move with the disc's history), the mass-luminosity and mass-radius curves (textbook forms; the radius curves are anchored on solar-system bodies, not fitted), the atmosphere and water rules, white-dwarf luminosities, and every score, weight and quota.

## Tunables

All fields of `GeneratorTunables` (SGEN-05). Defaults are starting values; changing one is a spec change only if the default is being settled. A row with several values lists them in the order the requirement names them.

| Tunable | Default | Used by |
|---|---|---|
| `preset_small`, `preset_medium`, `preset_large` | 30, 80, 200 | SGEN-01 |
| `min_gated_count`, `max_gated_count` | 8, 300 | SGEN-01, SGEN-91, SGEN-92 |
| `stars_per_gated` | 3 | SGEN-10 |
| `wide_pair_fraction` | 0.035 | SGEN-10 |
| `brown_dwarf_per_system` | 0.2 | SGEN-10 |
| `system_density_per_pc3` | 0.1 | SGEN-11 |
| `slab_thickness_ratio` | 0.25 | SGEN-11 |
| `plan_elongation_max` | 1.25 | SGEN-12 |
| `min_system_separation_pc` | 0.4 pc | SGEN-12 |
| `designation_prefix` | "OW" | SGEN-20 |
| `star_mix` | M 0.72, K 0.12, G 0.07, F 0.03, A 0.007, white dwarf 0.052, giant 0.001 | SGEN-21 |
| `star_mass_ranges` (solar masses) | M 0.08 to 0.45, K 0.45 to 0.80, G 0.80 to 1.04, F 1.04 to 1.40, A 1.40 to 2.10, white dwarf 0.50 to 0.70, giant 1.0 to 3.0 | SGEN-22 |
| `star_mass_luminosity` | L = 0.23 M^2.3 below 0.43, M^4 from 0.43 to 2, 1.4 M^3.5 above | SGEN-22 |
| `star_mass_radius` | R = M^0.8 below 1, M^0.57 above | SGEN-22 |
| `white_dwarf_radius_rsun`, `white_dwarf_luminosity_lsun` | 0.012, 0.0001 to 0.003 | SGEN-22 |
| `giant_properties` | radius 10 to 50 R☉, luminosity 30 to 1,000 L☉ | SGEN-22 |
| `age_min_gyr`, `age_max_gyr`, `white_dwarf_age_min_gyr` | 0.2, 10, 1 | SGEN-23 |
| `metallicity_mean`, `metallicity_sigma`, `metallicity_clamp` | −0.05, 0.2, 0.6 | SGEN-24 |
| `multiplicity_by_class` | M 0.27, K 0.35, G 0.44, F 0.50, A 0.60, white dwarf 0.25 | SGEN-25 |
| `star_count_weights` | 2: 0.75, 3: 0.18, 4: 0.07 | SGEN-25 |
| `separation_log10_au` | M: mean 1.0, spread 1.3; other: mean 1.7, spread 1.5 | SGEN-26 |
| `companion_min_au` | 0.1 AU | SGEN-26 |
| `wide_pair_min_au`, `wide_pair_max_au` | 1,000 AU, 20,000 AU | SGEN-26, SGEN-27 |
| `brown_dwarf_mass_msun` | 0.013 to 0.075 | SGEN-28 |
| `companion_truncation_fraction` | 0.3 | SGEN-30 |
| `template_weights` | M: compact 0.65, solar-like 0.12, hot giant 0.002, debris 0.228. K: 0.40, 0.35, 0.01, 0.24. G: 0.35, 0.40, 0.01, 0.24. F: 0.30, 0.40, 0.01, 0.29. A: 0.20, 0.35, 0.01, 0.44. White dwarf: remnant 1 | SGEN-31 |
| `planet_kind_masses` (Earth masses) | rocky 0.05 to 2, super-Earth 2 to 8, sub-Neptune 3 to 12, ice giant 12 to 60, gas giant 60 to 1,600 | SGEN-32 |
| `template_rules` | as SGEN-33; kinds per template: compact 0.35 rocky, 0.30 super-Earth, 0.35 sub-Neptune; solar-like 0.7 rocky, 0.3 super-Earth; belts per template: compact 0 to 1, solar-like 1 to 2, hot giant 0 to 1, debris 2 to 3, remnant 1 to 2 | SGEN-33, SGEN-37 |
| `min_mutual_hill` | 8 | SGEN-34 |
| `cold_giant_chance` | FGKA 0.17, M 0.04 | SGEN-35 |
| `giant_metallicity_exponent`, `cold_giant_chance_cap` | 2.0, 0.6 | SGEN-35 |
| `cold_giant_count_weights`, `cold_giant_gas_share` | 1: 0.65, 2: 0.25, 3: 0.10; 0.6 | SGEN-35 |
| `cold_giant_orbit_snow_lines` | 0.8 to 8 | SGEN-35 |
| `mass_radius` | as SGEN-36 | SGEN-36 |
| `belt_width_ratio` | 1.2 to 2.0 | SGEN-37 |
| `belt_centre_snow_lines` | 0.15 to 6 | SGEN-37 |
| `belt_composition` | inside: icy ≤ 0.10, metallic 0.10 to 0.30; beyond: icy 0.60 to 0.90, metallic ≤ 0.05 | SGEN-37 |
| `belt_mass_kg` | inside 1e19 to 5e21; beyond 1e20 to 2e23; white dwarf 1e18 to 1e21 | SGEN-37 |
| `debris_mass_factor` | 30 | SGEN-37 |
| `equilibrium_albedo` | 0.3 | SGEN-39 |
| `volatile_endowment_range` | 0.5 to 2 | SGEN-40 |
| `atmosphere_thin_index`, `atmosphere_dense_index` | 0.07, 0.30 | SGEN-40 |
| `surface_water_t_k`, `surface_water_chance` | dense atmosphere 200 to 290 K, thin 255 to 290 K; 0.5 | SGEN-41 |
| `ice_t_eq_max_k` | 330 K | SGEN-41 |
| `ice_chance_beyond_snow_line`, `ice_chance_inside_snow_line` | 0.9, 0.35 | SGEN-41 |
| `hz_inner_au`, `hz_outer_au`, `snow_line_au` | 0.99, 1.70, 2.7 | SGEN-42 |
| `lock_initial_day_h` | 12 h | SGEN-44, SYSG-33 |
| `lock_q_over_k2` | rocky and super-Earth 330; sub-Neptune and giants 20,000; moon 6,700 | SGEN-44, SYSG-33 |
| `radiogenic_flux_w_m2`, `radiogenic_mass_exponent`, `radiogenic_reference_age_gyr`, `radiogenic_decay_gyr` | 0.09 W/m², 0.42, 4.5 Gyr, 3 Gyr | SGEN-45, SYSG-33 |
| `tidal_response` (k₂ ÷ Q, calibrated to Io) | 0.0165 | SGEN-45, SYSG-33 |
| `forced_eccentricity` | planet and belt body 0.001; moon 0.004 | SGEN-45, SYSG-33 |
| `activity_active_w_m2`, `activity_extreme_w_m2` | 0.05, 0.5 | SGEN-45, SYSG-33 |
| `settle_gravity_g` | 0.2, 0.4, 1.2, 1.6 | SGEN-50 |
| `settle_t_eq_k` | 150, 220, 300, 380 | SGEN-50 |
| `young_m_calm`, `calm_age_gyr` | 0.5, 2 Gyr | SGEN-50 |
| `belt_value_scale_kg` | 1e22 | SGEN-51 |
| `value_metal`, `value_rock`, `value_ice` | 3, 1, 1.5 | SGEN-51 |
| `giant_extraction_bonus` | 0.1 | SGEN-51 |
| `metallicity_value_slope` | 0.5 | SGEN-51 |
| `industry_radius_ratio`, `planet_source_index` | 4, 1 | SGEN-52 |
| `science_values` | white dwarf 1.0, young system 0.6, flare star 0.5, debris disc 0.5, hot giant 0.4, extreme metallicity 0.3 | SGEN-53 |
| `young_system_gyr`, `flare_age_gyr`, `extreme_metallicity` | 1 Gyr, 2 Gyr, 0.4 | SGEN-53 |
| `core_share`, `reason_quota` | 0.8, 0.10 | SGEN-55 |
| `max_gate_rounds` | 20 | SGEN-57 |
| `ring_snow_line_factor` | 1.35 | SGEN-58 |
| `ring_giant_clearance_hill` | 5 | SGEN-58 |
| `ring_planet_gap` | 0.05 (of the ring's radius) | SGEN-58 |
| `ring_slide_limit` | 0.5 (of the starting radius) | SGEN-58 |
| `link_range_factor`, `link_range_growth` | 2.0, 0.1 | SGEN-56, SGEN-60 |
| `links_min`, `links_max`, `mean_links_min` | 2, 5, 4.0 | SGEN-61, SGEN-62 |
| `strategic_top_share`, `strategic_min_valuable` | 0.15, 2 | SGEN-64 |
| `working_share`, `dormant_share`, `collapsed_share` (targets for the result) | 0.30, 0.15, 0.55 | SGEN-70 |
| `draw_working_share`, `draw_dormant_share` (the draw; collapsed takes the rest) | 0.25, 0.05 | SGEN-70, SGEN-71 |
| `state_share_shift` | 0.05 | SGEN-70 |
| `failure_traffic_weight` | 0.5 | SGEN-71 |
| `main_region_min_share` | 0.40 | SGEN-72 |
| `max_name_retries` | 20 | SGEN-81 |
| `belt_body_max` | 3 | SGEN-82, SYSG-41 |
| `invariant_seeds` | 50 | SGEN-91 |
| `generation_test_cap_s` | 30 s | SGEN-92 |

## Open points

- Every default above that the sources paragraph doesn't back is my draft: the scores and their weights, the template rules, the mass-radius curves for sub-Neptunes and ice giants, the atmosphere and water rules, the multiplicity of K, F and A stars and the failure weights. They need a look in play. Tune them before rewriting any rule.
- A quota pick can have a low score when a sector has few good candidates for a reason (no white dwarf for science, say). The quota is a floor on how many systems have the reason, not on their quality.
- Moving a planet to clear the gate ring (SGEN-59) leaves the system's scores as they were. The change is small and rare, so the score may not match the final planet list exactly.
- Circumbinary planets, planets of companion stars and eccentric, tilted or resonant orbits are not generated (SGEN-30, SCL-04). The forced eccentricity of SGEN-45 stands in for the resonances that keep real moons' orbits from circularising; it is one number per place rather than a check for resonant neighbours, so a moon with an inner neighbour near a 2:1 period ratio is no hotter than one without. Tune the number before adding that check.
- The locking and activity rules (SGEN-44, SGEN-45) are drafts checked against Sol's bodies only. In a sample of generated systems every moon of a giant locks, planets lock inside about 0.3 to 0.5 AU, and about a sixth of giants' moons and of compact-template rocky planets come out extreme.
- Hot-Jupiter occurrence is 0.5 to 1% in the surveys; the hot-giant weights (0.002 for M, 0.01 otherwise) sit at the top of that range.
- The shares are bound by the repair order. The links that have not collapsed must join every system, which takes at least N − 1 links, so the collapsed share cannot exceed 1 − (N − 1) ÷ L for L links. At the 1.45 links per system a sparse old network gets, that is about 31%, far below the 55% target. A prototype of SGEN-61 and SGEN-70 to SGEN-75 on random points (40 sectors for each of N = 30, 80 and 200, scores random) gave: with a mean of 2.9 links per system and the draw at the targets, a result near 40% working, 30% dormant and 29% collapsed; with a dense old network of 4.3 to 4.5 links per system (SGEN-61) and a draw of 25%, 5% and 70%, a result near 30%, 18% and 51%, the largest working region about 42% of the systems. Hence the dense mesh, which also fits `gates.md` (section 6, "a dense mesh"). If a sparser network is wanted, the collapsed target has to come down with it. The tolerances of SGEN-70 follow the prototype's spread and need checking against the real stage.
- With a dense mesh the old network has almost no cut points (the prototype found none in most sectors), so strategic (SGEN-64) marks approaches to valuable systems, not chokepoints. Chokepoints appear in the working network after failure, and a later view can show them.
- Whether the quota should regenerate a sector that has no habitable world at all is the fairness question for stage 8 (`docs/ideas/mechanics/world-generator.md`, section 8).

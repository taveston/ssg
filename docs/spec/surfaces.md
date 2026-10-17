# Spec: Planet surfaces

**In short:** every natural body's picture is drawn by one shader from its planet facts (SYSG-22), its star and a seed, with no art assets. A **look** (airless, icy, cold terrestrial, hazy, dense, lava, volcanic, sub-Neptune, ice giant, gas giant) is chosen from the facts by a fixed order of rules; modifiers for heat, craters and rays, the star's light and the knowledge state apply on top. When a body is opened its surface is **baked** once into a cubemap (albedo, height, emissive) at a resolution set by its knowledge state, and every frame draws the globe from that bake with a cheap pass for light, terminator, limb and grain. The same bake serves the inspection inset and the system view close in, where a globe replaces the disc marker once the body's projected disc is large enough. Corvane and Sol get authored facts so every body has a picture; Earth is drawn as a lava world by an authored override, the one nod to the back story.

*Scope:* the look chooser, the surface layers, the bake, the per-frame shading, where the picture is drawn in the inset and the system view, and the facts of the hand-made systems. Not here: the planetary view (section 3 of `presentation.md`, not yet specced; it will draw the same bake), settlement lights and surface overlays (no settlement data yet), clouds on rocky worlds, rings, the temperate look (surface water: reserved, SURF-14), knowledge states beyond surveyed and unsurveyed (INS-04), and a rotation period for bodies that are not locked. *Source ideas:* `docs/ideas/presentation/presentation.md` (section 2, procedural surfaces; section 5, planet looks from facts), mock-ups `29-planets.html` and `30-rays-and-hot-worlds.html`, `docs/ideas/mechanics/world-generator.md` 5.2. Where a mock-up and this spec disagree, this spec wins.

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables (table at the end), read from `src/view/tunables.rs`, never hard-coded. Every built requirement has an automated test (ARCH-36). The look chooser, the temperature rule and the resolution rule are plain functions of plain values (ARCH-20), tested without a window; the shader's layers are tested by baking small faces in a headless render pass and reading pixels back, or marked *Manual* where that is not possible.

*Terms:* the **facts** are a body's planet facts (SYSG-22): kind, gravity, equilibrium temperature, atmosphere, water, tidally locked, geological activity; plus, from its star, the spectral type, luminosity and age (SYSG-10). The **look** is the row of SURF-10 to SURF-19 that applies. The **surface temperature** is the local temperature SURF-21 gives from the equilibrium temperature and locking. The **bake** is the body's cubemap. The **knowledge value** is a number from 0 to 1 set by the knowledge state (SURF-32). The **projected radius** is the body's true radius on screen in pixels (MRK-07).

*Retired IDs* (never reuse): none.

---

## 1. Contract

| ID | Requirement | Built |
|---|---|---|
| SURF-01 | Every natural body with facts has a surface picture, drawn from the facts, the star and the body's seed and nothing else: no texture asset, no per-body art. Two bodies with equal facts, star and seed draw the same picture, pixel for pixel, on one machine. | ✓ |
| SURF-02 | A body's seed is derived from the sector seed and the name "surface" followed by the body's id, by the derivation the generator's stages use (SGEN-03). A body of a hand-made system (Corvane, Sol) uses its authored surface seed (SURF-60). | ✓ |
| SURF-03 | The look chooser (SURF-10 to SURF-19), the surface temperature rule (SURF-21), the crater and ray rule (SURF-22) and the resolution rule (SURF-32) are plain functions of the facts and the tunables, with no Bevy types (ARCH-20). | ✓ |
| SURF-04 | The shader and the bake live in `src/view/surfaces/`, one module per concern (chooser, bake, shading, inset, map). Nothing in `sim/` or `content/` knows the looks exist; content carries facts and an optional override (SURF-61) only. *Checked in review.* | ✓ |
| SURF-05 | A body without facts (none is expected once SURF-60 is built) draws the airless look with the default seed and logs a warning naming the body once. | ✓ |

## 2. The look

The first row that matches wins, in the order the rows are listed here (SURF-10, 11, 12, 13, 14, 16, 17, 15, 18, 19): a dense atmosphere hides a hot surface, so dense comes before lava. "Surface temperature" is the peak of SURF-21.

| ID | Requirement | Built |
|---|---|---|
| SURF-10 | **Gas giant:** kind gas giant. Bands sheared by latitude, a storm oval in a contrasting colour, a few small white ovals, no relief, a limb halo in the envelope colour. | ✓ |
| SURF-11 | **Ice giant:** kind ice giant. Soft blue bands, bright streaks in one latitude band, no relief, a limb halo. | ✓ |
| SURF-12 | **Sub-Neptune:** kind sub-Neptune. The ice giant look with the envelope colour mixed toward `sub_neptune_haze_colour` by `sub_neptune_haze_mix` and the halo `sub_neptune_halo_factor` times thicker. | ✓ |
| SURF-13 | **Volcanic:** geological activity extreme. Sulphur and frost patches, irregular dark pits (paterae) a share of which glow as lava lakes, two plumes over the limb, no craters. | ✓ |
| SURF-14 | **Temperate:** reserved for atmosphere thin or dense with surface water. Until it is specced, such a body takes the next row that matches. The chooser's order leaves this row's place free, so adding it changes no other row. | ✓ |
| SURF-16 | **Dense:** atmosphere dense. An opaque cloud deck with bands and swirls in the atmosphere colour, a thick limb halo, no relief and no craters. | ✓ |
| SURF-17 | **Hazy:** atmosphere thin, water ice and equilibrium temperature at most `hazy_t_max_k`. The dense look in the `hazy_colour` palette with the halo `hazy_halo_factor` times thinner: a cold, hazy world whose surface is hidden (a Titan). | ✓ |
| SURF-15 | **Lava:** surface temperature above `melt_t_k`. Baked rock with a molten sea where the surface temperature (SURF-21) is above `melt_t_k`, with floating crust rafts and a glowing edge where the sea meets rock; craters removed where molten; emissive colour from the surface temperature (SURF-20). | ✓ |
| SURF-18 | **Cold terrestrial:** atmosphere thin. Warped ridged relief, dark patches, eroded craters, a thin limb haze; polar caps if water is ice, covering `cap_latitude_deg` from each pole with a noisy edge. | ✓ |
| SURF-19 | **Icy** if atmosphere is airless and water is ice: bright ice with patches of dark dusty terrain, fresh-ice crater rims, a few faint curved lineae, craters shallower than on rock. **Airless** otherwise: grey cratered rock with darker plains and craters at every scale. | ✓ |

### Modifiers

| ID | Requirement | Built |
|---|---|---|
| SURF-20 | **Heat:** on every rocky look (SURF-13, SURF-15, SURF-17 to SURF-19) the surface glows where the surface temperature is above `glow_t_k`, with an emissive colour that runs from dull red at `glow_t_k` through orange to pale yellow at `melt_t_k` + `heat_ramp_k`, and a strength rising from 0 at `glow_t_k` to full at `melt_t_k`. The glow is emission: it shows on the night side and in the telescope frame. On the dense look the glow is hidden by the cloud deck and only the halo warms toward the glow colour by the same strength. | ✓ |
| SURF-21 | **Surface temperature:** for a planet that is tidally locked (SGEN-44) with an airless atmosphere, the surface temperature at a point is `locked_peak_factor` × Teq × (cos θ)^(1/4) for θ the angle from the substellar point, and `night_floor_k` beyond 90°. For any other body it is Teq everywhere. A moon locked to its planet counts as not locked to the star. Teq is the equilibrium temperature of the facts. The substellar point of a locked planet is a fixed point of its surface, the origin of its body frame (SURF-41), so the temperature map is part of the bake. | ✓ |
| SURF-22 | **Craters and rays:** the crater density (the share of the crater grid's cells that hold a crater, at every scale) is `crater_density_min` at age 0 rising linearly to `crater_density_max` at `crater_density_full_age_gyr` and above, for the star's age, times the activity factor: dead `crater_factor_dead`, active `crater_factor_active`, extreme `crater_factor_extreme`. On the airless and icy looks with activity dead, a sparse set of fresh craters, `ray_density` of the cells of a coarse grid, carries bright ray systems: albedo only, radial streaks fading with distance, each with its own age drawn up to `ray_max_age_gyr` from the body's seed and a strength falling from full at `ray_full_age_gyr` to none at `ray_max_age_gyr`. No rays on any other look or activity. | ✓ |
| SURF-23 | **Light:** the light direction is from the body to its star by their positions at the current clock, and the light colour is the star's spectral type's colour from `star_light_colours` (one colour per class letter M, K, G, F, A, white dwarf, giant) at `star_light_strength`. The terminator is sharp: diffuse light falls to zero within `terminator_width` of the terminator, with a wrap of `wrap_light` on looks with an atmosphere or envelope and none on the rest, and the night side keeps only `ambient_light` of the albedo. | ✓ |
| SURF-24 | **Atmosphere:** looks with an atmosphere or envelope (SURF-10 to SURF-12, SURF-16 to SURF-18) draw a limb glow outside the disc, falling off over `halo_width` of the radius, and a rim inside it, both in the look's atmosphere colour, lit on the star's side only. The airless, icy, lava and volcanic looks draw none. | ✓ |
| SURF-25 | **Knowledge:** the surveyed state uses `knowledge_surveyed` and the unsurveyed state `knowledge_unsurveyed` as the knowledge value. The value sets the number of noise octaves between `octaves_min` and `octaves_max` and which crater scales are drawn (the coarsest always, the finer ones at knowledge values above `crater_scale_knowledge`, one threshold per scale), so the large forms stay put from one state to the next. Below `telescope_knowledge` the picture is the telescope frame: the render at `telescope_block` × lower resolution blurred back up, desaturated by `telescope_desaturate`, with grain of `telescope_grain`. | ✓ |
| SURF-26 | **Large forms persist:** for one body, the unsurveyed picture and the surveyed picture agree in their large forms: the albedo averaged over each of the `large_form_samples` caps of `large_form_deg` across, centred on evenly spread directions (a Fibonacci sphere), differs between the two by at most `large_form_tolerance`, before the telescope frame is applied. | ✓ |

## 3. The bake

| ID | Requirement | Built |
|---|---|---|
| SURF-30 | When a body's picture is first needed (the inset opens on it, or its globe is due on the map, SURF-50), its surface is baked into a cubemap of six square faces holding albedo (rgb), height and emissive (rgb). The bake depends on the facts, the star's type and age, the seed and the knowledge value, and on nothing that changes per frame: no light, no clock, no camera. | ✓ |
| SURF-31 | The bake is spread over at most `bake_frames` frames, one or more faces per frame, so no frame stalls. Until it is complete the body draws its disc marker (inset: an empty image frame). | ✓ |
| SURF-32 | The face size is `bake_face_px` for the surveyed state and `bake_face_unsurveyed_px` for the unsurveyed state, as a plain function of the knowledge state. | ✓ |
| SURF-33 | Bakes are cached by body and knowledge state, up to `bake_cache_count` bodies; the least recently drawn is dropped first. A change of a body's knowledge state (or its facts, or the override of SURF-61) drops its bake and bakes again when next needed. | ✓ |
| SURF-34 | Close in, when the globe's projected radius exceeds `bake_face_px` × `rebake_zoom_factor`, the visible face is baked again at `bake_face_px` × `rebake_scale` with the finer crater scales on, and that face is drawn in place of the cached one while it is in view. The cached cubemap is kept. | ✓ |
| SURF-35 | The bake is a cubemap, not an equirectangular map: no pole pinching. Faces are sampled with seamless edges: a point on a face edge samples the same value from either face. | ✓ |

## 4. Drawing the globe

| ID | Requirement | Built |
|---|---|---|
| SURF-40 | Each frame the globe is drawn from the bake with the per-frame pass only: light direction and colour (SURF-23), the terminator, the limb and rim (SURF-24), the heat glow (SURF-20, from the baked emissive), and the telescope frame (SURF-25). The molten area of the lava look is in the bake (SURF-21), as is the heat glow's emissive. The pass samples the bake and evaluates no surface noise. | ✓ |
| SURF-41 | **Body frame:** the bake's cube frame has its poles on the orbital plane's normal (CAM-02) and a prime meridian fixed by the seed. A locked planet is turned each frame so its prime meridian faces the star (its substellar point, SURF-21); every other body keeps its prime meridian at its seed's fixed heading in the sky frame, not turning. | ✓ |
| SURF-42 | The globe is anti-aliased at its edge: the disc edge covers partial pixels by their coverage, not a hard step. | ✓ |

## 5. In the inset

| ID | Requirement | Built |
|---|---|---|
| SURF-45 | The inset image (INS-04) is the globe (SURF-40) of the selected natural body, drawn `inset_image_px` across, seen along the system view camera's line of sight (in the sky frame, CAM-02) at the current clock, so its phase is the phase the map would show. Ships, stations, gates and salvage sites keep INS-04's placeholder. | ✓ |
| SURF-46 | The image keeps INS-04's two states: surveyed is the clean globe; unsurveyed is the telescope frame (SURF-25) with four corner brackets at the image's corners, drawn as HOV-06's brackets at `inset_bracket_alpha`. | ✓ |

## 6. In the system view

| ID | Requirement | Built |
|---|---|---|
| SURF-50 | When a natural body's projected radius (MRK-07) exceeds `globe_min_px`, its true-size disc (MRK-11) is drawn as the globe (SURF-40) instead of the filled disc: the fill and rim of MRK-11 are replaced by the picture, and the gap outside stays. The body marker (MRK-13), label and selection mark draw on top as before. Below `globe_min_px` the disc is MRK-11's. The disc returns only when the projected radius falls below `globe_release_px`, which is less than `globe_min_px`, so the picture doesn't flicker at the edge. | ✓ |
| SURF-51 | The globe on the map is lit by the same light as the inset (SURF-23), so the map and the inset agree on phase. | ✓ |
| SURF-52 | A body whose bake is not ready (SURF-31) draws MRK-11's disc until it is. | ✓ |

## 7. Facts for the hand-made systems

Corvane and Sol are not generated, so their facts are authored. Gravity and equilibrium temperature follow the rules (SGEN-38, SGEN-39) from each body's mass, radius, orbit and star; the rest are authored here. The star's luminosity follows SGEN-22 from its mass unless authored.

| ID | Requirement | Built |
|---|---|---|
| SURF-60 | Every natural body of Corvane and Sol carries planet facts (SYSG-22) and a surface seed, authored in `content/`. Each star carries its spectral type, luminosity and age (SYSG-10). Corvane's star: K2V, luminosity by SGEN-22, age 5.1 Gyr. Sol: G2V, 1.0 L☉, 4.6 Gyr. | ✓ |
| SURF-61 | Content may author a **look override** on a body: a look of SURF-10 to SURF-19 drawn instead of the chooser's, with the body's own facts for everything else (SURF-20 to SURF-26). The generator never sets one. Only Earth has one (SURF-63). | ✓ |
| SURF-62 | Corvane's bodies have the facts of table 1. | ✓ |
| SURF-63 | Sol's bodies have the facts of table 2. Earth's facts are real (dense atmosphere, surface water, active) and it carries the override **lava**, so it is drawn as the world the collapse left (`docs/ideas/world/setting.md`), with its equilibrium temperature raised to `earth_override_t_k` for the override's surface temperature only; the inset still reads the real facts. | ✓ |

**Table 1: Corvane facts.** Kinds, atmosphere, water, locking and activity. Teq and gravity from the rules.

| Body | Kind | Atmosphere | Water | Locked | Activity | Note |
|---|---|---|---|---|---|---|
| Pell | rocky | airless | none | yes | dead | at 0.35 AU of a K2V, Teq about 340 K; the description's "scorched" reads as the heat glow only if `glow_t_k` is reached, which it is not: a dark baked rock |
| Iver | rocky | thin | ice | yes | dead | "tidally locked" in its description |
| Halden IV | rocky | thin | ice | no | active | the capital; see the open point on its temperature |
| Halden IVa, IVb, IVd | moon | airless | none | yes | dead | |
| Halden IVc | moon | airless | ice | yes | dead | "an ice moon" |
| Morrow | gas giant | envelope | none | no | none | |
| Morrow I to XI | moon | airless | ice for V, VI, IX; none for the rest | yes | extreme for I; active for II; dead for the rest | an Io and a Europa inside, by the forced-eccentricity rule's outcome for a Jupiter-class giant |

**Table 2: Sol facts.** Real values, within the facts' vocabulary.

| Body | Kind | Atmosphere | Water | Locked | Activity |
|---|---|---|---|---|---|
| Mercury | rocky | airless | none | yes | dead |
| Venus | rocky | dense | none | yes | active |
| Earth | rocky | dense | surface | no | active (override: lava) |
| Mars | rocky | thin | ice | no | dead |
| Ceres | rocky | airless | ice | no | dead |
| Jupiter, Saturn | gas giant | envelope | none | no | none |
| Uranus, Neptune | ice giant | envelope | none | no | none |
| Moon, Phobos, Deimos, Amalthea | moon | airless | none | yes | dead |
| Mimas, Tethys, Dione, Rhea, Iapetus, Ariel, Umbriel, Titania, Oberon, Proteus | moon | airless | ice | yes | dead |
| Io | moon | airless | none | yes | extreme |
| Europa, Enceladus | moon | airless | ice | yes | active |
| Ganymede, Callisto | moon | airless | ice | yes | dead |
| Titan | moon | thin | ice | yes | dead |
| Triton | moon | thin | ice | yes | active |

Mercury and the Moon have water none: their polar ice is too little for the fact. Mercury is locked here because spin-orbit resonances are not modelled (SGEN-44). Venus is locked for the same reason: its slow retrograde spin has no fact. Titan takes the hazy look (SURF-17); Triton, at 38 K with a thin atmosphere and ice, also does.

## 8. Checks

| ID | Requirement | Built |
|---|---|---|
| SURF-70 | The chooser is tested on every row with a fact set that selects it and one that just misses it, and on the Sol bodies of table 2: Io volcanic, Titan and Triton hazy, Venus dense, Mars cold terrestrial with caps, the Moon airless, Europa icy, Jupiter gas giant, Neptune ice giant, Earth with its override lava. A generated compact-system innermost rocky planet at Teq 1,500 K, locked, is lava; the same not locked, at 1,000 K with water none, is airless with glow; a Teq of 1,500 K with a dense atmosphere is dense. | ✓ |
| SURF-71 | Determinism (SURF-01) is tested by baking one small face twice for the same inputs and comparing pixels, and once with a different seed and finding a difference. | ✓ |
| SURF-72 | SURF-26 is tested on a baked body at both knowledge values. | ✓ |
| SURF-73 | The map globe (SURF-50) is tested headlessly through what drives it: the projected radius against `globe_min_px` with hysteresis decides globe or disc. | ✓ |

## Tunables

All in the `Tunables` resource (`src/view/tunables.rs`). Defaults are starting values from mock-ups 29 and 30, to be judged on screen; changing one is a spec change only if the default is being set by this spec for a reason given here. Shader constants that shape a look (noise frequencies, crater scales and depths, palette colours, plume shapes) are not tunables: they are named constants in the shader with a comment each, taken from the mock-ups, and changing them is tuning, not a spec change.

| Tunable | Default | Used by |
|---|---|---|
| `melt_t_k` | 1,400 K | SURF-15, SURF-20 |
| `glow_t_k` | 800 K | SURF-20 |
| `heat_ramp_k` | 800 K | SURF-20 |
| `locked_peak_factor` | 1.414 (4^(1/4)) | SURF-21 |
| `night_floor_k` | 90 K | SURF-21 |
| `hazy_t_max_k` | 150 K | SURF-17 |
| `hazy_colour`, `hazy_halo_factor` | warm orange-brown (placeholder), 0.6 | SURF-17 |
| `sub_neptune_haze_colour`, `sub_neptune_haze_mix`, `sub_neptune_halo_factor` | pale blue-grey, 0.5, 1.5 | SURF-12 |
| `cap_latitude_deg` | 12° | SURF-18 |
| `crater_density_min`, `crater_density_max`, `crater_density_full_age_gyr` | 0.28, 0.75, 8 Gyr | SURF-22 |
| `crater_factor_dead`, `crater_factor_active`, `crater_factor_extreme` | 1.0, 0.5, 0 | SURF-22 |
| `ray_full_age_gyr`, `ray_max_age_gyr` | 0.15 Gyr, 1.6 Gyr | SURF-22 |
| `ray_density` | 0.3 (share of the ray grid's cells that hold a fresh crater) | SURF-22 |
| `star_light_colours` | M (1, .58, .30); K (1, .80, .55); G (1, .96, .90); F (.88, .92, 1); A (.80, .86, 1); white dwarf (.85, .90, 1); giant (1, .70, .45) | SURF-23 |
| `star_light_strength` | 1.2 | SURF-23 |
| `terminator_width` | 0.04 (of the cosine of the light angle) | SURF-23 |
| `wrap_light` | 0.12 | SURF-23 |
| `ambient_light` | one value per look group: airless, icy, lava, volcanic 0.002; thin (cold terrestrial, hazy) 0.002; dense and envelope 0.008 | SURF-23 |
| `halo_width` | one value per look group, of the radius: thin 0.03, dense 0.06, envelope 0.05 | SURF-24 |
| `knowledge_surveyed`, `knowledge_unsurveyed` | 1.0, 0.0 | SURF-25 |
| `octaves_min`, `octaves_max` | 2.5, 8.5 | SURF-25 |
| `crater_scale_knowledge` | a list, one threshold per crater scale after the coarsest: 0.1, 0.45, 0.75; scales beyond the list are drawn only close in (SURF-34) | SURF-25 |
| `telescope_knowledge` | 0.2 | SURF-25 |
| `telescope_block`, `telescope_desaturate`, `telescope_grain` | 4, 0.38, 22 (of 255) | SURF-25 |
| `large_form_deg`, `large_form_samples`, `large_form_tolerance` | 30°, 32, 0.08 | SURF-26 |
| `bake_frames` | 6 | SURF-31 |
| `bake_face_px`, `bake_face_unsurveyed_px` | 1024, 256 | SURF-32, SURF-34 |
| `bake_cache_count` | 8 | SURF-33 |
| `rebake_zoom_factor`, `rebake_scale` | 0.5, 4 | SURF-34 |
| `inset_image_px` | 300 | SURF-45 |
| `inset_bracket_alpha` | 0.75 | SURF-46 |
| `globe_min_px`, `globe_release_px` (release below min) | 24, 20 | SURF-50 |
| `earth_override_t_k` | 1,500 K | SURF-63 |

## Open points

- Halden IV sits at 2.0 AU round a 0.78 M☉ star (luminosity about 0.37 L☉ by SGEN-22), which gives a Teq of about 140 K, against its description's −41 °C (232 K), and with thin air and ice that is the hazy look: a capital whose surface is hidden. Either move it in to about 0.75 AU or accept it. Logged as F-300.
- The temperate look (SURF-14) needs oceans, land and clouds, none of which the facts describe. When it is specced it may want new facts (land fraction, cloud cover) that the generator would have to supply.
- A hot world with a thin atmosphere takes the lava look when it melts and the cold terrestrial look with glow when it doesn't, and in both its thin haze is not drawn (SURF-24 keys on the look). A hot dense world hides its glow under the clouds (SURF-20). All first guesses.
- Bodies that are not locked do not turn (SURF-41). A rotation period would let them, and would give a day length for the economy; it needs a fact the generator does not have.
- Settlement lights on the night side (mock-up 29) wait for settlements to be data.
- Whether the planetary view's globe needs a higher `bake_face_px` than the inset, and whether the close-in patch (SURF-34) should be a second cached level rather than a single face.
- The crater density rule reads the star's age for every body. A body resurfaced long ago but now dead (an old lava plain) keeps full density; a surface age fact would fix that and is not worth one yet.

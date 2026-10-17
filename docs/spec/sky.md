# Spec: Sky

**In short:** a sky behind the map, in the system view and in the sector view, to give a sense of scale: the sector is a small part of a galaxy. It is one sky for the whole game, made from a seed derived from the sector seed, and tied to no system. It holds a field of stars of many brightnesses and a Milky Way: a denser band of faint stars with a very faint glow and dark lanes, tilted off the orbital plane so that it always crosses the opening view. It is drawn at infinity, so it turns with the camera's heading and tilt and never moves when the view pans or zooms. Its stars are soft points in pale versions of natural star colours. It is always on, never a target, and drawn below everything else. In the sector view it is dimmer, so it stays quieter than the sector's own stars.

*Scope:* the sky's content, its brightness and colour, how it is projected and where it is drawn, and its place in both views. Not here: a toggle (layer toggles are not specced), a richer sky in the planetary view or the inspection inset, any label on the sky, motion of the sky with time. *Source ideas:* `docs/ideas/presentation/presentation.md` (section 2, Sky), mock-up `28-skybox.png`. It reuses the palette, camera and map area of `system-view.md` and the seed derivation of `sector-generator.md`. Where the mock-up and this spec disagree, this spec wins.

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables (table at the end), read from `src/view/tunables.rs`, never hard-coded. Every built requirement has an automated test (ARCH-36). Rules are plain functions of a seed or of plain values (ARCH-20), so most are tested without a window.

*Terms:* the **sky frame** is east, north and up, where up is the normal of the orbital plane (CAM-02). A **direction** is a unit vector in that frame. The **standard view** is the camera at heading 0 and tilt `standard_tilt_deg` (CAM-02). **Magnitude** is a star's brightness on the astronomer's scale: each step of 1 is a factor of about 2.5 fainter, so larger means fainter. The **band** is a great circle on the sky, tilted off the orbital plane. The **sky tone** is the neutral cool white that stars and glow are drawn in.

*Retired IDs* (never reuse): none.

---

## 1. Content

| ID | Requirement | Built |
|---|---|---|
| SKY-01 | The sky is a plain value made from a seed and the tunables by one function, with no Bevy types: a list of stars (a direction, a magnitude and a colour), the band's orientation (SKY-07) and the offsets of its noise (SKY-14). The same seed and tunables give the same sky, field for field, on every run and machine. The function reads no clock, environment or unseeded random source. The view makes the sky when a game starts and never stores it in the sector or a save. | ✓ |
| SKY-02 | The sky's seed is derived from the sector seed and the name "sky", by the derivation the generator's stages use (SGEN-03). A system loaded without a sector (Corvane, Sol) uses `sky_default_seed`. Sol has no sky of its own. |  |
| SKY-03 | Different seeds give different skies: the star lists differ, and so does the band's orientation. | ✓ |
| SKY-04 | A sky holds `sky_field_stars` field stars and `sky_band_stars` band stars. | ✓ |
| SKY-05 | A field star's direction is uniform over the sphere. Its magnitude is drawn between `sky_mag_min` and `sky_mag_max` so that the number of stars at magnitude *m* is proportional to 10 to the power (`sky_mag_slope` × *m*): a few bright stars, more medium ones and a faint majority. | ✓ |
| SKY-06 | A band star lies near the band. Its latitude from the band is drawn from a normal distribution of standard deviation `sky_band_sigma_deg`. Its longitude along the band, measured from the core, is drawn uniformly and accepted with probability `sky_core_floor` + (1 − `sky_core_floor`) × exp(−*l*² / (2 × `sky_core_sigma_deg`²)), so the band is denser towards the core. Its magnitude is drawn as SKY-05 does between `sky_band_mag_min` and `sky_band_mag_max` with slope `sky_band_mag_slope`, so the band is made of fainter stars than the field. | ✓ |
| SKY-07 | The band's orientation comes from the seed. The angle between the band's normal and the orbital plane's normal is drawn uniformly between `sky_tilt_min_deg` and `sky_tilt_max_deg`. The azimuth of that normal (its direction round the plane, from north) is chosen by SKY-08. The core's position along the band is uniform. | ✓ |
| SKY-08 | For every seed, the angular distance between the band's great circle and the standard view's line of sight is at most `sky_band_view_margin_deg`, so a new game opens with the band on screen. The azimuth is drawn uniformly from the values that satisfy this. If none does (the tunables put the range out of reach), it is the value with the smallest distance. | ✓ |

## 2. Brightness, colour and softness

| ID | Requirement | Built |
|---|---|---|
| SKY-09 | A star of magnitude *m* is drawn at the opacity `sky_peak` × 10 to the power (−0.4 × `sky_gamma` × *m*) in its colour (SKY-12), over what is beneath it. The brightest possible star is `sky_peak`, and the faintest star keeps a visible opacity, at least `sky_faint_floor`. | ✓ |
| SKY-10 | A star is a soft point: a Gaussian of standard deviation `sky_sigma_px` + `sky_sigma_bright_px` × max(0, `sky_bright_mag` − *m*), in screen pixels, so brighter stars are larger. Sizes are in screen pixels and do not depend on the zoom or the field of view. | ✓ |
| SKY-11 | A star is drawn at its exact position on screen, not rounded to a whole pixel (SYM-16 does not apply), with its light spread over the pixels round it. The total light deposited by one star is within `sky_flicker_tolerance` of its mean for every sub-pixel offset, so a star neither flickers nor drops out as the view turns. | ✓ |
| SKY-12 | A star's colour is drawn by type: bluish-white `sky_type_blue` with weight 0.25, white `sky_type_white` with weight 0.30, yellow-white `sky_type_yellow` with weight 0.25 and orange `sky_type_orange` with weight 0.20, from the seed. The colour used is the sky tone, moved `sky_colour_keep` of the way towards the type's colour, so it stays pale. No star's colour has an HSV saturation above `sky_max_saturation`, so no star can read as an affiliation colour or the warning orange (SYM-02, MRK-01). | ✓ |
| SKY-13 | The band has a glow: a translucent fill in the sky tone along the band. Its opacity is `sky_glow_alpha` × a profile that falls off across the band as a normal curve of standard deviation `sky_glow_sigma_deg` from the band's centre line, and along it as `sky_core_floor` + (1 − `sky_core_floor`) × exp(−*l*² / (2 × `sky_core_sigma_deg`²)), × the factor of SKY-14. It is not drawn as a hard edge: the glow is under 5% of its peak at three standard deviations from the centre line. | ✓ |
| SKY-14 | The glow has dark lanes and a slight mottle, from seeded noise of the direction: a lane lowers the glow by up to `sky_lane_depth` and the mottle varies it by up to `sky_glow_mottle` either way. The pattern depends on the seed. The glow's factor from both stays between (1 − `sky_lane_depth`) × (1 − `sky_glow_mottle`) and 1 + `sky_glow_mottle`. | ✓ |
| SKY-15 | The glow is dithered: before it is rounded to the display's levels, a seeded noise of `sky_dither_levels` levels at most, either way and averaging zero, is added per pixel, so the gradient shows no visible bands on an 8-bit display. | ✓ |

## 3. Drawing

| ID | Requirement | Built |
|---|---|---|
| SKY-16 | A star is drawn at the screen position of its direction at infinity through the camera of the view being drawn: the camera's heading, tilt and field of view, with the map area as its viewport (INS-07). The camera's position, pivot, frame and scale play no part. So panning, zooming and changing the frame never move a star on screen, and a change of heading by Δ turns every direction by Δ about the plane's normal. A direction behind the camera is not drawn. | ✓ |
| SKY-17 | The sky is static: it does not change with the game clock, pause or the time rate. | ✓ |
| SKY-18 | The sky is drawn inside the map area only: not under the top bar or the inset (INS-07). When the inset slides, the sky follows the map area as the map does. | ✓ |
| SKY-19 | The sky is never a hover or click target and never changes what a click hits (MRK-08), and it carries no labels. | ✓ |
| SKY-20 | The sky is the bottom layer in both views: below the belts, orbit rings, tails and the frame cue in the system view (MRK-12), and below the links in the sector view (SECT-05). | ✓ |
| SKY-21 | No star is drawn as an outline or a ring. Every star is a filled soft point (SKY-10), so none can be mistaken for a sector node (SECT-03) or a body marker (MRK-13). | ✓ |
| SKY-22 | The sky is always on. It has no toggle until layer toggles are specced. | ✓ |

## 4. Sector view

| ID | Requirement | Built |
|---|---|---|
| SKY-23 | The sector view draws the same sky as the system view, from the same seed, projected as SKY-16 says, with the opacity of every star and of the glow multiplied by `sector_sky_factor`. Changing layer changes nothing in the sky but that factor. |  |
| SKY-24 | `sky_peak` × `sector_sky_factor` is at most `sector_faint_alpha` (SECT-03): in the sector view, the brightest star in the sky is no brighter than the faintest sector node. The default tunables satisfy it. |  |

---

## Tunables

All in the `Tunables` resource (`src/view/tunables.rs`). Defaults are starting values, taken from mock-up 28 and to be judged on screen; changing one is a spec change only if the default is being settled.

| Tunable | Default | Used by |
|---|---|---|
| `sky_default_seed` | 1 | SKY-02 |
| `sky_field_stars` | 40,000 | SKY-04 |
| `sky_band_stars` | 110,000 | SKY-04 |
| `sky_mag_min` | 0 | SKY-05 |
| `sky_mag_max` | 8.6 | SKY-05 |
| `sky_mag_slope` | 0.42 | SKY-05 |
| `sky_band_mag_min` | 2.5 | SKY-06 |
| `sky_band_mag_max` | 9.2 | SKY-06 |
| `sky_band_mag_slope` | 0.5 | SKY-06 |
| `sky_band_sigma_deg` | 9° | SKY-06 |
| `sky_core_sigma_deg` | 40° | SKY-06, SKY-13 |
| `sky_core_floor` | 0.4 | SKY-06, SKY-13 |
| `sky_tilt_min_deg` | 45° | SKY-07 |
| `sky_tilt_max_deg` | 75° | SKY-07 |
| `sky_band_view_margin_deg` | 10° | SKY-08 |
| `sky_tone` | #C7D6E0 | SKY-12, SKY-13 |
| `sky_peak` | 0.55 | SKY-09, SKY-24 |
| `sky_gamma` | 0.22 | SKY-09 |
| `sky_faint_floor` | 0.04 | SKY-09 |
| `sky_sigma_px` | 0.55 px | SKY-10 |
| `sky_sigma_bright_px` | 0.17 px | SKY-10 |
| `sky_bright_mag` | 3.2 | SKY-10 |
| `sky_flicker_tolerance` | 0.05 | SKY-11 |
| `sky_type_blue` | #A8C7FF | SKY-12 |
| `sky_type_white` | #E6F0FF | SKY-12 |
| `sky_type_yellow` | #FFF2D1 | SKY-12 |
| `sky_type_orange` | #FFC794 | SKY-12 |
| `sky_colour_keep` | 0.4 | SKY-12 |
| `sky_max_saturation` | 0.25 | SKY-12 |
| `sky_glow_alpha` | 0.085 | SKY-13 |
| `sky_glow_sigma_deg` | 10° | SKY-13 |
| `sky_lane_depth` | 0.6 | SKY-14 |
| `sky_glow_mottle` | 0.25 | SKY-14 |
| `sky_dither_levels` | 1 | SKY-15 |
| `sector_sky_factor` | 0.5 | SKY-23, SKY-24 |

## Open points

- How the sky is drawn is not prescribed. Tens of thousands of points are one instanced draw, and the glow is a cheap field over the map area. If the frame time suffers, the glow can be baked to a small texture when the seed or the camera changes. The tests (projected positions, deposited light, factors) don't depend on the method.
- The brightest few stars, at about 0.55 of the sky tone, are a little brighter than the belt's grain (`belt_speck_alpha` 0.5 of the orbit colour, MRK-18). What tells them apart is that the grain sits in a ring round the star and moves with the map while the sky holds still. If the two get confused on screen, lower `sky_peak`.
- Sizes are in screen pixels, like every marker (MRK-03). There is no display-scale setting yet; if one comes, whether stars follow it is open.
- The planetary view and the inspection inset show no sky. Whether they should, and with what, is open.
- The sky has no toggle (SKY-22). It joins the layer list when layer toggles are specced.
- Every look here is a first draft from mock-up 28. Judge it on screen and add findings.

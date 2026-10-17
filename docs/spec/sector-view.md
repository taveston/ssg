# Spec: Sector view

**In short:** the sector view is the top layer of the map: the generated sector drawn as a vector display in the style of the system view, with the same camera. It is a **truth view**: it shows every star in the sector and every link, whatever any faction knows, and says so on screen. Gated systems are star markers with names; other stars are dimmer dots. Links are lines, drawn by state: working solid, dormant dashed, collapsed faint. Clicking a star opens the inspection inset with its summary, planets, belts and gates. Double-clicking a gated system, or the inset's button, enters it; the system view opens on it, with a Back button and the breadcrumb leading out. Scrolling never changes layer. The game starts in the sector view with `--sector <small|medium|large|N>` and an optional `--seed <n>`.

*Scope:* the sector view as a layer, entering and leaving a generated system, and the start-up arguments for a sector. Not here: what any faction knows (fog of war), drop lines or a plane grid, layer toggles, links as click targets, travel between systems, ships in the sector. *Source ideas:* `docs/ideas/presentation/presentation.md` (sections 2 and 3: the sector view, camera and changing layer), `docs/ideas/mechanics/world-generator.md`, `docs/ideas/mechanics/gates.md` (section 7). It reuses the palette, type, buttons, camera, inset and labels of `system-view.md`; where it names one of those requirements, the sector view follows it with the changes it states.

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables (table at the end), read from `src/view/tunables.rs`, never hard-coded. Every built requirement has an automated test (ARCH-36); positions, hover, clicks, double-clicks and arguments are tested through projected positions and simulated input in a headless app.

*Retired IDs* (never reuse): none.

---

## 1. What it shows

| ID | Requirement | Built |
|---|---|---|
| SECT-01 | The sector view and the system view are layers: exactly one is shown at a time, held in one state that both read. The sector view shows the sector the game was started with (SGEN-90). | |
| SECT-02 | The sector view uses the palette (MRK-01), type roles (MRK-02), buttons (BTN-01), top bar (CLK-02) and sidebar (INS-07) of the system view, and adds no colour. The clock runs as in the system view, and its keys work; nothing in the sector view moves with time. | |
| SECT-03 | Every catalogue entry that has a star is a **node** at its position (SGEN-12). A gated system's node is a star marker (MRK-13) `sector_node_px` across. An ungated system's node is a plain circle outline `sector_dot_px` across at `sector_dot_alpha`. A brown dwarf's is the same at `sector_faint_alpha`. Marker size never depends on physical size or distance (MRK-03). | |
| SECT-04 | A **link** is a straight line between its two nodes' centres. Working: solid, `link_working_px` wide at `link_working_alpha`. Dormant: dashed, with dashes `link_dash_px` long and gaps `link_gap_px`, at `link_dormant_alpha`. Collapsed: solid, `link_collapsed_px` wide at `link_collapsed_alpha`. All are in the orbit colour (MRK-01). Links are not hover or click targets. | |
| SECT-05 | Draw order, from the bottom: the sky (SKY-20); links; nodes, nearer the camera on top of farther; labels and the hover and selection marks (HOV-06); the sidebar. | |
| SECT-06 | A gated node is labelled with its system's name, in the label face in the neutral colour. An ungated node is labelled with its designation only while hovered or selected. Labels are placed by LBL-01 to LBL-04, and a gated node's label is kept before another's when there is not room for both. | |
| SECT-07 | Hovering a node draws the hover mark and selecting one the selection mark (HOV-06), round the node's box (HOV-07). | |
| SECT-08 | A readout at the top left of the map area, `plot_margin_px` from its edges, has two lines. The first, in small spaced capitals in the secondary colour, reads "TRUTH VIEW · SHOWS EVERYTHING, NOT WHAT ANYONE KNOWS". The second, in the data face in the secondary colour, reads "Seed *seed* · *N* gated systems". | |
| SECT-09 | The scale bar (MRK-16) and the north dial (MRK-17) are shown, with the bar's lengths in light years (1, 2 or 5 times a power of ten) measured along the screen's horizontal at the pivot. North is the same fixed direction as in the system view (CAM-02). The frame cue (REF-01) is not drawn. | |

## 2. Camera and navigation

The sector view has no frame body. Its camera behaves as the system view's does in the star's frame: the pivot is the sector's centre plus a pan offset, on the sector's mean plane.

| ID | Requirement | Built |
|---|---|---|
| SECT-10 | The camera follows CAM-01 to CAM-04, CAM-10 to CAM-20 (the mouse, the keys, the bindings resource and the pan bound, with the sector fit's radius for the system fit's) with the sector's centre in place of the star. The standard orientation is CAM-02's. | |
| SECT-11 | Zoom follows ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-13 (anchored on the point where the cursor's ray meets the mean plane) and CAM-16. There is one stop, the **sector fit**: every node fits inside the map area, on both axes and in perspective, at the current tilt, with the margin `zoom_fit_margin`. The inner limit is a view radius of `sector_min_view_radius_ly`. Scrolling past a limit does nothing and never changes layer. | |
| SECT-12 | Home returns to the sector fit and the standard orientation in the animated move of ZOOM-10, and N to the standard orientation (CAM-03). The view starts at the sector fit in the standard orientation with nothing selected. Bookmarks (ZOOM-11) do nothing in the sector view. | |
| SECT-13 | A distance between stars is shown in light years to one decimal place, with thousands separators, for example "10.4 ly" (UNIT-05). The sector's positions are stored in km (SGEN-12). | |

## 3. Selection and the inset

| ID | Requirement | Built |
|---|---|---|
| SECT-20 | A click on a node selects it and opens the inset (INS-01, INS-03, INS-07); a click on empty space, Esc and the inset's Close button clear the selection. There is at most one selection and one inset. | |
| SECT-21 | The inset has a header (the system's name, or the designation of an ungated entry, and a line giving its class: "Gated system · *main reason*", "Ungated system", "Brown dwarf" or "Giant"), a Close button (quiet) and, for a gated system, an **Enter system** button (secondary). It has no Focus button. | |
| SECT-22 | The key facts (INS-09) of an entry with a star are: **Star** (the spectral type of each star, the primary first), **Mass** in solar masses to 2 decimals, **Luminosity** in solar units to 3 significant figures, **Age** in Gyr to one decimal, **Metallicity** ("[Fe/H]" and the value to 2 decimals), **Distance from centre** (UNIT-05), **Companions** ("none", or the number and the nearest separation in AU), and, if it has one, **Wide partner** (its name or designation). A gated system also has **Reason** (the main reason, SGEN-54) and, if labelled, **Strategic**: "yes". | |
| SECT-23 | An entry with a summary has a section "Planets": one row per planet, in orbit order, with its name (a gated system's) or "Planet *n*" (an ungated system's), its kind, and on a second line its orbit (UNIT-01), mass in Earth masses to 2 significant figures, gravity in g to 2 decimals, equilibrium temperature in whole kelvin, atmosphere and water. A section "Belts" has one row per belt: its name (a gated system's, SYSG-40) or "Belt *n*", its inner and outer radius (UNIT-01), mass in kg in scientific form to 2 significant figures, and the rocky, metallic and icy shares as whole percentages. | |
| SECT-24 | A gated system's inset has a section "Gates" with one row per link: the destination's name, the link's state ("Working", "Dormant" or "Collapsed"), for a dormant link which ends are faulted ("faulted here", "faulted there" or "faulted at both ends"), and its length (UNIT-05). Clicking a row selects the destination's node and does not move the camera. | |

## 4. Entering and leaving a system

| ID | Requirement | Built |
|---|---|---|
| SECT-30 | A double-click on a gated node, the inset's Enter system button, and the Enter key (bound through the bindings resource, CAM-18) with a gated node selected enter that system. A double-click on any other node only selects it. The layer is never changed by scrolling or zooming (ZOOM-14). | |
| SECT-31 | Entering generates the system (SYSG-01) and replaces the running system with it. The view switches to the system view at the system fit, in the standard orientation, in the star's frame with nothing selected. The clock and date carry on. The breadcrumb reads "Sector / *system name*" (CAM-25). | |
| SECT-32 | A system opened from a sector has a **Back** button (quiet) at the left of the top bar, before the breadcrumb, and the breadcrumb's Sector entry. Either returns to the sector view with the camera and selection as they were left, and the system's records are dropped. Entering the same system again gives the same system (SYSG-02). | |
| SECT-33 | In a system started with `--system` (SOL-22) there is no sector: the Sector entry does nothing and there is no Back button. | |
| SECT-34 | In the sector view the breadcrumb reads "Sector" alone, drawn in the frame colour (MRK-01), as the entry that stands for the current frame (CAM-25). | |

## 5. Start-up

| ID | Requirement | Built |
|---|---|---|
| SECT-40 | The argument `--sector <small\|medium\|large\|N>` starts the game in the sector view with a new sector. `small`, `medium` and `large` are matched exactly, in lower case, and mean the presets (SGEN-01). N is a decimal integer in the allowed range (SGEN-01). Anything else, or a missing value, stops the program with a message that lists the presets and the range. | |
| SECT-41 | The argument `--seed <n>`, a decimal `u64`, sets the sector's seed. Without it the seed is random (SGEN-04). `--seed` without `--sector`, and an unparsable value, stop the program with a message. | |
| SECT-42 | `--sector` together with `--system` stops the program with a message. With neither, the game starts as before on Corvane (SOL-22). | |
| SECT-43 | The arguments are read in `content/` by a plain function (ARCH-20) alongside SOL-22's; `main.rs` only adds the plugin (SOL-23). *Checked in review.* | |
| SECT-44 | The screenshot tool (`OW_SHOT`) accepts the keys `sector` (a preset or N) and `seed`, which start the shot in the sector view; `select` then takes a gated system's name, or a designation. The key `enter` takes a gated system's name and enters it before the shot, so that `frame`, `select` and `hover` name its bodies. An unknown name is an error, as for the existing keys. | |

## Tunables

All in the `Tunables` resource (`src/view/tunables.rs`). Defaults are starting values; changing one is a spec change only if the default is being settled. The system view's tunables used here (`zoom_fit_margin`, `standard_tilt_deg`, `pan_bound_factor`, `plot_margin_px` and the rest) keep their values.

| Tunable | Default | Used by |
|---|---|---|
| `sector_node_px` | 14 px | SECT-03 |
| `sector_dot_px` | 5 px | SECT-03 |
| `sector_dot_alpha` | 0.5 | SECT-03 |
| `sector_faint_alpha` | 0.3 | SECT-03 |
| `link_working_px` | 1.5 px | SECT-04 |
| `link_working_alpha` | 0.9 | SECT-04 |
| `link_dormant_alpha` | 0.7 | SECT-04 |
| `link_dash_px` | 6 px | SECT-04 |
| `link_gap_px` | 4 px | SECT-04 |
| `link_collapsed_px` | 1 px | SECT-04 |
| `link_collapsed_alpha` | 0.2 | SECT-04 |
| `sector_min_view_radius_ly` | 3 ly | SECT-11 |

## Open points

- Every look above is a first draft from the system view's style, to be judged on screen. Mock-ups of the sector view are not drawn yet.
- Height is not shown beyond tilting the camera. Drop lines or a faint plane grid (`docs/ideas/presentation/presentation.md`) are candidates once the view is seen.
- A cut between layers is the whole transition. A zoom-through is not specced.
- Links are not targets, and the sector view has no Focus, bookmarks, selection cycling, layer toggles or search. A way to find a system by name will want one once sectors reach 200 systems.
- The look of a gate by state in the system view is open (`system-generator.md`, open points).
- The view shows the truth. A view filtered by what a faction knows replaces it later; its rules are not specced.
- The Sector entry in the breadcrumb of a test system does nothing (SECT-33); whether Corvane and Sol should sit in a sector of their own is open.

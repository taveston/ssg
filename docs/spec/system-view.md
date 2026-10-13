# Spec: System view

**In short:** one star system drawn as a vector display at true scale, with fixed-size markers; ships and stations use NATO-style symbols. Clicking an object, or its orbit ring, selects it and opens the inspection inset, docked at the right of the map; there is one selection and one inset; the camera always sits in the reference frame of one object, the star by default, shown as a path in the breadcrumb. Zoom is stepped, with two stops: the whole system and the frame body's satellites. The frame is only ever changed deliberately. Satellites fold into their parent when their orbits get small on screen, and fast orbits are drawn as exposure rings. Corvane is the test system.

*Scope:* scale, clock, markers, units in readouts, ship and station symbols, inspection, zoom, camera, satellite grouping, the motion rule and the Corvane test system. *Source ideas:* `docs/ideas/presentation/presentation.md` (section 2), `docs/ideas/presentation/presentation-sketches.md` (boards 1, 2, 5, 6), `docs/ideas/presentation/symbology.md` (symbols), mock-ups in `docs/ideas/presentation/mockups/`. Where a mock-up and this spec disagree, this spec wins.

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables (table at the end), read from `src/view/tunables.rs`, never hard-coded.

*Retired IDs* (never reuse): CAM-21, CLK-03, ZOOM-03, ZOOM-04, ZOOM-06, ZOOM-07, ZOOM-08, ZOOM-09, ZOOM-12, MAG-01 to MAG-08, MOT-03, MOT-05, MOT-06, MOT-09, CAM-09.

---

## 1. Data and scale

| ID | Requirement | Built |
|---|---|---|
| SCL-01 | Simulation data is stored in real units (km, kg, days) as `f64`, and converted to render space in one place. | ✓ |
| SCL-02 | Positions and orbits are drawn at true scale, with one linear scale for planetary and satellite orbits alike. Only markers are stylised, never geometry. | ✓ |
| SCL-03 | The system is data (a tree of body records) that a generator can replace without changing rendering code. | ✓ |
| SCL-04 | Orbits are circular and coplanar, with periods from Kepler's third law using the parent's mass. | ✓ |
| SCL-05 | Each body record holds its mean physical radius in km. | ✓ |

## 2. Clock and top bar

| ID | Requirement | Built |
|---|---|---|
| CLK-01 | The game clock runs continuously at a default of 1 game day per real second. | ✓ |
| CLK-02 | The top bar shows the breadcrumb (CAM-25), the calendar date, and Pause, 1 d/s and 5 d/s buttons; the active rate is highlighted. | ✓ |
| CLK-05 | Space toggles pause; `.` doubles and `,` halves the rate (minimum 1/16 d/s). | ✓ |
| CLK-04 | Pausing stops all orbital motion; every view stays fully interactive while paused. | ✓ |
| CLK-06 | A debug readout of frames per second can show in the top bar. It is hidden by default and toggled by a binding (default F3). | |

## 3. Markers and palette

| ID | Requirement | Built |
|---|---|---|
| MRK-01 | Palette: ground #070A0E; neutral bodies and text #DCE6EC; orbits and secondary text #9FB6C4; own assets #5AA9E6 (blue); warnings #F0A040 (orange); selection and hover as a white ring. Ships, groups and stations take their colour from their affiliation (SYM-02). | ✓ |
| MRK-02 | Type: Barlow for labels and UI; JetBrains Mono for dates, figures and status readouts. | ✓ |
| MRK-03 | Each object is a fixed-size screen marker by class, projected from its 3D position each frame; marker size never depends on physical size. | ✓ |
| MRK-04 | Marker shapes: filled disc = body; X = salvage site. Ships, groups of ships and stations are drawn as symbols (section 3a). | ✓ |
| MRK-05 | Orbits are faint rings; the belt is a translucent shaded band. A ring stays within half a pixel of the true circle, also when the camera is close to it, and a ring passing behind the camera is cut at the near limit, not dropped. | ✓ |
| MRK-06 | Separate planets are never merged, however close on screen. | ✓ |
| MRK-08 | Every marker and symbol has a hit area of at least `min_hit_radius_px` round its centre, even where the drawn marker is smaller. Where hit areas overlap, the marker whose centre is nearest the pointer wins. | ✓ |
| MRK-07 | A body whose true diameter on screen is larger than its marker is drawn as a filled disc at its true size, in its marker's colour, in place of the marker. | ✓ |
| EDGE-01 | A gate outside the viewport is shown as an arrow pinned to the edge of the map area (below the top bar) in its direction, labelled with its destination. A gate behind the camera is pinned by its direction in the camera's own right and up axes. Arrows slide round the edge as the heading turns. | ✓ |

## 3a. Ship and station symbols

Ships and stations are drawn as symbols adapted from NATO APP-6: the **frame** shape shows affiliation, the **icon** inside shows what it is, and **amplifiers** around the frame show its state. Reference: mock-up `7-symbology.png`, except the installation role icons (naval base, shipyard, habitat, industrial site), which are not specced.

**Affiliation** is how the state sees an object: own, allied, neutral, hostile or unknown. **Command** is whether the state gives it orders. An **own-flag civilian** ship is own (registered to one of the state's organisations) but not commanded. A **group** is several ships drawn as one symbol.

| ID | Requirement | Built |
|---|---|---|
| SYM-01 | Each ship, group and station holds in simulation data what the state knows of it: affiliation; whether the state commands it; class (warship, fleet supply, cargo, troop transport, station, or unknown); whether it is known to be armed; for a troop transport, its load (unknown, known empty, or embarked formations, each with a name and an echelon that may be unknown); its track state (firm, weak, or stale with the date last observed); and, for own ships, a condition from 0 to 1. Symbols are drawn from these values only. | ✓ |
| SYM-02 | The frame shape and colour follow affiliation: own = circle, blue #5AA9E6 (own-flag civilian: paler blue #A9CBE3); allied and neutral = square, green #8CCB8A; hostile = diamond, red #EE6A5F; unknown = quatrefoil, orange #F0A040. The outline is `symbol_stroke_px` wide. | ✓ |
| SYM-03 | Fill shows command: a commanded own ship, group or station has a tint of its colour at `symbol_fill_state`; an own-flag civilian has no fill; every other affiliation has a tint at `symbol_fill_other`. When the state takes command of a civilian ship (requisition or charter) it is drawn in the state blue with the state fill, and returns to the civilian look when command ends. | ✓ |
| SYM-04 | A symbol's frame fits a square of `ship_symbol_px` on screen at every zoom (MRK-03). Amplifiers and labels sit outside that square. | ✓ |
| SYM-05 | The icon inside the frame, in the frame's colour: warship = filled triangle; fleet supply or tanker = outline drop; cargo = outline crate (a box with a horizontal centre line); troop transport = outline unit box (a landscape rectangle); unknown class = "?" in JetBrains Mono; station = no icon. A ship known to be armed shows the warship icon whatever its class, so filled always means armed. | ✓ |
| SYM-06 | A troop transport shows what is known of its load. Unknown: empty box, and "?" in the echelon slot above the frame. Known empty: empty box, nothing above. Embarked: the box with both diagonals drawn (the infantry cross), and the echelon of the largest embarked formation above the frame (II battalion, III regiment, X brigade), or "?" if its size is unknown. | ✓ |
| SYM-07 | A station has its affiliation frame with a small filled bar centred on top of the frame (the NATO installation bar), and no icon. | ✓ |
| SYM-08 | Track state: a firm track has a solid frame; a weak track has a dashed frame; a stale track has a dashed frame, is drawn at `stale_symbol_alpha` opacity, and its label reads "last seen *N* d ago" in whole days of game time. Affiliation, class, load and track state each change the symbol independently as the state learns more. | ✓ |
| SYM-09 | A group is drawn as one symbol, and its members are not drawn individually. All members share the group's affiliation and command. The icon is that of its most significant member (warship, then troop transport, then fleet supply, then cargo, then unknown), and the member count is shown in JetBrains Mono at the frame's upper right. A **task force** (members under one commander) has a bracket above the frame, above any echelon. A **loose group** (moving together with no formation) has a second copy of the frame offset up and right behind it. | |
| SYM-10 | Two groups are never merged into one symbol, even at the same position; an escorted convoy is a loose group and a task force. | |
| SYM-11 | A ship or group in transit has a velocity leader: a line from the edge of the frame in the on-screen direction of its velocity relative to the star. Its length rises with the logarithm of speed from `leader_min_px` at `leader_speed_min_kms` to `leader_max_px` at `leader_speed_max_kms`, clamped at both ends; below `leader_speed_min_kms`, and for stale tracks, there is no leader. Orbiting and parked ships have no leader. | |
| SYM-12 | Own ships and own groups (commanded or civilian) have a condition bar under the frame, `ship_symbol_px` wide and 3 px high, filled to their condition in the symbol's colour, or in orange below `condition_warn_fraction`. A group shows the mean of its members. | |
| SYM-13 | A symbol's label sits to the right of it, in the symbol's colour: the name in Barlow, then a second line in JetBrains Mono in the secondary text colour summarising the parts that apply, joined by " · ": a group's member count by class ("4 warships"); embarked formations ("1st Arden Guard"); "own flag · civilian" or "requisitioned"; and the stale age (SYM-08). With none of these, there is no second line. | ✓ |
| SYM-14 | Hover and click target a symbol through its `ship_symbol_px` square. The hover ring, the selection ring and the frame brackets (CAM-22) are sized to enclose that square, not the amplifiers. | ✓ |
| SYM-15 | Ownership text in tooltips and insets uses: "own", "own flag · civilian", "requisitioned", "allied", "neutral", "hostile" or "unknown". | ✓ |

## 4. Hover and inspection

| ID | Requirement | Built |
|---|---|---|
| HOV-01 | Hovering a marker draws a white ring and a tooltip: name · ownership · contents summary — "Click: inspect · Double-click: frame its moons · Double-click again: planetary view". | ✓ |
| HOV-02 | Every drawn orbit ring (dot mode or ring mode, MOT-01 and MOT-02) is a hover and click target for its object, as its marker is. A pointer within `ring_hit_px` of the ring hits it. A marker, pill, label or panel under the pointer wins over a ring, and of two rings in reach the nearer wins. Clicking a ring selects as clicking the marker does (INS-01); double-clicking it behaves as ZOOM-16 and ZOOM-17. The hover tooltip (HOV-01) sits by the pointer, not by the object. Folded satellites have no drawn orbit and are reached through the pill (GRP-07). | ✓ |
| HOV-03 | A hovered object's orbit ring is drawn in white at `orbit_hover_alpha`; the selected object's orbit ring is drawn in full white for as long as it is selected, in either motion mode. A ring that is both takes the selected style. Hovering or selecting the marker highlights its ring the same way. | ✓ |
| INS-01 | Clicking a marker or its orbit ring (HOV-02) selects that object and opens the inspection inset (INS-07); the clock keeps running. Only the primary button selects, and only when the release is within `drag_threshold_px` of the press, so the end of a pan never selects. | ✓ |
| INS-02 | The inset shows: header (name, class and ownership line), Focus and Close buttons; an image for the knowledge state (INS-04); key facts; and an "Open planetary view" button, disabled until the planetary view exists. | ✓ |
| INS-03 | There is at most one selection and one inset. Selecting another object replaces the inset's contents in place. Close, Esc and a click on empty map space (a press and release within `drag_threshold_px` that hits no marker, pill, ring or panel) clear the selection and close the inset. | ✓ |
| INS-04 | The inset image is a placeholder for the knowledge state: **surveyed** = clean render; **unsurveyed** = grainy telescope frame. There is no status text or date, and no stale state, until information gathering is designed. | ✓ |
| INS-05 | The inset of a body with satellites has an "In orbit" list: one entry per satellite (its symbol per section 3a at `list_symbol_px`, or the body disc, and its name), in orbit order from the inside out. Clicking an entry selects that satellite (its inset replaces the parent's) once `double_click_window_ms` passes without a second click; double-clicking it behaves as ZOOM-16 and ZOOM-17. | ✓ |
| INS-07 | The inset is docked at the right edge of the map area, `inset_dock_margin_px` below the top bar and from the right edge, `inset_width_px` wide. It does not move while the selection, camera or clock changes. Content taller than the map area scrolls inside the inset. There is no leader line. | ✓ |
| INS-08 | A body's inset lists, in its key facts: **Orbit** (distance to its parent, UNIT-01) and **Period** (UNIT-02); for the star, **Mass** in solar masses to 2 decimals instead; and **Mean radius** (UNIT-01, from SCL-05). | |
| INS-06 | When the selected object is folded into its parent (GRP-01), the parent's pill carries the white selection ring. When the object unfolds, the ring moves to its own marker without a jump. | ✓ |

## 4a. Units in readouts

Shared formatting for every readout in the game, not tied to one panel. Figures are in JetBrains Mono (MRK-02).

| ID | Requirement | Built |
|---|---|---|
| UNIT-01 | A distance is shown in km below `distance_au_threshold_au`, and in AU at or above it, to 3 significant figures, with thousands separators (for example "850 km", "182,000 km", "0.362 AU", "12.4 AU"). | |
| UNIT-02 | A duration is shown in the largest unit it reaches: whole minutes ("42 min"), or hours, days or years of 365.25 days to one decimal place ("5.3 h", "31.2 d", "2.4 y"). | |
| UNIT-03 | Distances and durations are formatted by one shared module that any panel calls; no panel formats them itself. *Checked in review.* | |

## 5. Navigation and zoom

| ID | Requirement | Built |
|---|---|---|
| ZOOM-01 | Each wheel notch multiplies the view scale by `zoom_step_factor`, animated over `zoom_anim_ms` with ease-out. Raw wheel deltas are never used: line-unit input counts notches, and pixel-unit input is converted at 20 px per notch, rounded towards zero. | ✓ |
| ZOOM-02 | New input during a zoom animation retargets from the current scale; it is not queued. | ✓ |
| ZOOM-05 | A zoom step that lands on a stop takes longer by `zoom_stop_resistance`, then settles exactly on the stop. | ✓ |
| ZOOM-10 | Home returns to the system fit and the standard orientation (CAM-02) with the same animated move, and sets the frame to the star. | ✓ |
| ZOOM-11 | Ctrl+1 to Ctrl+4 save a bookmark (scale, centre and frame, CAM-06); 1 to 4 recall it with the same animated move. | ✓ |
| ZOOM-13 | Zoom is anchored on the frame body when the frame is not the star (CAM-05); in the star's frame, on the point where the cursor's ray meets the orbital plane, or the screen centre for keyboard zoom (CAM-16). Selection never sets the anchor. The anchor keeps its screen position through the zoom. The anchored pivot is clamped to the pan bound (CAM-19); a ray that misses the plane anchors on the pivot. | ✓ |
| ZOOM-14 | Hard limits: outer = system fit. Inner, in a body's frame other than the star's: the scale at which that body's true diameter spans `body_max_screen_fraction` of the map area's height. Inner, in the star's frame: a view radius of `min_view_radius_au`. Scrolling past a limit does nothing and never changes layer. | ✓ |
| ZOOM-15 | There are two stops, computed per system from its real orbits: the **system fit** (all planetary orbits fit inside the map area, on both axes and in perspective, at the current tilt; gates stay as edge indicators) and, while the frame body is not the star and has satellites, its **satellite fit** (all its satellites fit). There are no other stops. | ✓ |
| ZOOM-16 | Double-click on an object that is not the frame body selects it, makes it the frame (CAM-06) and moves the view in one pan-and-zoom of `focus_jump_ms`: to its satellite fit if it has satellites; otherwise its parent's satellite fit if it orbits a body; otherwise the inner limit (ZOOM-14). Double-click on the star selects it, sets the star's frame and moves to the system fit. F does the same for the current selection, and the inset's Focus button for the inset's object. An object that can't be a frame (CAM-24) is only selected. | ✓ |
| ZOOM-17 | Double-click on the frame body, other than the star, selects it and is the planetary-view trigger. Until that view exists it calls a clearly named hook and does nothing visible. | ✓ |

## 5a. Camera

Terms used below:
- **Pivot:** the point on the orbital plane the camera looks at, from a distance set by the zoom scale.
- **Heading:** rotation about the plane's normal. **Tilt:** 0° is looking straight down.
- **North:** a fixed direction in the orbital plane, the same for every system.
- **Selection** (INS-01): the object whose details are shown. It never moves the camera.
- **Reference frame** (or **frame**): the object whose non-rotating frame the camera sits in; the camera tracks it as it moves. There is always exactly one. By default it is the star, which doesn't move, so the star's frame behaves as a free camera. The **frame body** is that object. The player-facing verb for moving into an object's frame is "Focus" (F, the inset's Focus button).
- **Ancestors** of an object: its parent, its parent's parent, and so on up to the star.

Selection and frame may be different objects: for example, the frame is Halden IV while IVc is selected.

| ID | Requirement | Built |
|---|---|---|
| CAM-01 | Heading is unlimited. Tilt is clamped between 0° and `max_pitch_deg`; the camera never goes below the orbital plane. | ✓ |
| CAM-02 | The **standard orientation** is north at the top of the screen and tilt `standard_tilt_deg`. The view starts in it. | ✓ |
| CAM-03 | N animates to the standard orientation (CAM-02) over `focus_jump_ms`, keeping the pivot and the scale. | ✓ |
| CAM-04 | Home (ZOOM-10) returns to the standard orientation in the same animated move. | ✓ |
| CAM-05 | The pivot is recomputed every frame as the frame body's position plus a pan offset, so the frame body keeps its place on screen at any time rate. The offset does not rotate with the body's orbit. The pivot and the markers use the same frame's positions. | ✓ |
| CAM-06 | The frame is changed only by double-click, F or the inset's Focus button (ZOOM-16), by a breadcrumb entry (CAM-25), each with zero offset; by recalling a bookmark; and as CAM-08 lists. Zooming never changes it. A bookmark (ZOOM-11) stores heading, tilt, the frame and its offset; recalling it restores all of them. | ✓ |
| CAM-07 | Panning changes the offset and never changes the frame. Outside the star's frame, the offset is clamped so the frame body stays at least `focus_keep_margin_px` inside the map area. | ✓ |
| CAM-08 | The frame returns to the star on: Home; a zoom that reaches the system fit (CAM-23); and recalling a bookmark saved in the star's frame. It moves to an ancestor through the breadcrumb (CAM-25) or CAM-24. Nothing else changes it: not selection, Esc, rotating, panning or zooming inside the system fit. | ✓ |
| CAM-10 | Left drag pans: the point on the orbital plane under the cursor when the drag starts stays under the cursor throughout, with no inertia after release. A press and release that moves less than `drag_threshold_px` is a click, never a pan. A press that starts on the top bar or an inset never pans. Shift+left drag is reserved and does nothing. | ✓ |
| CAM-11 | Middle drag pans exactly as CAM-10. | ✓ |
| CAM-12 | Right drag rotates about the pivot: horizontal movement changes heading and vertical movement changes tilt, both at `rotate_deg_per_px`, with tilt clamped (CAM-01). Dragging left turns the camera anticlockwise seen from above (the scene turns clockwise on screen); dragging down tilts towards the horizon. | ✓ |
| CAM-13 | A right press and release that moves less than `drag_threshold_px` is a right-click. It is reserved for context menus and orders: until those exist it calls a clearly named hook and does nothing visible. | ✓ |
| CAM-14 | W, A, S, D and the arrow keys pan relative to the screen (up = towards the top of the screen, along the plane) at `key_pan_screen_per_s` of the visible height at the pivot per second, so the speed looks the same at every zoom and tilt. Movement ramps up and down over `key_pan_ramp_ms`. Holding Shift multiplies the speed by `key_pan_fast_factor`. | ✓ |
| CAM-15 | Q and E change heading (Q as dragging left, E as dragging right), Page Up tilts towards the horizon and Page Down towards straight down, all at `key_rotate_deg_per_s`, with tilt clamped (CAM-01). | ✓ |
| CAM-16 | + (also = and numpad +) zooms in one step and − (also numpad −) zooms out one step, exactly as a wheel notch (ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-14), anchored per ZOOM-13. | ✓ |
| CAM-17 | Edge scrolling: when `edge_scroll_enabled` is true, a cursor within `edge_scroll_px` of a window edge pans that way at the CAM-14 speed. The default is off. | ✓ |
| CAM-18 | Every camera and clock key and mouse button is read from one bindings resource; no system hard-codes a key. Checked in review, not by a test. (A rebinding screen is not yet specced.) | ✓ |
| CAM-19 | The pivot never moves further than `pan_bound_factor` × the system fit radius from the star, in any frame. | ✓ |
| CAM-20 | Pan or rotate input during an animated move (ZOOM-10, ZOOM-11, ZOOM-16, CAM-03) ends the move and takes over from the camera's current state, with no jump. | ✓ |
| CAM-23 | A zoom-out step that reaches the system fit also sets the star's frame, and eases the pivot to the star and the tilt to `standard_tilt_deg`, within the same zoom animation. The heading is kept. | ✓ |
| CAM-24 | An object in exposure-ring mode (MOT-02) can't be a frame, and the inset's Focus button is disabled while it is in that mode. If the frame body enters ring mode, the frame moves to its nearest ancestor not in ring mode (the star if none) and the view moves as ZOOM-16 would for it, or to the system fit for the star. The frame never moves back when the object leaves ring mode. | ✓ |
| CAM-25 | The breadcrumb in the top bar reads "Cluster / *system name*", then the frame body's ancestors below the star, then the frame body: for example "Cluster / Corvane / Halden IV / Arden Highport". The system entry stands for the star's frame. Clicking an ancestor's entry makes it the frame and moves the view as ZOOM-16 would (the system entry: to the system fit). Clicking the frame body's own entry re-centres it with zero offset at its current scale. The Cluster entry is unchanged until the cluster view exists. | ✓ |
| CAM-22 | The frame body's marker, unless it is the star, carries four corner brackets in the neutral colour (MRK-01), so the frame is never mistaken for the white selection ring. The bracket box encloses the selection ring's size (whether or not the ring is showing) with `focus_bracket_margin_px` to spare on each side. | ✓ |

## 6. Satellite grouping

A **satellite** is anything orbiting a body other than the star: moons, stations, ships parked in orbit. Its **parent** is the body it orbits. Moons of moons follow the same rules. Ships in transit are not satellites.

| ID | Requirement | Built |
|---|---|---|
| GRP-01 | A satellite is drawn individually only while its orbit's on-screen radius is at least `unfold_at_px`; below `fold_below_px` it folds into its parent and its marker, label, ring and trail are hidden. Between the two it keeps its current state. | ✓ |
| GRP-02 | The test is per satellite and re-evaluated as zoom changes, so outer satellites unfold before inner ones. | ✓ |
| GRP-03 | Folding and unfolding animate the satellite into or out of its parent over `fold_unfold_anim_ms`, including during zoom animations. | ✓ |
| GRP-04 | At the system fit every moon and planetary station is folded. | ✓ |
| GRP-05 | A parent with folded satellites shows a "+N" pill beside its marker without overlapping it. | ✓ |
| GRP-06 | The pill's outline takes the colour (SYM-02) of the most pressing affiliation among its folded satellites: hostile, then unknown, then neutral or allied; with none of these it keeps its normal outline. A small blue dot shows if own ships are parked there. | ✓ |
| GRP-07 | Hovering the pill lists the folded satellites; clicking it does exactly what clicking the parent does. | ✓ |
| GRP-08 | Ships in transit are never folded. | ✓ (rule only; no transit ships yet) |
| GRP-09 | A group (SYM-09) folds as one satellite: it counts once in the "+N" pill and is listed by name with its member count. | |

## 7. Motion rule

The **apparent period** of an orbit is its orbital period divided by the selected time rate, in real seconds. Pausing does not change it: the selected rate is kept while paused (CLK-05).

| ID | Requirement | Built |
|---|---|---|
| MOT-01 | Apparent period at or above `tail_secs`: the object is a dot at its true position, and its orbit ring carries a tail behind it. The tail is a brightness gradient along the ring itself, one continuous anti-aliased stroke, running from the exposure-ring brightness (MOT-02) at the dot down to the ring's normal alpha at the tail's end. | ✓ |
| MOT-10 | The tail covers the arc swept in the last `tail_secs` real seconds: 360° × `tail_secs` ÷ apparent period. Its length follows the time rate continuously, with no steps. | ✓ |
| MOT-02 | While running, apparent period below `tail_secs`: the tail has closed into the exposure ring. No dot; the whole ring is at full exposure brightness and `exposure_ring_width_px` wide; the label sits on the ring at a fixed spot. | ✓ |
| MOT-11 | As the tail grows from `dot_fade_from_degrees` to 360°, the dot fades out and the ring width grows to `exposure_ring_width_px`, both linearly, so the closing is seamless. | ✓ |
| MOT-04 | Pausing keeps every tail at its running length, so the view does not jump. | ✓ |
| MOT-12 | While paused, each tail is capped at `dot_fade_from_degrees`, so every orbiting object shows as a full-brightness dot with its tail, objects in exposure-ring mode included. Unpausing restores the running state. | ✓ |
| MOT-07 | While the selected object is in ring mode, its inset has the line "Position: shown when paused". | ✓ |
| MOT-08 | The rule applies to every orbiting object (moons, stations, parked ships, close-in planets); ships in transit are ignored. | ✓ |

## 8. Test content: Corvane

The prototype system, kept as data (SCL-03). It exists to exercise the rules above, so its values serve testing first. It is also a plausible output of the generator, so its bodies use realistic masses and spacings (TEST-07, TEST-08). Where a test needs it, a body may be less typical than the generator's usual output, but never implausible.

| ID | Requirement | Built |
|---|---|---|
| TEST-01 | Corvane contains at least: the star; planets Pell, Iver (with Iver Yard station), Halden IV (capital, own) and Morrow (outer giant with many moons); a belt with salvage sites Wreck Field and Lantern; and off-screen gates. | ✓ |
| TEST-02 | Halden IV's satellites have periods of about: Arden Highport 0.1 d, IVa 1.5 d, IVb 4.2 d, IVc 31 d, IVd 68 d, so both motion modes show at 1 d/s. Radii follow Kepler from Halden IV's mass. | ✓ |
| TEST-03 | Halden IV also has contact K-3 (unknown affiliation, unknown class, weak track, slow orbit) and one own commanded warship parked in orbit, as static orbiting objects. | ✓ |
| TEST-05 | Corvane has, as static orbiting objects in slow orbits (dot mode at 1 d/s), at least one of each: own fleet tanker; own troop transport with 1st Arden Guard (battalion) embarked; own empty troop transport; own-flag civilian ore carrier (Varga Mining); requisitioned civilian ore carrier; own-flag civilian armed hauler; neutral trader; allied warship; hostile troop transport with unknown load; hostile raider on a stale track last seen 6 days before the start date; unknown-affiliation contact with known class cargo. Iver Yard and Arden Highport are own stations. | ✓ |
| TEST-06 | Corvane also has, as static orbiting objects: own task force TF-1 (4 warships, one damaged so the group's mean condition is below `condition_warn_fraction`); and a loose group of 6 own-flag civilian haulers. | |
| TEST-04 | The inspection data covers both knowledge states: Halden IV surveyed, Halden IVc unsurveyed. | ✓ |
| TEST-07 | Morrow has the mass of a Jupiter-class gas giant: 318 Earth masses. | ✓ |
| TEST-08 | Morrow's eleven moons have periods of about 0.35, 0.55, 0.9, 1.8, 3.6, 7.2, 16, 30, 50, 75 and 110 d. Radii follow Kepler from Morrow's mass, and the innermost is no closer than 2 × Morrow's radius (71,500 km). The periods straddle both motion modes at 1 d/s: six moons in ring mode (under 8 s), five in dot mode (8 s or more). | ✓ |

## Tunables

All in the `Tunables` resource (`src/view/tunables.rs`). Defaults are starting values; changing one is a spec change only if the default is being settled.

| Tunable | Default | Used by |
|---|---|---|
| `unfold_at_px` (R_show) | 24 px | GRP-01 |
| `fold_below_px` | 20 px | GRP-01 |
| `fold_unfold_anim_ms` | 200 | GRP-03 |
| `tail_secs` | 8 s | MOT-01, MOT-02, MOT-10 |
| `dot_fade_from_degrees` | 300° | MOT-11, MOT-12 |
| `exposure_ring_width_px` | 3 | MOT-02 |
| `zoom_step_factor` | ×2 | ZOOM-01 |
| `zoom_anim_ms` | 200 (range 150–250) | ZOOM-01 |
| `zoom_stop_resistance` | 0.35 | ZOOM-05 |
| `focus_jump_ms` | 400 | ZOOM-10, ZOOM-11, ZOOM-16, CAM-03 |
| `zoom_fit_margin` | 0.8 | ZOOM-15 |
| `min_view_radius_au` | 0.05 AU | ZOOM-14 |
| `body_max_screen_fraction` | 0.25 | ZOOM-14 |
| `standard_tilt_deg` | 60° | CAM-02, CAM-23 |
| `focus_keep_margin_px` | 40 px | CAM-07 |
| `double_click_window_ms` | 350 | INS-05 |
| `max_pitch_deg` | 85° | CAM-01, CAM-12, CAM-15 |
| `drag_threshold_px` | 4 px | INS-01, CAM-10, CAM-13 |
| `rotate_deg_per_px` | 0.25 | CAM-12 |
| `key_pan_screen_per_s` | 0.6 | CAM-14, CAM-17 |
| `key_pan_ramp_ms` | 120 | CAM-14 |
| `key_pan_fast_factor` | 3 | CAM-14 |
| `key_rotate_deg_per_s` | 90 | CAM-15 |
| `edge_scroll_enabled` | false | CAM-17 |
| `edge_scroll_px` | 8 px | CAM-17 |
| `pan_bound_factor` | 1.5 | CAM-19 |
| `focus_bracket_margin_px` | 2 px | CAM-22 |
| `min_hit_radius_px` | 10 px | MRK-08 |
| `inset_width_px` | 320 px | INS-07 |
| `inset_dock_margin_px` | 12 px | INS-07 |
| `ring_hit_px` | 7 px | HOV-02 |
| `orbit_hover_alpha` | 0.6 | HOV-03 |
| `distance_au_threshold_au` | 0.01 AU | UNIT-01 |
| `ship_symbol_px` | 22 px | SYM-04, SYM-12, SYM-14 |
| `list_symbol_px` | 14 px | INS-05 |
| `symbol_stroke_px` | 1.6 px | SYM-02 |
| `symbol_fill_state` | 0.30 | SYM-03 |
| `symbol_fill_other` | 0.16 | SYM-03 |
| `stale_symbol_alpha` | 0.55 | SYM-08 |
| `leader_min_px` | 6 px | SYM-11 |
| `leader_max_px` | 24 px | SYM-11 |
| `leader_speed_min_kms` | 0.5 km/s | SYM-11 |
| `leader_speed_max_kms` | 50 km/s | SYM-11 |
| `condition_warn_fraction` | 0.5 | SYM-12 |

## Open points

- Input targets mouse and keyboard only. Gamepad, touch and trackpad are not targets, but designs should avoid what would be hard to port to them (for example, relying on hover alone or on raw wheel deltas).
- Per-system camera memory, a frame that rotates with its body, breadcrumb drop-downs, selection cycling (Tab), and gamepad and touch input: in `docs/ideas/presentation/presentation.md` (Camera), not yet specced.

## Not yet specced

From the ideas, not to be built until specced: installation role icons, data blocks beyond SYM-13, layer toggles, alerts, legend, Administration button, inspection fidelity for foreign contacts (thermal track), ships in transit and trajectories, uncertainty ellipses, the planetary view and its transition, the cluster view, gate distance labels, and every open question in `docs/ideas/presentation/presentation.md` section 6.

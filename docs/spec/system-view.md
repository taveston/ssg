# Spec: System view

**In short:** one star system drawn as a vector display at true scale, with fixed-size markers; ships and stations use NATO-style symbols. Clicking an object, or its orbit ring, selects it and opens the inspection inset, docked at the right of the map; there is one selection and one inset; the camera always sits in the reference frame of one object, the star by default, shown as a path in the breadcrumb. Zoom is stepped, with two stops: the whole system and the frame body's satellites. The frame is only ever changed deliberately. Satellites fold into their parent when their orbits get small on screen, and fast orbits are drawn as exposure rings. Several things at one place on screen, whether folded, docked or merely overlapping, share one multi-marker: a frame with a count and a second frame behind it, which opens a tray to pick one of them. A dotted crosshair in the orbital plane, centred on the frame body, shows north and the frame of reference. Corvane is the main test system; Sol, built from real figures, is a second (`docs/spec/sol.md`).

*Scope:* scale, clock, buttons, markers, labels, the frame cue, units in readouts, ship and station symbols, inspection, zoom, camera, satellite grouping, several objects on one marker, the motion rule and the Corvane test system. *Source ideas:* `docs/ideas/presentation/presentation.md` (section 2), `docs/ideas/presentation/presentation-sketches.md` (boards 1, 2, 5, 6), `docs/ideas/presentation/symbology.md` (symbols; section 4 for the multi-marker), mock-ups in `docs/ideas/presentation/mockups/`. Where a mock-up and this spec disagree, this spec wins.

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables (table at the end), read from `src/view/tunables.rs`, never hard-coded.

*Retired IDs* (never reuse): CAM-21, CLK-03, ZOOM-03, ZOOM-04, ZOOM-06, ZOOM-07, ZOOM-08, ZOOM-09, ZOOM-12, MAG-01 to MAG-08, MOT-03, MOT-05, MOT-06, MOT-09, CAM-09, CAM-22, GRP-06, SYM-11.

---

## 1. Data and scale

| ID | Requirement | Built |
|---|---|---|
| SCL-01 | Simulation data is stored in real units (km, kg, days) as `f64`, and converted to render space in one place. | ✓ |
| SCL-02 | Positions and orbits are drawn at true scale, with one linear scale for planetary and satellite orbits alike. Only markers are stylised, never geometry. | ✓ |
| SCL-03 | The system is data (a tree of body records) that a generator can replace without changing rendering code. | ✓ |
| SCL-04 | Orbits are circular and coplanar, with periods from Kepler's third law using the parent's mass. | ✓ |
| SCL-05 | Each body record holds its mean physical radius in km. | ✓ |
| SCL-07 | A body record carries its **named orbits** (each a name and a radius in km). A ship or station in orbit holds a reference to the orbit it occupies: one of its parent's named orbits (a station is placed in a band exactly as a ship is, and a player-built one can be placed nowhere else). It carries no radius of its own. Natural bodies each have an orbit of their own and never share one: the generator keeps them apart (`docs/ideas/mechanics/world-generator.md`). | ✓ |
| SCL-06 | Render positions are taken relative to the frame body: its position is subtracted from each object's in `f64` km before conversion to render space. So objects near the frame body keep full precision however far it is from the star, and a framed body and its satellites hold still on screen at any zoom. | ✓ |

## 2. Clock and top bar

| ID | Requirement | Built |
|---|---|---|
| CLK-01 | The game clock runs continuously at a default of 1 game day per real second. | ✓ |
| CLK-02 | The top bar shows the breadcrumb (CAM-25), the calendar date, and Pause, 1 d/s and 5 d/s buttons. The active rate is a primary button and the others are secondary (BTN-01). | ✓ |
| CLK-05 | Space toggles pause; `.` doubles and `,` halves the rate (minimum 1/16 d/s). | ✓ |
| CLK-04 | Pausing stops all orbital motion; every view stays fully interactive while paused. | ✓ |
| CLK-06 | A debug readout of frames per second can show in the top bar. It is hidden by default and toggled by a binding (default F3). | ✓ |

## 2a. Buttons

| ID | Requirement | Built |
|---|---|---|
| BTN-01 | A button has one of three styles: **primary** = filled in the neutral colour with ground-colour text; **secondary** = outlined `button_stroke_px` wide in the secondary colour, with neutral text; **quiet** = text only, in the secondary colour. A panel or bar has at most one primary button. | ✓ |
| BTN-02 | Buttons never use affiliation colours (SYM-02) or the warning orange, except that a button whose action can't be undone is outlined in orange. | ✓ |

## 3. Markers and palette

| ID | Requirement | Built |
|---|---|---|
| MRK-01 | Palette: ground #070A0E; natural bodies and text #DCE6EC; orbits and secondary text #9FB6C4; own ships, groups and stations #5AA9E6 (blue); warnings #F0A040 (orange); the current frame #9B8CE8 (violet): the frame cue (REF-01) and the breadcrumb entry for the frame body (CAM-25), used for nothing else. Ships, groups and stations take their colour from their affiliation (SYM-02). | ✓ |
| MRK-02 | Type has three roles. The **label face** is for names on the map, labels, buttons, headings and the breadcrumb; the **text face** is for running prose (report and request bodies, descriptions); the **data face** is for dates, figures, status readouts and stamps. Every requirement names a role, never a font. The font behind each role, and its weights, is named once, in the type roles table below; no other code names a font file. | ✓ |
| MRK-03 | Each object is a fixed-size screen marker by class, projected from its 3D position each frame; marker size never depends on physical size. Natural bodies are drawn as body markers (MRK-13) of `star_marker_px`, `planet_marker_px` and `moon_marker_px` across. | ✓ |
| MRK-04 | Marker shapes: body marker (MRK-13) = star, planet or moon; X = salvage site. Ships, groups of ships and stations are drawn as symbols (section 3a). | ✓ |
| MRK-05 | Orbits of natural bodies are faint rings in the orbit colour (MRK-01) at `orbit_ring_alpha`, `orbit_ring_width_px` wide; the belt is drawn as MRK-18 says. A ring stays within half a pixel of the true circle, also when the camera is close to it, and a ring passing behind the camera is cut at the near limit, not dropped. | ✓ |
| MRK-09 | Ships and stations draw no orbit ring of their own in dot mode (MOT-01): only their tail, which fades to transparent at its end rather than to a ring's alpha. Their full ring is drawn only while the object is hovered or selected (HOV-03). In exposure-ring mode (MOT-02) the exposure ring is drawn as usual. | ✓ |
| MRK-10 | The star, planets and moons are drawn and labelled in the neutral colour (MRK-01) whoever owns them. A body's ownership is not shown on the map. | ✓ |
| MRK-06 | Separate planets are never merged, however close on screen. | ✓ |
| MRK-08 | Every marker and symbol has a hit area of at least `min_hit_radius_px` round its centre, even where the drawn marker is smaller. Where a body is drawn as a true-size disc (MRK-07), its hit area is at least the disc, out to the edge of its rim (MRK-11). Where hit areas overlap, the marker whose centre is nearest the pointer wins. | ✓ |
| MRK-07 | A body whose true diameter on screen is larger than its marker is drawn as a disc at its true size, in the style of MRK-11, in place of the marker. | ✓ |
| MRK-11 | A natural body's true-size disc (MRK-07) is filled with the neutral colour mixed into the ground colour at `body_fill_mix` (an opaque colour, not transparency), has a neutral rim `body_rim_px` wide, and is ringed outside the rim by a gap `body_gap_px` wide in the ground colour, so overlapping discs stay distinct. | ✓ |
| MRK-12 | Draw order, from the bottom: the belt (MRK-18), orbit rings, tails, exposure rings and the frame cue (REF-09); then natural bodies, nearer the camera on top of farther, recomputed as the view turns; then the upper half of the frame cue's vertical axis (REF-06); then ship, group and station symbols and multi-markers (MUL-01), always above bodies and above that axis; then labels and the hover and selection brackets (HOV-06), above all markers; then the tray (MUL-08); then edge arrows and the sidebar. One exception: where a body is drawn as a true-size disc (MRK-07), the part of each of its satellites' orbit rings, tails and exposure rings that lies nearer the camera than the body's centre is drawn above the disc, so a close orbit passes in front of its body rather than behind it on both sides. Draw order never changes which marker a click hits (MRK-08). | ✓ |
| MRK-13 | A body marker is an outline: a circle with an opaque ground-colour inside and a neutral rim `body_rim_px` wide. The class shows in its shape: star = the circle with `star_tick_count` short radial ticks `star_tick_px` long outside the rim; planet = the circle with a neutral centre dot `planet_dot_px` across; moon = the plain circle. | ✓ |
| MRK-14 | Any orbit ring (natural body, ship or station, shared or not), with its tail, fades out as its on-screen radius grows from `orbit_fade_from` to `orbit_fade_to` times half the map area's diagonal, and is not drawn above that, so an orbit too large to read as a ring never crosses the view as a near-straight line. The on-screen radius is the ring's radius at the scale of its point nearest the camera, so a ring the camera sits close to counts as large: near the camera it does cross the view as a near-straight line. The fade multiplies the ring's alpha in every style, hovered and selected included. The same rule applies in every frame; being the frame body changes nothing. | ✓ |
| MRK-15 | Outside the star's frame, while the frame body's parent is outside the viewport, the parent is shown as an edge tab placed and styled as EDGE-01 to EDGE-03, pointing in the parent's direction and labelled with its name. Inside the tab, beside the name, it shows the parent's body marker (MRK-13) at `list_symbol_px`, so it never reads as a gate. While the parent is in the viewport there is no tab. | ✓ |
| MRK-16 | A scale bar sits at the bottom left of the map area, `plot_margin_px` from its edges: a neutral line 1 px wide with ticks at its ends and middle, labelled "0" at the left and its length at the right, in the data face in the secondary colour. Its length on the orbital plane is a round distance (1, 2 or 5 times a power of ten, in km below `distance_au_threshold_au` and in AU at or above it, as UNIT-01), measured along the screen's horizontal at the pivot, and its drawn length is the longest such distance that fits within `scale_bar_max_px`. It updates as the zoom changes. | ✓ |
| MRK-17 | A north dial sits at the top right of the map area, `plot_margin_px` from its edges: a flat, upright circle `north_dial_px` across, outlined 1 px in the secondary colour. Inside it is a filled isosceles triangle in the neutral colour, `north_head_px` long, its tip touching the circle; there is no shaft. An "N" in the data face in the neutral colour sits just outside the circle beyond the tip, and moves round with it while staying upright. The triangle points along the on-screen direction of the orbital plane's north (CAM-02) at the pivot, projected through the camera, so it follows both heading and tilt and always agrees with the map. Clicking the dial does what N does (CAM-03). | ✓ |
| MRK-18 | The belt is a band round the star between its inner and outer radius, in the orbit colour (MRK-01), drawn in two parts. The **band** is a translucent fill, `belt_band_alpha` along its centre line, falling to nothing at both edges as 1 − *u*², where *u* is the distance from the centre line as a fraction of the half-width, so it has no hard edge. The **grain** is `belt_speck_count` points, each 1 px across at `belt_speck_alpha`, scattered across the band with the same falloff from a fixed seed, so the scatter is the same every run and every frame. The points lie in the orbital plane at fixed places in the frame: they do not orbit, are not objects and are never hover or click targets. | ✓ |
| EDGE-01 | A gate outside the viewport is shown as an arrow pinned to the edge of the map area (below the top bar) in its direction, labelled with its destination. A gate behind the camera is pinned by its direction in the camera's own right and up axes. Arrows slide round the edge as the heading turns. | ✓ |
| EDGE-02 | Every edge arrow has a label, and the label is drawn whole inside the map area: it sits on the inner side of its arrow and slides along the edge as needed, never clipped. | ✓ |
| EDGE-03 | An edge arrow and its label sit together as one unit. The label is in a tab: ground-colour fill, a neutral outline `button_stroke_px` wide, and the destination in the neutral colour. The arrow is outside the tab, in the neutral colour, on the tab's side facing the map area's edge, at the pinned point (EDGE-01). When the tab slides along the edge (EDGE-02) the arrow slides with it, so both lie whole inside the map area. | ✓ |

## 3a. Ship and station symbols

Ships and stations are drawn as symbols adapted from NATO APP-6: the **frame** shape shows affiliation, the **icon** inside shows what it is, and **amplifiers** around the frame show its state. Reference: mock-up `7-symbology.png`, except the installation role icons (naval base, shipyard, habitat, industrial site), which are not specced.

**Affiliation** is how the faction sees an object: own, allied, neutral, hostile or unknown. **Command** is whether the faction gives it orders. An **own-flag civilian** ship is own (registered to one of the faction's organisations) but not commanded. A **group** is several ships drawn as one symbol.

| ID | Requirement | Built |
|---|---|---|
| SYM-01 | Each ship, group and station holds in simulation data what the faction knows of it: affiliation; whether the faction commands it; class (warship, fleet supply, cargo, troop transport, station, or unknown); whether it is known to be armed; for a troop transport, its load (unknown, known empty, or embarked formations, each with a name and an echelon that may be unknown); its track state (firm, weak, or stale with the date last observed); and, for own ships, a condition from 0 to 1. A group holds its members, and takes its class, armed status, load and condition from them (SYM-09). Symbols are drawn from these values only. | ✓ |
| SYM-02 | The frame shape and colour follow affiliation: own = circle, blue #5AA9E6 (own-flag civilian: paler blue #A9CBE3); allied and neutral = square, green #8CCB8A; hostile = diamond, red #EE6A5F; unknown = quatrefoil, orange #F0A040. The outline is `symbol_stroke_px` wide. | ✓ |
| SYM-03 | Fill shows command. Every frame has an opaque inside, so no ring, tail or other line beneath shows through it. A commanded own ship, group or station has an inside of its colour mixed into the ground colour at `symbol_fill_faction`; an own-flag civilian's inside is the ground colour; every other affiliation mixes at `symbol_fill_other`. These are opaque colours, not transparency. When the faction takes command of a civilian ship (requisition or charter) it is drawn in the faction blue with the faction fill, and returns to the civilian look when command ends. | ✓ |
| SYM-04 | A symbol's frame fits a square of `ship_symbol_px` on screen at every zoom (MRK-03). Amplifiers and labels sit outside that square. | ✓ |
| SYM-05 | The icon inside the frame, in the frame's colour: warship = filled triangle; fleet supply or tanker = outline drop; cargo = outline crate (a box with a horizontal centre line); troop transport = outline unit box (a landscape rectangle); unknown class = "?" in the data face; station = no icon. A ship known to be armed shows the warship icon whatever its class, so filled always means armed. | ✓ |
| SYM-06 | A troop transport shows what is known of its load. Unknown: empty box, and "?" in the echelon slot above the frame. Known empty: empty box, nothing above. Embarked: the box with both diagonals drawn (the infantry cross), and the echelon of the largest embarked formation above the frame (II battalion, III regiment, X brigade), or "?" if its size is unknown. | ✓ |
| SYM-07 | A station has its affiliation frame with a small filled bar centred on top of the frame (the NATO installation bar), and no icon. | ✓ |
| SYM-08 | Track state: a firm track has a solid frame; a weak track has a dashed frame; a stale track has a dashed frame, is drawn at `stale_symbol_alpha` opacity, and its label reads "last seen *N* d ago" in whole days of game time. Affiliation, class, load and track state each change the symbol independently as the faction learns more. | ✓ |
| SYM-09 | A group is drawn as one symbol, and its members are not drawn individually. All members share the group's affiliation and command. A group's class, armed status, load and condition are not held by it: they come from its members. The icon is the most significant of the icons its members show (SYM-05): warship, then troop transport, then fleet supply, then cargo, then unknown. When that icon is the troop transport, SYM-06 reads the group's troop transports together: embarked if any is known to be embarked (the echelon is that of the largest formation among them), else unknown if the load of any is unknown, else known empty. The member count is shown in the data face to the right of the frame's box (HOV-07), level with the frame's top, `amplifier_gap_px` clear of the box. A **task force** (members under one commander) has a bracket above the frame, its feet `amplifier_gap_px` above the box, or above the echelon if that is higher. The count and the bracket sit outside the box so the selection and hover brackets never cover them. | ✓ |
| SYM-10 | Two groups are never merged into one group, and a group is never split across markers. Two groups at the same position, or whose symbols would overlap, are drawn as one multi-marker holding both (MUL-05). | ✓ |
| SYM-12 | Own ships and own groups (commanded or civilian) have a condition bar under the frame, `ship_symbol_px` wide and 3 px high, starting `amplifier_gap_px` below the frame's box (HOV-07) so the selection and hover brackets never cross it. It is filled to their condition in the symbol's colour, or in orange below `condition_warn_fraction`; the unfilled part is drawn in the same colour mixed into the ground at `condition_track_mix`, so a part-full bar shows its whole length. A group shows the mean of its members. An own ship whose condition is not known has no bar. | ✓ |
| SYM-13 | A symbol's label sits to the right of it, clear of its amplifiers (the count, the echelon, the bracket and the condition bar): `label_gap_px` (LBL-01) is measured from the outer edge of the frame's box (HOV-07) and its amplifiers, not from the frame alone, and other labels keep clear of the amplifiers too. The label is in the symbol's colour: the name in the label face, then a second line in the data face in the secondary text colour summarising the parts that apply, joined by " · ": a group's member count by class ("4 warships"); embarked formations ("1st Arden Guard"); "own flag · civilian" or "requisitioned"; a station's docked count ("3 docked", MUL-03); the stale age (SYM-08); and the countdown of the ship's order (PLNR-66). With none of these, there is no second line. | ✓ |
| SYM-14 | Hover and click target a symbol through its `ship_symbol_px` square. The hover and selection brackets (HOV-06) enclose that square, not the amplifiers. | ✓ |
| SYM-15 | Ownership text in insets uses: "own", "own flag · civilian", "requisitioned", "allied", "neutral", "hostile" or "unknown". | ✓ |
| SYM-16 | Each marker, symbol and multi-marker is placed with its centre on a whole screen pixel, so its outlines are as sharp as the orbit rings. | ✓ |
| SYM-17 | Symbols, icons, body markers and multi-markers are drawn as vector geometry at their screen size each frame, never as bitmaps scaled to size. *Checked in review.* | ✓ |

## 3b. Labels

Applies to every label on the map: body labels and symbol labels (SYM-13).

| ID | Requirement | Built |
|---|---|---|
| LBL-01 | Labels never overlap one another, or any marker, symbol or multi-marker other than their own. Each label takes the first of these positions that is free: right of its marker, left, above, below, each `label_gap_px` from the edge of the marker's box (HOV-07), whether or not the object carries a mark, so a label never moves when its object is hovered or selected. A parent with an object marker is the exception (GRP-12). |  |
| LBL-02 | Labels are placed in priority order, higher first: the selection; the hovered object; the frame body; the star and planets; then ships, groups and stations by affiliation (hostile, unknown, own commanded, own-flag civilian, allied and neutral); then moons. Within a rank, the object nearer the camera goes first. | ✓ |
| LBL-03 | A label with no free position loses its second line and tries again; if the name alone has none, the label is hidden. The selection's and the hovered object's labels are never hidden: lower labels give way to them. | ✓ |
| LBL-04 | A label keeps its position for as long as that position stays free. A change of position or visibility fades over `label_fade_ms`. | ✓ |
| LBL-05 | Every label on the map has a halo: an outline `label_halo_px` wide round its glyphs in the ground colour, drawn under the text and over everything beneath it. A ring, tail, axis or other line breaks where a label crosses it, close round the letters, and is whole elsewhere. The halo follows the glyphs; it is not a box. | ✓ |

## 3c. Frame cue

The orbital plane and the frame are invisible once the orbit rings fade (MRK-14), and the north dial (MRK-17) is flat by design. The **frame cue** is a dotted crosshair in the plane, through the frame body, with a vertical axis through the plane's normal. It shows which way north lies, that the view is a plane seen at a tilt, and which body is the frame. Reference: mock-up `19-frame-cue-and-label-halo.png`.

| ID | Requirement | Built |
|---|---|---|
| REF-01 | A **frame cue** is drawn in the orbital plane in every frame, the star's included: two axes through the frame body (CAM-05), one along north (CAM-02) and one across it, so four arms leave the body. Each arm is one stroke of round dots `ref_dot_px` across at a pitch of `ref_dot_pitch_px`, in the reference colour (MRK-01), projected through the camera like every other position so it turns with heading and tilt and holds still as the frame body does. | ✓ |
| REF-02 | An arm starts, on screen, half the frame body's box (HOV-07) plus `ref_gap_px` from the body's centre, so the cue never touches the marker or the selection brackets and does not read as a target. | ✓ |
| REF-03 | An arm runs to the edge of the map area (INS-07), or to where the plane's horizon crosses the view if that is nearer. It fades from `ref_alpha_near` at its start to `ref_alpha_far` at its end. The north arm is drawn at that strength; the other three at `ref_minor_factor` times it. | ✓ |
| REF-04 | The north arm ends in an "N" in the data face, `ref_n_inset_px` inside its end, in the reference colour with a halo (LBL-05) so the dots break behind it. The arm points the way the north dial does (MRK-17). | ✓ |
| REF-05 | Each arm carries a tick at every multiple of the scale bar's distance (MRK-16) from the frame body, measured along the plane, so ticks crowd together as the view tilts. A tick is `ref_tick_px` long, drawn across the arm on screen, in the reference colour at the arm's strength at that point. A tick that would fall inside the arm's gap (REF-02) is not drawn. | ✓ |
| REF-06 | A **vertical axis** runs along the plane's normal through the frame body, as one stroke of round dots in the reference colour, drawn as an arm is (REF-01). Its upper half is on the side of the plane the camera is on (CAM-01). It starts at the body's pole (the point at its true radius above its centre) where the body is drawn as a true-size disc (MRK-07), and `ref_gap_px` beyond its box otherwise. Its length is `ref_up_fraction` of the map area's height (INS-07) on screen when the plane is seen edge-on, so it grows and shrinks with the window and the inset, and is shortened by the view as an arm is, so it is nothing looking straight down. It ends in a filled arrowhead `ref_arrow_px` long, which is solid, and fades as REF-03 says. | ✓ |
| REF-07 | The vertical axis's lower half starts at the frame body's centre, is `ref_down_fraction` of the upper half's length, has no arrowhead, and fades as REF-03 says. It is drawn behind the body, so only the part outside a disc shows. | ✓ |
| REF-08 | A vertical half whose length on screen is under `ref_vertical_min_px` is not drawn. | ✓ |
| REF-09 | Draw order (extends MRK-12): the arms, ticks and the lower half of the vertical axis are drawn with the orbit rings, at the bottom. The upper half is drawn above natural bodies, so it runs over a true-size disc, and below symbols, multi-markers, labels and brackets. | ✓ |
| REF-10 | The frame cue is never a hover or click target and never changes what a click hits (MRK-08). | ✓ |

## 4. Hover and inspection

| ID | Requirement | Built |
|---|---|---|
| HOV-01 | Hovering a marker, symbol or multi-marker draws the hover brackets round it (HOV-06). There is no hover tooltip: the name and type are already on the map, and everything else is in the inset. This includes a multi-marker: what it holds is listed in its tray (MUL-08), and for a fold marker in the inset's "In orbit" outline (INS-05), not on hover. | ✓ |
| HOV-02 | Every drawn orbit ring, tail or exposure ring (dot mode or ring mode, MOT-01 and MOT-02) is a hover and click target for its object, as its marker is; a shared orbit's ring targets its members together (HOV-05). A pointer within `ring_hit_px` of the ring hits it. A marker, multi-marker, label, tray or panel under the pointer wins over a ring, and of two rings in reach the nearer wins. Clicking a ring selects as clicking the marker does (INS-01); double-clicking it behaves as ZOOM-16 and ZOOM-17. Folded satellites have no drawn orbit and are reached through the fold marker (GRP-07). An orbit that isn't drawn is not a target, and the frame body's own orbit ring and tail are drawn but are never targets. If the frame body is in a shared orbit (HOV-04), the shared ring is not a target either, but the other members' dots, tails and markers still target those members (HOV-05). | ✓ |
| HOV-03 | A hovered object's orbit ring is drawn in white at `orbit_hover_alpha`; the selected object's orbit ring is drawn in white at `orbit_selected_alpha` for as long as it is selected, in either motion mode. A ring that is both takes the selected style. Hovering or selecting the marker highlights its ring the same way. A hovered or selected ring is drawn `orbit_highlight_width_px` wide. | ✓ |
| HOV-04 | A named orbit (SCL-07) with members is a **shared orbit**, drawn as one ring at its radius: in exposure-ring mode one exposure ring, drawn once at `exposure_ring_alpha`, never stacked; in dot mode each member's dot and tail are drawn on it as MOT-01 says. A folded member (GRP-01) is not drawn. Nothing is grouped by radius at render time. | ✓ |
| HOV-05 | A shared orbit with one member is that member's ring (HOV-02). With several members: hovering highlights the ring (HOV-03) and brackets its identity marker (HOV-06, MOT-14); clicking selects the parent, opens the inset with that orbit's row expanded in the "In orbit" outline (INS-05) and opens the identity marker's tray (MUL-07); double-clicking behaves as a double-click on the parent (ZOOM-16). A member's own dot, marker or tail still targets that member, and wins over the shared ring. | ✓ |
| HOV-06 | The **selection mark** is four white corner brackets round the selected object; the **hover mark** is the same brackets at `bracket_hover_alpha`. An object that is both takes the selection mark. They go round whatever stands for the object on the map: its marker, symbol or true-size disc; the multi-marker that holds it while it is folded or joined (INS-06, MUL-05; a folded ship, group or station pops its marker out, GRP-13), or, for a moon folded into a moon ring, its parent's marker and the ring (GRP-10); or, while its exposure ring shows its identity mark (MOT-14), that mark. For a member of a shared orbit that mark is the orbit's multi-marker (MOT-14), which stands for every member: it carries the selection mark while any member is selected, and the hover mark while the orbit or any member is hovered. A marker that is fully folded (GRP-01), joined into a multi-marker (MUL-05) or faded into its exposure ring (MOT-11) has none. While a satellite folds or unfolds, its mark stays at full strength and its box slides between the multi-marker's box and the marker's with the satellite (GRP-03); a mark fades only as its dot fades into the exposure ring. A bracket is two straight arms meeting at a corner of the object's box (HOV-07): each arm is `bracket_arm_fraction` of the box's shorter side long, at most `bracket_arm_max_px`, and `bracket_stroke_px` wide, drawn inside the box's edge, so the corners stay small when a close disc makes the box large. The brackets are drawn with the labels, above all markers (MRK-12). The frame body (CAM-05) has no mark of its own: the breadcrumb (CAM-25) names it. |  |
| HOV-07 | The **box** is the smallest square that clears the object by `bracket_margin_px` on every side, centred on it: a multi-marker's box is the smallest square that clears both its frames (MUL-01) by the same margin. The object is what is drawn: a symbol's `ship_symbol_px` square (SYM-14), a body's marker (MRK-03), with its moon ring when it has one (GRP-10), or, when MRK-07 draws one, its true-size disc measured to the outer edge of its rim (MRK-11), and any other marker as drawn. So the box stays at marker size until the disc outgrows the marker, then grows with the disc as the camera closes in. Amplifiers and labels are never enclosed. |  |
| INS-01 | Clicking a marker or its orbit ring (HOV-02) selects that object and opens the inspection inset (INS-07); the clock keeps running. Only the primary button selects, and only when the release is within `drag_threshold_px` of the press, so the end of a pan never selects. While the transfer planner is open, a click that hits a place picks a destination instead (PLNR-03). | ✓ |
| INS-02 | The inset shows: header (name, class and ownership line), a Focus button (secondary) and a Close button (quiet); for a ship the faction commands, a Transfer button (secondary, PLNR-01) beside Focus; an image for the knowledge state (INS-04); key facts (INS-09); and an "Open planetary view" button (secondary), disabled until the planetary view exists. Button styles follow BTN-01. | ✓ |
| INS-03 | There is at most one selection and one inset. Selecting another object replaces the inset's contents in place. Close, Esc and a click on empty map space (a press and release within `drag_threshold_px` that hits no marker, multi-marker, ring, tray or panel) clear the selection and close the inset. While the transfer planner is open, Esc ends the planner first and a click on empty map space does nothing (PLNR-03). While a tray is open, Esc closes the tray first (MUL-10). | ✓ |
| INS-04 | The inset image is a placeholder for the knowledge state: **surveyed** = clean render; **unsurveyed** = grainy telescope frame. There is no status text or date, and no stale state, until information gathering is designed. | ✓ |
| INS-05 | The inset has an "In orbit" outline: the system as a tree, from the star down, in orbit order from the inside out and indented one level per generation. Each entry shows its symbol (section 3a, at `list_symbol_px`) or body marker (MRK-13), and its name. An entry with satellites has a toggle that expands or collapses them. Each shared orbit (HOV-04) is one orbit row, however few members it has, even one. The row sits among its parent's entries at the orbit's radius, with its members listed under it. It reads the orbit's name and the word "orbit", then the member count (groups counted once, GRP-09), for example "High orbit · 14", and it expands and collapses like an entry with satellites. A named orbit with no members has no row, and a member of a named orbit is never listed outside its row. The path from the star to the selection, and the selection's own satellites, are expanded when the selection changes; other entries keep the state the player left them in. The selection's row is highlighted and scrolled into view. Clicking an entry selects that object at once; the outline stays in place, so no row moves under the pointer. Double-clicking an entry behaves as ZOOM-16 and ZOOM-17. | ✓ |
| INS-07 | The inset is a sidebar at the right of the window, from the top bar to the bottom, `inset_width_px` wide. While it is open, the map area ends at its left edge: the camera's viewport, the centre the frame body is placed at (CAM-05), the fits (ZOOM-15), the pan clamp (CAM-07) and the edge arrows (EDGE-01) all use the narrower map area. Opening or closing it slides it over `sidebar_slide_ms`; the map area and the view follow the slide, so the frame body stays centred in the visible map throughout, at the same scale. Selecting another object while it is open changes neither. Content taller than the sidebar scrolls inside it. | ✓ |
| INS-08 | A body's inset lists, in its key facts: **Orbit** (distance to its parent, UNIT-01) and **Period** (UNIT-02); for the star, **Mass** in solar masses to 2 decimals instead; and, for a star, planet or moon only, **Mean radius** (UNIT-01, from SCL-05). Ships, stations, gates and salvage sites show no radius. | ✓ |
| INS-09 | The inset's key facts are label-over-value pairs, laid out in rows and wrapping as needed: the label in small spaced capitals in the secondary colour, the value in the data face in the neutral colour. Each section below them (the "In orbit" outline, INS-05) has a heading in small spaced capitals and is separated by a hairline. Type sizes in the sidebar: running text, values, buttons and outline entries at `sidebar_text_px`, spaced capitals at `sidebar_caps_px`, the header name at `sidebar_header_px`; the outline's body markers are `list_marker_px` across. | ✓ |
| INS-06 | When the selected object is folded into its parent (GRP-01) or joined into a multi-marker (MUL-05), that multi-marker carries the selection mark (HOV-06); a moon folded into a moon ring (GRP-10) is carried by its parent's marker and the ring. When the object unfolds or splits off, the mark moves to its own marker without a jump, its box sliding with the object (HOV-06). If it was the multi-marker's last other thing, the multi-marker hides at once and the mark starts from the parent's marker or the object's own symbol; if it was the moon ring's last folded body, the ring fades out (GRP-10) and the mark starts from the parent's marker. |  |

## 4a. Units in readouts

Shared formatting for every readout in the game, not tied to one panel. Figures are in the data face (MRK-02).

| ID | Requirement | Built |
|---|---|---|
| UNIT-01 | A distance is shown in km below `distance_au_threshold_au`, and in AU at or above it, with thousands separators. Km are rounded to the nearest whole number; AU to 2 decimal places, with trailing zeros dropped (for example "850 km", "182,345 km", "0.36 AU", "12.4 AU", "2 AU"). | ✓ |
| UNIT-02 | A duration is shown in the largest unit it reaches: whole minutes ("42 min"), or hours, days or years of 365.25 days to one decimal place ("5.3 h", "31.2 d", "2.4 y"). | ✓ |
| UNIT-03 | Distances and durations are formatted by one shared module that any panel calls; no panel formats them itself. *Checked in review.* | ✓ |
| UNIT-04 | A speed or a delta-v is shown in km/s, to one decimal place at 1 km/s or more and to two below (for example "9.9 km/s", "0.29 km/s"). It is formatted by the shared module of UNIT-03. *Checked in review.* | ✓ |

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
| ZOOM-16 | Double-click on an object that is not the frame body selects it, makes it the frame (CAM-06) and moves the view in one pan-and-zoom of `focus_jump_ms`: to its satellite fit if it has satellites; otherwise its parent's satellite fit if it orbits a body; otherwise the inner limit (ZOOM-14). Double-click on the star selects it, sets the star's frame and moves to the system fit. F does the same for the current selection, and the inset's Focus button for the inset's object. An object that can't be a frame (CAM-24) is selected, and the frame moves to its nearest ancestor that can be one, with the view moving as for that ancestor (as CAM-24 does when a frame body enters ring mode). | ✓ |
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
| CAM-10 | Left drag pans: the point on the orbital plane under the cursor when the drag starts stays under the cursor throughout, with no inertia after release. A press and release that moves less than `drag_threshold_px` is a click, never a pan. A press that starts on the top bar, an inset or a tray never pans. Shift+left drag is reserved and does nothing. | ✓ |
| CAM-11 | Middle drag pans exactly as CAM-10. | ✓ |
| CAM-12 | Right drag rotates about the pivot: horizontal movement changes heading and vertical movement changes tilt, both at `rotate_deg_per_px`, with tilt clamped (CAM-01). Dragging left turns the camera anticlockwise seen from above (the scene turns clockwise on screen); dragging down tilts towards the horizon. | ✓ |
| CAM-13 | A right press and release that moves less than `drag_threshold_px` is a right-click. It is reserved for context menus and orders: until those exist it calls a clearly named hook and does nothing visible. | ✓ |
| CAM-14 | W, A, S, D and the arrow keys pan relative to the screen (up = towards the top of the screen, along the plane) at `key_pan_screen_per_s` of the visible height at the pivot per second, so the speed looks the same at every zoom and tilt. Movement ramps up and down over `key_pan_ramp_ms`. Holding Shift multiplies the speed by `key_pan_fast_factor`. While a tray is open the arrow keys go to the tray (MUL-10) and do not pan; W, A, S and D still do. | ✓ |
| CAM-15 | Q and E change heading (Q as dragging left, E as dragging right), Page Up tilts towards the horizon and Page Down towards straight down, all at `key_rotate_deg_per_s`, with tilt clamped (CAM-01). | ✓ |
| CAM-16 | + (also = and numpad +) zooms in one step and − (also numpad −) zooms out one step, exactly as a wheel notch (ZOOM-01, ZOOM-02, ZOOM-05, ZOOM-14), anchored per ZOOM-13. | ✓ |
| CAM-17 | Edge scrolling: when `edge_scroll_enabled` is true, a cursor within `edge_scroll_px` of a window edge pans that way at the CAM-14 speed. The default is off. | ✓ |
| CAM-18 | Every camera and clock key and mouse button is read from one bindings resource; no system hard-codes a key. Checked in review, not by a test. (A rebinding screen is not yet specced.) | ✓ |
| CAM-19 | The pivot never moves further than `pan_bound_factor` × the system fit radius from the star, in any frame. | ✓ |
| CAM-20 | Pan or rotate input during an animated move (ZOOM-10, ZOOM-11, ZOOM-16, CAM-03) ends the move and takes over from the camera's current state, with no jump. | ✓ |
| CAM-23 | A zoom-out step that reaches the system fit also sets the star's frame, and eases the pivot to the star and the tilt to `standard_tilt_deg`, within the same zoom animation. The heading is kept. | ✓ |
| CAM-24 | An object in exposure-ring mode (MOT-02) can't be a frame, and the inset's Focus button is disabled while it is in that mode. If the frame body enters ring mode, the frame moves to its nearest ancestor not in ring mode (the star if none) and the view moves as ZOOM-16 would for it, or to the system fit for the star. The frame never moves back when the object leaves ring mode. | ✓ |
| CAM-25 | The breadcrumb in the top bar reads "Cluster / *system name*", then the frame body's ancestors below the star, then the frame body: for example "Cluster / Corvane / Halden IV / Arden Highport". The system entry stands for the star's frame. Clicking an ancestor's entry makes it the frame and moves the view as ZOOM-16 would (the system entry: to the system fit). Clicking the frame body's own entry re-centres it with zero offset at its current scale. The Cluster entry is unchanged until the cluster view exists. The entry that stands for the current frame, the frame body's own or the system entry when the star is the frame, is drawn in the frame colour (MRK-01); the others are drawn as usual. | ✓ |

## 6. Satellite grouping

A **satellite** is anything orbiting a body other than the star: moons, stations, ships parked in orbit. Its **parent** is the body it orbits. Moons of moons follow the same rules. Ships in transit are not satellites.

| ID | Requirement | Built |
|---|---|---|
| GRP-01 | A satellite is drawn individually only while its orbit's on-screen radius is at least `unfold_at_px`; below `fold_below_px` it folds into its parent and its marker, label, ring and trail are hidden. Between the two it keeps its current state. | ✓ |
| GRP-02 | The test is per satellite and re-evaluated as zoom changes, so outer satellites unfold before inner ones. | ✓ |
| GRP-03 | Folding and unfolding animate the satellite into or out of its parent over `fold_unfold_anim_ms`, including during zoom animations. | ✓ |
| GRP-04 | At the system fit every moon and planetary station is folded. | ✓ |
| GRP-05 | A parent with folded satellites shows up to two things with its marker, and they never affect each other: a **moon ring** for its folded natural bodies (GRP-10), and an **object marker** for its folded ships, groups and stations (GRP-12). Both are part of the parent's marker. The object marker at rest is a pip; popped out it is a multi-marker (MUL-01), the **fold marker**. |  |
| GRP-07 | Hovering the fold marker draws the hover mark round it, as for a marker (HOV-01); clicking the fold marker or its pip does what clicking the parent does, so the inset's outline shows the folded satellites, and also opens its tray (MUL-07); double-clicking it does what double-clicking the parent does (ZOOM-16). |  |
| GRP-08 | Ships in transit are never folded. | ✓ |
| GRP-09 | A group (SYM-09) folds as one satellite: it counts once in the fold marker's count (MUL-04) and is listed by name with its member count in the tray (MUL-09) and the outline (INS-05). | ✓ |
| GRP-10 | A parent with at least one folded natural body (GRP-01) shows a **moon ring**, whether or not it also has an object marker (GRP-12). The moon ring is a second circle behind the parent's marker (MRK-13): the same circle with its opaque ground inside, without the centre dot or ticks, offset `moon_ring_offset_px` to the right and up, and drawn under the marker (MRK-12). It is always in the neutral colour: it says that moons are folded in, and never takes an affiliation. It is part of the parent's marker: it is drawn only while the marker is and fades with it (MOT-11), it appears and disappears over `fold_unfold_anim_ms` (GRP-03), it adds to the marker's box (HOV-07) and hit area (MRK-08), hovering or clicking it does what the parent does, a click opens no tray, and double-clicking behaves as ZOOM-16. |  |
| GRP-11 | A parent with a moon ring has a second label line, "+N", N being the number of folded natural bodies, in the data face at `moon_count_px` in the secondary text colour at `moon_count_alpha`. It is placed with its label (LBL-01 to LBL-04) and is the first thing dropped when the label has no free position (LBL-03). Folded ships, groups and stations are not counted in it. |  |
| GRP-12 | A parent with at least one folded ship, group or station (GRP-01) has an **object marker**, which stands for those things as the fold marker (MUL-01, MUL-02) holds them. At rest it is a **pip**: a copy of the front frame's shape (SYM-02) `object_pip_px` across with the inside of SYM-03, centred `h + object_pip_gap_px` to the right of and below the parent's centre, h being half the side of the parent's marker box (HOV-07, without the ring). Its colour is the front frame's (MUL-02). When two or more things are folded in, a second pip, the back pip, sits `pip_back_offset_px` to the right and up and is drawn under it, in the back frame's colour (MUL-02); one thing has no back pip. The pip has no count. It is drawn above the parent's marker and ring (MRK-12). It is part of the parent's marker: it is drawn only while the marker is, it fades with it (MOT-11), and it appears and disappears over `fold_unfold_anim_ms` (GRP-03). It has a hit area of at least `min_hit_radius_px` (MRK-08). It never joins another multi-marker (MUL-05). The parent's label tries left, above, below, then right measured from the popped fold marker's box (GRP-13), so it does not move when the marker pops out (LBL-01). |  |
| GRP-13 | The object marker **pops out**: the pip grows into the fold marker over `pop_anim_ms`, the pip's colour and shape carrying over, and its centre slides to `h + ship_symbol_px / 2` to the right of and below the parent's centre, so the front frame's box touches the parent's box without overlapping it. The count (MUL-01) appears when the growth ends. It pops out while any of these holds: the pointer is within `pop_reach_px` of the parent's marker centre, is nearer to it than to any other parent's, and no button is held down; it is already out and the pointer is within `pop_release_px`; the pointer is over the fold marker or its tray; its tray is open (MUL-08); the parent is selected, or the selection is a thing it holds (INS-06). So a player with no pointer gets it by selecting the parent. It goes back to a pip over `pop_anim_ms` when none holds. The popped marker is the whole object marker: hover, click, tray and selection mark (HOV-06, MUL-07) work on it, and a click on a pip at rest, which a pointer cannot do without first popping it, is treated as a click on the fold marker. |  |

## 6a. Several objects on one marker

Several things can stand for one place on screen: satellites folded into a parent, ships docked at a station, ships in one phase, symbols that merely overlap. One idiom draws them, the **multi-marker**: a ship symbol with a count and a second frame behind it. Clicking it opens a **tray** that lists what it holds, so any one of them can be picked without leaving the map. A **thing** is what the count counts: a natural body, a ship, a group or a station. Reference: mock-up `18-multiple-objects.png`; where it and this section disagree, this section wins.

| ID | Requirement | Built |
|---|---|---|
| MUL-01 | A **multi-marker** is a ship symbol (SYM-04) with a second frame behind it. The **front frame** fits a square of `ship_symbol_px` and has the outline and opaque inside of SYM-02 and SYM-03. The count, in the data face at `multi_count_px` in the frame's colour, is centred where an icon would be; a count above 99 reads "99+". The **back frame** has the same size and outline, is offset `back_frame_offset_px` to the right and up, and is drawn under the front frame. A multi-marker has no icon, bracket or other amplifier. | ✓ |
| MUL-02 | The frames show affiliation, and natural bodies are ignored for it. When any thing is a ship, a group or a station, the front frame has the shape and colour (SYM-02) of the most pressing affiliation among them, in the order hostile, unknown, neutral or allied, own (an own-flag civilian counts as own). The back frame is the own frame when any of them is own, and otherwise a copy of the front frame. Things docked at a station inside a larger marker (MUL-04) count for the frames, though not for the count, so a hostile ship docked there still shows in the front frame; a station marker's own frame is unchanged (MUL-03). A multi-marker always holds at least one ship, group or station: satellites that are all natural bodies fold into a moon ring instead (GRP-10). An inside is the SYM-03 mix for its affiliation: `symbol_fill_faction` for own, `symbol_fill_other` for the rest. |  |
| MUL-03 | **Docked.** A ship or group that is together (PHAS-02) with a station is docked at it. A station with docked things is drawn as one multi-marker with the station's own frame (SYM-02) and installation bar (SYM-07), so it keeps its own affiliation whatever is docked. Its count is the number of docked things; the station is not counted. Its back frame follows MUL-02, over the docked things. A station with nothing docked is the plain station symbol (SYM-07), with no count and no back frame. | ✓ |
| MUL-04 | Each ship, group and station counts as one thing; a group's members are not counted (GRP-09). Docked things (MUL-03) are counted only by their station's own marker. Inside a larger marker (MUL-05) the station is one thing, and what is docked at it is listed under its row in the tray (MUL-09) and not counted. A parent's fold marker holds only its folded ships, groups and stations: its folded natural bodies are counted by the moon ring (GRP-11) and listed in the inset's outline (INS-05), not in the tray. |  |
| MUL-05 | A thing is in at most one multi-marker. Three things make one. **Together:** ships, groups and stations that are together (PHAS-02) in one named orbit are one marker at every zoom, a station marker (MUL-03) when a station is among them and otherwise a multi-marker of those things. **Overlap:** two symbols (a station marker counts as one symbol) join when their on-screen centres are closer than `cluster_join_px` and part when they are farther apart than `cluster_split_px`, keeping their state in between; a cluster is every symbol joined to another by a chain of such pairs, and it is a multi-marker when it holds two or more things, while a single thing keeps its own symbol; ships in transit never join (GRP-08), and nor does a fold marker. **Fold:** the satellites folded into a parent (GRP-01) are held by the parent's fold marker (GRP-05). Joining, parting, folding and unfolding animate over `fold_unfold_anim_ms`, the symbols sliding into or out of the marker, also during zoom animations (GRP-03). A joined or folded thing draws no symbol, label or tail of its own; its orbit ring is unchanged. | ✓ |
| MUL-06 | A multi-marker's centre is: for things together, their shared point; for an overlap cluster, the mean of its symbols' centres; for a fold marker, the point `h + ship_symbol_px / 2` to the right of and below the parent's centre (GRP-13). It sits on a whole screen pixel (SYM-16). |  |
| MUL-07 | A click on a multi-marker (INS-01) opens its tray (MUL-08) and selects an object: for a fold marker or the marker of a shared orbit (MOT-14, HOV-05), the parent, as clicking the parent does; for any other multi-marker, the first row of the tray. Double-clicking selects and focuses the same object (ZOOM-16). At most one tray is open: opening another closes the first. | ✓ |
| MUL-08 | The **tray** is a panel at the marker, in the sidebar's panel fill with a hairline outline (INS-09), at least `tray_min_width_px` wide and as wide as its widest row. A notch on the edge facing the marker points at it. It sits to the right of the marker's box (HOV-07), `tray_gap_px` clear of it, centred on the marker vertically and moved to lie wholly inside the map area (INS-07). If it does not fit on the right it goes to the left, except for a fold marker, whose left would cover its parent: that tray goes below the marker's box, or above it if it does not fit below, centred on the marker horizontally and moved inside the map area in the same way. A tray taller than the map area scrolls inside it. From the top: a header `tray_header_px` high, in small spaced capitals in the secondary colour (INS-09); the rows (MUL-09), each `tray_row_px` high; a footer `tray_footer_px` high with the key hint in the data face in the secondary colour. The header reads "NAME · N DOCKED" for a station marker; "ORBIT ORBIT · N THINGS AT ONE POINT" for things together, with the orbit's name in place of the first ORBIT (for example "HIGH ORBIT · 4 THINGS AT ONE POINT"); "ORBIT ORBIT · N OF M" for an overlap cluster of one named orbit, M counting that orbit's things; "PARENT · N IN ORBIT" for a fold marker; and "N OBJECTS" for any other. |  |
| MUL-09 | The tray has one row per thing: the station first when there is one, then the others in the order of the "In orbit" outline (INS-05). A row shows the thing's symbol or body marker at `list_symbol_px`, its name in the label face in the neutral colour, and at the right a tag in the data face in the secondary colour: "station" for a station; "×N · class summary" (SYM-13) for a group, N being its member count; the condition to one decimal for a member of an open group that is own and known. A group's row opens in place to list its members one level in, and so does a station's row in a larger marker (docked things, MUL-04); in a station marker the docked things follow the station unindented. The selected row is highlighted as in the outline (INS-05). | ✓ |
| MUL-10 | With a tray open: a click on a row selects its thing (INS-01), the inset follows and the tray stays open; Up and Down move the selection to the previous and next row, stopping at the ends; Right opens a selected row that has members and Left closes it; Esc closes the tray and leaves the selection, and a further Esc clears it (INS-03). A click anywhere else closes the tray and does what that click does (INS-01, INS-03). The arrow keys go to the tray and do not pan (CAM-14), and a press that starts on the tray never pans (CAM-10). The tray follows its marker as the camera and clock move, and its rows follow the marker's things; when the marker stops existing, because its things have parted or unfolded, the tray closes and the selection is kept. Its keys are read from the bindings resource (CAM-18). | ✓ |
| MUL-11 | A multi-marker's hit area is the union of its two frames' `ship_symbol_px` squares (SYM-14), and at least `min_hit_radius_px` round the front frame's centre (MRK-08). Hovering it draws the hover mark round its box (HOV-06, HOV-07). | ✓ |
| MUL-12 | While the planner is open (PLNR-03), a click on a fold marker picks what a click on its parent picks (PLNR-11) and opens the tray; a click on any other multi-marker opens the tray and picks nothing. A click on a row picks that thing as a click on its symbol would (PLNR-11); a row that can't be picked (PLNR-11) is dimmed and a click on it does nothing. | ✓ |
| MUL-13 | A station marker is labelled with the station's name, with "N docked" on the second line (SYM-13), placed by LBL-01 to LBL-04 from the marker's box. No other multi-marker has a label: the labels of the things it holds are hidden while they are in it, and the tray names them. | ✓ |

## 7. Motion rule

The **apparent period** of an orbit is its orbital period divided by the selected time rate, in real seconds. Pausing does not change it: the selected rate is kept while paused (CLK-05).

| ID | Requirement | Built |
|---|---|---|
| MOT-01 | Apparent period above `tail_secs`: the object is a dot at its true position, and its orbit ring carries a tail behind it. The tail is a brightness gradient along the ring itself, one continuous anti-aliased stroke, running from the exposure ring's look (MOT-02) at the dot down to the ring's normal alpha at the tail's end. Ships and stations fade to transparent instead (MRK-09). The fade flattens as the tail closes (MOT-11). | ✓ |
| MOT-02 | While running, apparent period at or below `tail_secs`: the tail has closed into the exposure ring. No dot; the whole ring is drawn in the neutral colour at `exposure_ring_alpha`, `exposure_ring_width_px` wide; the label sits on the ring at a fixed spot, beside its identity mark (MOT-14). A lone object's ring is labelled with its name; a shared orbit's ring (HOV-04) with the orbit's name (for example "High"), since its parent is at the centre. Under the name, a second line gives the orbit's period (UNIT-02) in the data face in the secondary colour, so the ring says it is a fast orbit; it is the first part to go when there is no room (LBL-03). Being partly transparent, an exposure ring never matches the full-white selected ring (HOV-03). | ✓ |
| MOT-10 | The tail covers the arc swept in the last `tail_secs` real seconds: 360° × `tail_secs` ÷ apparent period. Its length follows the time rate continuously, with no steps. | ✓ |
| MOT-11 | As the tail grows from `dot_fade_from_degrees` to 360°, the dot fades out and the ring width grows to `exposure_ring_width_px`, both linearly, so the closing is seamless. The tail's brightness fade (MOT-01) flattens over the same range: at `dot_fade_from_degrees` the tail fades fully to the ring's normal brightness at its end, and at 360° it is even all round, so it meets the exposure ring with no jump. | ✓ |
| MOT-13 | The displayed tail length eases towards the length MOT-10 or MOT-12 gives, reaching it within `tail_ease_ms`, so a change of time rate, a pause or an unpause animates rather than snaps. The dot fade and ring width (MOT-11) follow the displayed length, so a switch between dot and exposure ring animates too. | ✓ |
| MOT-04 | Pausing keeps every tail at its running length, so the view does not jump. | ✓ |
| MOT-12 | While paused, each tail is capped at `dot_fade_from_degrees`, so every orbiting object shows as a full-brightness dot with its tail, objects in exposure-ring mode included. Unpausing restores the running state. | ✓ |
| MOT-07 | While the selected object is in ring mode, its inset has the line "Position: shown when paused". | ✓ |
| MOT-08 | The rule applies to every orbiting object (moons, stations, parked ships, close-in planets); ships in transit are ignored. | ✓ |
| MOT-14 | An exposure ring carries an **identity mark** at its label spot, so a fast ship, station and moon are told apart without reading the label: a lone ship or station shows its symbol (section 3a) at `ship_symbol_px`, a lone body its marker (MRK-13), and a shared orbit (HOV-04) a multi-marker (MUL-01, MUL-02) holding its members, whose count counts members with groups counted once (GRP-09). A symbol, body marker or multi-marker is centred on the ring point, as a marker is (SYM-16). The mark shows only while the displayed tail is at 360° (the dot fully faded, MOT-11), fading in and out over `identity_mark_fade_ms`, so a ring held short of closing never shows a moving dot and a still mark together. It is a target for the same thing the ring is. The ring also carries a **direction chevron**: an open arrowhead `chevron_px` across, drawn `symbol_stroke_px` wide in the neutral colour on the ring, `chevron_gap_px` beyond the mark's edge in the direction the object circles. It shows and fades with the mark, is part of it for the labels (LBL-01), which move to another side of the mark if the chevron would run under them, and implies no position beyond the one the mark already holds. | ✓ |

## 8. Test content: Corvane

The prototype system, kept as data (SCL-03). It exists to exercise the rules above, so its values serve testing first. It is also a plausible output of the generator, so its bodies use realistic masses and spacings (TEST-07, TEST-08). Where a test needs it, a body may be less typical than the generator's usual output, but never implausible.

| ID | Requirement | Built |
|---|---|---|
| TEST-01 | Corvane contains at least: the star; planets Pell, Iver (with Iver Yard station), Halden IV (capital, own) and Morrow (outer giant with many moons); a belt with salvage sites Wreck Field and Lantern; and off-screen gates. | ✓ |
| TEST-02 | Halden IV's satellites have periods of about: Arden Highport (in the Low band) 0.07 d, IVa 1.5 d, IVb 4.2 d, IVc 31 d, IVd 68 d, so both motion modes show at 1 d/s. Radii follow Kepler from Halden IV's mass. | ✓ |
| TEST-03 | Halden IV also has contact K-3 (unknown affiliation, unknown class, weak track, slow orbit) and one own commanded warship parked in orbit, as static orbiting objects. | ✓ |
| TEST-05 | Corvane has, as static orbiting objects in slow orbits (dot mode at 1 d/s), at least one of each: own fleet tanker; own troop transport with 1st Arden Guard (battalion) embarked; own empty troop transport; own-flag civilian ore carrier (Varga Mining); requisitioned civilian ore carrier; own-flag civilian armed hauler; neutral trader; allied warship; hostile troop transport with unknown load; hostile raider on a stale track last seen 6 days before the start date; unknown-affiliation contact with known class cargo. Iver Yard and Arden Highport are own stations. | ✓ |
| TEST-06 | Corvane also has, as static orbiting objects round Halden IV (TEST-10): own task force TF-1 (4 warships with conditions 0.9, 0.5, 0.3 and 0.1, so the group's mean, 0.45, is below `condition_warn_fraction`). The own ships of TEST-03 and TEST-05 have conditions above `condition_warn_fraction`, so both bar colours show. |✓ |
| TEST-09 | Morrow IX has two satellites of its own, as static orbiting objects within its Hill sphere: own station Tern Relay in its Low band, period about 0.13 d (ring mode at 1 d/s), and a parked neutral trader, period about 9 d (dot mode at 1 d/s). They exercise nested folding, nested multi-markers and a frame three levels deep. | ✓ |
| TEST-10 | Halden IV has exactly two named orbits (SCL-07): "Low", holding the station Arden Highport (TEST-02), and "High", holding every ship parked at Halden IV: the warship of TEST-03, a second own commanded warship, and the TEST-05 and TEST-06 ships and groups that orbit Halden IV. Members take distinct phases, spread round the ring, except that the ships of a group share one phase (PHAS-14). The contact K-3 keeps an orbit of its own. So Corvane has one shared orbit (HOV-04) with many members, and no ship has a parking orbit of its own. | ✓ |
| TEST-04 | The inspection data covers both knowledge states: Halden IV surveyed, Halden IVc unsurveyed. | ✓ |
| TEST-07 | Morrow has the mass of a Jupiter-class gas giant: 318 Earth masses. | ✓ |
| TEST-08 | Morrow's eleven moons have periods of about 0.35, 0.55, 0.9, 1.8, 3.6, 7.2, 16, 30, 50, 75 and 110 d. Radii follow Kepler from Morrow's mass, and the innermost is no closer than 143,000 km (2 × Morrow's radius of 71,500 km). The periods straddle both motion modes at 1 d/s: six moons in ring mode (under 8 s), five in dot mode (8 s or more). | ✓ |
| TEST-11 | Iver Yard has two ships docked at it, together with it (PHAS-02): an own commanded warship and a neutral trader. Corvane therefore shows a station marker with a count of 2 (MUL-03). Sol's Perigee Station with Marlin docked (SOL-14) shows one with a count of 1. | ✓ |
| TEST-12 | Halden IV's High band also holds, besides the ships of TEST-10, an own task force TF-2 (two warships) and two own-flag civilian haulers at one phase, so they are things together (MUL-05) and show as one multi-marker with a count of 3. | ✓ |

## Tunables

All in the `Tunables` resource (`src/view/tunables.rs`). Defaults are starting values; changing one is a spec change only if the default is being settled.

| Tunable | Default | Used by |
|---|---|---|
| `unfold_at_px` (R_show) | 24 px | GRP-01 |
| `fold_below_px` | 20 px | GRP-01 |
| `fold_unfold_anim_ms` | 200 | GRP-03 |
| `moon_ring_offset_px` | 3 px | GRP-10 |
| `moon_count_px` | 9 px | GRP-11 |
| `moon_count_alpha` | 0.7 | GRP-11 |
| `object_pip_px` | 6 px | GRP-12 |
| `object_pip_gap_px` | 2 px | GRP-12 |
| `pip_back_offset_px` | 3 px | GRP-12 |
| `pop_reach_px` | 24 px | GRP-13 |
| `pop_release_px` | 32 px | GRP-13 |
| `pop_anim_ms` | 120 | GRP-13 |
| `tail_secs` | 8 s | MOT-01, MOT-02, MOT-10 |
| `dot_fade_from_degrees` | 300° | MOT-11, MOT-12 |
| `orbit_ring_width_px` | 1 px | MRK-05, MOT-11 |
| `orbit_ring_alpha` | 0.35 | MRK-05, MOT-01 |
| `orbit_highlight_width_px` | 1 px | HOV-03 |
| `exposure_ring_width_px` | 3 | MOT-02 |
| `exposure_ring_alpha` | 0.5 | MOT-01, MOT-02 |
| `tail_ease_ms` | 500 ms | MOT-13 |
| `identity_mark_fade_ms` | 150 ms | MOT-14 |
| `zoom_step_factor` | ×2 | ZOOM-01 |
| `zoom_anim_ms` | 200 (range 150–250) | ZOOM-01 |
| `zoom_stop_resistance` | 0.35 | ZOOM-05 |
| `focus_jump_ms` | 400 | ZOOM-10, ZOOM-11, ZOOM-16, CAM-03 |
| `zoom_fit_margin` | 0.8 | ZOOM-15 |
| `min_view_radius_au` | 0.05 AU | ZOOM-14 |
| `body_max_screen_fraction` | 0.25 | ZOOM-14 |
| `standard_tilt_deg` | 60° | CAM-02, CAM-23 |
| `focus_keep_margin_px` | 40 px | CAM-07 |
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
| `min_hit_radius_px` | 10 px | MRK-08 |
| `inset_width_px` | 320 px | INS-07 |
| `sidebar_slide_ms` | 200 ms | INS-07 |
| `ring_hit_px` | 7 px | HOV-02 |
| `orbit_hover_alpha` | 0.6 | HOV-03 |
| `orbit_selected_alpha` | 0.85 | HOV-03 |
| `bracket_hover_alpha` | 0.6 | HOV-06 |
| `bracket_arm_fraction` | 0.3 | HOV-06 |
| `bracket_arm_max_px` | 10 px | HOV-06 |
| `bracket_stroke_px` | 2 px | HOV-06 |
| `bracket_margin_px` | 4 px | HOV-07 |
| `distance_au_threshold_au` | 0.1 AU | UNIT-01 |
| `star_marker_px` | 16 px | MRK-03, MRK-13 |
| `planet_marker_px` | 12 px | MRK-03, MRK-13 |
| `moon_marker_px` | 8 px | MRK-03, MRK-13 |
| `body_fill_mix` | 0.32 | MRK-11 |
| `body_rim_px` | 1 px | MRK-11, MRK-13 |
| `star_tick_count` | 8 | MRK-13 |
| `star_tick_px` | 3 px | MRK-13 |
| `planet_dot_px` | 3 px | MRK-13 |
| `orbit_fade_from` | 4 | MRK-14 |
| `orbit_fade_to` | 12 | MRK-14 |
| `plot_margin_px` | 24 px | MRK-16, MRK-17 |
| `scale_bar_max_px` | 160 px | MRK-16 |
| `north_dial_px` | 34 px | MRK-17 |
| `north_head_px` | 10 px | MRK-17 |
| `body_gap_px` | 2 px | MRK-11 |
| `ship_symbol_px` | 22 px | SYM-04, SYM-12, SYM-14 |
| `list_symbol_px` | 14 px | INS-05, MRK-15 |
| `symbol_stroke_px` | 1.5 px | SYM-02 |
| `button_stroke_px` | 1 px | BTN-01, EDGE-03 |
| `sidebar_text_px` | 13 px | INS-02, INS-05, INS-09 |
| `sidebar_caps_px` | 11 px | INS-09 |
| `sidebar_header_px` | 17 px | INS-02 |
| `list_marker_px` | 10 px | INS-05 |
| `label_gap_px` | 4 px | LBL-01 |
| `label_fade_ms` | 150 ms | LBL-04 |
| `label_halo_px` | 2 px | LBL-05 |
| `amplifier_gap_px` | 1 px | SYM-09, SYM-12 |
| `condition_track_mix` | 0.25 | SYM-12 |
| `chevron_px` | 8 px | MOT-14 |
| `chevron_gap_px` | 8 px | MOT-14 |
| `belt_band_alpha` | 0.10 | MRK-18 |
| `belt_speck_count` | 1,200 | MRK-18 |
| `belt_speck_alpha` | 0.5 | MRK-18 |
| `ref_dot_px` | 2 px | REF-01 |
| `ref_dot_pitch_px` | 5 px | REF-01 |
| `ref_gap_px` | 8 px | REF-02, REF-06 |
| `ref_alpha_near` | 1.0 | REF-03 |
| `ref_alpha_far` | 0.45 | REF-03 |
| `ref_minor_factor` | 0.6 | REF-03 |
| `ref_n_inset_px` | 13 px | REF-04 |
| `ref_tick_px` | 9 px | REF-05 |
| `ref_up_fraction` | 0.45 | REF-06 |
| `ref_arrow_px` | 8 px | REF-06 |
| `ref_down_fraction` | 0.7 | REF-07 |
| `ref_vertical_min_px` | 10 px | REF-08 |
| `symbol_fill_faction` | 0.30 | SYM-03 |
| `symbol_fill_other` | 0.16 | SYM-03 |
| `stale_symbol_alpha` | 0.55 | SYM-08 |
| `condition_warn_fraction` | 0.5 | SYM-12, TEST-06 |
| `cluster_join_px` | 24 px | MUL-05 |
| `cluster_split_px` | 28 px | MUL-05 |
| `back_frame_offset_px` | 4 px | MUL-01 |
| `multi_count_px` | 10.5 px | MUL-01 |
| `tray_min_width_px` | 240 px | MUL-08 |
| `tray_row_px` | 26 px | MUL-08 |
| `tray_header_px` | 28 px | MUL-08 |
| `tray_footer_px` | 26 px | MUL-08 |
| `tray_gap_px` | 8 px | MUL-08 |

## Type roles

Every font is under the SIL Open Font License, with its licence file beside it in `assets/fonts/`; a font under any other licence needs approval before it is added.

| Role | Font | Weights | Used for |
|---|---|---|---|
| Label face | IBM Plex Sans Condensed | Regular, SemiBold | Map names, labels, buttons, headings, breadcrumb |
| Text face | IBM Plex Sans | Regular, SemiBold | Running prose |
| Data face | IBM Plex Mono | Regular | Dates, figures, status readouts, stamps |

All three are one family (IBM Plex), so their proportions and details match.

## Open points

- A station marker's back frame is the own frame when any docked thing is own (MUL-02), so a hostile ship docked beside an own one at a neutral station shows only in the tray. Whether the back frame should show the most pressing docked affiliation instead is open.
- Input targets mouse and keyboard only. Gamepad, touch and trackpad are not targets, but designs should avoid what would be hard to port to them (for example, relying on hover alone or on raw wheel deltas).
- The frame cue has no toggle until layer toggles are specced. The vertical axis is dotted like the arms, so where it and the north arm share a screen line (heading 0, a moderate tilt) the two read as one: check on screen, and if it is a problem, thin the dots or tint the axis. The vertical axis could later carry more of the physical picture (a faint plane disc, drop lines to the plane in the cluster view); only the axes are specced now.
- Per-system camera memory, a frame that rotates with its body, breadcrumb drop-downs, selection cycling (Tab), and gamepad and touch input: in `docs/ideas/presentation/presentation.md` (Camera), not yet specced.

## Not yet specced

From the ideas, not to be built until specced: installation role icons, data blocks beyond SYM-13, layer toggles, alerts, legend, Administration button, inspection fidelity for foreign contacts (thermal track), the projected paths of foreign ships (own ships' paths are in `planner.md`), uncertainty ellipses, the planetary view and its transition, the cluster view, gate distance labels, and every open question in `docs/ideas/presentation/presentation.md` section 6.

# Orphan Worlds: Symbology

This doc sets out how ships, groups of ships and stations are drawn on the vector display, using NATO joint military symbology (APP-6 / MIL-STD-2525) as the reference. The main ideas:

- **Frame shape = affiliation, icon = what it is, amplifiers around the frame = state.** This is the NATO structure, kept because it reads without colour and scales to new ship types.
- Ships and stations get a frame; bodies, belts and salvage sites stay as plain markers. The frame is what tells "a ship" from "a place".
- The identification of a contact builds up on its symbol step by step, which ties the display to the uncertainty pillar.

**Decided** 2026-10-11, except where a point is marked otherwise. **Specced** in `docs/spec/system-view.md` as SYM-01 to SYM-15, GRP-09 and TEST-05 to TEST-06, with MRK-01, MRK-04, INS-05 and TEST-03 reworded and new tunables. Mock-up: `mockups/7-symbology.png` (source `mockups/7-symbology.html`).

---

## 1. Why NATO symbology

- It suits the look: the strategic display already reads as the government's own instruments (*Presentation & Visual Style*, section 2). A staff plot or ship's combat information centre is the closest real model.
- It is a tested grammar. Each part of a symbol has one job, so new types slot in without redesigning the set.
- It works in monochrome. Affiliation is in the frame shape, so colour only reinforces it. This helps colour-blind players more than palette choices alone.
- Players of wargames and many strategy games already know the basics (blue circle friendly, red diamond hostile).

We adapt rather than copy. Full APP-6 has thousands of icons and dense text fields; we take its structure and a small icon set drawn for a fixed-size screen marker.

## 2. Frames: affiliation and command

Uses the NATO sea-surface frames. They are closed, compact shapes. The NATO space-domain frames are open arcs that read poorly at small sizes, and every ship in this game is in space, so a domain mark would carry no information. (NATO land units use rectangles; we keep those for ground formations, which also appear inside the troop transport icon.)

| Affiliation | Frame | Colour |
|---|---|---|
| Own | Circle | Blue #5AA9E6 (own-flag civilian: paler blue #A9CBE3) |
| Allied and neutral | Square | Green #8CCB8A |
| Hostile | Diamond | Red #EE6A5F |
| Unknown | Quatrefoil | Orange #F0A040, standing in for NATO yellow |

- **Allies** use the neutral frame for now. The split that matters most to the player is "mine to command" versus "not mine". NATO itself puts allies in the friend frame. If allies later need to stand apart from neutrals, they need a cue other than the unfilled circle, which own-flag civilians use.
- **Flag and command** are separate questions with separate cues:
  - *Flag* (whose ship it is) sets the frame shape. Ships registered to the faction's own organisations keep the circle; a foreign merchant has the square.
  - *Command* (does the faction give it orders) sets the fill. Faction ships have a solid tint (about 30 %). Own-flag civilians have no fill and a paler blue, so they read as "ours, but not under orders". Other affiliations have a faint tint (about 16 %).
  - Requisitioning or chartering a civilian ship fills its frame in for as long as the faction commands it, so the player sees the takeover on the map.
  - **Opaque insides (decided 2026-10-13, from F-123).** Fills are solid colours mixed into the ground, not transparency, and every frame covers what lies under it. See-through fills let orbit lines cross the icons and looked glassy, like a HUD rather than a plot. (Specced: SYM-03.)
  - **Sharp at screen size (decided 2026-10-13, from F-124).** Symbols are drawn as vectors on whole pixels; scaled bitmaps blurred beside the orbit lines. (Specced: SYM-16, SYM-17.)
- **Open:** whether "hostile" means at war, or just "a faction we treat as a threat". It should follow the faction's declared stance, not the AI's intent, so the player never learns hidden intent from a colour.

## 3. Icons: what it is

Drawn inside the frame. **Filled icon = armed; outline icon = unarmed.** That one rule gives an at-a-glance threat read.

| Type | Icon | Basis |
|---|---|---|
| Warship | Filled triangle | Keeps the old ship marker as the combatant icon. |
| Fleet supply / tanker | Outline drop | NATO supply-of-fuel symbol. Propellant is the critical stock in this game. |
| Cargo ship | Outline crate (box with a centre line) | Hauler. |
| Troop transport | Unit box (the NATO land-unit frame) | The box contents and the echelon slot show the load (below). |
| Class unknown | "?" | NATO leaves the frame empty. A "?" is clearer to players, and keeps "empty transport" distinct. |

- **Troop transport load** follows what the faction knows:
  - *Load unknown:* empty box with "?" in the echelon slot. The normal case for foreign transports, since you rarely know whether one is carrying troops.
  - *Known empty:* empty box, nothing above. Normal for own transports.
  - *Known embarked:* box with the infantry cross and an echelon mark (II battalion, III regiment, X brigade), or "?" if the size is unknown. With several formations aboard, the largest sets the mark.
  - The embarked formation uses the same symbol as on the planetary view (sketch board 4, "1st Arden Guard"), so it can be followed from ship to surface.
- An armed hauler gets the warship icon once it is known to be armed. The icon shows the role as observed, not the hull type.
- More icons later as types appear (survey ship, salvage tug, gate tender, courier). Keep each a simple shape that reads at about 10 px.

## 4. Several objects on one marker

Several things can stand for one place on screen. There are three cases, and only the third has a design today:

- **Always on one point.** A task force's ships, a ship docked at a station, an escort that shares the phase of the convoy it covers (PHAS-02, PHAS-14). Zooming never pulls them apart.
- **On one point by chance.** Ships in one band whose symbols overlap at this zoom, or on a paused frame. Zooming does separate them.
- **Folded.** Satellites inside the fold radius of their parent (GRP-01), shown by the fold marker (GRP-05).

**Decided 2026-10-16: no loose groups.** SYM-09 once drew "ships moving together with no formation" as a stacked frame with a count, taken from game state. Nothing forms such groups (F-159), and the problem it was meant for is the first two cases above. SYM-09's loose-group clause, SYM-10's convoy example, the TEST-06 haulers and `group_offset_px` are withdrawn.

**Task force (specced).** Ships under one commander with shared orders: the NATO task-force bracket over the frame, a count at the upper right, and the icon of the most significant member (warship > transport > supply > cargo > unknown). It is one thing in any multi-marker it is part of (MUL-04), and two groups are never merged into one (SYM-10).

### One idiom (mock-up 18)

**Decided and specced 2026-10-16** as MUL-01 to MUL-13, with changes to GRP-05, GRP-07, GRP-09, HOV-01, HOV-02, HOV-05 to HOV-07, INS-03, INS-06, MOT-14, MRK-12, SYM-10, SYM-13, SYM-16, SYM-17, LBL-01, REF-09, CAM-10, CAM-14 and PLNR-11. GRP-06 is withdrawn. Anything that stands for several objects is a **multi-marker**, which replaces the fold pill:

- A normal frame with the **count where the icon would be**, and a **second frame behind it** to say "several" (MUL-01).
- The front frame has the most pressing affiliation among the things (hostile, then unknown, then neutral or allied, then own), so the shape reads without colour. The back frame is the own frame when any thing is own. Bodies only (moons and the like): plain neutral circles, as bodies are drawn (MUL-02, MRK-10).
- A group is **one thing** in any marker it is part of (MUL-04).
- A station keeps its own frame and installation bar, with the number of things docked where an icon would be. Its frame keeps the station's own affiliation (MUL-03). Inside a larger marker it is one thing, with what is docked listed under it (MUL-04).
- Selection and hover bracket the whole marker, the second frame behind included (HOV-06, HOV-07).

### The member tray

Picking one member through the inset's "In orbit" outline takes several clicks and moves the eye away from the map. A marker with several members opens a **tray** beside it (MUL-07 to MUL-10):

- Opens on click. Each row is the member's symbol (or body marker) and its name. A group is one row that opens in place to show its members; a station is the first row, as the place.
- Clicking a row selects that member and the inset follows. The tray stays open. Up and Down step the selection, Right and Left open and close a group, Esc or a click elsewhere closes it.
- The tray lists only what is on the marker. The outline stays the whole-system view.

### Two sources, one marker

- **Structural.** Docking and ships in one phase come from game state, so the markers they make never split with zoom (MUL-05, together). The tray is the only way in.
- **Incidental.** Symbols that overlap on screen join by proximity, as satellites fold by orbit size, with the same kind of hysteresis (MUL-05, overlap; GRP-01). A cluster may hold ships from different orbits, so it cannot be read from one branch of the outline.

### Decisions on the open points (2026-10-16)

- **Join, not fan out.** Small overlaps collapse into a marker. A fan-out would draw ships off their orbit.
- **A click on a marker** opens the tray, and selects the parent for a fold marker or a shared orbit's marker (as clicking the parent does) and the first row for any other (MUL-07).
- **Own ships in a mixed cluster** show as the back frame, not as a pill-style dot (MUL-02).
- **Collapse distance:** a frame's width plus a gap to join (`cluster_join_px`, 24 px), and a wider margin to part (`cluster_split_px`, 28 px).
- **The count** counts things, so a task force is one (MUL-04).

### Open

- Whether a station's back frame should show the most pressing docked affiliation rather than own. Today a hostile ship docked beside an own one shows only in the tray (`system-view.md`, open points).
- What a docked task force can be ordered to do (F-159).

## 5. Stations

Stations use the affiliation frames with the NATO installation bar on top and no icon. This replaces the old hollow square, which would clash with the neutral frame.

- **Not specced (candidate):** role icons were sketched (naval base = anchor, shipyard = crane over a hull, habitat = dome, industrial site = sawtooth roof) and set aside for now. Later types might include extraction sites, gate stations, research institutes and sensor stations; avoid triangles and crosses, which are taken.
- **Candidate:** a site on a surface folds into its body like other satellites and appears on the planetary view with the same symbol. The planetary-view sketch (board 4) draws settlements as squares and extraction sites as triangles, which would clash; they would switch to station symbols.
- **Candidate:** dormant or abandoned stations use the stale treatment (dimmed), since what we know of them is old.

## 6. Amplifiers: state

Placed around the frame, never inside it, so the core symbol always reads the same.

- **Velocity leader:** a line from the frame in the direction of motion (NATO direction-of-movement indicator). Its length grows with the logarithm of speed, since transit speeds vary widely. Waits for ships in transit.
- **Offsets (decided 2026-10-16):** the condition bar and the group count start a small gap outside the selection box (HOV-07), not against the frame, so the brackets never touch them. An empty part of the bar is drawn dim; a ship with no known condition has no bar. (Specced: SYM-09, SYM-12.)
- **Condition bar** under the frame: own ships only; it turns orange below half. A group shows its mean.
- **Dashed frame:** the position is estimated, not a current observation (NATO uses dashed for anticipated).
- **Stale track:** dashed and dimmed; the age goes in the label.
- **Echelon** above the frame (troop transports).
- **Selection and hover** are white corner brackets round the frame, hover dimmer (HOV-06, HOV-07). They enclose the frame's square, not the amplifiers. The frame body has no mark.

## 7. Identification ladder

How a contact firms up as tracking improves. Each step adds to the symbol rather than swapping it, so the player sees what changed.

1. **Detected:** dashed quatrefoil with "?" (weak track, class and owner unknown). In time, inside its uncertainty region (not yet specced).
2. **Tracked:** solid frame, and a velocity leader once ships in transit exist.
3. **Classified:** the icon appears ("medium hauler").
4. **Identified:** the frame takes the owner's affiliation shape and colour.
5. **Re-assessed:** the icon can still change, for example a hauler found to be armed.

Steps can come in other orders (a ship's transponder may give the owner before the class). The rule is only that each part of the symbol reflects what the faction knows about that part.

## 8. Labels and paths

- **Label:** name in the label face to the right of the symbol, then one line in the data face summarising group make-up, embarked troops, civilian or requisitioned status, and track age.
- **Candidate:** a fuller data block (ETA, propellant) for own task forces and the hovered or selected object, hidden elsewhere at low zoom to limit clutter.
- **Paths:** planned paths dashed in the affiliation colour; flown arcs solid (already the convention in the sketches).
- **Missiles in flight (candidate, 2026-10-15):** a launched missile is an uncrewed ship in the combat simulation, but drawn far smaller: a solid affiliation shape (circle, diamond, quatrefoil) of about 8 px with no frame or icon, a velocity leader and a salvo count. Affiliation still reads in monochrome, and a missile never reads as a ship. A salvo is one mark. See *Combat View* 3 and mock-up 17.
- **Simpler fallback (candidate):** if full frames prove too busy, drop to the older NTDS naval radar style (the same affiliation shapes, small, with no icon inside) at low zoom, and show icons from mid zoom.
- Panels, windows and reports: see *Interface Style* (`ui-style.md`).

## 9. Open questions

- Whether allies need their own look once diplomacy is designed (section 2).
- Whether stations show defences (an armed naval base). The filled-icon rule is for ships; stations might need an amplifier instead.
- Whether neutral green and hostile red work for colour-blind players alongside the shapes. Red and orange are close for some players; the frame shape carries the difference.
- Warnings share orange with unknown contacts. Probably fine, since a new unknown contact usually is the warning.
- How many icon types before they stop being learnable. Aim for under a dozen.
- Whether the paler civilian blue is too close to the off-white used for bodies and text. Unfilled circle versus filled disc should still tell them apart.
- Whether ships of faction-owned organisations count as faction or civilian. Probably civilian unless requisitioned: ownership isn't command.
- Own-flag civilian stations (a privately owned yard) would follow the same rule: circle, no fill.

## Rendering the mock-up

`7-symbology.html` builds every symbol from one function, `sym()`, so changes to the rules stay consistent. The sheet predates the opaque insides (SYM-03) and the group rules of SYM-09: it draws the frames translucent, puts the count nearer the frame, shows a commander's name in TF-1's label, and still has the loose-group stack and Convoy C-12, which are withdrawn (section 4). The spec decides. To regenerate the PNG, open the page in a browser at 1440 px wide and take a full-page screenshot (or use Playwright, pointing the fonts at `assets/fonts/`).

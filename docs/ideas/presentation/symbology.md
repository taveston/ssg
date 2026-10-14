# Orphan Worlds: Symbology

This doc sets out how ships, groups of ships and stations are drawn on the vector display, using NATO joint military symbology (APP-6 / MIL-STD-2525) as the reference. The main ideas:

- **Frame shape = affiliation, icon = what it is, amplifiers around the frame = state.** This is the NATO structure, kept because it reads without colour and scales to new ship types.
- Ships and stations get a frame; bodies, belts and salvage sites stay as plain markers. The frame is what tells "a ship" from "a place".
- The identification of a contact builds up on its symbol step by step, which ties the display to the uncertainty pillar.

**Decided** 2026-10-11, except where a point is marked otherwise. **Specced** in `docs/spec/system-view.md` as SYM-01 to SYM-15, GRP-09 and TEST-05 to TEST-06, with MRK-01, MRK-04, GRP-06, INS-05 and TEST-03 reworded and new tunables. Mock-up: `mockups/7-symbology.png` (source `mockups/7-symbology.html`).

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
  - *Flag* (whose ship it is) sets the frame shape. Ships registered to the state's own organisations keep the circle; a foreign merchant has the square.
  - *Command* (does the state give it orders) sets the fill. State ships have a solid tint (about 30 %). Own-flag civilians have no fill and a paler blue, so they read as "ours, but not under orders". Other affiliations have a faint tint (about 16 %).
  - Requisitioning or chartering a civilian ship fills its frame in for as long as the state commands it, so the player sees the takeover on the map.
  - **Opaque insides (decided 2026-10-13, from F-123).** Fills are solid colours mixed into the ground, not transparency, and every frame covers what lies under it. See-through fills let orbit lines cross the icons and looked glassy, like a HUD rather than a plot. (Specced: SYM-03.)
  - **Sharp at screen size (decided 2026-10-13, from F-124).** Symbols are drawn as vectors on whole pixels; scaled bitmaps blurred beside the orbit lines. (Specced: SYM-16, SYM-17.)
- **Open:** whether "hostile" means at war, or just "a faction we treat as a threat". It should follow the state's declared stance, not the AI's intent, so the player never learns hidden intent from a colour.

## 3. Icons: what it is

Drawn inside the frame. **Filled icon = armed; outline icon = unarmed.** That one rule gives an at-a-glance threat read.

| Type | Icon | Basis |
|---|---|---|
| Warship | Filled triangle | Keeps the old ship marker as the combatant icon. |
| Fleet supply / tanker | Outline drop | NATO supply-of-fuel symbol. Propellant is the critical stock in this game. |
| Cargo ship | Outline crate (box with a centre line) | Hauler. |
| Troop transport | Unit box (the NATO land-unit frame) | The box contents and the echelon slot show the load (below). |
| Class unknown | "?" | NATO leaves the frame empty. A "?" is clearer to players, and keeps "empty transport" distinct. |

- **Troop transport load** follows what the state knows:
  - *Load unknown:* empty box with "?" in the echelon slot. The normal case for foreign transports, since you rarely know whether one is carrying troops.
  - *Known empty:* empty box, nothing above. Normal for own transports.
  - *Known embarked:* box with the infantry cross and an echelon mark (II battalion, III regiment, X brigade), or "?" if the size is unknown. With several formations aboard, the largest sets the mark.
  - The embarked formation uses the same symbol as on the planetary view (sketch board 4, "1st Arden Guard"), so it can be followed from ship to surface.
- An armed hauler gets the warship icon once it is known to be armed. The icon shows the role as observed, not the hull type.
- More icons later as types appear (survey ship, salvage tug, gate tender, courier). Keep each a simple shape that reads at about 10 px.

## 4. Groups of vessels

Two kinds, drawn differently because they mean different things:

- **Task force:** ships under one commander with shared orders. The NATO task-force bracket over the frame, a count at the upper right, and the icon of the most significant member (warship > transport > supply > cargo > unknown).
- **Loose group or convoy:** ships moving together without a formation. A second frame stacked behind, with a count.

- An escorted convoy is two symbols: the convoy stack and the escort task force. Merging them would hide the escort's strength.
- A group parked at a body folds into the body's pill as one entry, like any satellite.
- **Open:** whether unrelated ships that overlap on screen should also fold into a loose group automatically (like GRP folding). Not specced: groups come from game state only.

## 5. Stations

Stations use the affiliation frames with the NATO installation bar on top and no icon. This replaces the old hollow square, which would clash with the neutral frame.

- **Not specced (candidate):** role icons were sketched (naval base = anchor, shipyard = crane over a hull, habitat = dome, industrial site = sawtooth roof) and set aside for now. Later types might include extraction sites, gate stations, research institutes and sensor stations; avoid triangles and crosses, which are taken.
- **Candidate:** a site on a surface folds into its body like other satellites and appears on the planetary view with the same symbol. The planetary-view sketch (board 4) draws settlements as squares and extraction sites as triangles, which would clash; they would switch to station symbols.
- **Candidate:** dormant or abandoned stations use the stale treatment (dimmed), since what we know of them is old.

## 6. Amplifiers: state

Placed around the frame, never inside it, so the core symbol always reads the same.

- **Velocity leader:** a line from the frame in the direction of motion (NATO direction-of-movement indicator). Its length grows with the logarithm of speed, since transit speeds vary widely. Waits for ships in transit.
- **Condition bar** under the frame: own ships only; it turns orange below half. A group shows its mean.
- **Dashed frame:** the position is estimated, not a current observation (NATO uses dashed for anticipated).
- **Stale track:** dashed and dimmed; the age goes in the label.
- **Echelon** above the frame (troop transports).
- **Selection and focus** keep the white ring and corner brackets (CAM-22), sized to the frame.

## 7. Identification ladder

How a contact firms up as tracking improves. Each step adds to the symbol rather than swapping it, so the player sees what changed.

1. **Detected:** dashed quatrefoil with "?" (weak track, class and owner unknown). In time, inside its uncertainty region (not yet specced).
2. **Tracked:** solid frame, and a velocity leader once ships in transit exist.
3. **Classified:** the icon appears ("medium hauler").
4. **Identified:** the frame takes the owner's affiliation shape and colour.
5. **Re-assessed:** the icon can still change, for example a hauler found to be armed.

Steps can come in other orders (a ship's transponder may give the owner before the class). The rule is only that each part of the symbol reflects what the state knows about that part.

## 8. Labels and paths

- **Label:** name in the label face to the right of the symbol, then one line in the data face summarising group make-up, embarked troops, civilian or requisitioned status, and track age.
- **Candidate:** a fuller data block (ETA, propellant) for own task forces and the hovered or selected object, hidden elsewhere at low zoom to limit clutter.
- **Paths:** planned paths dashed in the affiliation colour; flown arcs solid (already the convention in the sketches).
- **Simpler fallback (candidate):** if full frames prove too busy, drop to the older NTDS naval radar style (the same affiliation shapes, small, with no icon inside) at low zoom, and show icons from mid zoom.
- Panels, windows and reports: see *Interface Style* (`ui-style.md`).

## 9. Open questions

- Whether allies need their own look once diplomacy is designed (section 2).
- Whether stations show defences (an armed naval base). The filled-icon rule is for ships; stations might need an amplifier instead.
- Whether neutral green and hostile red work for colour-blind players alongside the shapes. Red and orange are close for some players; the frame shape carries the difference.
- Warnings share orange with unknown contacts. Probably fine, since a new unknown contact usually is the warning.
- How many icon types before they stop being learnable. Aim for under a dozen.
- Whether the paler civilian blue is too close to the off-white used for bodies and text. Unfilled circle versus filled disc should still tell them apart.
- Whether ships of state-owned organisations count as state or civilian. Probably civilian unless requisitioned: ownership isn't command.
- Own-flag civilian stations (a privately owned yard) would follow the same rule: circle, no fill.

## Rendering the mock-up

`7-symbology.html` builds every symbol from one function, `sym()`, so changes to the rules stay consistent. To regenerate the PNG, open the page in a browser at 1440 px wide and take a full-page screenshot (or use Playwright, pointing the fonts at `assets/fonts/`).

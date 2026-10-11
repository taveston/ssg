# Orphan Worlds: Symbology & UI Style

This doc proposes how ships, groups of ships and installations are drawn on the vector display, using NATO joint military symbology (APP-6 / MIL-STD-2525) as the reference. The main ideas:

- **Frame shape = affiliation, icon = what it is, amplifiers around the frame = state.** This is the NATO structure, kept because it reads without colour and scales to new ship types.
- Ship symbols get a frame; bodies, belts and salvage sites stay as plain markers. The frame is what tells "a ship" from "a place".
- The identification of a contact builds up on its symbol step by step, which ties the display to the uncertainty pillar.

Everything here is **candidate** unless marked otherwise. Mock-up: `mockups/7-symbology.png` (source `mockups/7-symbology.html`). Nothing is specced yet; adopting it would change MRK-01 and MRK-04 (see *Conflicts with the spec*).

---

## 1. Why NATO symbology

- It suits the look: the strategic display already reads as the government's own instruments (*Presentation & Visual Style*, section 2). A staff plot or ship's combat information centre is the closest real model.
- It is a tested grammar. Each part of a symbol has one job, so new types slot in without redesigning the set.
- It works in monochrome. Affiliation is in the frame shape, so colour only reinforces it. This helps colour-blind players more than palette choices alone.
- Players of wargames and many strategy games already know the basics (blue circle friendly, red diamond hostile).

We adapt rather than copy. Full APP-6 has thousands of icons and dense text fields; we take its structure and a small icon set drawn for a fixed-size screen marker.

## 2. Frames: affiliation

Uses the NATO sea-surface frames. They are closed, compact shapes; the NATO space-domain frames are open arcs that read poorly at small sizes, and in this game every ship is in space, so a domain mark would carry no information.

| Affiliation | Frame | Colour | Notes |
|---|---|---|---|
| Own | Circle | Blue #5AA9E6 | As today (MRK-01). |
| Neutral (and allied, for now) | Square | Green #8CCB8A | New colour. |
| Hostile | Diamond | Red #EE6A5F | New colour. Matches today's diamond for foreign contacts. |
| Unknown | Quatrefoil | Orange #F0A040 | Today's "foreign contact" orange, standing in for NATO yellow. |

- Frames are outlines with a faint tint fill (about 16 %) so they stand out on the dark ground without becoming solid blobs.
- **Allies (decided for now, 2026-10-11):** allied ships use the neutral frame. The important split for the player is "mine to command" versus "not mine". NATO itself puts allies in the friend frame. If allies later need to stand apart from neutrals, they need a cue other than the unfilled circle, which own-flag civilians now use (below).
- **Own-flag versus foreign shipping (candidate):** two separate questions, shown by two separate cues.
  - *Flag* (whose ship it is) sets the frame shape. Ships registered to own organisations are own flag, so they keep the circle. A foreign neutral merchant has the square.
  - *Command* (does the state give it orders) sets the fill. State ships have a solid tint. Own-flag civilians have no fill and a paler blue (#A9CBE3), so they read as "ours, but not under orders".
  - Requisitioning or chartering a civilian ship fills its frame in for as long as the state commands it, so the player sees the takeover on the map.
  - The armed rule still holds: a licensed armed hauler gets the filled warship icon in an unfilled frame.
  - The state tint is stronger (about 30 %) than for other affiliations, so filled and unfilled circles stay distinct at 22 px.
- **Open:** whether "hostile" means at war, or just "a faction we treat as a threat". It should follow the state's declared stance, not the AI's intent, so the player never learns hidden intent from a colour.

## 3. Icons: what it is

Drawn inside the frame. **Filled icon = armed; outline icon = unarmed.** That one rule gives an at-a-glance threat read.

| Type | Icon | Basis |
|---|---|---|
| Warship | Filled triangle | Keeps today's ship marker (MRK-04) as the combatant icon. |
| Fleet supply / tanker | Outline drop | NATO supply-of-fuel symbol. Propellant is the critical stock in this game. |
| Cargo ship | Outline crate (box with a centre line) | Hauler. |
| Troop transport | Unit box (the NATO land-unit frame) | What is aboard is shown by the box contents and the echelon slot above the frame (below). |
| Type unknown | "?" | NATO leaves the frame empty. A "?" is clearer to players, and keeps "empty transport" distinct. |

- **Troop transport load** follows what the state knows, like the rest of the symbol:
  - *Load unknown:* empty box with "?" in the echelon slot. This is the normal case for foreign transports, since you rarely know whether one is carrying troops.
  - *Known empty:* empty box, nothing above. Normal for own transports.
  - *Known embarked:* box with the infantry cross and an echelon mark. If troops are known to be aboard but not how many, the echelon slot shows "?".
  - The embarked formation uses the same symbol as on the planetary view (sketch board 4, "1st Arden Guard"), so it can be followed from ship to surface.
- The echelon mark shows the size of the embarked force: II battalion, III regiment, X brigade. If several formations are aboard, show the largest and list the rest in the data block.
- An armed hauler gets the warship icon once it's known to be armed. The icon shows the role as observed, not the hull type.
- More icons later as types appear (survey ship, salvage tug, gate tender, courier). Keep each a simple shape that reads at about 10 px.

## 4. Groups of vessels

Two kinds, drawn differently because they mean different things:

- **Task force:** ships under one commander with shared orders. The NATO task-force bracket over the frame, a count to the right, and the icon of the most significant member (warship > transport > supply > cargo).
- **Loose group or convoy:** ships moving together without a formation (for example haulers with no escort, or several tracks that overlap on screen). A stacked second frame offset behind, with a count.

- An escorted convoy is two symbols: the convoy stack and the escort task force, each with its own leader. Merging them would hide the escort's strength.
- Ships parked at a body still fold into the body's count pill (GRP). These group symbols are for ships in transit.
- **Open:** whether loose groups form automatically from screen overlap (like GRP folding) or only from game state. Probably both: game-state groups always, plus overlap folding for unrelated ships.

## 5. Installations

Stations and surface sites use the same affiliation frames with the NATO installation bar on top, and an outline icon for their role.

| Type | Icon | Notes |
|---|---|---|
| Station, general | None (bar only) | An orbital structure with no specific role. |
| Naval base | Anchor | Fleet anchorage, repair, rearming. NATO's naval symbol. |
| Shipyard | Crane over a hull | Pre-collapse yards; often dormant. |
| Habitat | Dome | Pressurised settlement, in orbit or on a surface. |
| Industrial site | Sawtooth factory roof | Ore processing, refining, foundries. |

- A site on a surface folds into its body like other satellites (GRP), and appears on the planetary view with the same symbol.
- An installation with several roles shows its main one; the rest go in the inset.
- Dormant or abandoned installations could use the stale-track treatment (dimmed), since what we know of them is old.
- Later: extraction sites (mines, ice quarries), gate stations, research institutes, sensor stations. Avoid triangles and crosses, which are taken by warships, infantry and salvage.

## 6. Amplifiers: state

Placed around the frame, never inside it, so the core symbol always reads the same.

- **Velocity leader:** a line from the frame in the direction of motion, longer when faster (NATO direction-of-movement indicator). Drawn in screen space from the projected velocity.
- **Condition bar** under the frame: own ships only, showing hull or readiness; it turns orange below half.
- **Dashed frame:** not a current observation, meaning an estimated or planned position (NATO uses dashed for planned or anticipated).
- **Stale track:** dashed and dimmed; the age goes in the data block.
- **Echelon** above the frame (troop transports).
- **Installation bar:** stations use the same affiliation frames with a small bar on top (NATO installation). This replaces today's hollow square, which would otherwise clash with the neutral frame. Role icons are in section 5.
- **Selection and focus** keep today's white ring and corner brackets (CAM-22), sized to the frame.

## 7. Identification ladder

How a contact firms up as tracking improves. Each step adds to the symbol rather than swapping it, so the player sees what changed.

1. **Detected:** dashed quatrefoil, no icon, inside its uncertainty region.
2. **Tracked:** solid frame and a velocity leader.
3. **Classified:** icon appears ("medium hauler?").
4. **Identified:** the frame takes the owner's affiliation shape and colour.
5. **Re-assessed:** the icon can still change, for example a hauler found to be armed.

Steps can come in other orders (a ship's transponder may give the owner before the class). The rule is only that each part of the symbol reflects what the state knows about that part.

## 8. Data blocks and UI style

- **Data block:** a short text block to the right of the symbol, like an air-traffic or naval plot: name in Barlow, then one line of JetBrains Mono (contents, ETA, track age). Shown for own task forces and selected or hovered objects; hidden elsewhere at low zoom to limit clutter.
- **Paths:** planned paths dashed in the affiliation colour; actual flown arcs solid (already the convention in the sketches).
- **Panels** stay as today: near-black, translucent, thin pale-blue borders, small uppercase section headings. The look is a well-kept government plotting console, not a sci-fi hologram.
- **Simpler fallback:** if full frames prove too busy, drop to the older NTDS naval radar style (the same affiliation shapes, but small and with no icon inside) at low zoom, and show icons from mid zoom. This could follow the same unfold thresholds as GRP.

## 9. Conflicts with the spec

Adopting this would change built requirements:

- **MRK-01 (palette):** adds neutral green and hostile red; orange narrows from "foreign contacts" to "unknown affiliation" (and warnings).
- **MRK-04 (shapes):** "filled triangle = own ship, filled diamond = foreign contact, hollow square = station" becomes the frame system above. The disc for bodies and the X for salvage stay.
- **Planetary view sketch** (board 4): settlements as squares and extraction sites as triangles would clash with the neutral frame and the warship icon. They would switch to installation symbols. Not specced yet, so nothing built changes.
- **Marker size:** framed symbols need about 22 px against today's 14 px (see the mock-up's size row). Probably a new tunable, `ship_symbol_px`.

## 10. Open questions

- Whether allies need their own look once diplomacy is designed (section 2).
- Whether installations show defences (an armed naval base, a fortified habitat). The filled-icon rule is for ships; installations might need an amplifier instead.
- Whether neutral green and hostile red work for colour-blind players alongside the shapes, or need adjusting. Red and orange are close for some players; the frame shape carries the difference.
- Warnings currently share orange with unknown contacts. Probably fine, since a new unknown contact usually is the warning.
- How many icon types before they stop being learnable. Aim for under a dozen.
- Whether the paler civilian blue is too close to the neutral off-white used for bodies and text. Unfilled circle versus filled disc should still tell them apart.
- Whether state-owned organisations' ships (owned by a company the state owns, but not navy) count as state or civilian. Probably civilian unless requisitioned: ownership isn't command.
- Own-flag civilian installations (a privately owned yard or refinery) would follow the same rule: circle, no fill.
- Leader length scale: linear or logarithmic in speed, given transit speeds vary widely.

## Rendering the mock-up

`7-symbology.html` builds every symbol from one function, `sym()`, so changes to the rules stay consistent. To regenerate the PNG, open the page in a browser at 1440 px wide and take a full-page screenshot (or use Playwright, pointing the fonts at `assets/fonts/`).

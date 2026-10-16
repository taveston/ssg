# Orphan Worlds: Overview

*Hard sci-fi space strategy for PC.* *Status:* concept.

**In short:** you lead a successor faction after the fall of a gate-linked human civilisation. You rebuild and defend the failing gate network through ministers, officers and companies with their own agendas. Time runs continuously at about a day per second. Ships come from bids by organisations, and research means reverse engineering what the old age left. Combat is a small, mostly auto-resolved part of play.

This is the overview. Each system has its own doc, listed in section 4; detail and open questions live there. What is committed for building is in `docs/spec/`.

Status tags, used in every idea doc: **Decided** = settled. **Direction** = settled in principle, details open. **Candidate** = under consideration.

---

## 1. Vision

- **Pitch (draft):** lead a successor faction of a shattered human civilisation as it rebuilds and defends its gate network and institutions across a hard sci-fi sector. Distance, orbits and the ambitions of your own people shape every decision.
- **Elevator pitch (draft):** Earth's gate network once tied humanity together. Now Earth is gone, the network is failing and the shipyards are idle. You head a successor government with a few hostile worlds, some surviving institutions and the wreckage of a vanished age. You govern through ministers, officers and companies with their own agendas. You commission ships from bidders whose designs never turn out quite as planned. And you push through the gates to reclaim what was built before the collapse.
- **Player role (decided):** always head of government; no election or legislature can remove them. Probably not an in-world character. Other factions are led by real characters.
- **Player fantasy (draft):** the person at the top of a struggling faction, not the hero in the cockpit. You decide what to rebuild, whom to trust and what to risk.
- **Pillars (decided):**
  1. **Grounded space:** plausible technology. Orbital mechanics shape strategy, presented accessibly.
  2. **Rebuild a broken order:** salvage, re-open and defend what the old network left.
  3. **Rule through people and institutions:** characters and organisations carry out the player's will, with ambitions of their own.
  4. **Uncertainty over optimisation:** designs come from bids and vary, so players adapt rather than min-max.
- **Rejected pillar:** "Logistics decide wars".
- **References:** Terra Invicta, Crusader Kings, Stellaris; Shadow Empire for design bids; Defcon for combat feel.

## 2. Game structure (decided)

- **Time:** continuous real time. By default about 1 real second is 1 game day, so a year is about 6 minutes and a decade about an hour. Events on a scale of minutes or hours effectively pause the game. The player has full control while paused. All orders are delegated; there is no direct real-time control.
- **Scale:** a sector of 30 (Small), 80 (Medium) or 200 (Large) gated systems, among about three times as many stars that have no gate. Total human population is in the hundreds of thousands to millions. That makes invasion plausible; it doesn't mean few characters.
- **Match:** long sandbox games on a generated map. The external threat is a possible end-game.

## 3. Core loop (draft)

Combat is a small part of play. Most time goes to running the faction.

- **Moment to moment:** alerts arrive (reports, arrivals, bids, requests). You pause, review, give orders and resume.
- **Session:** assess, then decide: commission designs, allocate scarce resources, appoint people, fund organisations, send expeditions, dispatch convoys and fleets, choose gates to re-open or defend. Wait weeks to months, see the consequences, adjust.
- **Campaign arc:** stabilise the starter worlds and salvage stocks. Reactivate the dormant yards and build the extraction and industry to feed them. Re-open and secure gates and expand. Reverse engineer recovered tech. Uncover the Earth mystery and face hints of the external threat.
- **Tension:** the decay clock, scarce components and people, long delays, uncertainty, the agendas of characters and organisations, rival factions, the external threat.

## 4. Systems

One line per system and the doc that holds it. All paths are under `docs/ideas/`.

| System | In brief | Doc |
|---|---|---|
| Setting | Earth, hub of the gate network, is presumed destroyed; colonies formed successor factions. The cause is a mystery; an external threat looms. | `world/setting.md` |
| Economy | Few concrete resources, one currency that allocates but doesn't create, a private economy of organisations. Salvage first, then extraction. | `mechanics/economy.md` |
| Characters, organisations and events | Skills and opposing traits, partly hidden. The player appoints senior posts only. No canned events. | `mechanics/people.md` |
| Factions and ideology | Factions differ mechanically. A triangle of Cohesion, Authority and Autonomy, set by a three-phase history. | `mechanics/factions.md` |
| Requirements and bids | The player sets requirements; organisations bid with concrete designs whose as-built figures are revealed over time. | `mechanics/requirements-and-bids.md` |
| Salvage and research | No tech tree: salvage finds, reverse engineering, understanding per component type, designs as objects. | `mechanics/research.md` |
| Components | Component types and tiers; the stats they carry and how they assemble into ships. | `mechanics/components.md`, `mechanics/component-stats.md` |
| Drives and transit | Drive families, transit times, transfer windows, gate placement. | `mechanics/drives-and-transit.md` |
| Orbital transfers | Ships on rails along precomputed patched-conic arcs; two-slider transfer planning; scheduled and re-taskable orders. | `mechanics/transfers.md` |
| Gates | Paired wormhole mouths, slowly failing; most links already collapsed. How gates work stays unexplained on screen. | `mechanics/gates.md` |
| World generation | A flat 3D sector at realistic star density, gates where the old network had reason to go, links most of which have failed, and a historical layer. | `mechanics/world-generator.md` |
| Combat, raiders and diplomacy | Combat simulated in full when it starts and revealed as a replay on its own clock; raiders from simulated conditions. | `mechanics/conflict.md`, `mechanics/combat-simulation.md` |
| Presentation | Sector, system and planet layers as a vector display at true scale, semi-photoreal inspection, a separate administration view. | `presentation/presentation.md`, `presentation/symbology.md`, `presentation/ui-style.md`, `presentation/combat-view.md` |

## 5. Not yet started

- **Player experience:** UI detail beyond the view structure in *Presentation & Visual Style*, onboarding, difficulty, multiplayer. (Unrestricted pausing suits single-player.)
- **Art and audio:** anything beyond the visual direction in *Presentation & Visual Style*.
- **Technical approach:** simulation load across 200 systems, AI.
- **Scope:** MVP, milestones, team, budget.

## 6. Risks

| Risk | Mitigation ideas |
|---|---|
| Scope: characters, orbits, economy and bids are each large | Pick the core system early; define an MVP |
| Bids feel arbitrary | Track records, confidence ranges, surprises revealed in trials |
| Late game stalls | Decay clock, external threat, gate effort |
| Generated maps feel unfair | Generation rules and fairness checks early |
| Economy too complex | Few, concrete resources |
| Time acceleration breaks orbital timing | Auto-slowdown, alerts, queued orders |
| Auto-resolved combat feels detached | Pre-battle decisions, partial information, real stakes, after-action reveal |
| Spreadsheet feel | Tie decisions to people and organisations; clear reports |
| Bland without canned events | Specific names and histories, memory of decisions, a few authored beats |
| True-scale system view cluttered or unreadable | Fixed-size markers, grouping, density shading, switchable layers, focus bookmarks |
| Vector display feels cold or abstract | Photoreal inspection views of everything; renders that reflect designs and history |

## 7. Open questions

Questions about one system are in that system's doc. These cut across the game:

- **Vision:** stewardship or conquest; what to take from the references and what to avoid; audience and session length.
- **Structure:** win and lose conditions; late-game engagement; factions and bodies per system; speed settings and auto-pause rules.

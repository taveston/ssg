# Orphan Worlds: Design Document

*Hard sci-fi space strategy for PC*  |  *Last updated:* 2026-10-09  |  *Status:* Concept

Status tags: **Decided** = settled. **Direction** = settled in principle, details open. **Candidate** = under consideration. Open questions are in section 9, shelved ideas in section 8. Companion documents (paths under `docs/ideas/`): *Setting & Back Story* (`world/setting.md`), *Faction History Options*, *Component Catalogue*, *Component Stats & Ship Assembly*, *Requirements & Bids*, *Drives & Transit*, *World Generator* (all in `mechanics/`), *Presentation & Visual Style*, *Reference Sketches* and *Symbology & UI Style* (`presentation/`), *ideology-triangle.svg* (diagram, `mechanics/`). What is committed for building is in `docs/spec/`; see `docs/README.md`.

---

## 1. Vision

- **Pitch (draft):** Lead a successor state of a shattered human civilisation as it rebuilds and defends its gate network and institutions across a hard sci-fi cluster, where distance, orbits, and the ambitions of your own people shape every decision.
- **Elevator pitch (draft):** Earth's gate network once tied humanity together; now Earth is gone, the network is failing and the shipyards idle. You head a successor government with a few hostile worlds, some surviving institutions, and the wreckage of a vanished age. Govern through ministers, officers, and companies with their own agendas, commission ships from bidders whose designs never turn out quite as planned, and push through the gates to reclaim what was built before the collapse.
- **Player role (decided):** Head of government, always; no election or legislature can remove them. Probably not an in-world character. Other factions are led by real characters.
- **Player fantasy (draft):** The person at the top of a struggling state, not the hero in the cockpit: deciding what to rebuild, whom to trust, and what to risk.
- **Pillars (decided):**
  1. **Grounded space:** plausible technology; orbital mechanics shape strategy, presented accessibly.
  2. **Rebuild a broken order:** salvage, re-open, and defend what the old network left.
  3. **Rule through people and institutions:** characters and organisations carry out the player's will, with ambitions of their own.
  4. **Uncertainty over optimisation:** designs come from bids and vary; players adapt rather than min-max.
- **References:** Terra Invicta, Crusader Kings, Stellaris (plus Shadow Empire for design bids, Defcon for combat feel).

## 2. Game Structure (decided)

- **Time:** Continuous real-time, default about 1 real second = 1 game day (a year ≈ 6 minutes, a decade ≈ 1 hour). Minute- or hour-scale events effectively pause the game. Full control while paused; all orders are delegated, never direct real-time control.
- **Scale:** A cluster of 10 to 100 systems; total human population in the hundreds of thousands to millions (makes invasion plausible; does not imply few characters).
- **Match:** Long sandbox games on a generated map; the external threat is a possible end-game.

## 3. Core Loop (draft)

Combat is a small part of play; most time goes to running the state.

- **Moment to moment:** alerts arrive (reports, arrivals, bids, requests); pause, review, order, resume.
- **Session:** assess → decide (commission designs, allocate scarce resources, appoint people, fund organisations, send expeditions, dispatch convoys and fleets, choose gates to re-open or defend) → wait weeks to months → see consequences → adjust.
- **Campaign arc:** stabilise starter worlds and salvage stocks → reactivate the dormant yards and build extraction and industry to feed them → re-open and secure gates and expand → reverse engineer recovered tech → uncover the Earth mystery and face hints of the external threat.
- **Tension:** decay clock, scarce components and people, long delays, uncertainty, agendas of characters and organisations, rival states, the external threat.

## 4. Systems

### 4.1 Economy

- **Start (decided):** Starter worlds support human life; most people live on hostile surfaces. Factions are just venturing back into space: there are no interplanetary fleets or trade yet. Pre-collapse orbital shipyards, owned by organisations, sit dormant or barely maintained and need asteroid raw material.
- **Resources (decided):** few and concrete.
  - **Water (ice):** life support, reaction mass, shielding, trace deuterium (a deliberate double-duty trade-off).
  - **Metals:** iron-nickel, aluminium, titanium.
  - **Rare metals:** platinum-group, rare earths; scarce and localised.
  - **Components:** machinery, electronics, spares; the bottleneck for small populations.
  - Helium-3 exists but harvesting it is not a major mechanic.
- **Non-material tracks (candidate):** energy capacity, skilled personnel, influence. Recovered tech is handled as discrete finds, not a pool.
- **Logistics (candidate):** bulk goods mostly move within a system; dense high-value goods justify gate transport.
- **Currency (decided):** one shared currency; conversion abstracted. *Candidate model:* balances and flows with no money supply or inflation; local prices per system from supply, demand, and transport; gate tolls as a tax point; bids priced in money; optional state debt later. Money allocates but doesn't create, so scarcity stays physical. Influence is separate.
- **Private economy (direction):** organisations produce, trade, hire, and compete, visible to the player but outside direct control.
- **Ownership (direction):**
  - State-ownable: military bases and ships, gates, settlements, basic infrastructure.
  - Organisation-only: shipyards, mines, refineries, foundries, factories, shipping, research institutes. Organisations may themselves be state-owned.
  - Military assets are plain state assets under admirals; their political weight comes via a defence portfolio or military bloc.
- **Civilian shipbuilding (decided):** yards have order books and finite capacity, so the state competes with private orders. Requisition in emergencies at a large cost in influence, reputation, relations, or opinion (direction). Licensing or forbidding armed haulers (candidate). Organisations too big or distant to police, up to breakaway (later).
- **Backbone arc (decided):** salvage → extraction, with re-opening, maintaining, and defending gates as the main reason to be in space. Unmaintained gates still work at reduced throughput or reliability.
- **Expansion drivers (candidates, more needed):** position near gates, buffers against rivals, decay clock, depleting legacy stocks, tech in abandoned sites, population caps on hostile worlds, organisation ambitions, legitimacy. Mostly small outposts; full colonies for special worlds. Costs: supply lines, defence, organisation power, instability.

### 4.2 Map, Gates & Orbits (decided)

- Generated map. Several gates per system, one per destination, at the system edge, each linking to a small handful of nearby systems. Network largely fixed at start. How gates work is deliberately unexplained.
- Edge placement means weeks-to-months transits to inner bodies. Links out of the cluster could be dark (candidate). Exact gate distance may need tuning (section 9).
- Orbits realistic but accessible: at this drive performance, transfer windows soften into a trade-off between time, delta-v, and timing.

### 4.3 Characters, Organisations & Events

- **Characters (direction):** people within the state (governors, commanders, company heads, scientists), not a dynasty. To avoid copying Crusader Kings 3: traits act as behavioural rules rather than stat bundles, with hidden information; key figures get individuality from goals, history, and relationships; minor characters stay light.
  - **Skills (candidate):** Administration, Industry, Science, Command, Diplomacy, 1 to 10. How well a character does things.
  - **Traits (decided for now):** opposing pairs (Ambitious/Content, Bold/Cautious, Principled/Opportunistic, Innovative/Traditionalist, possibly Ruthless/Humane); at most one per pair (none = average), usually two or three; mostly fixed. What a character chooses to do. Extra pairs if a system needs them (Idealist/Pragmatist, Secretive/Forthcoming, Frugal/Extravagant, Territorial/Cooperative).
  - **Ideology (candidate):** characters hold a position in the same ideology triangle as factions (4.7), with the same pool of 12 and the same labels ("a paternalist governor"). Differences between a character, the population, and government policy create friction.
  - **Relationships (candidate):** opinion, grievances, debts; separate from personality; the memory of how the player treated someone.
  - **Uncertainty (candidate):** skills known roughly, traits revealed through behaviour; UI must separate "average" from "unknown".
- **Roles (direction):** the player appoints only senior posts: cabinet, leaders of colonies, settlements, bases and stations, admirals, captains, ground formation leaders. Organisation staff are not appointed by the player unless state-owned. Candidates: elected posts depending on settlement type and civic alignment; a legislature as a few bloc leaders.
- **Organisations (direction):** yards, mining and shipping firms, institutes, often pre-collapse remnants. Subjects of the faction but independent actors that drive the private economy; very powerful ones might break away (later). The player works through policy, contracts, favour, and pressure.
- **Direct-control principle (candidate):** the player can always take direct control at a price (efficiency penalty, expenditure, or relations cost), with requisition and state ownership at the far end.
- **Events (direction):** no canned pop-up events. Instead: requests and problems arising from simulated state and agendas; reports from real data; choices that differ in kind, with uncertain or delayed consequences; lore as optional recovered records. A few authored beats for the Earth mystery and external threat only (decided).

### 4.4 Design by Requirements & Bids (decided)

- The player sets requirements (role, mass, delta-v, crew, budget, deadline); affiliated organisations bid with concrete designs (Shadow Empire-style).
- Designs persist and can be upgraded.
- Alternatively, design and produce through state-owned organisations, trading capacity, management attention, variety, and goodwill.

### 4.5 Salvage & Research

- **Salvage (direction):** generated sites (wrecks, stations, yards, ruins) with partly known contents. Yields: bulk material (a finite legacy stock), discrete tech finds, lore records, relics. Surveys give rough estimates; expeditions take weeks to months under a leader whose skills and traits matter; sites deplete; richer sites lie further out or behind dark gates. Organisations may salvage independently; the state can license, tax, claim, or requisition.
- **Research (direction):** no tech tree or research points. Populations are too small for frontier research, so progress means reverse engineering.
  - Know-how is held by organisations and state labs, per component type, gained by studying finds and through production and use.
  - Finds can be studied (maybe destroyed) or used; documentation and tooling speed reproduction.
  - Reproduction is a fidelity gradient: early copies are heavier, less efficient, or less reliable, and improve with experience, samples, staff, and materials.
  - Capability is local: it spreads by licensing or poaching and can be lost with an organisation or its people.
  - The player directs by funding projects, allocating finds, and licensing.
- **Two design levels (decided):** component designs (mostly pre-collapse and reverse engineered; see *Component Catalogue*) and ship designs (assembled from components by organisations through bids).
- **Designs as objects (direction):** each has a documentation level (fragmentary → complete), owners and rights, and a version lineage. Every organisation and faction keeps an archive, with sharing, licensing, and theft.
  - **Operationalisation:** an organisation works out how to build a design with its own tooling, materials, and staff, producing an as-built version; output varies and improves with practice. Fragmentary designs may be unbuildable until raised.
  - **Transfer:** copying records is cheap but the receiver must still operationalise; poaching people moves know-how; captured ships can be studied.
- **Understanding levels (candidate):** organisations turn staff, facilities, and character skill into research output that raises their understanding of a component type; understanding sets the quality of designs they can produce, and reverse engineering feeds it. Refinements: understanding can fill gaps in fragmentary designs; general investment is scaled down by a "theoretical research capacity" factor so invention is possible but impractical; decay without practice or when staff leave; soft spillover between related types; organisations track only types in their sector.
- **Priorities (direction):** organisations set their own; the player nudges via funding, finds, and contracts, or takes direct control at a cost. Levels vs. point counters is a UI question for later.
- **Knowledge sharing (candidate):** three layers: documents (copyable), tooling (physical), tacit know-how (stays with teams). The state can fund on terms that give it the data, classify results, license to several organisations, or keep state labs. Sharing spreads capability but leaks; secrecy concentrates know-how in organisations that can fail or hold the state to ransom. Documentation projects convert tacit know-how into data at a cost.

### 4.6 Combat, Raiders & Diplomacy

- **Combat (direction):** lightly simulated, essentially auto-resolved. Engagements happen at extreme velocity and range; the player sets a posture, has few levers during the fight, and watches blips merge on a radar-like display (Defcon-style tension). The display is the ordinary system view with the clock slowed (see *Presentation & Visual Style*).
- **Raiders (direction):** arise from simulated conditions, not random spawns. Origins: scavengers contesting salvage, failed organisations, state-sponsored privateers (most plausible early), smugglers dodging tolls. Chokepoints and predictable trajectories invite interdiction. Delays make age-of-sail analogies apt: standing orders, convoys, blockades, chartered companies, privateers.
- **Diplomacy (decided):** AI leaders change through elections, coups, and death, so relations shift over a long game.

### 4.7 Factions

- **Differentiation (direction):** mechanical, not hand-crafted backstory. Other differentiators (candidate): geography and gates, starting organisations, seeded designs and understanding, government form, leader traits.
- **Specced:** the ideology profile (pool of 12, labels, events) is in `docs/spec/ideology.md`.
- **Popular ideology (direction):** a position in a triangle of three forces: **Cohesion** (shared ownership and burdens), **Authority** (central direction), **Autonomy** (private initiative). See *ideology-triangle.svg*.
  - **Representation (decided):** the three scores always add up to 12, so the numbers are the position ("Coh 6 · Auth 4 · Auto 2"). Shown to one decimal; stored as integers 0 to 120. Starting scores come from faction history (each option distributes 4 points).
  - **Events (decided):** an event changes one score and the other two absorb the change in proportion to their size, so the faction moves in a straight line towards or away from that corner. Events may also name both sides ("Autonomy → Authority 2").
  - **Ideology labels (decided):** a score of 7+ gives a corner label; otherwise a score of 3 or less puts the faction on the edge opposite that corner; otherwise Pluralist. Corners cover about 17% of the triangle each, edges about 14%, Pluralist about 6%; random histories give an even spread of labels except Pluralist. Each label works as an adjective for a state and a noun for a person:

    | Region | Rule | Label | Meaning |
    |---|---|---|---|
    | Cohesion corner | Coh 7+ | Collectivist | Shared ownership and burdens above all |
    | Authority corner | Auth 7+ | Authoritarian | Central direction above all |
    | Autonomy corner | Auto 7+ | Individualist | Private initiative above all |
    | Cohesion + Authority | Auto 3 or less | Paternalist | The state directs shared resources for the common good |
    | Authority + Autonomy | Coh 3 or less | Meritocratic | Free individuals striving within a firm order the state maintains |
    | Cohesion + Autonomy | Auth 3 or less | Mutualist | Self-organising communities and cooperatives with a light centre |
    | Centre | Otherwise | Pluralist | No dominant force; politics is a negotiation |

  - **Summary line:** ideology label, then Outlook and Militancy, for example "Meritocratic, outward, strongly militant". All five profile numbers span 12 points (Outlook and Militancy are capped at ±6).
  - Possible effects: Cohesion cheapens state ownership and strengthens state organisations but shrinks the private sector; Authority cheapens direct control but strains organisations and raises unrest; Autonomy boosts the private economy but makes direct control costlier and revenue weaker.
  - Distinct from government form (how decisions are made) and actual policy; a gap between ideology and policy causes friction. May vary by settlement and drift over time. Characters may hold positions in the same triangle (4.3).
- **Extra axes (direction):** **Outlook** (Insular/Outward: contact, trade, diplomacy) and **Militancy** (Pacifist/Militant: readiness, mobilisation, willingness to use force, the military's political weight), each −6 to +6. Every position should be a different way to play, not a handicap.
- **Faction history (direction):** the player picks a history in three phases (origins before the collapse, turmoil during it, recovery of social and political order after it) and the profile emerges. AI factions draw histories too, with weighted probabilities instead of strict balance. Draft options and the full ideology rules are in *Faction History Options*.

## 5. Setting & Narrative (decided)

Moved to `world/setting.md`: tone, premise, timeline, the Earth mystery, the external threat and the technology baseline.

## 6. Not Yet Started

Player experience (UI detail beyond the view structure in *Presentation & Visual Style*, onboarding, difficulty, multiplayer; unrestricted pausing suits single-player); art beyond the visual direction in *Presentation & Visual Style*, and audio; technical approach (generation, simulation load at 100 systems, AI, engine); scope (MVP, milestones, team, budget).

## 7. Risks

| Risk | Mitigation ideas |
|---|---|
| Scope: characters, orbits, economy, and bids are each large | Pick the core system early; define an MVP |
| Bids feel arbitrary | Track records, confidence ranges, surprises revealed in trials |
| Late game stalls | Decay clock, external threat, gate effort |
| Generated maps feel unfair | Generation rules and fairness checks early |
| Economy too complex | Few, concrete resources |
| Time acceleration breaks orbital timing | Auto-slowdown, alerts, queued orders |
| Auto-resolved combat feels detached | Pre-battle decisions, partial information, real stakes, after-action reveal |
| Spreadsheet feel | Tie decisions to people and organisations; clear reports |
| Bland without canned events | Specific names and histories, memory of decisions, a few authored beats |
| True-scale system view cluttered or unreadable | Fixed-size markers, grouping, density shading, switchable layers, focus bookmarks |
| Vector display feels cold or abstract | Photoreal inspection views of everything, renders that reflect designs and history |

## 8. Parking Lot

- **Shelved:** regional specialisation (each state missing something); risks unbalancing factions.
- **Rejected:** "Logistics decide wars" as a pillar; Reverent/Pragmatic as a faction axis (a reverent faction would struggle in a salvage game).
- **Later:** feudal-style government as an optional twist; dark gates at the cluster edge; relic AI boosting research capacity.

## 9. Open Questions

- **Vision:** stewardship vs. conquest; what to take from and avoid in the references; audience and session length.
- **Structure:** win and lose conditions; late-game engagement; factions and bodies per system; speed settings and auto-pause rules.
- **Economy:** state income and spending; how the player influences and sees the private economy; costs and benefits of state ownership; bids for civilian designs; measuring influence, reputation, and opinion.
- **Map and gates:** fair starts; gate distance from the star and its effect on transit times; gate transit time and throughput; can gates be destroyed or captured; fog of war; what orbital detail is modelled.
- **Characters:** growth and ageing; goals and history; cabinet portfolios; what counts as senior; elections and legislature; talent pool and careers; how character ideology is set and changes.
- **Bids:** what varies and why; hidden vs. visible information; ground forces and stations; AI use; refit vs. replace.
- **Research:** legible summary of capability; studying vs. using finds; classification and leaks; operationalisation rules; size of the research-capacity factor.
- **Combat and raiders:** pre-battle orders; information limits; after-action reports; ground combat and invasion; making losses matter; suppressing raiders; privateer licences and prize shares.
- **Factions:** government forms and their link to history and ideology; ideology drift and typical event sizes; final history options and randomness; continuous vs. discrete extra axes; whether a research-oriented modifier is needed.
- **Setting:** threat hints and escalation, whether it can be switched off, why it hasn't arrived; hard sci-fi no-go list; delta-v range and reaction mass model.
- **Presentation:** see *Presentation & Visual Style*, section 6.

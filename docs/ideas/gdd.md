# Orphan Worlds: Design Document

*Hard sci-fi space strategy for PC.* *Status:* concept.

**In short:** you lead a successor state after the fall of a gate-linked human civilisation. You rebuild and defend the failing gate network through ministers, officers and companies with their own agendas. Time runs continuously at about a day per second. Ships come from bids by organisations, and research means reverse engineering what the old age left. Combat is a small, mostly auto-resolved part of play. Most systems below are at the direction or candidate stage; the open questions are in section 9.

Status tags: **Decided** = settled. **Direction** = settled in principle, details open. **Candidate** = under consideration.

Companion documents (under `docs/ideas/`): *Setting & Back Story* (`world/setting.md`); *Faction History Options*, *Component Catalogue*, *Component Stats & Ship Assembly*, *Requirements & Bids*, *Drives & Transit*, *World Generator* and the diagram *ideology-triangle.svg* (all in `mechanics/`); *Presentation & Visual Style*, *Reference Sketches*, *Symbology* and *Interface Style* (`presentation/`). What is committed for building is in `docs/spec/`.

---

## 1. Vision

- **Pitch (draft):** lead a successor state of a shattered human civilisation as it rebuilds and defends its gate network and institutions across a hard sci-fi cluster. Distance, orbits and the ambitions of your own people shape every decision.
- **Elevator pitch (draft):** Earth's gate network once tied humanity together. Now Earth is gone, the network is failing and the shipyards are idle. You head a successor government with a few hostile worlds, some surviving institutions and the wreckage of a vanished age. You govern through ministers, officers and companies with their own agendas. You commission ships from bidders whose designs never turn out quite as planned. And you push through the gates to reclaim what was built before the collapse.
- **Player role (decided):** always head of government; no election or legislature can remove them. Probably not an in-world character. Other factions are led by real characters.
- **Player fantasy (draft):** the person at the top of a struggling state, not the hero in the cockpit. You decide what to rebuild, whom to trust and what to risk.
- **Pillars (decided):**
  1. **Grounded space:** plausible technology. Orbital mechanics shape strategy, presented accessibly.
  2. **Rebuild a broken order:** salvage, re-open and defend what the old network left.
  3. **Rule through people and institutions:** characters and organisations carry out the player's will, with ambitions of their own.
  4. **Uncertainty over optimisation:** designs come from bids and vary, so players adapt rather than min-max.
- **References:** Terra Invicta, Crusader Kings, Stellaris; Shadow Empire for design bids; Defcon for combat feel.

## 2. Game structure (decided)

- **Time:** continuous real time. By default about 1 real second is 1 game day, so a year is about 6 minutes and a decade about an hour. Events on a scale of minutes or hours effectively pause the game. The player has full control while paused. All orders are delegated; there is no direct real-time control.
- **Scale:** a cluster of 10 to 100 systems. Total human population is in the hundreds of thousands to millions. That makes invasion plausible; it doesn't mean few characters.
- **Match:** long sandbox games on a generated map. The external threat is a possible end-game.

## 3. Core loop (draft)

Combat is a small part of play. Most time goes to running the state.

- **Moment to moment:** alerts arrive (reports, arrivals, bids, requests). You pause, review, give orders and resume.
- **Session:** assess, then decide: commission designs, allocate scarce resources, appoint people, fund organisations, send expeditions, dispatch convoys and fleets, choose gates to re-open or defend. Wait weeks to months, see the consequences, adjust.
- **Campaign arc:** stabilise the starter worlds and salvage stocks. Reactivate the dormant yards and build the extraction and industry to feed them. Re-open and secure gates and expand. Reverse engineer recovered tech. Uncover the Earth mystery and face hints of the external threat.
- **Tension:** the decay clock, scarce components and people, long delays, uncertainty, the agendas of characters and organisations, rival states, the external threat.

## 4. Systems

### 4.1 Economy

- **Start (decided):** the starter worlds support human life, but most people live on hostile surfaces. Factions are just venturing back into space: there are no interplanetary fleets or trade yet. Pre-collapse orbital shipyards, owned by organisations, sit dormant or barely maintained and need raw material from asteroids.
- **Resources (decided):** few and concrete.
  - **Water (ice):** life support, reaction mass, shielding and trace deuterium. Using one resource for all of these is a deliberate trade-off.
  - **Metals:** iron-nickel, aluminium, titanium.
  - **Rare metals:** platinum-group metals and rare earths; scarce and localised.
  - **Components:** machinery, electronics, spares. The bottleneck for small populations.
  - Helium-3 exists, but harvesting it isn't a major mechanic.
- **Non-material tracks (candidate):** energy capacity, skilled personnel, influence. Recovered tech is handled as discrete finds, not a pool.
- **Logistics (candidate):** bulk goods mostly move within a system. Dense, high-value goods justify gate transport.
- **Currency (decided):** one shared currency; conversion is abstracted.
  - *Candidate model:* balances and flows, with no money supply or inflation. Local prices per system come from supply, demand and transport. Gate tolls are a point of taxation. Bids are priced in money. State debt may come later.
  - Money allocates but doesn't create, so scarcity stays physical. Influence is separate.
- **Private economy (direction):** organisations produce, trade, hire and compete. The player can see this but not control it directly.
- **Ownership (direction):**
  - The state can own military bases and ships, gates, settlements and basic infrastructure.
  - Only organisations own shipyards, mines, refineries, foundries, factories, shipping and research institutes. An organisation may itself be state-owned.
  - Military assets are plain state assets under admirals. Their political weight comes through a defence portfolio or a military bloc.
- **Civilian shipbuilding (decided):** yards have order books and finite capacity, so the state competes with private orders.
  - In emergencies the state can requisition, at a large cost in influence, reputation, relations or opinion (direction).
  - Licensing or forbidding armed haulers (candidate).
  - Later: organisations too big or distant to police, up to breaking away.
- **Backbone arc (decided):** salvage first, then extraction. Re-opening, maintaining and defending gates is the main reason to be in space. Unmaintained gates still work, with lower throughput or reliability.
- **Expansion drivers (candidates; more needed):** position near gates, buffers against rivals, the decay clock, depleting legacy stocks, tech in abandoned sites, population caps on hostile worlds, organisations' ambitions, legitimacy. Mostly small outposts, with full colonies only for special worlds. Costs: supply lines, defence, the power of organisations, instability.

### 4.2 Map, gates and orbits (decided)

- The map is generated. Each system has several gates at its edge, one per destination, each linking to a small handful of nearby systems. The network is largely fixed at the start. How gates work is deliberately unexplained.
- Because gates are at the edge, reaching inner bodies takes weeks to months. Links out of the cluster could be dark (candidate). The exact gate distance may need tuning (section 9).
- Orbits are realistic but accessible. At these drive performances, transfer windows soften into a trade-off between time, delta-v and timing.

### 4.3 Characters, organisations and events

- **Characters (direction):** people within the state (governors, commanders, company heads, scientists), not a dynasty. To avoid copying Crusader Kings 3, traits are behavioural rules rather than stat bundles, and some information is hidden. Key figures get individuality from goals, history and relationships; minor characters stay light.
  - **Skills (candidate):** how well a character does things. Administration, Industry, Science, Command, Diplomacy, each 1 to 10.
  - **Traits (decided for now):** what a character chooses to do. Opposing pairs: Ambitious/Content, Bold/Cautious, Principled/Opportunistic, Innovative/Traditionalist, possibly Ruthless/Humane. At most one of each pair (none means average); usually two or three in all; mostly fixed. More pairs can be added if a system needs them (Idealist/Pragmatist, Secretive/Forthcoming, Frugal/Extravagant, Territorial/Cooperative).
  - **Ideology (candidate):** characters hold a position in the same ideology triangle as factions (4.7), with the same pool of 12 and the same labels ("a paternalist governor"). Differences between a character, the population and government policy create friction.
  - **Relationships (candidate):** opinion, grievances, debts. Separate from personality: the memory of how the player treated someone.
  - **Uncertainty (candidate):** skills are known roughly and traits are revealed through behaviour. The UI must tell "average" apart from "unknown".
- **Roles (direction):** the player appoints only senior posts: cabinet; leaders of colonies, settlements, bases and stations; admirals; captains; ground formation leaders. The player doesn't appoint organisation staff unless the organisation is state-owned. Candidates: elected posts depending on settlement type and civic alignment; a legislature represented by a few bloc leaders.
- **Organisations (direction):** yards, mining and shipping firms, institutes, often remnants from before the collapse. They are subjects of the faction but act independently and drive the private economy. Very powerful ones might break away (later). The player works through policy, contracts, favour and pressure.
- **Direct control (candidate):** the player can always take direct control at a price: an efficiency penalty, expenditure or a cost in relations. Requisition and state ownership are the far end of this.
- **Events (direction):** no canned pop-up events. Instead: requests and problems that arise from the simulated state and agendas; reports from real data; choices that differ in kind, with uncertain or delayed consequences; lore as optional recovered records. Only the Earth mystery and the external threat get a few authored beats (decided).

### 4.4 Design by requirements and bids (decided)

- The player sets requirements (role, mass, delta-v, crew, budget, deadline). Affiliated organisations bid with concrete designs, as in Shadow Empire.
- Designs persist and can be upgraded.
- The alternative is to design and produce through state-owned organisations. That trades away capacity, management attention, variety and goodwill.

### 4.5 Salvage and research

- **Salvage (direction):** generated sites (wrecks, stations, yards, ruins) with partly known contents.
  - Yields: bulk material (a finite legacy stock), discrete tech finds, lore records, relics.
  - Surveys give rough estimates. Expeditions take weeks to months under a leader whose skills and traits matter. Sites deplete. Richer sites lie further out or behind dark gates.
  - Organisations may salvage independently; the state can license, tax, claim or requisition.
- **Research (direction):** no tech tree and no research points. Populations are too small for frontier research, so progress means reverse engineering.
  - Organisations and state labs hold know-how per component type. They gain it by studying finds and through production and use.
  - A find can be studied (and may be destroyed doing so) or used. Documentation and tooling speed up reproduction.
  - Reproduction improves gradually: early copies are heavier, less efficient or less reliable, and get better with experience, samples, staff and materials.
  - Capability is local. It spreads by licensing or poaching, and can be lost with an organisation or its people.
  - The player directs research by funding projects, allocating finds and licensing.
- **Two design levels (decided):** component designs, mostly from before the collapse and reverse engineered (see *Component Catalogue*); and ship designs, assembled from components by organisations through bids.
- **Designs as objects (direction):** each design has a documentation level (fragmentary to complete), owners and rights, and a version lineage. Every organisation and faction keeps an archive, with sharing, licensing and theft.
  - **Operationalisation:** an organisation works out how to build a design with its own tooling, materials and staff, which gives an as-built version. Output varies and improves with practice. A fragmentary design may be unbuildable until its documentation is raised.
  - **Transfer:** copying records is cheap, but the receiver still has to operationalise. Poaching people moves know-how. Captured ships can be studied.
- **Understanding levels (candidate):** organisations turn staff, facilities and character skill into research output, which raises their understanding of a component type. Understanding sets the quality of the designs they can produce; reverse engineering feeds it. Refinements:
  - Understanding can fill gaps in fragmentary designs.
  - General investment is scaled down by a "theoretical research capacity" factor, so invention is possible but impractical.
  - Understanding decays without practice or when staff leave.
  - There is soft spillover between related types.
  - Organisations track only the types in their sector.
- **Priorities (direction):** organisations set their own. The player nudges them through funding, finds and contracts, or takes direct control at a cost. Whether the UI shows levels or point counters is decided later.
- **Knowledge sharing (candidate):** three layers: documents (copyable), tooling (physical) and tacit know-how (stays with teams).
  - The state can fund research on terms that give it the data, classify results, license to several organisations, or keep state labs.
  - Sharing spreads capability but leaks. Secrecy concentrates know-how in organisations that can fail or hold the state to ransom.
  - Documentation projects turn tacit know-how into data, at a cost.

### 4.6 Combat, raiders and diplomacy

- **Combat (direction):** lightly simulated and essentially auto-resolved. Engagements happen at extreme velocity and range. The player sets a posture, has few levers during the fight, and watches blips merge on a radar-like display, for Defcon-style tension. The display is the ordinary system view with the clock slowed (see *Presentation & Visual Style*).
- **Raiders (direction):** they arise from simulated conditions, not random spawns. Possible origins: scavengers contesting salvage, failed organisations, state-sponsored privateers (most plausible early on), smugglers avoiding tolls. Chokepoints and predictable trajectories invite interdiction. Long delays make age-of-sail analogies apt: standing orders, convoys, blockades, chartered companies, privateers.
- **Diplomacy (decided):** AI leaders change through elections, coups and death, so relations shift over a long game.

### 4.7 Factions

- **Differentiation (direction):** mechanical, not hand-crafted back story. Other possible differentiators (candidate): geography and gates, starting organisations, seeded designs and understanding, government form, leader traits.
- **Specced:** the ideology profile (pool of 12, labels, events) is in `docs/spec/ideology.md`.
- **Popular ideology (direction):** a position in a triangle of three forces: **Cohesion** (shared ownership and burdens), **Authority** (central direction) and **Autonomy** (private initiative). See *ideology-triangle.svg*.
  - **Representation (decided):** the three scores always add up to 12, so the numbers are the position ("Coh 6 · Auth 4 · Auto 2"). They are shown to one decimal place and stored as integers from 0 to 120. Starting scores come from faction history, where each option distributes 4 points.
  - **Events (decided):** an event changes one score, and the other two absorb the change in proportion to their size. So the faction moves in a straight line towards or away from that corner. An event may also name both sides ("Autonomy → Authority 2").
  - **Labels (decided):** a score of 7 or more gives a corner label. Otherwise, a score of 3 or less puts the faction on the edge opposite that corner. Otherwise the label is Pluralist. Corners cover about 17% of the triangle each, edges about 14% and Pluralist about 6%. Random histories give an even spread of labels, except Pluralist. Each label works as an adjective for a state and a noun for a person:

    | Region | Rule | Label | Meaning |
    |---|---|---|---|
    | Cohesion corner | Coh 7+ | Collectivist | Shared ownership and burdens above all |
    | Authority corner | Auth 7+ | Authoritarian | Central direction above all |
    | Autonomy corner | Auto 7+ | Individualist | Private initiative above all |
    | Cohesion + Authority | Auto 3 or less | Paternalist | The state directs shared resources for the common good |
    | Authority + Autonomy | Coh 3 or less | Meritocratic | Free individuals striving within a firm order the state maintains |
    | Cohesion + Autonomy | Auth 3 or less | Mutualist | Self-organising communities and cooperatives with a light centre |
    | Centre | Otherwise | Pluralist | No dominant force; politics is a negotiation |

  - **Summary line:** the label, then Outlook and Militancy, for example "Meritocratic, outward-leaning, strongly militant". All five profile numbers span 12 points (Outlook and Militancy are capped at ±6).
  - **Possible effects:** Cohesion makes state ownership cheaper and state organisations stronger, but shrinks the private sector. Authority makes direct control cheaper, but strains organisations and raises unrest. Autonomy boosts the private economy, but makes direct control costlier and revenue weaker.
  - Ideology is distinct from government form (how decisions are made) and from actual policy. A gap between ideology and policy causes friction. Ideology may vary by settlement and drift over time. Characters may hold positions in the same triangle (4.3).
- **Extra axes (direction):** **Outlook** (Insular to Outward: contact, trade, diplomacy) and **Militancy** (Pacifist to Militant: readiness, mobilisation, willingness to use force, the military's political weight), each from −6 to +6. Every position should be a different way to play, not a handicap.
- **Faction history (direction):** the player picks a history in three phases: origins before the collapse, turmoil during it, and recovery of social and political order after it. The profile emerges from those choices. AI factions draw histories too, with weighted probabilities instead of strict balance. Draft options and the full ideology rules are in *Faction History Options*.

## 5. Setting and narrative (decided)

In `world/setting.md`: tone, premise, timeline, the Earth mystery, the external threat and the technology baseline.

## 6. Not yet started

- **Player experience:** UI detail beyond the view structure in *Presentation & Visual Style*, onboarding, difficulty, multiplayer. (Unrestricted pausing suits single-player.)
- **Art and audio:** anything beyond the visual direction in *Presentation & Visual Style*.
- **Technical approach:** generation, simulation load at 100 systems, AI, engine.
- **Scope:** MVP, milestones, team, budget.

## 7. Risks

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

## 8. Parking lot

- **Shelved:** regional specialisation (each state missing something). It risks unbalancing factions.
- **Rejected:** "Logistics decide wars" as a pillar. Reverent/Pragmatic as a faction axis: a reverent faction would struggle in a salvage game.
- **Later:** feudal-style government as an optional twist; dark gates at the cluster edge; relic AI boosting research capacity.

## 9. Open questions

- **Vision:** stewardship or conquest; what to take from the references and what to avoid; audience and session length.
- **Structure:** win and lose conditions; late-game engagement; factions and bodies per system; speed settings and auto-pause rules.
- **Economy:** state income and spending; how the player sees and influences the private economy; costs and benefits of state ownership; bids for civilian designs; how to measure influence, reputation and opinion.
- **Map and gates:** fair starts; gate distance from the star and its effect on transit times; gate transit time and throughput; whether gates can be destroyed or captured; fog of war; which orbital details are modelled.
- **Characters:** growth and ageing; goals and history; cabinet portfolios; what counts as senior; elections and legislature; talent pool and careers; how character ideology is set and changes.
- **Bids:** what varies and why; hidden and visible information; ground forces and stations; AI use; refit or replace.
- **Research:** a readable summary of capability; studying or using finds; classification and leaks; operationalisation rules; the size of the research-capacity factor.
- **Combat and raiders:** pre-battle orders; information limits; after-action reports; ground combat and invasion; making losses matter; suppressing raiders; privateer licences and prize shares.
- **Factions:** government forms and how they link to history and ideology; ideology drift and typical event sizes; final history options and randomness; continuous or discrete extra axes; whether a research-oriented modifier is needed.
- **Setting:** how the threat is hinted at and escalates, whether it can be switched off, and why it hasn't arrived; the hard sci-fi no-go list; the delta-v range and reaction mass model.
- **Presentation:** see *Presentation & Visual Style*, section 6.

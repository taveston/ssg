# Orphan Worlds: Faction History Options

**In short:** a faction's ideology profile comes from three history choices: **origins** (what the settlement was before the collapse), **turmoil** (how it survived the collapse) and **recovery** (how social and political order was restored). Each of the six options per phase gives 4 ideology points, so three choices fill the pool of 12, and each option also sets Outlook, Militancy, starting assets and scars. The ideology rules are specced in `docs/spec/ideology.md`; this doc holds the option tables, flavour text and reasoning.

*Status:* draft. All numbers are placeholders for balancing. See the design document, 4.7, and the diagram *ideology-triangle.svg*. A summary label and history blurb can be generated from the choices.

**Timeline:** the old network collapses, then years of turmoil, then order is restored on the ground, then **game start**, when the faction is just venturing back into space. Recovery is political and social, not industrial: there are no interplanetary fleets or trade yet. Orbital yards and stations from before the collapse exist but sit dormant or barely maintained.

*Terminology:* "organisation" is the mechanical term. In the setting, firms are corporations, guilds and institutes.

---

## 1. How ideology is represented (decided)

Design goals: readable at a glance, numbers of similar size to the rest of the profile, and the effect of any event obvious.

> **Specced** in `docs/spec/ideology.md` (IDEO-01 to IDEO-31). The exact rules (storage in tenths, label thresholds, the event and rounding rules, the summary line) live there; this section keeps the shape and the reasoning.

- **A fixed pool of 12** shared by **Cohesion**, **Authority** and **Autonomy**, so the three numbers *are* the position in the triangle. "Coh 6 · Auth 4 · Auto 2" is the whole picture; 4 / 4 / 4 is the exact centre.
- **Labels** by thresholds of 7 (corner) and 3 (edge), Pluralist otherwise. The thresholds were chosen so random histories give an even spread (section 7). Labels and reasoning are in section 8.
- **An event changes one score and the other two absorb it in proportion**, so the faction moves in a straight line towards or away from a corner, and the effect is obvious at a glance. Two-sided events ("Autonomy → Authority 2") move along an edge.
- **Outlook** (Insular/Outward) and **Militancy** (Pacifist/Militant), each −6 to +6, so all five profile numbers span 12 points.
- **Characters** can use the same pool, thresholds and labels.

## 2. How the history modifiers work

> The 4-points-per-option rule and loading options from data are specced as IDEO-30 and IDEO-31. The option tables below are still placeholders.

- **Ideology:** each option distributes exactly **4 points** across the three forces, so three choices add up to the pool of 12. No baseline or scaling.
- **Design pattern:** in each phase, three options lean firmly to one force and three blend two forces. Each phase's options total 8 / 8 / 8, so no force is favoured.
- **Outlook and Militancy:** plain sums of the options, from 0.
- **Starting effects and scars:** qualitative for now. Effects: organisations, designs, settlements, characters. Scars: lost population, damaged infrastructure, distrust, weak institutions.
- **Randomness:** the player picks the path; specific scars or windfalls could be rolled within it.
- **Names** should paint a concrete picture rather than describe a policy.
- **Flavour text** is written from the faction's own point of view.

## 3. Origins (before the collapse)

| Option | Coh | Auth | Auto | Out | Mil | Starting effects | Scar |
|---|---|---|---|---|---|---|---|
| Frontier mining colony | 0 | 0 | 4 | −1 | 0 | Extraction organisations; metal and water stocks; mining designs | No yards; thin component base |
| Shipyard hub | 0 | 1 | 3 | +1 | +1 | Dormant orbital yards; ship-design archive; some propulsion designs | Raw material always came from elsewhere |
| Administrative centre | 1 | 3 | 0 | 0 | 0 | Strong institutions; bigger talent pool; more state-ownable infrastructure | Weak private sector; legacy distrust |
| Research outpost | 3 | 0 | 1 | +1 | −1 | Research institutes; advanced but fragmentary designs; high understanding in a couple of types | Small population; thin industry |
| Naval base | 0 | 4 | 0 | −1 | +2 | Officer tradition and ground forces; weapon designs; a mothballed orbital base | Weak civil economy |
| Agricultural colony | 4 | 0 | 0 | −1 | −2 | Larger population and settlements; secure food and water | No orbital assets; low industry |

- **Frontier mining colony:** "We were the people who dug the rock others built with. Nobody sent help, so we learned to need none."
- **Shipyard hub:** "Our slips built the hulls that carried the old network's traffic. The cranes still hang in orbit, waiting."
- **Administrative centre:** "We kept the ledgers, scheduled the gates, and stamped the permits. When the orders stopped coming, the habit of order remained."
- **Research outpost:** "We were a community of people who shared their questions. When the answers stopped arriving from home, we kept asking together."
- **Naval base:** "We guarded lanes that no longer carry anything worth guarding. Our ships are gone; we still keep the watch."
- **Agricultural colony:** "We grew the food that fed the miners and the yards. Under the domes, nobody eats unless everybody works."

## 4. Turmoil (surviving the collapse)

| Option | Coh | Auth | Auto | Out | Mil | Starting effects | Scar |
|---|---|---|---|---|---|---|---|
| Isolation and self-reliance | 0 | 2 | 2 | −2 | 0 | Robust local infrastructure; self-sufficient settlements; capable local leaders | Lost contacts and designs; lower understanding |
| Civil conflict | 0 | 4 | 0 | −1 | +2 | Veteran officers; strong local identities | Population loss; damaged infrastructure; low social trust |
| Corporations rise | 0 | 0 | 4 | +1 | 0 | Large independent corporations; vibrant private economy | Weak state revenue; corporations may outgrow the state |
| Refugee influx | 4 | 0 | 0 | +2 | −1 | Larger, varied population; more skilled personnel | Resource strain; ideology varies by settlement |
| Emergency rationing | 2 | 2 | 0 | −1 | +1 | Strong state control; state-owned emergency organisations | Public resentment; organisations distrust the state |
| Survivor coalition | 2 | 0 | 2 | +1 | −1 | Strong local self-rule; elected local posts; legislature | Slow decisions; weaker centre |

- **Isolation and self-reliance:** "The gates went quiet and the ships stopped coming. Every settlement learned to stand alone, and the people who could get things done ended up in charge."
- **Civil conflict:** "We fought each other for the last scraps of the old order. Some of us never stopped."
- **Corporations rise:** "When the state faltered, the firms stepped in. They kept the lights on, and they remember who paid."
- **Refugee influx:** "They came in their thousands on the last ships through the gates. We shared what little we had, and we are still working out what that made us."
- **Emergency rationing:** "There was a winter when every kilogram had a name on it. Somebody had to decide, and everybody had to share."
- **Survivor coalition:** "No one settlement could survive alone, so we agreed to survive together. The agreement is older than any of our governments."

## 5. Recovery (restoring order)

| Option | Coh | Auth | Auto | Out | Mil | Starting effects | Scar |
|---|---|---|---|---|---|---|---|
| Garrison peace | 0 | 4 | 0 | 0 | +2 | A garrison in every settlement; strong ground forces and mobilisation; military bloc | Strained relations with organisations; less efficient economy |
| Many hands | 4 | 0 | 0 | +1 | −1 | Cooperatives and community organisations; strong solidarity | Slower private growth; less flexibility |
| Market revival | 0 | 0 | 4 | +2 | −1 | Many organisations; busy trade between settlements; flexible economy | Low state revenue; independent organisations |
| Technocratic restoration | 0 | 2 | 2 | 0 | 0 | Higher research output; skilled administrators | Narrow political base; opinion gap |
| Bureau of Provision | 2 | 2 | 0 | −1 | 0 | A bureau that allocates housing, work, and rations; loyal, well-provided population; state-owned organisations | High state spending; little personal choice; organisations chafe at oversight |
| Salvage boom | 2 | 0 | 2 | +1 | +1 | More early finds (surface caches, charted orbital wrecks); salvage organisations | Fewer well-documented designs; raider-like crews |

- **Garrison peace:** "The soldiers who ended the fighting never went home. Every settlement has its garrison, every road its checkpoint, and nobody remembers the last riot."
- **Many hands:** "Nobody was ordered to help. When a roof fell or a pump failed, the whole settlement turned up with tools and food and stayed until the job was done. We still rebuild that way, and we still share what we build."
- **Market revival:** "We opened the markets, lifted the tolls between settlements, and let anyone with something to sell make a living. Trade came back before the government did."
- **Technocratic restoration:** "We put the engineers and administrators in charge, chosen for what they could do rather than who they knew. The schedules finally started to work."
- **Bureau of Provision:** "The bureau decided where you would live, what you would do, and what you would eat. It decided well, and so we never asked it to stop."
- **Salvage boom:** "The old world left caches, depots, and ruins all over our world. Free crews and their towns grew rich picking them clean, and now they look to the sky for more."

## 6. Sample profiles

| Origins / Turmoil / Recovery | Coh / Auth / Auto | Out | Mil | Profile |
|---|---|---|---|---|
| Shipyard hub / Corporations rise / Market revival | 0 / 1 / 11 | +4 | 0 | Individualist, strongly outward, neutral |
| Administrative centre / Civil conflict / Garrison peace | 1 / 11 / 0 | −1 | +4 | Authoritarian, insular-leaning, strongly militant |
| Agricultural colony / Refugee influx / Many hands | 12 / 0 / 0 | +2 | −4 | Collectivist, outward-leaning, strongly pacifist |
| Research outpost / Emergency rationing / Technocratic restoration | 5 / 4 / 3 | 0 | 0 | Paternalist, neutral, neutral |
| Naval base / Isolation / Market revival | 0 / 6 / 6 | −1 | +1 | Meritocratic, insular-leaning, militant-leaning |
| Frontier mining colony / Survivor coalition / Many hands | 6 / 0 / 6 | +1 | −2 | Mutualist, outward-leaning, pacifist-leaning |
| Frontier mining colony / Emergency rationing / Bureau of Provision | 4 / 4 / 4 | −3 | +1 | Pluralist, strongly insular, militant-leaning |
| Research outpost / Survivor coalition / Salvage boom | 7 / 0 / 5 | +3 | −1 | Collectivist, strongly outward, pacifist-leaning |
| Naval base / Emergency rationing / Bureau of Provision | 4 / 8 / 0 | −3 | +3 | Authoritarian, strongly insular, strongly militant |

The first row as prose: a trading faction with dormant yards and a vibrant private economy, but little state revenue and corporations that could outgrow it.

## 7. Whole-space check (216 combinations)

- Random histories give an even spread: each corner label about 18%, each edge label 13 to 15%, Pluralist 4%.
- Outlook ranges from −4 to +5 and Militancy from −4 to +6, within the ±6 caps. There is a slight outward and militant skew, which AI weighting can offset.

## 8. Ideology labels (decided)

> **Specced** as IDEO-10 to IDEO-13.

**Corner names:** Cohesion, Authority and Autonomy name the three forces (the axes of the triangle). They were chosen for neutrality. Rejected alternatives: Communal/Command/Enterprise, Stewardship/Authority/Initiative, Commons/Directorate/Market, Commonwealth/Directorate/Company, Shared/Directed/Free.

**Region labels:** seven regions summarise a position. Each label works as an adjective for a state ("a Paternalist state") and as a noun for a person ("a paternalist"), so the same set can describe characters.

| Region | Rule | Label | Character |
|---|---|---|---|
| Cohesion corner | Coh 7+ | **Collectivist** | Shared ownership and shared burdens above all |
| Authority corner | Auth 7+ | **Authoritarian** | Central direction above all |
| Autonomy corner | Auto 7+ | **Individualist** | Private initiative above all |
| Cohesion + Authority edge | Auto 3 or less | **Paternalist** | The state directs shared resources for the common good (opposite Individualist) |
| Authority + Autonomy edge | Coh 3 or less | **Meritocratic** | Free individuals striving within a firm order the state maintains (opposite Collectivist) |
| Cohesion + Autonomy edge | Auth 3 or less | **Mutualist** | Self-organising communities and cooperatives with a light centre (opposite Authoritarian) |
| Centre | Otherwise | **Pluralist** | No dominant force; politics is a negotiation |

**Rejected labels:**
- Cohesive/Centralised/Autonomous and Stewarded/Chartered/Mutual: mixed parts of speech, and they don't work for characters.
- Communitarian: strained.
- Pragmatist for the centre: clashes with the candidate Idealist/Pragmatist trait pair.
- Pacific for the Militancy axis: an uncommon word.

**Rejected numeric approaches:**
- Open-ended sums: the effect of an event depends on the totals.
- Percentages: don't match the ±6 axes.
- Taking event points from the larger score first: felt arbitrary.

## 9. Open questions

- Final option list per phase; whether more are needed.
- Whether the remaining policy-style names (for example Technocratic restoration, Emergency rationing) should get more vivid names too.
- What is rolled, and how much.
- Turning qualitative effects and scars into concrete assets and values.
- How AI factions draw histories (random, or weighted by map position).
- Whether map generation places starts to match origins.
- How history sets government form and how that interacts with ideology.
- Character ideology: how it is set and changes.
- Typical event sizes and how fast ideology should drift.

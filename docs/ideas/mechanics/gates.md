# Orphan Worlds: Gates

**In short:** gates are paired wormhole mouths held open by fields from surrounding generators. They run alone on reserves, which is why they outlived most old tech, but they are slowly failing. Most of the old network's links have already collapsed, which gives the surviving network an irregular shape and leaves the wreck of each dead gate as salvage. Gate parts can only be salvaged, not made, so keeping gates alive is a race between salvage and decay. A dormant gate may be faulted on the far side, which leads to gates locked from the other side, the long way round, and contact by signal before contact by ship.

> **Speculative.** Everything here is early ideation and **Candidate** unless marked otherwise. Nothing is a requirement.

*Companion documents:* *Overview*, *Economy* (6, backbone arc), *Setting & Back Story*, *World Generator* (4, 7), *Drives & Transit* (6, gate placement and entry tolerance), *Factions & Ideology*.

---

## 1. In-game and behind the scenes

The overview keeps how gates work deliberately unexplained. That still holds on screen: no character explains the physics. This doc is the truth behind it, so that mechanics, salvage and clues stay consistent. Players can piece some of it together from records and research.

## 2. What a gate is

- **The network (decided):** the map is generated. Each system has several gates at its edge, one per destination, each linking to a small handful of nearby systems. The network is largely fixed at the start. Because gates are at the edge, reaching inner bodies takes weeks to months (*Drives & Transit*, 6).
- **Paired mouths:** each gate is one mouth of a wormhole. Its partner is the gate it links to. One pair per link, so one gate per destination (*Drives & Transit*).
- **Held open by fields:** a wormhole throat pinches shut on its own. Each mouth is held open by field generators around it, which need power and constant fine control, like magnetic confinement in a fusion reactor. The old age could build these; nobody can now. This fits "advanced fields are reproduced, not invented".
- **How they got there:** the far mouth of each pair was hauled to its system below light speed, over decades or centuries. The network grew outward from Earth, which is why Earth was the hub. Link range is how far a tow could go. New links can't be made, only old ones kept or reopened.

## 3. What a gate looks like

- **The mouth** is a sphere, not a hole in a ring. From outside it looks like a bubble showing a warped view of the other system's sky, with a bright lensed halo at its edge. Ships can enter from any side. Its size, perhaps a few hundred metres, bounds the largest ship that can pass.
- **Nothing sits close to the mouth.** Ships arrive at up to about 10 km/s and cross a kilometre in a tenth of a second. The generators float in a loose shell well clear of it, tens to hundreds of kilometres out, each holding station on its own.
- **Radiators:** generators run hot, so each one is a bright point fringed with radiator fins. Together they give the gate a distinct silhouette for the inspection view.
- **Swept space:** dust and ice hit at 10 km/s like shrapnel. A working gate keeps the space around it clean; a neglected one gets dirty, which is a hazard of its own.
- **The maintenance station** sits off to one side, with crew, spares and power. It is the only part people live on, and the part worth attacking.

## 4. Entry tolerance

*Drives & Transit* (6.2) proposes that gates accept ships arriving at up to about 10 km/s. The field explains it: it steadies a ship as it enters. A faster ship brings more energy than the field can absorb, and the throat flexes violently. A degraded gate steadies less, so its tolerance drops.

## 5. Why gates still work

1. **Built to run alone.** Light takes years between stars, so no gate could wait for orders. Each was self-sufficient, redundant and self-repairing: the most robust thing the old age made. Yards and stations needed crews and supply chains; gates didn't, for a while.
2. **Running on reserves.** Long-life reactors, spare generators and maintenance drones. The redundancy has been eaten since the collapse. This is the decay clock.
3. **Drifting.** Earth sent updated control models. Without them the field tuning drifts out of true. Gates still pass ships, but less reliably and at a lower tolerance.
4. **Left alone.** During the turmoil, people fought over worlds, not unmanned machines in deep space.
5. **Survivors.** Most did not survive (section 6). The gates players use are the ones that did.

## 6. Most links have already collapsed

- **Most of the old network is dead.** The working links at game start are a minority of those the old network built.
- **Shares and choice (decided 2026-10-16):** at game start about 30% of links work, 15% are dormant and 55% are collapsed, shifted a little for each map; the shares are targets. Failure falls more on links between busy, valuable systems, plus chance. Every system keeps at least one gate that isn't collapsed, the main working region holds at least 40% of the systems, and a repair order reaches every pocket from the main region, so no pocket stays out of play for good. **Specced** in `docs/spec/sector-generator.md` (7).
- **Topology:** the old network was a dense mesh. With most links gone, what's left is sparse and irregular: chokepoints where one gate holds a region together, dead ends, backwaters, and isolated pockets that connect to each other but not to anyone else.
- **Orphan worlds:** a system whose every gate has failed is cut off. It's still visible in the sky and known from the records. These give the title a second meaning, and are candidates for late-game reopening, lore and regret.
- **Two salvage sites per dead link:** when a throat collapses, both mouths go. The generator shells at each end stay behind, dark and drifting. So one failed link leaves a wreck in each of two systems, near the gate ring.

## 7. Gate states

| State | What it means | What the player can do |
|---|---|---|
| Working | Passes ships, possibly degraded (lower tolerance, lower reliability) | Maintain it, defend it, strip it |
| Dormant | Intact, but the throat is pinched too narrow for ships. A fault, or a deliberate lock, at one or both ends | Restart it, if the fault is on your side |
| Collapsed | The throat is gone. Two wrecks remain | Salvage gate parts. Rebuild only with very-late-game technology |

## 8. Gate parts

- **Salvage only:** gate tenders can swap modules using parts recovered from collapsed gates and stored spares. Nobody can build a generator until research recovers how (a top research goal).
- **Race against decay:** parts in stock against the rate of failure. When salvage runs out, gates start dying. This makes the decay clock visible.
- **Cannibalisation:** a player can strip a working but minor gate to save an important one, deliberately reshaping the network and cutting some worlds off.
- **Rivals:** a rival may strip parts from a gate you rely on, so a quiet gate still needs guarding.

## 9. Dormant gates and the far side

A dormant gate is faulted, or locked, at one end or both. Which end decides who can fix it.

- **Faulted on your side:** your engineers can restart it with parts and time.
- **Faulted on the far side:** nothing you do at your end helps. Someone has to reach the other end, by a longer route through the network if one exists.
- **The long way round:** a dormant gate next door might take a two-year expedition through several systems to fix from behind. Shortcuts are earned.
- **Locked from the other side:** the far side may be a dead system with nobody to fix it, a pocket of survivors who can't reach their end either, or a faction that doesn't want it open.
- **Deliberate locking:** a faction can put its own end into dormancy, making the gate a door with a lock on one side. Sealing yourself in is a defensive move. Reopening is a bargaining chip. A pocket that sealed itself during the turmoil and stayed sealed is a strong mid-game event.
- **Uncertain diagnosis:** your end may report only "no response from far end". Broken, locked or dead is unknown until someone gets there. This fits fidelity following knowledge (*Presentation & Visual Style*, 2).
- **The keyhole:** a dormant throat is too narrow for ships but wide enough for light. You can see and signal through it. You can talk to people behind a locked gate long before anyone opens it, or watch a dead system and know nothing is coming. First contact with a sealed pocket is a voice, not a fleet.

## 10. Lore hooks

- **The collapse:** the refugee flood on the last ships through the gates (*Factions & Ideology*) may have helped break the network.
- **The threat:** rebuilding collapsed links, or reopening long-sealed ones, may let the threat in (*Setting & Back Story*).
- **Free-floating planets:** a gate at a starless planet could be a mouth abandoned mid-tow. The generator leaves free-floating planets out for now (*World Generator*, 3).

## 11. Open questions

- Throat size, and whether it varies by gate.
- How a lock works: who can set it, how long it takes, and whether it can be forced from the far side at a cost.
- What the keyhole carries: speech and pictures only, or data and research too.
- Whether collapsed links can ever be rebuilt, and what that costs.
- How gate parts are counted: one generic resource, or a few module types.
- Gate transit time and throughput.
- Whether gates can be destroyed or captured.
- Fog of war: what a faction knows of links it hasn't used.
- Later: links out of the sector that are dark. There are none in the first version (*World Generator*, 4.2).

# Orphan Worlds: Requirements & Bids

**In short:** how the state gets ships. The player sets requirements from a role template; organisations decide whether to bid and generate bids from their understanding, capacity, people and traits; the state's technical office shows each claim with an honest 80% range; the player awards a contract; and the as-built figures, drawn once at design freeze, are revealed through yard reports, trials and service. Every difference between bids has a learnable cause, and every surprise has tells. Section 9 works through one tender (HR-1) end to end; section 11 is the MVP cut.

*Status:* draft. Status tags as in the overview (**Decided**, **Direction**, **Candidate**). The HR-1 figures in section 9 come from `scripts/bids/hr1_bids.py`, which runs the bid rules here on the as-built distributions of *Component Stats & Ship Assembly* (CSA). Figures marked **[P: CSA]** or **[P: D&T]** (*Drives & Transit*) are open in those documents; plain **[P]** marks illustrative values for this document.

> **Repo note:** `scripts/bids/` isn't in this repo yet; the figures below are as last generated.

Companion documents: *Overview* (6, risks), *Salvage & Research*, *Component Catalogue*, *Component Stats & Ship Assembly* (understanding modifier curves, cost scale, assembly formulas), *Drives & Transit* (drive baselines, transit times), *Presentation & Visual Style*, *Reference Sketches*.

---

## 1. Purpose and principles

- **Decided:** the player sets requirements (role, mass, delta-v, crew, budget, deadline), and affiliated organisations bid with concrete designs, as in Shadow Empire. Designs persist and can be upgraded. The alternative is for the state to design and produce through state-owned organisations, which costs capacity, management attention, variety and goodwill.
- **Decided:** two design levels: component designs (mostly from before the collapse, or reverse engineered) and ship designs (assembled from components by organisations through bids). Every design has *rated* figures; each organisation's build of it has *as-built* figures.
- **Candidate principles:**
  - **Every difference has a learnable cause.** Two bids differ only because of understanding, documentation held, tooling and yard condition, capacity and order book, the people involved (traits, skills), relationship with the state, or contract terms. No unexplained dice.
  - **Honest but uncertain.** Ranges shown to the player are calibrated: about 8 in 10 as-built figures land inside the shown 80% range. Bad assessors give wider ranges, never false ones. The rules are documented in the in-game encyclopedia. What is hidden is the draw and unobserved traits, never the rules.
  - **Drawn once, revealed gradually.** As-built figures are fixed at design freeze and revealed later; nothing is rolled at the moment of reveal. Every surprise has tells the player saw beforehand.
  - **A conversation, not a shop.** The player asks questions, negotiates terms, and lives with the institution afterwards (track record, opinion, grievances, debts).
- **Where the main bid questions are answered:** what varies and why (section 5.1); hidden and visible information (6.1); ground forces and stations (10.2); AI use (10.4); refit or replace (10.5).

## 2. Lifecycle **(Candidate)**

At 1 day per second: 60 game days is about one real minute.

| # | Stage | Duration (game days) | What happens |
|---|---|---|---|
| 1 | Requirement | 0 (player, paused); optional feasibility check 10 | Player fills a role template; the state technical office can check feasibility (3.4) |
| 2 | Invitation | open 30 · invited 20 · sole source or direct assignment 10 | Window in which bids arrive; organisations reusing held designs bid early |
| 3 | Bids | within the window | Each organisation decides whether to bid (4.2) and generates a bid (5) |
| 4 | Evaluation and clarification | player-paced; each clarification 10; paid design study 30–60 | Bids stay valid 90 days; questions, revisions, best and final offer (6.4) |
| 5 | Award and contract | 10 (0 for direct assignment) | Contract type, terms and rights agreed (7) |
| 6 | Design finalisation | reuse 30 · derived 60 · new configuration 120 | Ends with **design freeze**: as-built figures drawn (hidden); first revised estimates |
| 7 | Build | queue wait + build time (CSA 5: effort over yard teams plus tooling lead); first-of-class 400 t hauler 216–404 (CSA 7) | Monthly yard reports (every 30 days): progress, weigh-ins, costs, slips |
| 8 | Trials | first-of-class 30 civilian, 60 military; follow-on hulls 10 | Performance figures revealed (8) |
| 9 | Acceptance and delivery | player decision; rectification 30–120 | Accept, rectify, renegotiate, reject, take over |
| 10 | Service | ongoing; reviews at acceptance, at 365 days, at each major failure | Reliability and upkeep revealed statistically; track record updated |
| 11 | Upgrades, refits, variants | variant tender 20 + design 30–60; refit 60–180 [P: CSA] | Same pipeline, shorter (10.5) |

- A typical first-of-class programme (10 + 20 + 10 + 10 + 60 + 450 + 30 days) takes about 590 days: 1.6 years, or about 10 minutes of play. Follow-on hulls skip stages 1 to 6 and most of 8.

## 3. Requirements **(Candidate)**

### 3.1 Fields

Every field has a `status`:
- **hard:** must be met; failing it is a declared deviation.
- **target:** scored by weight.
- **provisional:** the state is unsure. Deviations are invited and not penalised beyond their weight.
- **open:** the bidder proposes.

| Field | Unit | Default status | Notes |
|---|---|---|---|
| `role` | template id | hard | Sets defaults below and the comparison layout |
| `cargo_t` / `payload_t` | t | hard | Bulk cargo or mission payload |
| `delta_v_loaded_kms` | km/s | hard | At full cargo; ship-level figure (rocket equation, see CSA) |
| `accel_min_mg` | mg | open | Leaving it open admits low-thrust drives (see HR-1) |
| `transit_target_d` | days on a named route | target | Computed from *Drives & Transit*; friendlier than acceleration |
| `crew_max` | people | hard | |
| `endurance_d` | days | target | Life support and spares between port calls |
| `propellant_pref` | water / hydrogen / any | open | Water is the double-duty resource; hydrogen needs cryogenic storage |
| `unit_budget_cr`, `programme_budget_cr` | cr | target | Over-budget bids allowed but flagged |
| `deadline_d` | days to first delivery | target | |
| `quantity` | hulls | hard | May be split between bidders |
| `delivery_point` | body or station | hard | |
| `rights_wanted` | state / licence / shared | target | See 7.3; state rights cost more |
| `weights` | performance, cost, schedule, reliability, upkeep | sum 100 | Drives the default sort of the comparison view |

### 3.2 Templates by role

Templates pre-fill fields, statuses and weights, so the player edits rather than fills in a blank form. Typical values come from *Drives & Transit* and CSA.

| Template | Hard by default | Weights (perf/cost/sched/rel/upkeep) |
|---|---|---|
| Bulk hauler | cargo, delta-v, crew, quantity | 20/30/15/20/15 |
| Tanker | propellant cargo, delta-v, cryogenic or not | 20/25/15/25/15 |
| Tug | thrust class, delta-v, docking | 30/20/20/20/10 |
| Survey and salvage tender | payload, endurance, lander | 25/20/20/25/10 |
| Patrol and escort | delta-v, acceleration, signature max | 40/15/15/20/10 |
| Shuttle and lander | surface gravity class, payload | 25/25/20/25/5 |

### 3.3 Over-tight requirements

Each organisation tests its best configuration at its own median estimate (P50). If a hard field fails:

- **No bid:** the organisation declines with a stated reason ("Arden: cannot reach 10 km/s with 400 t on our NTR"). A tender with no bids returns all the reasons, so the player sees what to relax.
- **Bid with a declared deviation:** the bid states the field, the figure offered and why. This is normal for Principled and Cautious heads, and any bidder must declare a hard-field miss at its quoted figure.
- **Stretch bid:** a Bold or Ambitious head may claim compliance if it is reachable within its optimism limit (5.3). If the office's assessor has Science 6 or more, the office can note "claim near the limit of this organisation's understanding".
- **Padded bid:** marginal compliance raises contingency, price and schedule.
- **Remedies:** relax a hard field to target or provisional, raise the budget, extend the deadline, split the role, or fund understanding first (dependency: research).

### 3.4 Feasibility check

- Before issuing a requirement, the state technical office (a post led by a character; dependency: cabinet portfolios) can spend 10 days listing which drive families and organisations are likely to meet it, using what the state knows of their understanding bands. Accuracy follows the assessor's skill.

## 4. Who bids and why

### 4.1 Invitation modes **(Candidate)**

| Mode | Who | Window | Gains | Costs |
|---|---|---|---|---|
| Open tender | Every affiliated organisation with sector match | 30 d | Most variety; competition trims margins (3 pts with 3+ bidders); surfaces unknown capability | Slowest; unknown bidders with no record; losers who designed from scratch lose a little opinion |
| Invited tender | 2 to 5 chosen organisations | 20 d | Faster; focus on known bidders; invitees gain opinion | Uninvited capable organisations may take a grievance (more if Ambitious) |
| Sole source | One private organisation | 10 d | Fast; builds the relationship | No competition (margin +5 pts); dependency on one supplier |
| Direct assignment (**Decided** option) | State-owned organisation | 10 d | Cost-based price; state sets priorities; no private margin | Uses scarce state capacity (crowds out other state work); management attention [dependency: administration]; low variety; private sector goodwill erodes if habitual |

### 4.2 Bid decision **(Candidate)**

An organisation bids when its interest passes its threshold. Inputs, in order of weight:

1. **Capability fit:** the best feasible configuration at P50 against the hard fields. The gap is measured in understanding levels.
2. **Capacity:** free yard-days in the delivery window against what the build needs (order book, leased capacity, a dormant yard needing reactivation).
3. **Agenda fit:** the organisation's own priorities: its sector, wanting rights to a design it can sell or use, wanting to raise understanding.
4. **Value and prestige:** contract size relative to the organisation. State military work counts as prestige.
5. **Relationship:** opinion of the state, grievances, debts owed either way.
6. **Traits of the head:** adjust the threshold and the inputs (5.3).

- Declining is information: the reason is shown ("No capacity until day 410", "Outside our sector").

## 5. How a bid is generated **(Candidate)**

### 5.1 What varies and why

| Cause | What it changes | Where the player learns it |
|---|---|---|
| Understanding per component type (0–10) | As-built median and spread (`σ_build`) of mass, efficiency, reliability (CSA 6: modifier curves and variance) | Organisation dossier (bands); bid card |
| Documentation level held | Buildability, design time, spread | Bid card: design source line |
| Tooling and yard condition | Build time, cost; dormant yards need reactivation | Yard inspection; dependencies |
| Capacity and order book | Schedule, queue wait | Organisation dossier; decline reasons |
| Component supply (imports, licences, the components resource) | Schedule, cost, dependency risk | Dependencies line |
| Head's traits | Claim optimism, margins, design choice, disclosure, behaviour after award | Known traits; track record |
| Chief designer's Science skill | Scales `σ_build` by (1.2 − 0.04 × skill): 1.0 at 5, 0.8 at 10 [Candidate; CSA's variance is per understanding only, so this scaling lives here] | Character card (skill range) |
| Programme risks (dormant yard, licensed or first-built components, leased slipways, partial documentation) | Adds a programme spread `σ_prog` to schedule (log σ 0.05 to 0.15 per risk, combined in quadrature); the per-hull variance in CSA is too small to carry these alone | Dependencies line; yard inspection |
| Practice (hulls built) | Follow-on hulls improve (CSA 6: +0.6 / n understanding per unit) | Track record by hull |
| Relationship | Margin (opinion high: −2 pts; grievance: +5 pts), disclosure | Opinion shown on card |

### 5.2 Steps (simulation)

1. **Candidate configurations:** for each relevant component type, the designs the organisation holds or can license or import, filtered by the documentation levels it can operationalise (CSA 6: minimum understanding per documentation level).
2. **Size the ship** to the requirement with as-built component figures (CSA 5). This gives a median and spread for each ship-level figure: dry mass, wet mass, delta-v, acceleration, power and heat balance, reliability, cost, build time, upkeep, signature.
3. **Choose a configuration** by fit score, filtered by the design-choice rules (5.3).
4. **Quote figures:** each claim is a quantile of the organisation's own estimate. `optimism` is the chance that the as-built figure turns out worse than the claim.
   - Higher-is-better figures: `claim = median + z(optimism) × σ`.
   - Schedule and cost (log-normal): `claim = median × exp(−z(optimism) × σ_log)`.
5. **Price:** estimated cost × (1 + margin + contingency), adjusted by competition and relationship.
6. **Schedule:** design time + queue wait + build time + dependency lead times, quoted at `schedule_optimism`.
7. **Disclosure:** declared deviations, risk flags, dependencies; rights offered; preferred contract type.
8. **Store the hidden record:** medians and spreads, for the assessment model (6.2) and the draw at design freeze (8.1).

### 5.3 Trait rules

**Baseline** (no trait in a pair): `perf_optimism` 0.50, `schedule_optimism` 0.50, `cost_optimism` 0.50, `margin` 10%, `contingency` 5%. Discloses hard-field deviations only, and reports problems at the next monthly report.

Offsets add. Optimism is clamped to 0.10–0.90. Caps apply last.

| Trait | Bid decision | perf / sched optimism | Price | Design choice | Disclosure | After award |
|---|---|---|---|---|---|---|
| Ambitious | Bids on most tenders; bids beyond capacity on prestige work (plans expansion) | +0.10 / +0.10 | margin −3 pts on prestige work | Sizes towards the top of the requirement | Baseline | Chases follow-on work; rejection gives a double grievance |
| Content | Bids only if free capacity covers 80% of the build | 0 / 0 | margin +2 pts | Baseline | Baseline | Steady; rarely seeks variations |
| Bold | Bids with a capability gap of up to 2 levels | +0.25 / +0.25 | contingency 0 | Pushes performance, accepts novel risk | Baseline | Absorbs problems quietly (reports 30 d late); seeks renegotiation only when large |
| Cautious | Declines if any hard field fails at P50 and no deviation is acceptable | −0.15 / −0.30 | contingency +10 pts | Oversizes margins (heavier, safer) | Baseline plus obvious risks | Reports early; frequent pleasant surprises |
| Principled | Baseline | capped at 0.60 / 0.60 | never below cost | Baseline | Declares all deviations and flags all known risks | Reports at once; accepts fixed-price losses; variations at cost plus margin |
| Opportunistic | Bids where variations or cost-plus pay | 0 / 0 | margin −10 pts on fixed price (may go below cost); cost inflated +15% on cost-plus | Leaves requirement ambiguities unresolved | Minimum legally required | Files a variation claim for each ambiguity; variations marked up 30% |
| Innovative | Attracted to requirements needing new configurations | 0 / 0 | Baseline | Scores configurations on best case: may pick a type up to 2 understanding levels below its best; may add an alternative bid deviating on a soft field | Baseline | Proposes improvements mid-build (variations that may help or delay) |
| Traditionalist | Declines if the best feasible type has understanding below 4 | 0 / 0 | variations +20% | Reuses held ship designs; highest-understanding type; only working or complete documentation; reuse halves design time | Baseline | Resists changes |

- Trait behaviour is also how traits are revealed: a pattern of missed claims reveals Bold, and evasive answers reveal Opportunistic [dependency: character trait-reveal rules].
- Examples of the resulting quantiles:
  - Bold + Ambitious quotes performance at 0.85 (z = +1.04): the claim is one spread above its own median, and is missed 85% of the time.
  - Cautious quotes schedule at 0.20 (z = −0.84): long, and met 80% of the time.

## 6. What the player sees

### 6.1 Hidden vs visible **(Candidate)**

| Exact | As ranges | Hidden until revealed |
|---|---|---|
| Claims, price, contract terms offered, declared deviations and flags, dependencies, rights offered, design source and documentation level, track record entries | As-built figures (office estimate), understanding (bands: None, Rudimentary, Basic, Competent, Advanced, Mastery (CSA 6)), people's skills, cost estimate under cost-plus | Organisation's internal medians, the as-built draw, unobserved traits, undisclosed risks |

- **Unknown is not average.** A trait slot never observed shows a dashed "? not yet observed"; an observed absence shows "none". A track record with no deliveries shows "NO RECORD", never an average. This matches the character rule (*Characters, Organisations & Events*).

### 6.2 Confidence ranges **(Candidate)**

The office shows an 80% range for each figure:

- `centre = claim − z(known_optimism) × σ_build`, where `known_optimism` uses only observed traits and the track record (0.50 if neither).
- `σ_shown = sqrt(σ_build² + σ_bias²) × k_assess`.
- `σ_bias` (uncertainty about the bidder's bias, in units of `σ_build`): 0.8 with no record, minus 0.12 per delivery under the same head, minus 0.2 if any trait with an optimism offset is known (Ambitious, Bold, Cautious, Principled), floor 0.2.
- `k_assess = 1.5 − 0.05 × assessor skill` (Science for performance, Industry for cost and schedule): 1.45 at skill 1, 1.2 at 6, 1.0 at 10. A poor assessor's range is wider, never wrong.
- `σ_build` comes from CSA 6 (understanding and documentation, through u_eff), scaled by the chief designer (5.1). `σ_prog` is added for schedule (5.1).
- A new head resets the head-specific part of the record. The dossier marks which head led each delivery.

### 6.3 The bid card **(Candidate)**

```
HR-1 BULK HAULER · BID 1 OF 3 · IVER YARD COMPANY (private)       [silhouette]
Head      Chairman Teodor Brask   Ambitious · ? · ?     Industry 5–8
Designer  Lena Szabo              ? · ?                 Science 6–8
Design    Iver Long-Hauler · NTR solid-core, pre-collapse, complete
                     CLAIMED        OFFICE 80%       REQUIRED
Delta-v loaded       11.25 km/s     11.0–11.4        ≥10.0 (prov.)
First delivery       276 d          208–400          ≤600
Unit price           66.5 Mcr       fixed            ≤60 Mcr  (over budget)
Track record         NO RECORD since the collapse
Dependencies         cryogenic storage: licence from Arden Orbital Works
                     yard reactivation: asteroid metal from Varga Mining
                     hydrogen: about 2.1 kt LH2 (18.6 kt water) per loaded trip
Deviations           none declared
```

- Card sections:
  - header: organisation, ownership, opinion
  - people: head and chief designer, known traits, dashed unknown slots, skill ranges
  - design source: component designs, documentation level, held or licensed
  - figures: claim, office range, requirement
  - track record
  - deviations and flags (orange)
  - dependencies
  - contract preference and rights offered
  - a render or silhouette assembled from the component designs, at a common scale (*Presentation & Visual Style* 2)

### 6.4 Comparison view and clarification **(Candidate)**

- It lives in the administration view (2D). Bids are columns (up to five) and figures are rows, sorted by the requirement's weights. Figures are in JetBrains Mono, labels in Barlow.
- Each figure row is a range bar: the requirement as a vertical line, the office range as a blue bar, the claim as an off-white tick. It's orange where the range crosses a hard requirement or a deviation is declared, and pale grey-blue dashed for unknowns.
- Silhouettes sit side by side at one scale above the columns. Portraits link to dossiers; dependencies link to the organisations involved.
- **Clarification actions** (each 10 days unless stated):
  - Ask for the basis of a claim. An honest answer cuts `σ_bias` for that bid by 0.2; an evasive answer is evidence of Opportunistic.
  - Request a revision.
  - Best and final offer.
  - Yard inspection by the office: reveals condition and order book.
  - Meet the head (Diplomacy): may reveal a trait; costs attention.
  - Paid design study (30–60 days, cost [P]): `σ_build` × 0.7 for that bid, and counts towards design finalisation if awarded.

## 7. Contracts **(Candidate)**

### 7.1 Types and terms

| Type | Who carries the risk | Notes |
|---|---|---|
| Fixed price | Bidder | Price padded by contingency; overruns hurt the bidder, so problems surface as variation claims or renegotiation |
| Cost-plus (fee 8% [P]) | State | Cheapest to start, open-ended; cost estimate shown as a range |
| Milestone | Shared | Priced per phase (design, build, trials); the state can stop after each phase, paying for work done; higher price |

- **Penalties:**
  - Late delivery: 0.2% of the price per day, capped at 10%.
  - Performance: each figure has a tolerance (default 3% worse than the claim). Beyond it, the price drops 2% per 1%, capped at 15%.
- **Incentives:** early delivery earns 0.1% per day, capped at 5%; figures beyond the claim earn a performance bonus.
- **Variations:** the bidder prices changes the state requests (traits apply). Bidder claims arise from ambiguity or from delays the state caused (a late licence, late materials).
- **Requisition (direction):** in emergencies the state takes yard capacity or a hull, at a large cost in influence, reputation, relations or opinion. This breaks the contract and records a grievance.

### 7.2 Traits and contract type

| Trait | Fixed price | Cost-plus | Milestone |
|---|---|---|---|
| Opportunistic | Underbids, then claims variations | Inflates booked costs: the worst pairing | Front-loads early phases |
| Bold | Accepts tight terms and penalties; may ask to renegotiate later | Fair, overruns from optimism | Fair |
| Cautious | Pads price | Fair, accurate | Prefers it |
| Principled | Honours losses | Honest booking | Fair |
| Ambitious | Accepts penalties to win | Expands scope | Pushes for later phases |

### 7.3 Rights to the design

| Option | Price effect | State can give it to another yard | Notes |
|---|---|---|---|
| State-owned design | +15% [P] | Yes; receiver must operationalise at its own understanding | Archive receives working documentation (complete if specified, extra cost) |
| Bidder-owned, state licence to operate and maintain | none | No; refits and variants go to the bidder first | Cheapest; dependency on the bidder |
| Shared | +7% [P] | Yes, with royalty to the bidder | Bidder may also sell to private buyers |

- Giving a bidder-owned design to another yard anyway is theft: grievance, track record of the state as a client, and possible poaching of staff [dependency: knowledge sharing rules].
- Direct assignment to a state-owned organisation always yields state rights.

## 8. Trials and surprises **(Candidate)**

### 8.1 Mechanism

- At design freeze the simulation draws each as-built figure from the organisation's distribution (median, and `σ_build` as reduced by the design work done). Nothing is redrawn later.
- Reveal schedule:
  - The design freeze report narrows estimates (spread × 0.6).
  - Monthly yard reports reveal mass by weigh-in, cost and schedule.
  - Trials reveal performance figures exactly.
  - Service reveals reliability and upkeep statistically over the first year.

### 8.2 Surprise types

| Type | Typical causes | Revealed by | Tells beforehand |
|---|---|---|---|
| Performance shortfall | High `perf_optimism`; low understanding; partial documentation; novel configuration | Design freeze estimate, drive tests, trials | Claim in the upper part of the range; Bold or Ambitious; no record; low band |
| Overweight | Low structure understanding; licensed component built for the first time; tankage | Yard weigh-ins, trials | Dependency on an unfamiliar component; structure band |
| Reliability problems | Low understanding; first of class; partial documentation | Trials (early failures), first year of service | Documentation level; first-of-class flag; Principled risk flags |
| Schedule slip | High `schedule_optimism`; dormant yard; order book; component shortages; late dependencies | Monthly yard reports | Yard condition; order book; dependencies list |
| Cost overrun | Cost-plus with Opportunistic head; materials; variations | Monthly reports | Contract type; price below office cost estimate |
| Upkeep or signature higher | Low understanding of power, thermal or life support | Service | Bands for those types |
| Dependency failure | Licensor delays or refuses; import route cut | Yard reports | Dependencies list; relationship between licensor and bidder |
| Pleasant surprise | Cautious padding; complete documentation; high understanding; practice | Trials, service | Cautious head; claim at the bottom of the range |

### 8.3 Player options

| Option | Effect | Relationship and record |
|---|---|---|
| Accept | Ship enters service; contract reductions apply | Fair acceptance builds opinion; record notes the shortfall |
| Demand rectification | Bidder fixes (its cost under fixed price, state's under cost-plus); 30–120 days; result redrawn within a narrower spread | Neutral if outside tolerance; seen as harsh inside it (Opportunistic and Ambitious heads take a grievance; Principled do not) |
| Renegotiate | Change price or requirement (relaxing a provisional field is cheap) | Small opinion gain for the bidder |
| Reject | Refuse delivery; payments returned minus completed milestones; hull may go to the private market | Major grievance; record "rejected" |
| Take over the design | State acquires the design (needs or buys rights) and moves it to a state-owned organisation | Costs attention and goodwill; receiver operationalises at its own understanding |

- Every outcome updates the track record: each figure claimed against as-built, days late, cost against price, rectifications, and whether problems were reported early.

## 9. Worked example: HR-1 **(Candidate)**

**Requirement HR-1:** a state bulk hauler for Corvane.
- 400 t of cargo, Iver → Halden IV.
- Delta-v loaded at least 10 km/s (**provisional**; a Hohmann hop needs 7.5 km/s, see *Drives & Transit*).
- Crew at most 12 (hard). Quantity 2. Unit budget 60 Mcr.
- Build time at most 400 days, so first delivery within about 600 days of award.
- Acceleration and transit time left **open**.

Invited tender, 20 days. The office is led by Chief Engineer Ottilie Marsh (Science and Industry about 6, so `k_assess` 1.2). The three designs and their as-built distributions are in CSA section 7; this section adds the bidders' behaviour.

### 9.1 What each bids and why

- **Iver Yard Company** (private; Chairman Teodor Brask, Bold and Ambitious; chief designer Lena Szabo).
  - Holds a complete pre-collapse NTR solid-core record (u_eff 7).
  - Bids three Kestrel NTRs on hydrogen, because a water NTR (v_e about 3.2 km/s) can't reach 10 km/s with 400 t (*Drives & Transit*).
  - Bold + Ambitious: quotes performance and schedule at optimism 0.85, with no contingency. Ambitious: bids although the yard is dormant.
  - Dependencies: cryogenic storage licensed from Arden Orbital Works, asteroid metal from Varga Mining for reactivation, and about 2.1 kt of LH2 per loaded trip.
- **Arden Orbital Works** (state-owned; Works Director Margit Hollander, Cautious and Traditionalist; chief designer Piet Overbeek).
  - Chemical 7, NTR 4, on a working-level record.
  - Traditionalist prefers chemical, which can't reach 10 km/s with 400 t, so it falls back to two Tern NTRs (4 is the Traditionalist floor).
  - Its own median is 10.05 km/s. Quoting at optimism 0.35 puts the claim at 9.97, so it **declares a deviation**.
  - Cautious adds 10 points of contingency. The build fills its only large slipway.
- **Morrow Ice & Freight** (private; Managing Director Idris Calloway, Innovative and Opportunistic; chief designer Yusuf Aranda).
  - Rebuilds its salvaged Sable NEP hauler from partial records (NEP 5, fission 4).
  - Innovative: NEP meets the delta-v easily, because acceleration and transit were left open.
  - Opportunistic: margin cut to 0 on fixed price. The requirement says nothing about radiator margin, so it designs with none.
  - Leases a slipway at Arden Highport, competing with Arden's own bid.

### 9.2 What the bid cards show

| | Iver Yard Company | Arden Orbital Works | Morrow Ice & Freight |
|---|---|---|---|
| Rated delta-v (design record) | 11.50 km/s | 11.00 km/s | 12.00 km/s |
| Bidder's own median (hidden) | 11.13 km/s | 10.05 km/s | 11.01 km/s |
| Claimed delta-v (office 80%) | 11.25 (11.0–11.4) | 9.97 (9.7–10.4), deviation | 11.01 (10.7–11.3) |
| Iver → Halden IV, loaded (office) | about 82 d | about 89 d | about 320 d |
| First delivery, days (office 80%) | 276 (208–400) | 525 (447–551) | 456 (344–605) |
| Unit price | 66.5 Mcr fixed, over budget | 64.3 Mcr fixed; about 51.5 at cost if directly assigned | 54.3 Mcr fixed |
| Office cost estimate (P10–P90) | 58.1–62.9 | 48.0–54.7 | 49.6–53.6 |
| Track record | NO RECORD since the collapse | 4 state deliveries (tugs, shuttles): 3 on time, 11 of 12 figures met | 2 refits of its own fleet observed: 1 late |
| Head's traits shown | Ambitious · ? · ? | Cautious · Traditionalist | Innovative · ? · ? |
| Flags (orange) | over budget; two dependencies; dormant yard | deviation: delta-v; slipway conflict | slow transit; no radiator margin; partial documentation; single reactor |

- **Ranges are narrow on performance and wide on schedule.** CSA's per-hull variance is small at these understanding levels, so delta-v claims are within a few per cent. The big uncertainty is programme risk (dormant yard, first-built licensed tanks, leased slipway), carried by `σ_prog` (5.1). Brask's claim of 276 days sits well inside a range running to 400.
- Known traits move the office's centre (Brask: Ambitious known, Bold not yet). Calloway's unknown Opportunistic trait doesn't touch the performance range. Its danger shows in the design choices the requirement left open, which the flags name.

### 9.3 Choices and trade-offs

- **Iver Yard Company:** the fastest ship and the best claims, and reactivating Iver Yard gives the state a second large yard (Pillar 2). But it's 10.8% over budget, with the widest schedule range, no record, and a hydrogen appetite of about 18.6 kt of water per trip (about six months of Halden's ice quarry). It suits fixed price with a reactivation milestone and a late penalty, and only if the state also funds electrolysis.
- **Arden Orbital Works:** honest and state-owned; direct assignment gives state rights at about 51.5 Mcr, with the state carrying overruns. But it misses the provisional 10 km/s at its own estimate, and blocks Arden's slipway (and Morrow's lease) for about a year and a half. Relaxing delta-v to 9.9 km/s costs only a day or two per trip with good phasing, but leaves less margin for poor phasing (*Drives & Transit* 5).
- **Morrow Ice & Freight:** the only bid under budget, and frugal (about 200 t of water per trip against Iver's 18.6 kt). But a loaded leg takes most of a year, and as built the reactor derates and the chance of a clean trip falls to about 35% (CSA 7). Avoid cost-plus; use fixed price with a tight variation clause and a stated radiator margin.
- **Split award** (one Iver, one Morrow): a fast ship for urgent cargo and a cheap one for bulk, at the cost of two learning curves and double the attention.

### 9.4 A foreseeable surprise

The player awards both hulls to Iver Yard Company on fixed price with milestones.

- **Schedule slip:** monthly reports from day 30 show reactivation falling behind (Varga Mining is short of asteroid metal), then the licensed Rime tanks failing their first weigh-in. First delivery is on day 390: 114 days past the claim, but inside the office range (208–400). The late penalty hits its 10% cap: 6.7 Mcr.
- **Cost overrun absorbed:** Iver's cost lands near its 60.4 Mcr median. On fixed price that is Iver's problem. Brask (Bold) reports it late, then asks to renegotiate the penalty. Accepting builds opinion; refusing risks a grievance from an Ambitious head.
- **Performance as claimed:** trials give 11.1 km/s against a claim of 11.25, inside tolerance. The NTRs' reliability beats expectation (complete documentation).
- **The operating surprise that wasn't:** hull 1 then waits at Halden IV for hydrogen. This was on the card from the start, in the dependencies line.
- **Tells the player had:** the dormant yard, a licensed component built for the first time, no record, an unobserved trait slot, the hydrogen line. Brask's Bold trait is now revealed; his record reads "1 delivery, 114 d late, delta-v met, reliability exceeded". Hull 2 benefits from practice (CSA 6, learning).
- Had Morrow won, the likely surprise was the as-built reactor derating with no radiator margin (in trials), followed by a variation claim for extra radiators. The tells were the fission band, partial documentation, the "no radiator margin" flag and an Opportunistic margin of zero.

## 10. Other uses **(Candidate)**

### 10.1 Components

- The state can tender component production (for example 20 NTR drives for its own yards) with the same lifecycle. The card shows the component stat block (CSA) instead of ship-level figures. Component *development* (raising understanding or documentation) belongs to research [dependency].

### 10.2 Stations and ground equipment

- **Stations:** the same lifecycle, with templates by role (habitat, depot, yard module, gate maintenance post). Built in place by organisations with construction tools; trials become commissioning. Needs the station catalogue (*Component Catalogue*, open question).
- **Ground equipment:** tendered to works and factories rather than yards. Requirements add landing mass and surface gravity class. Needs the ground equipment catalogue and ground combat rules.

### 10.3 Civilian designs

- Private organisations commission their own ships with the same bid generation and an automatic award; the state sees them in yards' order books. The state may sponsor a "standard design" tender and release the design to all yards, to grow civilian shipping [dependency: economy].

### 10.4 AI factions and organisations

- AI states run the same lifecycle: requirements from their strategic needs, awards by weighted score adjusted by their leader's traits, and the same trial draws. Organisations bid to AI states under the same rules. There are no AI shortcuts, so foreign fleets show the same variety. Foreign designs are known only as well as they've been observed (*Presentation & Visual Style*, inspection fidelity).

### 10.5 Upgrades, refits, variants, replacement

- **Variant (decided: designs persist and can be upgraded):** a new version in the design's lineage (HR-1 → HR-1a), tendered to the rights holder first and used for new builds.
- **Refit:** yard work on hulls in service to fit a variant's changes. Limited to the hull's structural envelope: no change of drive family beyond its mass and thrust limits [P: CSA].
- **Replace:** a new requirement. Rule of thumb: refit when the refit costs under about half a new hull and the change stays within the envelope [P]. The office states this comparison on refit bids.
- Follow-on hulls improve with practice without any refit; the track record shows this as a separate line.

## 11. MVP scope **(Candidate)**

The smallest version that still delivers Pillar 4:

- **In:**
  - three templates: bulk hauler, tug, patrol
  - hard, target and provisional fields
  - invited tender and direct assignment
  - 3 to 4 organisations
  - bid generation with optimism quantiles, margins, and design choice for Bold/Cautious, Principled/Opportunistic and Innovative/Traditionalist
  - office ranges (6.2)
  - the bid card and comparison view
  - fixed price with a late penalty and performance tolerance
  - the draw at design freeze, monthly yard reports, acceptance trials
  - accept, rectify or reject
  - track record and trait reveal
- **Deferred:**
  - open tenders and sole source
  - cost-plus and milestone contracts
  - rights options beyond state-owned
  - clarification actions other than "ask for the basis of a claim"; design studies
  - variations; requisition; refits and variants
  - component, station and ground tenders; civilian standard designs
  - AI states using the full pipeline (a simplified auto-award first)
  - photoreal renders (silhouettes from component blocks instead)

## 12. Open questions

- Whether the chief designer's skill should scale variance here or in CSA. The size of `σ_prog` per risk type (only illustrative so far).
- Is `optimism` per figure or one value per bid? (For now, one per category: performance, schedule, cost.)
- How many deliveries before a track record is trusted? Does a reputation from before the collapse exist at game start?
- Who leads the state technical office: a cabinet portfolio, a state lab, or an advisor?
- How management attention is measured and spent [dependency: administration].
- Should rejected hulls, cancelled designs and losing bids enter the private market or archives?
- Can the player write a bid's design themselves (state design bureau), and at what cost?
- Do organisations ever bid to rival states, and what does that do to secrecy?
- Is the performance tolerance a contract choice per figure or a fixed rule?
- How often do bids need to happen to keep the system central without becoming busywork (target: a few per game year)?

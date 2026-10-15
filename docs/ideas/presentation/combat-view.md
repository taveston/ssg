# Orphan Worlds: Combat View

This doc proposes how combat is shown: the engagement on the map while it runs, the Watch or Skip prompt, the replay view, orders and standing orders, and the after-action report. The main ideas:

- **The replay is a relative plot.** It is drawn in the own force's frame, where the enemy's coasting path is a straight line past you and the closest approach is obvious. It is the same vector display as the system view, on its own clock.
- **No spoilers.** Nothing shows the outcome until the player watches or skips: not the map, the tray, the inset or the timeline.
- **The replay shows what the state saw.** By default it shows the player's picture as it was, with weak tracks and unknown figures. A second mode adds what was learned afterwards. The true state is never shown.
- **The fight is decided before contact, so show it before contact.** The orders panel carries an estimate of the central contest: their salvo against our point defence.

All **Candidate**. Mock-ups: `mockups/13-combat-replay.png` to `17-missile-symbols.png`, all from the worked example in *Combat, Raiders & Diplomacy* 1.7. Builds on *Presentation & Visual Style* (2), *Symbology*, *Interface Style* and *Combat Simulation*.

---

## 1. Where combat appears

| Moment | Where | Mock-up |
|---|---|---|
| Before | Orders and standing orders on the force's inset; the estimate | 15 |
| While it runs | One engagement object on the system map | 14 A |
| When it ends | FLASH item in the tray, clock paused, Watch or Skip | 14 B |
| Watching | The replay view, a layer below the system view | 13 |
| After | Report in traffic; results on the map; dealings in the commander's file | 16 |

## 2. The engagement on the map

- **One object.** While a fight runs, its ships are drawn as one engagement: each side's symbol at its position, joined by a short link with inward chevrons. It is selected, hovered and inspected as one object, like a folded group. The inset shows when it began and when it ends, and who is in it.
- **Not commandable.** Orders to its ships are accepted but queued, and the inset says so. The rest of the map carries on at the main clock.
- **At the end.** The clock pauses with a FLASH item: "Engagement ended: E-7". Its buttons are Watch and Skip to result, with the date the recording is kept to. Until one is chosen, the engagement stays on the map as "ended · result held".
- **The queue.** Overlapping fights queue in end order. The tray item names the next one. A fight the player could watch but that isn't theirs (seen in sensor coverage) joins the queue; whether it also pauses the clock is a tunable (*Combat, Raiders & Diplomacy* 1.8).
- **Precedence.** A fight with own ships is FLASH and pauses. A seen fight between others is IMMEDIATE and doesn't, unless the tunable says so.
- **Map symbol (answers the open question in *Combat, Raiders & Diplomacy* 4):** the two-sided link. A single generic engagement icon was considered; it hides which side is which and how big each is, and the symbols already carry that.

## 3. The replay view

**Layout** (board 13): the top bar shows the breadcrumb (… / Halden IV / Engagement E-7), a REPLAY chip, the tactical clock, the rate controls, "main clock paused", Skip to result and Close. The map fills the left. A sidebar holds the forces and the log. A timeline runs under the map.

**Frame.** The default frame is the own force's, non-rotating, labelled "FRAME · TF-1 (relative)". Over a few hours, gravity barely bends the paths, so in this frame the enemy's coast is a straight line, and the closest approach and standoff can be read off it. It is the naval relative-motion plot, which suits the plotting-console look. The frame can be switched to the enemy force, or to the engagement's centre of mass. The parent body shows as an edge tab (MRK-15).

**Scale.** True scale, as the system view. A window covers tens of thousands of km, and point-defence range is a few hundred, so the camera zooms like the system view. Range rings show weapon and sensor ranges round the selected force: point defence at 300 km, radar at 20,000 km, lidar at 50,000 km. These are the capability rings *Presentation & Visual Style* asked for.

**What is drawn:**

- Ships and forces as their symbols (SYM), with velocity leaders along the relative motion.
- Each ship's path: solid behind, dashed ahead (as *Orbital Transfers* 3.3), in the relative frame.
- Missiles as **salvos** (board 17): one small solid affiliation shape per salvo (own circle, hostile diamond, unknown quatrefoil, about 8 px, no frame or icon), so affiliation reads without colour and a missile never reads as a ship. A velocity leader shows direction and grows with speed (SYM-11). The count sits beside it, and the label says what the state knows: the design name when known ("Dart" for our own), otherwise "Missile". Speed and range to target show when selected or close.
- **A salvo never splits.** Its missiles are within a few km of each other, the same pixel at any zoom that shows point-defence range. The count drops as missiles are shot down (a small burst at the mark per kill) or decoyed. A decoyed missile gets its own mark only if its path visibly diverges.
- **Spent salvos fade.** Once the viewing side could see a salvo is no longer a threat, it is drawn hollow and faint, with no leader or count, and drops out of the inbound figures. Spent means: moving away from every ship it could hit, burnt out on a path that misses by more than the track error, or seen to take a decoy. It is judged from what that side observed, never from the simulation's truth, so an enemy salvo isn't dimmed the moment its seeker is fooled. Unguided rounds follow the same rule. Spent marks fly on out of view.
- Point-defence fire as short ticks from the mount towards the salvo mark. Decoys as faint dots.
- A hit as a flash on the target's frame; damage as the condition bar dropping; disabled as the stale-style dimming plus a "disabled" label; destroyed as the salvage X.
- Missed missiles and rounds fly on and off the edge of the view. A missile that is shot down ends in a small burst.
- A force that splits (stragglers left behind, non-combatants detached) shows as two symbols from that moment.
- Track quality as the symbology already shows it: a weak track has a dashed frame and its uncertainty region.

**The clock.**

- Time counts from the moment the engagement formed: "T+75m 30s", in minutes and seconds (hours once past 60 min: "T+1h 15m 30s"), with the date and time beside it.
- Closest approach is information, not the clock's origin. It is a prediction from the current rails and moves when anyone burns, a fight between several forces has one per pair, and a Rendezvous fight has none. Counting towards the true closest approach would also give away the result. So it shows as a timeline marker and a caption: "Closest approach in 17m 30s (predicted) · 1,500 km".
- **Auto** rate is the default: about ×60 while everything coasts, easing to ×1 near events and slower inside point-defence range, where a missile crosses in 18 s. Fixed rates (×1, ×10, ×60) are there too. Pause is Space.
- A short caption above the map names the next thing: "R-2's salvo reaches TF-1's point-defence range in 30 s". Captions come from the event log (*Combat Simulation* 11).
- Skip to result ends the replay and shows the report.

**The timeline.** The window as predicted at its start, with the predicted closest approach marked (moving if a burn changes it) and the fine-step stretch shaded. Events are ticks. Past events are labelled; later ones are unlabelled, so scrubbing ahead tells the player only that something happens, not what. [ and ] jump to the previous or next event.

**The sidebar.**

- **Forces:** each side's symbol, name, commander, and the figures the state knew at that moment, in the certainty notation: "8 missiles launched · more ?", "PD ~1 mount", "Δv 3–7 km/s".
- **Log:** events up to the playhead only, with decision tags attached to the events they caused ("Marr held 75% of magazines").
- **Picture:** As seen then (default) or As known now. The second adds what the state learned from the fight and since: the confirmed class, the observed loadout, a later inspection of the wreck. Neither mode shows the truth: an enemy figure nobody observed stays "?". This keeps the fog consistent with *Combat Simulation* 4, and lets a replay improve as intelligence arrives.

**Keys:** Space pause, [ and ] previous and next event, . and , rate, F frame on the selection, Esc close. Mouse as the system view.

## 4. Candidate overlays

- **Reachable envelope** of a contact: the region it could reach with its remaining delta-v, drawn as a dashed shape round its predicted position, growing ahead. A ship that hasn't burned has a thin one; one with lots of delta-v and a weak track has a wide one. This is the uncertainty *Combat, Raiders & Diplomacy* 1.5 says to show. It also belongs on the system map.
- **Launch window:** the stretch of the relative track from which a missile can still reach the target, ending at closest approach.
- **Point-defence load:** a bar per force showing missiles inbound against what its mounts can stop in the time they have.

## 5. Orders and standing orders

Board 15 shows the orders from *Combat Simulation* 10.

- **Orders** are a section of the force's inset: objective, approach and standoff, weapons, target priority, emissions, and the reserves (return delta-v and magazines held back). Controls are segmented buttons; one choice is always set. Choices the simulation can't yet do (Capture) are shown and disabled.
- **The commander** heads the panel with what the state knows: name, Command as a range, observed traits as chips and unobserved ones as a dashed chip. A line under the orders says how boldly to carry them out is the commander's call.
- **Standing orders** are a list of trigger and response, checked in order. Triggers name places and threat levels ("armed hostile or unknown inside Halden IV High reach"). Responses are Intercept (with the orders), Hold, Evade or Alert me. The list shows when an order last fired.
- **The estimate** is a card beside the orders against a chosen or likely target. One bar shows the central contest: their salvo (solid for the sure part, dashed for the range) against the number our point defence can stop from one side. Under it: closing speed, time in radar range, missile time in point-defence range, whether kinetics get a shot, their delta-v, and ours after the pass with the return need.
  - It uses only what the state knows, in the certainty notation.
  - It is the player's main tool for "the fight is decided before the shooting". Without it, the decisive choice (the intercept order days before) is made blind.
  - It is a guide, not a forecast: it doesn't predict the outcome and doesn't model the commander.

## 6. The after-action report

Board 16. It follows the report layout in *Interface Style* 7.

- **Signal header,** then a **one-sentence conclusion** ("R-2 destroyed on an intercept at Halden IV High, with no loss to TF-1"), then a short account in the commander's voice.
- **Ships:** status, condition, missiles used and delta-v spent, per ship or per force. Enemy figures in the certainty notation.
- **Timeline:** six to ten lines from the event log, the decisive ones only.
- **Decisions:** the commander's decision tags, written as behaviour ("held 75% of magazines", "left *Wren* behind"), never as the trait.
- **Track plot:** a small relative plot with the tracks, the closest approach and where ships were hit. Coarse; the replay is the detail.
- **What we learned:** knowledge updates, new salvage sites, and our own state.
- **Actions:** the decisions the report asks for (divert the wreck, refit, resupply), then Show on map and Watch replay while the recording is kept.
- **The commander's file** gains a dated line per engagement with the tags, and a combat record (engagements, kills, losses). The trait chip is the state's reading of those lines. Drift from experience (*Combat, Raiders & Diplomacy* 1.9) is never stated; the player sees it as a change in the tags over time.
- **Fights the player didn't see** produce no report (fog, 1.8). A lost own ship produces a short loss report from the last known contact, with no replay.

## 7. Open questions

- Whether the replay can be entered while the fight still runs (a "live" view), or only after. Proposed: after only, because the result is already computed and a live view would invite orders that can't be given.
- Whether Auto rate should be the only rate.
- How to show many salvos at once in a large fight without clutter.
- Whether point-defence ticks are worth drawing, or a counter on the defending force reads better.
- Whether the estimate card should appear in the transfer planner when the destination is a contact.
- How a long blockade (many phases) is shown: one engagement with phases, or one per phase.
- Whether As known now should be offered for old replays, which would then change after they were first watched.
- How the replay handles a ship leaving the window early (it vanishes, or is shown leaving).
- Ground combat's view, which belongs with the planetary view.

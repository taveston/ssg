# Spec: Transfer planner

**In short:** how the player plans, schedules and watches a transfer for one ship. A Transfer button on the inset of an own commanded ship opens the planner in the inset. The player clicks a place on the map or in the list, then tunes two sliders, Depart and Travel time. Each slider's track is coloured by the delta-v its settings need, so windows show as bright bands, and three presets cover the usual choices. One live path, drawn in the geometry of its own dates with ghost marks at both ends, shows the result. A scheduled order shows on its ship, and a ship in flight always shows its path. A ship in its own orbit has no Depart slider, and its Travel time snaps to laps. The planner works on a ship parked in an orbit with no order, or shows the order it has. Groups, multi-leg plans, the return reserve, alerts, other people's ships and re-tasking in flight come later.

*Scope:* the planner's panel, its controls and readouts, the search rules behind them (ranges, tracks, presets), and how orders and paths are drawn on the map. The simulation is `docs/spec/transfers.md` and is not repeated here. *Source ideas:* `docs/ideas/mechanics/transfers.md` sections 3.1 to 3.3, 3.5 and the planner parts of 4.4; mock-ups 9 and 10 in `docs/ideas/presentation/mockups/`, and the arrival-speed row of 11. Mock-ups 11 and 12 otherwise show the return reserve and multi-leg plans, which are not specced. Where a mock-up and this spec disagree, this spec wins (the mock-ups' blue Schedule button, for example, breaks BTN-01 and BTN-02).

*How to read it:* each requirement has a stable ID. **Built** is ✓ once the requirement is in the code. Numbers in `code` are tunables (tables at the end). Simulation tunables are read from `src/sim/tunables.rs` and view tunables from `src/view/tunables.rs`, never hard-coded.

*Retired IDs* (never reuse): none.

---

## 1. Opening the planner (PLNR)

| ID | Requirement | Built |
|---|---|---|
| PLNR-01 | The inset of a ship that the state commands (SYM-01), that is not a group and that is parked in a named orbit shows a **Transfer** button beside Focus (secondary, BTN-01), when the ship has no plan that is Scheduled, InFlight or Holding (PLAN-02). A ship in flight or coasting (RAIL-09, BAND-07), a group, and a ship the state doesn't command show no Transfer button. | |
| PLNR-02 | Pressing Transfer opens the planner. The inset's image and key facts give way to the planner panel (sections 2 and 3), the ship stays selected, and the planner starts in the *choosing* state (PLNR-10). Esc and a quiet Cancel button end the planner, discard its settings and restore the inset's normal contents, with the ship still selected. Close closes the inset and ends the planner. | |
| PLNR-03 | While the planner is open: a click that hits a place picks it as the destination (PLNR-11) instead of selecting it (INS-01); a click on empty map space does nothing; Esc ends the planner before it clears the selection (INS-03); and the clock keeps running, because the planner never pauses it. Pan, zoom, rotate and every other camera control work as usual. | |
| PLNR-04 | The planner reads the ship's available delta-v (PLAN-21), its drive and its orbit. Every figure it shows, every range and every preset comes from the solver and the search rules of section 7, and every line of text and every track colour from a plain function of those results (ARCH-20). The rules are in `src/sim/transfer/`. The view only draws, moves sliders and writes an order through the simulation's order API (ARCH-14). *Checked in review.* | |
| PLNR-05 | The inset of an own ship that the state commands lists two more key facts (INS-09). **Delta-v** is the ship's `delta_v_kms` less `spent_kms` (UNIT-04), followed by " · *N* reserved" when `reserved_kms` is above zero. **Drive** is the drive's display name. A drive record carries a display name, which the content sets (TEST-30). | |

## 2. Choosing a destination

| ID | Requirement | Built |
|---|---|---|
| PLNR-10 | In the choosing state the panel reads "Choose a destination: click a place on the map or in the list below", and the inset's "In orbit" outline (INS-05) sits below it. The outline stays below the panel in the tuning state too, and picks destinations as map clicks do. | |
| PLNR-11 | What a click picks. A body's marker, true-size disc, fold pill (GRP-07) or outline entry picks that body's High band, or its Low band if it has no High band (BAND-01). A shared orbit's ring or outline row picks that body's band of that orbit (HOV-04). A lone object's ring picks what HOV-02 says it targets: that body, station or ship. A station symbol picks that station, a ship symbol that ship, a gate's marker or edge arrow that gate, a salvage site that site (BAND-02). These can't be picked, get no hover ring (HOV-01), and a click on them does nothing: the ship being planned; a group; a ship whose track is weak or stale (SYM-08); a ship in flight that the state doesn't command (its true path isn't known to the state); and a ship that has a plan that is Scheduled, InFlight or Holding (its meeting point could change under the order). A ship parked in an orbit, or coasting with no plan, can be picked. | |
| PLNR-12 | While the planner is choosing, hovering a place that can be picked draws the path of its Fastest setting (PLNR-83) with its ghosts (PLNR-63), and shows one line under the prompt. For a plan that can be flown it reads "Fastest: *verb* *date*, *x* km/s", with the verb of PLNR-25 and *x* the leg's delta-v. When no setting is affordable it reads "Can't reach: the cheapest trip needs *x* km/s, has *y*" in the warning colour. When the solver can't plan it, it gives the reason (PLNR-92). For a station or ship in the ship's own orbit, the line adds the gap (PLNR-41). | |
| PLNR-13 | Picking a destination, or changing a body's band (PLNR-14), starts the tuning state with both sliders at the Fastest setting, or at the Cheapest setting if none is affordable. Picking another place while tuning does the same. Moving a slider or changing Stop or Pass (PLNR-30) keeps every other setting. | |
| PLNR-14 | A body destination that has both bands shows a two-way choice, **High** and **Low**, in that order, with High selected by default. A click on a band's ring selects that band (PLNR-11). A body with one band shows no choice. In a two-way choice the selected option is filled with the neutral colour mixed into the ground at `segment_fill_mix`, with neutral text, and the other is outlined like a secondary button (BTN-01). | |

## 3. Tuning the transfer

| ID | Requirement | Built |
|---|---|---|
| PLNR-20 | The tuning panel holds, from the top: the heading **Transfer** in small spaced capitals (INS-09); the route "*ship* → *destination*", names in the label face; the ship's place and its drive in the secondary colour, "*place* · *drive*", where a band reads "High orbit, Halden IV" and a station, gate or site reads "At *name*"; the High / Low choice (PLNR-14) when there is one; Depart (PLNR-21); Travel time (PLNR-22); the arrival row (PLNR-25); the arrival speed row (PLNR-26); the closest approach row (PLNR-32), when there is one; Delta-v (PLNR-27); the presets (PLNR-29); Stop / Pass (PLNR-30); Schedule and its reason line (PLNR-31). Labels are in the label face and figures in the data face (MRK-02), at the inset's sizes (INS-09). | |
| PLNR-21 | The Depart slider sets the order's depart day over the Depart range (PLNR-80). Its readout is the **leave day**, the day the ship leaves its orbit: the depart day plus the departure wait (PHAS-03), shown as a date (PLNR-93) and "(in *N*)" (UNIT-02), or as "now" when it is today. | |
| PLNR-22 | The Travel time slider sets the order's `travel_days` over the Travel range (PLNR-81). Its readout is the duration (UNIT-02). For a low-thrust drive (XFER-06) there is no slider: the row shows the fixed travel time, and a line beneath it reads "Travel time is fixed by this drive". | |
| PLNR-23 | A slider moves in steps. The step is the smallest of 1 minute, 10 minutes, 1 hour, 6 hours, 1 day, 5 days and 30 days that leaves at most `slider_max_steps` positions along the range. The player drags the thumb or clicks the track. A slider that has focus takes Left and Right (one step, ten with Shift) in place of the panning of CAM-14, with its keys read from the bindings resource (CAM-18). The track is `slider_track_px` high and the thumb `slider_thumb_w_px` by `slider_thumb_h_px`. | |
| PLNR-24 | The depart day is an absolute day. As the clock runs, the start of the Depart range moves with it. A depart day that falls before today becomes today, and the travel time stays, so the arrival moves later. | |
| PLNR-25 | The arrival row shows the date the leg ends and "(in *N*)". For a Stop it is **Arrives** with the arrival day (XFER-02); for a Pass it is **Passes** with the crossing end day; when the destination is a ship it is **Meeting**, in both cases. A station reads as a body does: Arrives or Passes. The verbs "arrives", "passes" and "meeting" are used wherever a line below refers to this row. | |
| PLNR-26 | The arrival speed row reads **Arrival speed**, or **Closing speed** when the destination is a ship, in km/s (UNIT-04). For a Stop it is the speed relative to the destination body before capture, the magnitude of (arc end velocity − the body's velocity). For a Pass it is the relative speed of XFER-09, which for a station is the speed relative to the body, since a Pass doesn't read the station's own motion (PHAS-11). When the destination is a ship it is the magnitude of (arc end velocity − the target's velocity) (XFER-10). A phasing leg (PLNR-40) has no such row. | |
| PLNR-27 | The Delta-v row reads "*leg* of *available* km/s" (UNIT-04), with *leg* the leg's delta-v (XFER-04) and *available* the ship's available delta-v (PLAN-21). Under it is a bar `delta_bar_px` high, in the inset's hairline colour, filled in own blue (MRK-01) to *leg* ÷ *available*. A line beneath reads "*available* − *leg* km/s left after arrival", or "after the pass" for a Pass. When *leg* is more than *available*, the readout and the fill are in the warning colour, the bar is full, and the line reads "Over budget by *x* km/s". While an order is being changed (PLNR-53), *available* counts the delta-v that order reserved. | |
| PLNR-28 | Each slider's track is coloured by what its settings cost. The Depart track is a gradient through the grid's depart days (PLNR-82), each showing the lowest delta-v of any travel time on that day. The Travel time track is a gradient through `track_travel_samples` travel times spread over the Travel range on the current depart day, each showing its delta-v. A setting the solver rejects is the track's empty colour (the inset's hairline colour). A setting over budget is the warning colour at `track_over_alpha`. An affordable setting is own blue mixed into the empty colour at *m* = `track_dim_mix` + (1 − `track_dim_mix`) × (1 − *t*), where *t* = (*v* − *v_min*) ÷ (*available* − *v_min*) is held between 0 and 1, *v* is the setting's delta-v and *v_min* the lowest delta-v on the grid (*t* is 0 when *available* equals *v_min*). The cheapest settings are brightest. A window is never named: it shows as a bright band. | |
| PLNR-29 | Three presets sit in a row as secondary buttons (BTN-01): **Fastest**, **Cheapest** and **Next window** (PLNR-83 to PLNR-85). A preset sets both sliders and nothing else. Fastest is disabled when no setting is affordable. A preset that doesn't exist for this ship and destination, or whose setting equals an earlier preset's, is hidden. | |
| PLNR-30 | **Stop** and **Pass** are a two-way choice (drawn as PLNR-14), Stop first and selected by default. Pass is offered for a station as it is for a body: a drive-by past a station, without matching its orbit, is a raid. A station, or a ship parked in a band, is passed through that band, so Pass is disabled, drawn as the inset's other disabled buttons (INS-02), when the band the Pass would take is Low (XFER-08): a Low band, or a station or ship parked in one. The choice is hidden for a phasing leg, which is always a Stop (PHAS-05). Changing it solves the leg again with the same Depart and Travel time. | |
| PLNR-31 | **Schedule** is the panel's one primary button (BTN-01). It is disabled while the setting is infeasible or over budget. A one-line reason beneath it, in the warning colour, says why (PLNR-92), and the line is empty while Schedule is enabled. | |
| PLNR-32 | **Closest approach.** For a Pass at a station or at a ship parked in a band, a row under the arrival speed row reads **Closest approach** and a distance (UNIT-01): how near the ship goes to that target. Let *Δ* be the angle between the Pass's closest-approach phase (PHAS-11) and the target's phase on the crossing end day, and *r* the band's radius. The distance is 2 *r* sin(*Δ* ÷ 2). It is a reading of the plan and says nothing about whether anything meets or engages, which PHAS-11 leaves to Conflict. There is no such row for a Stop, a Pass at a body, a target in flight, or a phasing leg. | |

## 4. A ship's own orbit

A target in the ship's own orbit has no windows: the gap never changes, so any day will do, and the only choice is how many laps to fly (`transfers.md` 4.4, PHAS-05).

| ID | Requirement | Built |
|---|---|---|
| PLNR-40 | A destination that is a station or a parked ship in the ship's own named orbit, and not together with it (PHAS-05), is planned as a phasing leg. The panel has no Depart slider: its row reads "Departs" and "now", and the depart day is today. Stop / Pass is hidden. The Travel time slider has one position for each meeting the solver can fly (PHAS-08: the times *t_k* of each feasible *k*, in order), its track coloured as PLNR-28 by the delta-v of each, and its readout reads "*duration* · *n* laps", with *n* the laps of the phasing orbit the solver chose (PHAS-06). Only Fastest and Cheapest are offered. | |
| PLNR-41 | For a destination in the ship's own orbit that is not together with it, the panel shows a **Gap** line: the straight-line distance to the target (UNIT-01), then "ahead" when the target is half a turn or less ahead in the sense of motion (PHAS-01) and "behind" when it is more, for example "Gap 12,000 km ahead". | |
| PLNR-42 | A destination that is the ship's own orbit, or a station or ship in it that is together with the ship (PHAS-04), disables Schedule with the reason "Already there". | |

## 5. Scheduling and orders

| ID | Requirement | Built |
|---|---|---|
| PLNR-50 | Schedule makes a one-leg plan (PLAN-01) for the ship with the chosen destination, arrival mode, depart day and travel time, through the simulation's order API (ARCH-14). The plan is Scheduled and reserves its delta-v (PLAN-21), and the ship doesn't move until its depart day. The planner closes and the inset shows the order (PLNR-51). | |
| PLNR-51 | A ship with a plan that is Scheduled or InFlight shows its order in its inset, in place of the Transfer button: "→ *destination*"; **Departs** with the leave day (PLNR-21), reading **Departed** once it has passed; the arrival row of PLNR-25; and the leg's delta-v (UNIT-04). A Scheduled order has **Cancel** and **Change** buttons (secondary). An InFlight order has neither (PLAN-22). | |
| PLNR-52 | Cancel cancels the plan (PLAN-22): its reservation is released, its path leaves the map, and the inset shows the Transfer button again. | |
| PLNR-53 | Change opens the planner in the tuning state with the order's destination, arrival mode, depart day and travel time, with the order still in force. Schedule replaces it with the new order. Esc and Cancel leave it as it was. | |
| PLNR-54 | A ship coasting after a Pass shows, as its place in the inset, "Coasting · passed *destination*, from *origin*" (PLAN-12), and no Transfer button. Its path is drawn as PLNR-62 says for a ship in flight. | |

## 6. On the map

Colours are the palette of MRK-01. Mock-up 10 shows the states.

| ID | Requirement | Built |
|---|---|---|
| PLNR-60 | A path is the crossing of a plan: its Conic and Spiral segments (RAIL-02), drawn at true scale (SCL-02) as seen from the camera's frame, with each segment's points placed relative to its primary, which is placed where it is now (SCL-06). It starts where the ship will be on the day the crossing starts and ends where the destination will be on the day it ends (XFER-02). It is never drawn from today's positions. Its accuracy, and its cut at the camera's near limit, are those of an orbit ring (MRK-05). | |
| PLNR-61 | A path is drawn in own blue, `path_width_px` wide: dashed (`path_dash_px` on, `path_gap_px` off) where the ship has yet to fly it, solid where it has flown it. A dot `departure_dot_px` across marks its start. The coast after a Pass (PLNR-67) is dotted in the warning colour, `consequence_width_px` wide, with dots `consequence_dot_px` long and `consequence_gap_px` apart. | |
| PLNR-62 | What is drawn, and when. **Planning:** the path of the current setting (and, while choosing, of the hovered place's Fastest setting), with the ghosts of PLNR-63; the ship's symbol stays where the ship is. **A Scheduled order:** nothing but the ship's label (PLNR-66) until its ship is selected, then its dashed path from the departure point and its arrival ghost. **From its leave day (PLNR-21) until its leg ends, and for a ship coasting after a Pass:** the ship's path, always, at `traffic_path_alpha` and `traffic_path_width_px`; at full opacity and `path_width_px` while the ship is hovered or selected; with the arrival ghost added while it is selected. **A ship the state doesn't command:** no path. | |
| PLNR-63 | A **ghost** is an object's marker as a dashed outline with an empty centre: a circle `star_marker_px`, `planet_marker_px` or `moon_marker_px` across for a star, planet or moon, and `ship_symbol_px` for a ship or station, in the neutral colour at `ghost_alpha`, `ghost_width_px` wide, with dashes `ghost_dash_px` long and as far apart. The **arrival ghost** is the destination where the path ends, on the day the crossing ends. The **departure ghost** is the departure body where the path starts, on the day the crossing starts, and is drawn only while the order is being planned. | |
| PLNR-64 | A ghost's opacity, `ghost_alpha`, is multiplied by the opacity of its object's dot (MOT-11), so there is none while the object is in exposure-ring mode. A path to such an object ends on its ring at the true arrival point, and the label of PLNR-65 sits there. Ghosts exist only for the order being planned, the hovered place (PLNR-12) and the selected order, never for every path. | |
| PLNR-65 | The arrival ghost's label, in the data face and the path's colour, reads "arrive *date*" for a Stop, "pass *date* · *speed*" for a Pass, and "meeting *date* · *speed* closing" when the destination is a ship, with *speed* as in PLNR-26 (UNIT-04). It and the consequence label (PLNR-67) take part in label placement (LBL-01, LBL-03) after every label that LBL-02 ranks, so they give way to the target's own label, and a label with no free position is hidden. | |
| PLNR-66 | A ship that has an order carries its countdown as the last part of its label's second line (SYM-13): "departs in *N*" until its leave day; then "arrives in *N*" for a Stop, "passes *place* in *N*" for a Pass at a body or a station, "rendezvous in *N*" for a Stop at a ship, "intercept in *N*" for a Pass at a ship. *N* is a duration (UNIT-02) to the leave day or the day the leg ends, and it counts down as the clock runs. | |
| PLNR-67 | After a Pass the ship coasts (BAND-07). Wherever the Pass's path is shown (PLNR-62), its coast is drawn from the day the crossing ends to the event of the consequence rule (PLNR-88), with a label at the event, in the warning colour: "then coasting · crosses *body*'s orbit in *N*" or "then coasting · leaves the system in *N*", with *N* counted from today (UNIT-02). With no event the coast isn't drawn. A coast round a body other than the star, as after a Pass at a moon, isn't drawn and has no label. | |
| PLNR-68 | A ship on a Conic or Spiral segment (RAIL-09) is drawn as its symbol (section 3a of `system-view.md`) at its rail position in dot style, never as an exposure ring (MOT-08), with its velocity leader (SYM-11). It has no orbit ring or tail (MRK-09) and is never folded (GRP-08). | |
| PLNR-69 | Paths and their start marks are drawn in the first layer of MRK-12, above the rings, tails and exposure rings. Ghosts are drawn in the layer of natural bodies, beneath the real markers. Labels are drawn in the label layer. A path is not a hover or click target (HOV-02). | |

## 7. The rules the planner reads

Pure functions in `src/sim/transfer/` (PLNR-04). They call the solver of `transfers.md` and decide nothing about drawing.

| ID | Requirement | Built |
|---|---|---|
| PLNR-80 | The **Depart range** runs from today to a last day. For an impulsive drive whose start and end states (XFER-01) orbit the crossing's primary with different periods *P_a* and *P_b*, the last day is today + (1 + `depart_margin_fraction`) × 1 ÷ abs(1 ÷ *P_a* − 1 ÷ *P_b*), so the next window is always on the slider. In every other case (a low-thrust drive, a coasting target, orbits of equal period) it is today + `depart_range_days`. A phasing leg has no Depart range (PLNR-40). | |
| PLNR-81 | For an impulsive drive the **Travel range** runs from `travel_min_factor` × *T_h* to `travel_max_factor` × *T_h*, where *T_h* is half the period of the ellipse that touches the radius of the start state's orbit and the radius of the end state's orbit round the primary: the trip that needs least fuel between two circular orbits. For a coasting target the radii are those on today. A low-thrust drive's travel time is fixed (XFER-06), and a phasing leg's are the meetings of PHAS-08. | |
| PLNR-82 | The **grid** holds the solver's result (XFER-02 to XFER-06) for `track_depart_samples` depart days spread evenly over the Depart range, ends included, by `track_travel_samples` travel times spread evenly over the Travel range, ends included. Each result is the leg's delta-v and arrival day, or the reason it is infeasible. For a low-thrust drive the grid has one travel time. For a phasing leg it has one depart day and one travel time for each meeting. | |
| PLNR-83 | **Fastest** is the affordable sample (delta-v within PLAN-21's available) that arrives earliest (XFER-02), or, of equal arrival, the one with less delta-v. It doesn't exist when no sample is affordable. | |
| PLNR-84 | **Cheapest** is the sample with least delta-v. Samples within `preset_tie_kms` of it count as equal, and the one among them that arrives earliest wins. It may be over budget. | |
| PLNR-85 | Let *b(d)* be the least delta-v of any travel time on depart sample *d*, and *b\** the least of those. A depart sample is in a **window** when *b(d)* is at most *b\** + `window_margin_kms`. **Next window** is the earliest depart sample in a window, with the travel time of that day's cheapest trip. It exists only when the Depart range comes from a period (PLNR-80, first case), and it may be over budget. | |
| PLNR-86 | A preset's sample is refined before it is applied. The solver is run for every setting, at the slider steps (PLNR-23), within one grid spacing either side of the sample on both sliders, and the preset picks again among those: Fastest the earliest arrival among the affordable ones (PLNR-83), Cheapest and Next window the least delta-v. The sliders are then set to the result. | |
| PLNR-87 | The grid and the presets are recomputed when the destination, the band, Stop or Pass, the ship's drive or its available delta-v changes, and when the day changes (at most once a game day). They are not recomputed by a slider move. A slider move solves only what it needs: the setting it makes, for the path and the readouts, and the Travel time track at the new depart day. | |
| PLNR-88 | The **coast** of a Pass is the conic the ship is on from the day the crossing ends. Its **event** is the first of: the day its distance from the star equals the orbit radius of a planet (a body orbiting the star; the passed body's own orbit counts only on a later return to it), and the day its distance from the star first exceeds `lost_beyond_au` (BAND-08). The rule returns Crosses `{ body, days }`, Leaves `{ days }` or None, with days counted from today. It is defined only for a coast round the star. | |

## 8. Words and figures

| ID | Requirement | Built |
|---|---|---|
| PLNR-90 | The planner's text never contains Hohmann, synodic, Lambert, phase angle, apoapsis, periapsis, eccentricity or inclination. It says Depart, Travel time, Arrive, Delta-v and Window. | |
| PLNR-91 | Hovering the **Delta-v** label, for `tooltip_delay_ms`, shows a tooltip: "Delta-v is how much a ship can change its speed before its tanks are dry. Every trip spends some." This is a label's tooltip, not a marker's (HOV-01). | |
| PLNR-92 | Every reason the solver gives for a leg it can't plan has a plain sentence, and the reason line (PLNR-31) and the hover line (PLNR-12) show that sentence, never a code. Over budget reads "Needs *x* km/s, has *y*". A path that comes too close to its primary (XFER-07) reads "Path passes too close to *primary*". Having no phasing orbit that fits (PHAS-08) reads "Can't close the gap in that time", a low-thrust drive in a phasing leg (PHAS-05) "This drive can't close a gap in its orbit", a target on a path the ship can't match (XFER-10) "Can't match that ship's path from here", and PLNR-42 "Already there". | |
| PLNR-93 | Dates are shown as the top bar shows them (CLK-02), durations as UNIT-02, distances as UNIT-01, and speeds and delta-v as UNIT-04. | |

## 9. Test content

| ID | Requirement | Built |
|---|---|---|
| TEST-30 | The drives of TEST-20 have display names: the warship Orison's is "hydrogen NTR" and RFA Tolland's is "NEP ion/Hall". Orison (impulsive, 9.9 km/s) and RFA Tolland (low-thrust, 24 km/s) are the ships the planner's tests plan with, from Halden IV's High band. | |

## Tunables

Simulation tunables live in `SimTunables` (`src/sim/tunables.rs`) beside those of `transfers.md`; view tunables in `Tunables` (`src/view/tunables.rs`) beside those of `system-view.md`. Defaults are starting values.

| Tunable (`SimTunables`) | Default | Used by |
|---|---|---|
| `depart_range_days` | 180 d | PLNR-80 |
| `depart_margin_fraction` | 0.1 | PLNR-80 |
| `travel_min_factor` | 0.3 | PLNR-81 |
| `travel_max_factor` | 2 | PLNR-81 |
| `track_depart_samples` | 60 | PLNR-82 |
| `track_travel_samples` | 36 | PLNR-82 |
| `preset_tie_kms` | 0.05 km/s | PLNR-84 |
| `window_margin_kms` | 0.5 km/s | PLNR-85 |
| `slider_max_steps` | 400 | PLNR-23, PLNR-86 |

| Tunable (`Tunables`) | Default | Used by |
|---|---|---|
| `path_width_px` | 1.5 px | PLNR-61, PLNR-62 |
| `path_dash_px` | 5 px | PLNR-61 |
| `path_gap_px` | 4 px | PLNR-61 |
| `departure_dot_px` | 5 px | PLNR-61 |
| `consequence_width_px` | 1.2 px | PLNR-61 |
| `consequence_dot_px` | 2 px | PLNR-61 |
| `consequence_gap_px` | 4 px | PLNR-61 |
| `traffic_path_alpha` | 0.35 | PLNR-62 |
| `traffic_path_width_px` | 1 px | PLNR-62 |
| `ghost_alpha` | 0.5 | PLNR-63 |
| `ghost_width_px` | 1 px | PLNR-63 |
| `ghost_dash_px` | 2 px | PLNR-63 |
| `slider_track_px` | 10 px | PLNR-23 |
| `slider_thumb_w_px` | 14 px | PLNR-23 |
| `slider_thumb_h_px` | 18 px | PLNR-23 |
| `track_dim_mix` | 0.35 | PLNR-28 |
| `track_over_alpha` | 0.45 | PLNR-28 |
| `segment_fill_mix` | 0.30 | PLNR-14 |
| `delta_bar_px` | 8 px | PLNR-27 |
| `tooltip_delay_ms` | 400 ms | PLNR-91 |

## Not yet specced

From `transfers.md`, not to be built until specced: other people's ships and their projected paths (3.4), which includes meeting a foreign ship in flight; meeting one of the state's own ships that is flying under an order (the order can change, and a Stop doesn't follow its later burns); the return reserve and one-way orders (3.6, mock-up 11); multi-leg plans, round trips and refuelling stops (3.7, mock-up 12); groups (3.1 step 1, section 5) and the gate entry rule; alerts, including what the player is told when an order fails on its depart day (PLAN-11) and when a window is about to close (3.5); ordering a ship that is in flight or coasting, aborts and cancelling an InFlight order (section 6, PLAN-22); gravity assists and a Pass through Low (2.6, 7), which a raid on a station in a Low band needs; a way to line a Pass up with a station or parked ship, such as a preset or a trim of the travel time that brings the closest approach (PLNR-32) down; a standoff distance for an intercept. Also not specced: the Delta-v tooltip showing once on first use (it needs remembered player settings); the general "where is this ship relative to the selection" line of 4.4, of which the planner shows only the gap (PLNR-41); layer toggles for paths; a keyboard shortcut or context menu for opening the planner; the camera moving to show a path.

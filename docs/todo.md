# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Planner tooltips** · PLNR-94 to PLNR-106
  - Tooltips for the planner's buttons and sliders, in plain words (PLNR-95). PLNR-94 changed (Rendezvous has a longer text), so its ✓ is cleared.
  - They sit on labels, not tracks, because a rejected stretch of a track has its own tooltip (PLNR-35). They use `tooltip_delay_ms`.
  - Tests: each text shows after the delay on its control; the variants show in their cases; no text contains a word PLNR-90 bans.
- [ ] **Several objects on one marker** · MUL-01 to MUL-13, TEST-11, TEST-12, GRP-05, GRP-07, GRP-09, HOV-01, HOV-02, HOV-05 to HOV-07, INS-03, INS-06, MOT-14, MRK-12, SYM-10, SYM-13, SYM-16, SYM-17, LBL-01, REF-09, CAM-10, CAM-14, PLNR-11
  - The multi-marker (a frame, a count and a second frame behind) replaces the fold pill, and a click opens a tray of what it holds. GRP-06 is withdrawn: delete its code and tests, and the pill's outline colour and own-ship dot with it. Tunables: `cluster_join_px`, `cluster_split_px`, `back_frame_offset_px`, `multi_count_px` and the five `tray_*` ones.
  - Meaning changed, so their ✓ is cleared: GRP-05, GRP-07, GRP-09, HOV-05, HOV-06, HOV-07, INS-03, INS-06, MOT-14, MRK-12, SYM-10, SYM-13, SYM-16, SYM-17, LBL-01, REF-09, CAM-10, CAM-14 and PLNR-11. HOV-01 and HOV-02 were reworded only (pill to multi-marker) and TEST-09 keeps its ✓. Their ✓ is cleared too, since the pill they name is gone.
  - Three sources make a marker (MUL-05): things together (docked, or ships in one phase), overlap with hysteresis, and fold. Build the together case first: Sol shows it on day 0 (Marlin docked at Perigee Station, TEST-11).
  - A station with docked things is drawn as a station marker (MUL-03). Inside a larger marker the station is one thing and its docked things sit under its row (MUL-04).
  - New content: TEST-11 (two ships docked at Iver Yard) and TEST-12 (TF-2 and two haulers in one phase in Halden IV's High band).
  - Tests: a ship docked at a station gives a station marker with the right count at every zoom; two symbols join below `cluster_join_px` and part only above `cluster_split_px`; the front and back frame colours for each mix of affiliations, and for bodies only; a click opens one tray and selects the parent or the first row (MUL-07); Up, Down, Right, Left and Esc in the tray, with the arrow keys not panning; the tray stays inside the map area and follows its marker; with the planner open a row picks as its symbol would.

## Fixes

## Other

- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

## Parked

Specced but not built, because nothing needs it yet. Don't pick these for a coding session. Each says what brings it back. Their requirements still count for the coverage check in `docs/README.md`.

- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - **Revisit when:** a system reads ideology (a faction, settlement or character that carries a profile, or an effect that uses the scores) and the UI for showing it is specced. Until then the code would have no caller.
  - Decide the open point in `docs/spec/ideology.md` first (whether characters share the structure), and say where the option values are loaded from (IDEO-31).
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

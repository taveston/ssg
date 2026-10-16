# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.
- **Parked:** specced items nothing needs yet. They are not built until what brings them back happens.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

## Fixes

## Other

- [ ] **Group the view folder and the headless tests** · ARCH-01, ARCH-31
  - From the code review (2026-10-16). Group `src/view/`'s 60 modules into subfolders by area (`rings/`, `symbols/`, `markers/`, `inset/` for inspect, outline and sidebar), keeping shared state modules at the top (ARCH-03).
  - Move the rest of `src/headless_tests.rs` (about 4,900 lines) into `headless_tests/` by area (camera and zoom, grouping and motion, markers and labels, inset and outline), keeping the shared app builders where they are. Moves only, no behaviour change (ARCH-35).
  - Move the frame-shape constants that `leaders.rs` copies from `symbol_shape.wesl` next to the shader binding in `symbol_shape.rs`.
- [ ] **`OW_SHOT` fails fast when its order can't be planned**
  - With `planner=<body>;fly=<days>`, a destination the ship can't reach (over budget, or rejected by the solver) leaves the shot waiting in its Scheduling phase until the timeout. Instead, exit at once with an error that gives the planner's reason (the PLNR-92 sentence) and what was asked.
  - Also exit at once when `planner` is given but nothing is selected, or the selected body isn't a ship that can open the planner (PLNR-01).
  - Code is in `src/view/debug_shot.rs`. Test the new exits headlessly. No change to the game's behaviour.
- [ ] **Bring older docs up to the writing style**
  - The rules are in `docs/README.md` (Writing). Do it one doc at a time, with no change of meaning.
  - The longest are the likeliest to need it: `ideas/mechanics/requirements-and-bids.md`, `ideas/mechanics/component-stats.md` and `ideas/mechanics/transfers.md`.

## Parked

Specced but not built, because nothing needs it yet. Don't pick these for a coding session. Each says what brings it back. Their requirements still count for the coverage check in `docs/README.md`.

- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - **Revisit when:** a system reads ideology (a faction, settlement or character that carries a profile, or an effect that uses the scores) and the UI for showing it is specced. Until then the code would have no caller.
  - Decide the open point in `docs/spec/ideology.md` first (whether characters share the structure), and say where the option values are loaded from (IDEO-31).
  - IDEO-11: an exact tie between the two lowest scores is Pluralist. Six of the 216 histories land on one (3/3/6 in some order), so test them.

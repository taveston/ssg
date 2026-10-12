# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-101.

---

### F5-07 · A single click on an "In orbit" entry waits for the double-click window
- **Observation:** selecting a satellite replaces the parent's inset, which removes the list before a double-click's second click can land. So a single click selects only after `double_click_window_ms` (350 ms) passes with no second click. List clicks feel a third of a second slow.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** parked (2026-10-12). Option: keep the parent's inset open when a satellite is picked from its list, so selection can be immediate.

### F-107 · Show some astronomical data for the selected body
- **Observation:** a selected body's inset would be more useful with limited astronomical data, such as its orbit radius in AU (and perhaps its period).
- **Affects:** INS-02 · **Type:** idea
- **Disposition:** open. Decide which figures the inset shows, and in what units.

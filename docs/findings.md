# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-101.

---

### F5-07 Â· A single click on an "In orbit" entry waits for the double-click window
- **Observation:** selecting a satellite replaces the parent's inset, which removes the list before a double-click's second click can land. So a single click selects only after `double_click_window_ms` (350 ms) passes with no second click. List clicks feel a third of a second slow.
- **Affects:** INS-05 Â· **Type:** spec gap
- **Disposition:** parked (2026-10-12). Option: keep the parent's inset open when a satellite is picked from its list, so selection can be immediate.

### F-108 · The standard tilt of 20° is too close to a top-down view
- **Observation:** playing the build, 20° from straight down looked too close to a view from directly above. The build now uses 30°. The spec's tunables table still says 20°.
- **Affects:** CAM-02, CAM-23, `standard_tilt_deg` · **Type:** spec wrong
- **Disposition:** open. Settle the default (30° in the build) and update the table.

### F-109 Â· A focus released by ring mode should bring the parent back into view
- **Observation:** CAM-24 keeps the view where it is when focus moves to the parent, which can leave the parent off screen. The build now moves to the parent and frames it like a focus jump (ZOOM-16). With no ancestor left, focus ends and the view returns to the system frame over the star.
- **Affects:** CAM-24 Â· **Type:** spec wrong
- **Disposition:** open. Reword CAM-24 to describe the framing move.

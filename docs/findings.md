# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-101.

---

### F5-07 · A single click on an "In orbit" entry waits for the double-click window
- **Observation:** selecting a satellite replaces the parent's inset, which removes the list before a double-click's second click can land. So a single click selects only after `double_click_window_ms` (350 ms) passes with no second click. List clicks feel a third of a second slow.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** parked (2026-10-12). Option: keep the parent's inset open when a satellite is picked from its list, so selection can be immediate.

### F-108 · The standard tilt should be shallow, and the steepest tilt shallower still
- **Observation:** playing the build, the standard view should be close to side-on. The build now uses 60° from straight down (the spec says 20°), and `max_pitch_deg` is 85° (the spec says 65°).
- **Affects:** CAM-01, CAM-02, CAM-23, `standard_tilt_deg`, `max_pitch_deg` · **Type:** spec wrong
- **Disposition:** open. Settle both defaults and update the tunables table.

### F-109 · A focus released by ring mode should bring the parent back into view
- **Observation:** CAM-24 keeps the view where it is when focus moves to the parent, which can leave the parent off screen. The build now moves to the parent and frames it like a focus jump (ZOOM-16). With no ancestor left, focus ends and the view returns to the system frame over the star. The focus chip's × does the same without refocusing: focus ends and the view centres on the parent at its satellite frame (system frame if the body orbits the star).
- **Affects:** CAM-24 · **Type:** spec wrong
- **Disposition:** open. Reword CAM-24 to describe the framing move.

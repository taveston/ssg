# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-101.

---

### F2-04 · Should a click on empty space close the inset?
- **Observation:** a left click on empty space does nothing; Esc closes the inset. Now that left drag pans (CAM-10), a click on empty space is still a click if it moves less than `drag_threshold_px`.
- **Affects:** INS-03 · **Type:** spec gap
- **Disposition:** open. Decide, and add the answer to INS-03.

### F3-01 · Trackpad pinch isn't built
- **Observation:** only wheel notches zoom. Pinch was deferred for lack of hardware to test on.
- **Affects:** ZOOM-07 · **Type:** bug (deferred)
- **Disposition:** open. Build it when it can be tested, or withdraw ZOOM-07 if trackpads aren't a target. Pixel-unit scroll is converted at 20 px per notch until then (ZOOM-01).

### F3-05 · Prototype Corvane doesn't match the reference layout
- **Observation:** the prototype has a 0.78 solar-mass star with Pell 0.35 AU, Iver 0.7 AU, Halden IV 2.0 AU, belt sites near 1.2 to 1.3 AU, Morrow 4.3 AU, gates at about 6 AU, and no Corvane II. *Drives & Transit* section 2 has a 0.75 solar-mass star with Pell 0.25, Corvane II 0.36, Iver 0.50, belt 0.58 to 0.70, Halden IV 0.80, Morrow about 4.0 and gates at 2.0 AU. *Component Stats* and *Requirements & Bids* use the reference layout.
- **Affects:** TEST-01, `docs/ideas/mechanics/drives-and-transit.md` · **Type:** conflict
- **Disposition:** open. Decide whether to move the test system to the reference layout (this changes zoom stops and Halden IV's year) and spec the orbits in TEST-01.

### F3-06 · Gate placement on screen
- **Observation:** gates at about 6 AU show as edge arrows at the system frame. *Drives & Transit* recommends about 2 AU, inside Morrow's orbit, which would put them on screen.
- **Affects:** EDGE-01, ZOOM-15 · **Type:** idea
- **Disposition:** open, with F3-05 and the gate-distance question in `presentation.md` section 6.

### F4-10 · Stale status shows time since, not the date
- **Observation:** the stale inset reads "LAST IMAGED · 2 Y 9 M AGO". INS-04 says "LAST IMAGED · *date*". Sketch board 5 shows both: the date on the image and the time since in the status.
- **Affects:** INS-04 · **Type:** conflict
- **Disposition:** open. Change INS-04 to "date, and time since", or change the code.

### F4-12 · FPS readout in the top bar isn't specced
- **Observation:** the top bar shows an FPS counter that no requirement covers.
- **Affects:** CLK-02 · **Type:** spec gap
- **Disposition:** open. Spec it as a debug readout, or remove it.

### F4-21 · Gate edge labels show the gate's name
- **Observation:** EDGE-01 says the arrow is labelled with its destination. The label is the gate's name, "Gate Ostra", which contains the destination but isn't literally it.
- **Affects:** EDGE-01 · **Type:** conflict
- **Disposition:** open. Accept the name, or label with the destination alone.

### F4-30 · Should zooming set focus at all?
- **Observation:** in the iteration 3 build, zooming in hopped focus between satellites. CAM-21 as now built can't hop: it focuses only when nothing is focused, only on the body whose satellite frame is the inner limit, and later steps anchor on that body. The playtester's view is that zooming to focus is poor UX. While the first focusing step is still moving, the body drifts with its orbit (about 0.2 render units at 5 d/s over a 200 ms step).
- **Affects:** CAM-21, CAM-06 · **Type:** spec wrong (suspected)
- **Disposition:** open. Play the current build, then keep CAM-21 or withdraw it so focus comes only from double-click, F and the Focus button.

### F5-07 · A single click on an "In orbit" entry waits for the double-click window
- **Observation:** selecting a satellite replaces the parent's inset, which removes the list before a double-click's second click can land. So a single click selects only after `double_click_window_ms` (350 ms) passes with no second click. List clicks feel a third of a second slow.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** open. Alternative: keep the parent's inset open when a satellite is picked from its list, so selection can be immediate.

### F5-08 · Focus bracket size
- **Observation:** a planet's marker plus its selection ring is about 20 px across, so 14 px brackets (`focus_bracket_px`) were hidden under the ring. The bracket box is now the larger of `focus_bracket_px` and the ring plus 2 px: 24 px on a planet, 30 px on the star.
- **Affects:** CAM-22, MRK-01 · **Type:** spec wrong
- **Disposition:** open. Decide whether `focus_bracket_px` should become a margin round the ring rather than a box side.

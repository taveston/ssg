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

### F5-07 · A single click on an "In orbit" entry waits for the double-click window
- **Observation:** selecting a satellite replaces the parent's inset, which removes the list before a double-click's second click can land. So a single click selects only after `double_click_window_ms` (350 ms) passes with no second click. List clicks feel a third of a second slow.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** open. Alternative: keep the parent's inset open when a satellite is picked from its list, so selection can be immediate.

### F5-08 · Focus bracket size
- **Observation:** a planet's marker plus its selection ring is about 20 px across, so 14 px brackets (`focus_bracket_px`) were hidden under the ring. The bracket box is now the larger of `focus_bracket_px` and the ring plus 2 px: 24 px on a planet, 30 px on the star.
- **Affects:** CAM-22, MRK-01 · **Type:** spec wrong
- **Disposition:** open. Decide whether `focus_bracket_px` should become a margin round the ring rather than a box side.

### F-101 · The inset can go off the screen
- **Observation:** an inspection inset can be placed partly or wholly outside the window. INS-01 anchors it to the marker by a leader line, but nothing keeps it on screen.
- **Affects:** INS-01, INS-06 · **Type:** spec gap
- **Disposition:** open. Decide whether the inset is clamped inside the window (below the top bar), with the leader line stretching to reach the marker.

### F-103 · Selecting a fast-orbiting body works but isn't obvious, and its inset is unreadable
- **Observation:** clicking an exposure ring selects its body without pausing (MOT-05), but nothing suggests the ring can be clicked. Once selected, the inset is anchored to the fast-moving body, so it races round the orbit and can't be read.
- **Affects:** MOT-05, MOT-07, INS-01, INS-06 · **Type:** spec wrong
- **Disposition:** open. Options for the cue: a hover highlight on the ring, or a wider hit area. Options for the inset: in ring mode, anchor it to a fixed point (the ring's label spot, as MOT-02 does for the label) or to the parent, instead of the moving body.

### F-104 · The inset sits too close to its body
- **Observation:** the inset is placed so near its body's marker that it partly hides the marker's label.
- **Affects:** INS-01 · **Type:** spec wrong
- **Disposition:** open. Decide the inset's offset from the marker (a tunable), so it clears the marker and its label.

### F-105 · Zooming out sometimes detaches the camera from the focus
- **Observation:** sometimes, zooming out leaves the camera no longer following the focused object. CAM-08 says focus ends only at the system frame, so this looks like a bug. No reliable repro yet.
- **Affects:** CAM-05, CAM-08, ZOOM-13 · **Type:** bug
- **Disposition:** open. Find a repro, then fix.

### F-107 · Show some astronomical data for the selected body
- **Observation:** a selected body's inset would be more useful with limited astronomical data, such as its orbit radius in AU (and perhaps its period).
- **Affects:** INS-02 · **Type:** idea
- **Disposition:** open. Decide which figures the inset shows, and in what units.

### F-108 · Small moving markers are hard to click
- **Observation:** some markers on moving bodies are small (a moon is 6 px across), so they are hard to hit while they move. The hit area is the marker itself.
- **Affects:** MRK-03, INS-01 · **Type:** spec wrong
- **Disposition:** open. Option: a minimum hit area round every marker (a tunable in pixels), larger than the drawn marker.

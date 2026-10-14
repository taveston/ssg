# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-144.

---

### F-142 · Large orbits fade out too soon
- **Observation:** In play, the MRK-14 fade feels too aggressive: rings that still read as rings fade or vanish. The defaults are `orbit_fade_from` 1.5 and `orbit_fade_to` 3 times half the map area's diagonal. Note F-140 too: the build measures a ring's radius at its point nearest the camera, which makes rings near the camera count as larger than they look.
- **Affects:** MRK-14 · **Type:** spec wrong
- **Disposition:** open. Raise the two tunables, change how the radius is measured (F-140), or both.

### F-143 · The parent tab's chevron sits awkwardly inside the box
- **Observation:** MRK-15 puts the chevron inside the tab, beside the parent's marker. On screen it reads poorly. It should either sit outside the box, on its edge-facing side, or be merged with the marker (for example, the marker with a pointer on its rim towards the parent).
- **Affects:** MRK-15, EDGE-03 · **Type:** spec wrong
- **Disposition:** open. Choose the chevron outside the tab or merged with the marker; the gate tabs (EDGE-03) should follow the same choice.

### F-140 · What "on-screen radius" means for a ring the camera is near
- **Observation:** MRK-14 fades a ring by its on-screen radius, but a ring seen in perspective has no single radius. Measured at the ring's centre, Halden IV's own orbit in its frame comes out about 1,100 px and would stay fully drawn, though it crosses the view as a near-straight line. The build measures the radius at the scale of the ring's point nearest the camera, so a ring the camera sits close to counts as huge and fades out. Seen from far off, both measures agree.
- **Affects:** MRK-14 · **Type:** spec gap
- **Disposition:** open. Accept, or say how the radius is measured.

### F-141 · The parent tab is a tab while gate arrows are not yet
- **Observation:** MRK-15 styles the parent's tab "as EDGE-01 to EDGE-03". EDGE-02 and EDGE-03 aren't built, so gates still show a bare arrow and a label. The build gives the parent tab the EDGE-03 look now (ground fill, neutral outline `button_stroke_px` wide, kept whole inside the map area), so for now the parent and the gates look different. The "Edge arrows as tabs" item will bring the gates into line.
- **Affects:** MRK-15, EDGE-02, EDGE-03 · **Type:** spec gap
- **Disposition:** open. Accept for now; recheck when the edge-tab item is built.

### F-139 · A ring in the hand-over band shows its dot and a still twin
- **Observation:** MOT-11 fades the dot out as the tail grows from `dot_fade_from_degrees` (300°) to 360°. MOT-14 puts an identity mark, the object's own symbol, at a fixed spot on the exposure ring, but doesn't say when it appears. The build fades the mark in over the same band, as the dot fades out. That is meant for the moment of switching, but a rate can hold a ring inside the band. K-3's 45-day orbit at 5 d/s has a tail of 320°, so its moving dot shows at about two thirds and a second, stationary K-3 symbol at about a third, for as long as the rate holds.
- **Affects:** MOT-11, MOT-14 · **Type:** spec gap
- **Disposition:** open. Options: show the mark only in ring mode (at 360°), so it appears as the dot goes; fade it in over a narrow band at the very end; or keep the cross-fade and accept the twin.

### F-138 · A close orbit ring is hidden behind its body's true-size disc
- **Observation:** MRK-12 draws every orbit ring beneath every natural body, and the build does that: rings are world gizmos and bodies are UI nodes above them. Zoomed in until a body is a true-size disc (MRK-07), a satellite's orbit close to it sits partly behind the disc and partly in front in 3D, but the whole ring is drawn behind, so the near half vanishes. It reads as the ring passing behind the planet on both sides.
- **Affects:** MRK-12, MRK-07 · **Type:** spec gap
- **Disposition:** open. Options: at tilt, draw the part of a ring nearer the camera than its parent's centre above the parent's disc (the ring would need a depth-aware pass or a split into a far and a near arc), or accept it.

### F-132 · What a shared exposure ring's label says
- **Observation:** MOT-02 puts a label on an exposure ring beside its identity mark, and MOT-14 gives a shared orbit a "+N" pill, but neither says what the label of a shared ring reads. The build labels it with the orbit's name alone ("High"), since the parent's marker is beside it; a lone body's ring keeps the body's name.
- **Affects:** MOT-02, MOT-14 · **Type:** spec gap
- **Disposition:** open. Keep the orbit name, or use the inset's form ("Halden IV · High").

### F-133 · The identity pill is not centred on its ring
- **Observation:** MOT-14 says the mark is placed as a marker is. A symbol or body marker is centred on the ring point. A pill's width depends on its count, so the build centres it vertically and puts its left edge half a pill-height before the point, so the ring passes through its left half. Centring it exactly would need the laid-out width, a frame late.
- **Affects:** MOT-14 · **Type:** spec gap
- **Disposition:** open. Accept, or spec the pill's anchor.

### F-134 · A shared orbit carrying the frame body
- **Observation:** HOV-02 says the frame body's ring is never a target. If the frame body is one member of a shared orbit (focus Orison), the build draws the ring as any other (MRK-14) but makes the whole ring no target, so the other members' tails can't be hovered or clicked either. The frame body can't be in ring mode (CAM-24), so no exposure ring is lost.
- **Affects:** HOV-02, HOV-04, MRK-14 · **Type:** spec gap
- **Disposition:** open. Accept, or let the other members' tails stay targets.

### F-129 · The pill's list is still a hover tooltip
- **Observation:** HOV-01 says there is no hover tooltip, but GRP-07 still has hovering a "+N" pill list the folded satellites. The build dropped the marker tooltip and kept the pill's list, which is the only way to see what a pill holds without clicking.
- **Affects:** HOV-01, GRP-07 · **Type:** spec gap
- **Disposition:** open. Keep the pill's list, or drop it and rely on the inset's outline.

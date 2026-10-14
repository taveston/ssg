# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-135.

---

### F-126 · A stale symbol's inside stays opaque
- **Observation:** SYM-03 says every inside is opaque; SYM-08 draws a stale track at `stale_symbol_alpha`. The build keeps the inside fully opaque on a stale track and fades only the frame, icon and text, so lines beneath never show through.
- **Affects:** SYM-03, SYM-08 · **Type:** spec gap
- **Disposition:** open. Keep this, or fade the inside too (lines would show through stale symbols).

### F-127 · A gate on screen looks like a moon
- **Observation:** with body markers as outlines (MRK-13), a moon is a plain circle 8 px across, and a gate on screen is a hollow circle 11 px across with a 1.5 px outline (hard-coded, MRK-04). Side by side they differ only in size.
- **Affects:** MRK-04, MRK-13 · **Type:** spec wrong
- **Disposition:** open. Options: give the gate its own shape (e.g. a ring with a gap, or a square), or keep it and rely on the label.

### F-132 · What a shared exposure ring's label says
- **Observation:** MOT-02 puts a label on an exposure ring beside its identity mark, and MOT-14 gives a shared orbit a "+N" pill, but neither says what the label of a shared ring reads. The build labels it with the orbit's name alone ("High"), since the parent's marker is beside it; a lone body's ring keeps the body's name.
- **Affects:** MOT-02, MOT-14 · **Type:** spec gap
- **Disposition:** open. Keep the orbit name, or use the inset's form ("Halden IV · High").

### F-133 · The identity pill is not centred on its ring
- **Observation:** MOT-14 says the mark is placed as a marker is. A symbol or body marker is centred on the ring point. A pill's width depends on its count, so the build centres it vertically and puts its left edge half a pill-height before the point, so the ring passes through its left half. Centring it exactly would need the laid-out width, a frame late.
- **Affects:** MOT-14 · **Type:** spec gap
- **Disposition:** open. Accept, or spec the pill's anchor.

### F-134 · A shared orbit carrying the frame body
- **Observation:** MRK-14 draws the frame body's orbit only as a tail, and HOV-02 says its ring is never a target. If the frame body is one member of a shared orbit (focus Orison), the build still draws the other members' tails on the ring and keeps `frame_orbit.rs`'s tail for the frame body, but makes the whole ring no target and gives it no identity mark. The frame body can't be in ring mode (CAM-24), so no exposure ring is lost.
- **Affects:** HOV-02, HOV-04, MRK-14 · **Type:** spec gap
- **Disposition:** open. Accept, or let the other members' tails stay targets.

### F-129 · The pill's list is still a hover tooltip
- **Observation:** HOV-01 says there is no hover tooltip, but GRP-07 still has hovering a "+N" pill list the folded satellites. The build dropped the marker tooltip and kept the pill's list, which is the only way to see what a pill holds without clicking.
- **Affects:** HOV-01, GRP-07 · **Type:** spec gap
- **Disposition:** open. Keep the pill's list, or drop it and rely on the inset's outline.

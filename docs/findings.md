# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-131.

---

### F-126 · A stale symbol's inside stays opaque
- **Observation:** SYM-03 says every inside is opaque; SYM-08 draws a stale track at `stale_symbol_alpha`. The build keeps the inside fully opaque on a stale track and fades only the frame, icon and text, so lines beneath never show through.
- **Affects:** SYM-03, SYM-08 · **Type:** spec gap
- **Disposition:** open. Keep this, or fade the inside too (lines would show through stale symbols).

### F-127 · A gate on screen looks like a moon
- **Observation:** with body markers as outlines (MRK-13), a moon is a plain circle 8 px across, and a gate on screen is a hollow circle 11 px across with a 1.5 px outline (hard-coded, MRK-04). Side by side they differ only in size.
- **Affects:** MRK-04, MRK-13 · **Type:** spec wrong
- **Disposition:** open. Options: give the gate its own shape (e.g. a ring with a gap, or a square), or keep it and rely on the label.

### F-129 · The pill's list is still a hover tooltip
- **Observation:** HOV-01 says there is no hover tooltip, but GRP-07 still has hovering a "+N" pill list the folded satellites. The build dropped the marker tooltip and kept the pill's list, which is the only way to see what a pill holds without clicking.
- **Affects:** HOV-01, GRP-07 · **Type:** spec gap
- **Disposition:** open. Keep the pill's list, or drop it and rely on the inset's outline.

### F-130 · The exposure ring snapped in at the end of a closing tail
- **Observation:** MOT-01 has a tail fade to the ring's normal brightness at its end, and MOT-02 makes the exposure ring bright all round. So at 360° the far end of the tail jumped from faint to bright in one frame, even with the tail easing (MOT-13). The build now flattens the tail's fade as it closes (MOT-11): at `dot_fade_from_degrees` it fades fully, and at 360° it is even all round, so the closing has no jump.
- **Affects:** MOT-01, MOT-11 · **Type:** spec gap
- **Disposition:** open. Keep this and add it to MOT-11, or choose another hand-over.

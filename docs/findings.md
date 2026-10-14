# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-129.

---

### F-126 · A stale symbol's inside stays opaque
- **Observation:** SYM-03 says every inside is opaque; SYM-08 draws a stale track at `stale_symbol_alpha`. The build keeps the inside fully opaque on a stale track and fades only the frame, icon and text, so lines beneath never show through.
- **Affects:** SYM-03, SYM-08 · **Type:** spec gap
- **Disposition:** open. Keep this, or fade the inside too (lines would show through stale symbols).

### F-127 · A gate on screen looks like a moon
- **Observation:** with body markers as outlines (MRK-13), a moon is a plain circle 8 px across, and a gate on screen is a hollow circle 11 px across with a 1.5 px outline (hard-coded, MRK-04). Side by side they differ only in size.
- **Affects:** MRK-04, MRK-13 · **Type:** spec wrong
- **Disposition:** open. Options: give the gate its own shape (e.g. a ring with a gap, or a square), or keep it and rely on the label.

### F-128 · The scale bar reads "1.00 AU"
- **Observation:** MRK-16 labels the bar's length as UNIT-01, which gives 3 significant figures. A round length then reads "1.00 AU" or "2.00 AU"; km lengths read cleanly ("50,000 km"). The build follows UNIT-01.
- **Affects:** MRK-16, UNIT-01 · **Type:** spec wrong
- **Disposition:** open. Options: show the scale bar's round length without trailing zeros ("1 AU"), or keep it.

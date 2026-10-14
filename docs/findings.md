# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-137.

---

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

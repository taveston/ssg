# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-138.

---

### F-137 · The frame body's orbit tail looks odd in play
- **Observation:** MRK-14 draws the frame body's own orbit only as a tail `frame_orbit_arc_px` (160 px) long on screen, ending at the body. The build does that, and three things look wrong in play. (1) The tail's share of the orbit changes with zoom: zoomed in, 160 px is a sliver of the orbit; zoomed out, it is most of the ring, then all of it, still fading to nothing at one end. (2) Switching focus from a planet to one of its moons makes the planet's orbit reappear as a full ring, because the moon is now the frame body and only its own orbit is cut to a tail, even though the view hardly changes. (3) The orbit is cut even when the whole ring would fit on screen, where a tail gains nothing over the ring.
- **Affects:** MRK-14 · **Type:** spec wrong
- **Disposition:** open. Options: draw the frame body's full ring whenever it fits on screen (or within some margin) and the tail only when it would cross the view; size the tail as an angle of the orbit rather than in pixels; or keep MRK-14 and accept the look.

### F-135 · A conic's elements don't say which way it goes round
- **Observation:** RAIL-05 lists a conic's elements as *p*, *e*, ω and *t_p*. Those four fix the shape and timing but not the sense of motion, and RAIL-06 asks for round trips in both senses. The crate's `Conic` stores a fifth element, `sense` (prograde or retrograde), and measures ω in the sense of motion.
- **Affects:** RAIL-05 · **Type:** spec gap
- **Disposition:** open. Add the sense to RAIL-05's element list.

### F-136 · XFER-12's figures come from a different Corvane than the content's
- **Observation:** XFER-12's worked checks hold for the layout in `drives-and-transit.md` section 2 (star 0.75 solar masses, Iver at 0.5 AU, Halden IV at 0.8 AU with radius 5,753 km and 0.82 g, belt at 0.64 AU); the crate's tests use those plain figures and pass within 3%. The content's Corvane (`content/corvane.rs`) has the star at 0.78 solar masses, Iver at 0.7 AU, Halden IV at 2.0 AU with radius 6,700 km and 1.1 Earth masses, and the belt at 1.15 to 1.45 AU. With those, Halden IV's Low departure cost is about 3.2 km/s and its High cost about 1.1 km/s, and the Iver → Halden IV Hohmann crossing is nothing like 111 days. A test of XFER-12 against the game content would fail.
- **Affects:** XFER-12, TEST-01, TEST-07 · **Type:** conflict
- **Disposition:** open. Either state XFER-12 in plain figures (as the crate's tests do) and keep it a crate-level check, or bring the content's Corvane to the reference layout, or recompute the figures for the content.

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

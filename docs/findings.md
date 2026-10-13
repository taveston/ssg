# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-115.

---

### F-121 · Plotting-instrument cues
- **Observation:** the view has no scale bar, range ring or north mark. These are the cheapest ways to make it read as a plotting instrument, and true scale is the premise of the display. Heading is free (CAM-01), so a north mark also helps orientation.
- **Affects:** new · **Type:** idea
- **Disposition:** open. Candidate for `ideas/presentation/presentation.md` §2: a scale bar in the map corner, a faint range ring round the frame body, and a small north tick.

### F-122 · The frame body's own orbit reads as a stray line
- **Observation:** zoomed into a planet's frame (Halden IV), the planet's orbit round the star is so large that it crosses the whole view as a near-straight line through the plot. Without context it reads as an unexplained line rather than an orbit. It does carry information: it shows the direction of the star and of the planet's motion.
- **Affects:** MRK-05, HOV-03 · **Type:** spec wrong
- **Disposition:** open. Options: keep it but fade it as zoom moves in past the point where it looks straight; replace it close in with a short arc segment near the frame body plus a bearing tick to the star; label it (the star's name at the edge where it leaves the map) so it reads as an orbit; leave it as it is.

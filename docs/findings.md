# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-115.

---

### F-115 · Labels print over each other
- **Observation:** in a busy system view (Halden IV, 19 Jun 2341), ship and body labels overlap until they can't be read: "Marten", "Transport Brask", "own flag · civilian" and "2nd Halden Regiment" sit on top of each other by Halden IVc; "RFA Tolland" collides with "Transport Hale"; "Varga Prospector" runs under the Sennet symbol; an echelon mark cuts into "Ostra trader". Overprinted text breaks the console look more than anything else on screen.
- **Affects:** SYM-13, MRK-12, new · **Type:** spec gap
- **Disposition:** open. SYM-13 only puts labels to the right; nothing says what happens when they collide. Options: try other sides (right, left, above, below) before overlapping; short leader ticks to displaced labels; drop the mono second line first, then the name, by priority (selection and hover, hostile and unknown, own commanded, others); show dropped text on hover.

### F-116 · The selected orbit is the loudest thing on screen
- **Observation:** a selected outer moon's orbit, drawn in full white at highlight width, spans the whole view and outweighs every contact. The console style calls for hairlines and no glow.
- **Affects:** HOV-03 · **Type:** spec wrong
- **Disposition:** open. Options: keep white but at the normal orbit width; or a brighter grey rather than white. The white ring on the marker still shows what is selected.

### F-117 · Buttons use reserved colours
- **Observation:** the inset's Close is orange and Focus is affiliation blue; the top bar's Pause is filled blue. Orange is reserved for "needs attention" and blue for own affiliation, so Close reads as a warning and Pause as an own ship.
- **Affects:** INS-02, MRK-01 · **Type:** conflict (build and `ui-style.md` §2)
- **Disposition:** open. Proposed: adopt the button rules of `ui-style.md` §2 in the spec: primary filled off-white, secondary outlined, quiet as text, orange outline only for actions that can't be undone. The active time rate shown as the primary style.

### F-119 · Gate edge arrows are weak and their labels clip
- **Observation:** the edge arrows are small chevrons that are easy to miss; the right-hand label is cut to "Gat" by the window edge, and the bottom arrow shows no label at all.
- **Affects:** EDGE-01 · **Type:** bug (labels), spec gap (look)
- **Disposition:** open. Labels must stay inside the map area. Candidate: draw arrows as small labelled tabs on the frame edge, in the console style.

### F-120 · Inset content doesn't follow the panel anatomy
- **Observation:** key facts are a mono list ("Class: Moon") rather than a row of label-over-value pairs; there are no small-capital section headings; the shaded sphere image is the most game-like element on screen. (The card shape is expected: INS-07, the sidebar, isn't built yet.)
- **Affects:** INS-02, INS-08 · **Type:** conflict (build and `ui-style.md` §5)
- **Disposition:** open. Proposed: spec the §5 anatomy for the inset. The image stays a placeholder (INS-04) but should be flat and muted until real renders exist.

### F-121 · Plotting-instrument cues
- **Observation:** the view has no scale bar, range ring or north mark. These are the cheapest ways to make it read as a plotting instrument, and true scale is the premise of the display. Heading is free (CAM-01), so a north mark also helps orientation.
- **Affects:** new · **Type:** idea
- **Disposition:** open. Candidate for `ideas/presentation/presentation.md` §2: a scale bar in the map corner, a faint range ring round the frame body, and a small north tick.

### F-122 · The frame body's own orbit reads as a stray line
- **Observation:** zoomed into a planet's frame (Halden IV), the planet's orbit round the star is so large that it crosses the whole view as a near-straight line through the plot. Without context it reads as an unexplained line rather than an orbit. It does carry information: it shows the direction of the star and of the planet's motion.
- **Affects:** MRK-05, HOV-03 · **Type:** spec wrong
- **Disposition:** open. Options: keep it but fade it as zoom moves in past the point where it looks straight; replace it close in with a short arc segment near the frame body plus a bearing tick to the star; label it (the star's name at the edge where it leaves the map) so it reads as an orbit; leave it as it is.

### F-123 · Symbol fills are see-through
- **Observation:** command fills are drawn as alpha tints, so orbit rings and tails run straight through every symbol (Delve, Transport Hale, Transport Cassel). Icons sit on a background of crossing lines and can't be read, and the symbols look glassy (a HUD look) rather than like opaque plotting symbols.
- **Affects:** SYM-03, MRK-12 · **Type:** spec wrong
- **Disposition:** open. Proposed: keep the fill as the command signal, but make it opaque. Every frame first covers what lies under it with the ground colour, then draws its tint as a solid colour equal to the old tint over the ground (about #1E3448 for state blue at 30 %). An own-flag civilian keeps no tint, so its inside is plain ground. In open space nothing changes; lines stop at the frame edge.

### F-124 · Symbols are blurred and their outlines are heavy
- **Observation:** frame edges have a soft band about 2 px wide while orbit lines are sharp, so symbols look out of focus beside them. The small icons suffer most: the infantry cross is a smudge, the drop barely reads, the crate's centre line is lost. The frame outline is also heavier than anything else on the map, against the hairline style.
- **Affects:** SYM-02, SYM-04, SYM-05, new · **Type:** bug (blur), spec gap (outline weight)
- **Disposition:** open. Likely cause: bitmaps scaled with smoothing, or symbols drawn off whole pixels. Proposed: symbols are drawn as vectors at their screen size and placed on whole pixels (Checked in review); the outline width becomes a named tunable at about 1.5 px; icon lines land on whole pixels at `ship_symbol_px`.

### F-125 · Body markers look like buttons, not plotted points
- **Observation:** planets and moons are mid-grey filled discs with a light rim. Mid grey reads as disabled or stale on a console; a solid disc is the heaviest mark on the map, so the terrain outweighs the contacts; planet and moon markers are barely different (Halden IV and IVa); the fill is see-through, so orbit lines cross the discs (as F-123); and IVc sits half under Marten's symbol, merging into one blob.
- **Affects:** MRK-03, MRK-11 · **Type:** spec wrong
- **Disposition:** open. Proposed: markers are outlines, not areas: opaque ground-colour inside, a sharp 1 px neutral rim, and class by shape: planet = circle with a centre dot; moon = smaller plain circle; star = circle with short radial ticks. The filled disc (MRK-11) is kept only for true-size discs close in (MRK-07), so crossing that zoom reads as a change from "marker" to "object". Opaque insides also keep overlapping markers distinct, which MRK-11's gap did before.

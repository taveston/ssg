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

### F-118 · Unexplained line across the system view
- **Observation:** a long straight line runs across the whole view, top left to bottom right, through the plot. Nothing in the spec draws it. If it's a gate bearing or a reference axis it needs a label and less weight; if not, it shouldn't be there.
- **Affects:** new · **Type:** bug, or spec gap if intended
- **Disposition:** open. Ask the coding session what draws it.

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

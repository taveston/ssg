# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-101.

---

### F5-07 · A single click on an "In orbit" entry waits for the double-click window
- **Observation:** selecting a satellite replaces the parent's inset, which removes the list before a double-click's second click can land. So a single click selects only after `double_click_window_ms` (350 ms) passes with no second click. List clicks feel a third of a second slow.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** parked (2026-10-12). Pinning is gone, so the parent's inset can't stay open beside the satellite's. Options: select at once and let a second click on the same spot in the satellite's inset do nothing; or give the satellite's inset a "back to parent" line in the list's place.

### F-101 · Frame bodies far from the star wobble on screen
- **Observation:** a frame body wobbles visibly when zoomed in. It is worst for the moons of the outer planets. The cause is precision: positions are relative to the star in `f32` render units. Morrow is about 387 units out, where an `f32` step is about 3 × 10⁻⁵ units. Zoomed in on a moon, the camera is only about 10⁻³ units away, so that step is a few per cent of the view, and the rounding changes from frame to frame.
- **Affects:** CAM-05, SCL-01 · **Type:** bug
- **Disposition:** proposed fix: render relative to the frame body (a floating origin). Subtract the frame body's position in `f64` km before converting to `f32`, in `view/scale.rs` and `projection.rs`, so everything near the frame body has full precision.

### F-102 · Exposure rings look like selected rings
- **Observation:** an exposure ring (bright neutral, 3 px) is hard to tell from a selected orbit ring (full white). Still true after the tails rework. Normal rings could also be thinner or darker, to give more contrast to tails, exposure rings and highlights.
- **Affects:** MRK-01, MRK-05, MOT-02, HOV-03 · **Type:** spec wrong
- **Disposition:** open; needs a palette decision (ring colours, alphas and the normal ring width, now 2 px).

### F-103 · The hover tooltip says too much
- **Observation:** the tooltip carries name, ownership, contents and the full click hints ("Click: inspect · Double-click: frame its moons · Double-click again: planetary view"). It is too long to read at a glance.
- **Affects:** HOV-01 · **Type:** spec wrong
- **Disposition:** open; needs design.

### F-104 · Double-clicking an exposure ring does nothing visible
- **Observation:** an object in exposure-ring mode can't be a frame (CAM-24), so double-clicking its ring only selects it. It feels like the double-click should do something.
- **Affects:** HOV-02, CAM-24, ZOOM-16 · **Type:** spec gap
- **Disposition:** open. One option: move into the parent's frame at its satellite fit, as CAM-24 does when a frame body turns into a ring.

### F-105 · Tails jump when the time rate changes
- **Observation:** changing the rate (for example 1 d/s to 5 d/s, or pausing) changes every tail's length and mode at once, so tails snap to their new length and bodies pop between dot and exposure ring. It feels abrupt.
- **Affects:** MOT-04, MOT-10, MOT-11, MOT-12 · **Type:** spec wrong
- **Disposition:** open. Option: ease the displayed tail length towards its new value over a short time (as the fold slide does), so a rate change animates.

### F-106 · The frame body's own orbit ring is a click target
- **Observation:** in a body's frame, its own orbit ring runs through the middle of the view and catches hovers and clicks meant for its satellites or empty space.
- **Affects:** HOV-02 · **Type:** spec wrong
- **Disposition:** open. Option: the frame body's own ring is drawn but not a hover or click target.

### F-107 · The docked inset covers the map and puts the view off-centre
- **Observation:** the inset is drawn over the right of the map. The frame body sits at the centre of the window, so with the inset open it looks off-centre in the visible map, and the inset hides part of the view.
- **Affects:** INS-07, CAM-05 · **Type:** spec wrong
- **Disposition:** open. Option: an expanding sidebar that shrinks the map area (and the camera viewport) instead of overlaying it, so the frame body stays centred in what's visible.

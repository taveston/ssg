# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-101.

---

### F5-07 · A single click on an "In orbit" entry waits for the double-click window
- **Observation:** selecting a satellite replaces the parent's inset, which removes the list before a double-click's second click can land. So a single click selects only after `double_click_window_ms` (350 ms) passes with no second click. List clicks feel a third of a second slow.
- **Affects:** INS-05 · **Type:** spec gap
- **Disposition:** parked (2026-10-12). Pinning is gone, so the parent's inset can't stay open beside the satellite's. Options: select at once and let a second click on the same spot in the satellite's inset do nothing; or give the satellite's inset a "back to parent" line in the list's place.

### F-102 · Exposure rings look like selected rings
- **Observation:** an exposure ring (bright neutral, 3 px) is hard to tell from a selected orbit ring (full white). Still true after the tails rework. Normal rings could also be thinner or darker, to give more contrast to tails, exposure rings and highlights.
- **Affects:** MRK-01, MRK-05, MOT-02, HOV-03 · **Type:** spec wrong
- **Disposition:** open; needs a palette decision (ring colours, alphas and the normal ring width, now 2 px).

### F-103 · The hover tooltip says too much
- **Observation:** the tooltip carries name, ownership, contents and the full click hints ("Click: inspect · Double-click: frame its moons · Double-click again: planetary view"). It is too long to read at a glance.
- **Affects:** HOV-01 · **Type:** spec wrong
- **Disposition:** open; needs design.

### F-105 · Tails jump when the time rate changes
- **Observation:** changing the rate (for example 1 d/s to 5 d/s, or pausing) changes every tail's length and mode at once, so tails snap to their new length and bodies pop between dot and exposure ring. It feels abrupt.
- **Affects:** MOT-04, MOT-10, MOT-11, MOT-12 · **Type:** spec wrong
- **Disposition:** open. Option: ease the displayed tail length towards its new value over a short time (as the fold slide does), so a rate change animates.

### F-107 · The docked inset covers the map and puts the view off-centre
- **Observation:** the inset is drawn over the right of the map. The frame body sits at the centre of the window, so with the inset open it looks off-centre in the visible map, and the inset hides part of the view.
- **Affects:** INS-07, CAM-05 · **Type:** spec wrong
- **Disposition:** open. Option: an expanding sidebar that shrinks the map area (and the camera viewport) instead of overlaying it, so the frame body stays centred in what's visible.

### F-112 · The hover tooltip still shows a body's ownership
- **Observation:** MRK-10 says a body's ownership is not shown on the map, but HOV-01 puts "name · ownership" in every marker's tooltip. The build reads MRK-10 as markers and labels: natural bodies are drawn neutral, and the tooltip still names the owner, as HOV-01 says.
- **Affects:** MRK-10, HOV-01 · **Type:** conflict
- **Disposition:** open. Option: drop ownership from the tooltip for the star, planets and moons.

### F-113 · Morrow IX's trader at 4 d is not in dot mode at 1 d/s
- **Observation:** TEST-09 asks for a trader round Morrow IX with a period of about 4 d, in dot mode at 1 d/s. With `tail_secs` at 8 s, anything under 8 d is an exposure ring at 1 d/s, so 4 d can't be a dot. The build uses 9 d: a dot at 1 d/s, at 0.46 of Morrow IX's Hill radius.
- **Affects:** TEST-09 · **Type:** spec wrong
- **Disposition:** open. Option: change TEST-09's period to about 9 d.

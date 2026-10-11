# Mock-ups

Six PNG boards from the design canvas "Inspect & Planetary Transition": reference for look, layout, palette and type. The spec decides behaviour. `../presentation-sketches.md` describes the same boards in words.

| File | Board |
|---|---|
| `1-system-view-hover.png` | 1 · System view, hovering Halden IV |
| `2-inspect-in-place.png` | 2 · Inspect in place, with a pinned comparison |
| `3-transition-storyboard.png` | 3 · Transition from inspect to planetary view |
| `4-planetary-view.png` | 4 · Planetary view |
| `5-inspection-fidelity.png` | 5 · Inspection fidelity follows knowledge |
| `6-satellite-grouping-and-motion.png` | 6 · Satellite grouping by zoom, and the motion rule |

How to use them:

- They are sketches, not pixel specs. Everything in them is a candidate unless the spec (`docs/spec/`) says otherwise.
- Where a mock-up and the spec (`docs/spec/system-view.md`) disagree, the spec wins. In particular, the bubble around Halden IV in board 2 and the magnifier in board 6 are rejected. Satellites are reached by focus and the inset's "In orbit" list.
- Fonts and colours in the images are the real ones (Barlow, JetBrains Mono, the palette in `../presentation-sketches.md`, specced as MRK-01 in `docs/spec/system-view.md`).
- Planet images are CSS placeholder gradients, not art direction for renders.

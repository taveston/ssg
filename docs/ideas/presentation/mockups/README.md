# Mock-ups

Boards 1 to 6 are from the design canvas "Inspect & Planetary Transition"; 7 onwards are HTML sources rendered to PNG. All are reference for look, layout, palette and type. The spec decides behaviour. `../presentation-sketches.md` describes the same boards in words.

| File | Board |
|---|---|
| `1-system-view-hover.png` | 1 · System view, hovering Halden IV |
| `2-inspect-in-place.png` | 2 · Inspect in place, with a pinned comparison |
| `3-transition-storyboard.png` | 3 · Transition from inspect to planetary view |
| `4-planetary-view.png` | 4 · Planetary view |
| `5-inspection-fidelity.png` | 5 · Inspection fidelity follows knowledge |
| `6-satellite-grouping-and-motion.png` | 6 · Satellite grouping by zoom, and the motion rule |
| `7-symbology.png` | Ship symbology sheet (2026-10-11). Source: `7-symbology.html`; described in `../symbology.md` |
| `8-ui-style.png` | Interface style: panels, message traffic, reports, requests, dossiers, Administration view (2026-10-11). Source: `8-ui-style.html`; described in `../ui-style.md` |
| `9-transfer-planner.png` | Transfer planner: two sliders with delta-v-coloured tracks, readouts, presets, Stop/Pass (now Rendezvous / Intercept), one preview arc with ghosts (2026-10-14). Source: `9-transfer-planner.html`, interactive; described in `../../mechanics/transfers.md` 3 |
| `10-trajectory-states.png` | How trajectories are drawn: scheduled, in flight, pass, intercept (2026-10-14). Source: `10-trajectory-states.html`; `transfers.md` 3.3 and 3.4 |
| `11-return-reserve.png` | The return reserve in the delta-v bar, and the one-way state (2026-10-14). Source: `11-return-reserve.html`; `transfers.md` 3.6 |
| `12-refuel-plan.png` | A two-leg plan with a refuel stop: leg stack and stacked bar (2026-10-14). Source: `12-refuel-plan.html`; `transfers.md` 3.7 |
| `13-combat-replay.png` | The combat replay view: relative plot, salvos, timeline, forces and log (2026-10-15). Source: `13-combat-replay.html`; `../combat-view.md` 3 |
| `14-engagement-on-map.png` | An engagement on the system map while it runs, and the Watch / Skip prompt when it ends (2026-10-15). Source: `14-engagement-on-map.html`; `../combat-view.md` 2 |
| `15-force-orders.png` | A force's orders, standing orders and the pre-fight estimate (2026-10-15). Source: `15-force-orders.html`; `../combat-view.md` 5 |
| `16-after-action-report.png` | The after-action report and the commander's file (2026-10-15). Source: `16-after-action-report.html`; `../combat-view.md` 6 |
| `17-missile-symbols.png` | Missiles in flight: arrow against small affiliation shapes in colour and greyscale, and a salvo from launch to the end (2026-10-15). Source: `17-missile-symbols.html`; `../combat-view.md` 3 |
| `18-multiple-objects.png` | Several objects, one symbol idiom: the pill against a frame-with-count marker, the idiom for mixed and formation cases, a member tray on a crowded ring, and the two always-together cases: an escort with its convoy and a formation docked at a station (2026-10-16, specced as MUL-01 to MUL-13). Source: `18-multiple-objects.html`; `../symbology.md` 4 |
| `19-frame-cue-and-label-halo.png` | The frame cue (flat cross against cross with a solid vertical axis), close in at an angle, across tilts, exposure rings with period and direction, and the label halo at 0 to 3 px (2026-10-16). Source: `19-frame-cue-and-label-halo.html`; `../presentation.md` 2 |
| `20-fold-marker-size.png` | Fold marker size against planet marker at the system overview: today's 22 px marker against a 16 px natural-only marker, with planet markers of 12, 13 and 14 px, and enlarged pairs (2026-10-16, from F-244). Source: `20-fold-marker-size.html`; `../symbology.md` 4 |
| `21-fold-marker-alternatives.png` | Moons as a property of the planet instead of a second marker: stacked ring with a count in the label, count badge, and moon pips, against today's 22 px marker; enlarged pairs (2026-10-16, from F-244). Source: `21-fold-marker-alternatives.html`; `../symbology.md` 4 |
| `22-moon-count-label-weight.png` | How loud the moon count beside a planet's stacked ring should be: "N moons" on a second line, number only in the dim colour, number inline, and ring only with the count on hover; labels enlarged (2026-10-16, from F-244). Source: `22-moon-count-label-weight.html`; `../symbology.md` 4 |
| `23-fold-marker-placement.png` | Where a planet's fold marker sits: level right against directly below, each quiet, with a neighbour in the label's left slot, and with the tray open to the left (2026-10-16, from F-229 and F-239). Source: `23-fold-marker-placement.html`; `../symbology.md` 4 |
| `24-contacts-on-the-planet-marker.png` | No fold marker for planets: the moon ring takes the affiliation of the most pressing contact folded in, as colour only, as frame shape and colour, or neutral with glyphs on the label; Sol overview and enlarged rings (2026-10-16, from F-229 and F-239). Source: `24-contacts-on-the-planet-marker.html`; `../symbology.md` 4 |
| `25-marker-pops-out-near-the-planet.png` | At rest a planet with things folded in shows only its ring; with the pointer near, the ring grows into the multi-marker below the planet, and a click opens its tray; the growth enlarged (2026-10-16, from F-229 and F-239). Source: `25-marker-pops-out-near-the-planet.html`; `../symbology.md` 4 |
| `26-objects-and-moons-on-the-planet-marker.png` | Moons and objects kept apart: a neutral moon ring with the moon count, and a separate resting object marker down and to the right, as a 12 px frame with a count (A) or a 6 px pip (B), growing into the multi-marker on approach (2026-10-16, from F-229 and F-239). Source: `26-objects-and-moons-on-the-planet-marker.html`; `../symbology.md` 4 |

How to use them:

- They are sketches, not pixel specs. Everything in them is a candidate unless the spec (`docs/spec/`) says otherwise.
- Where a mock-up and the spec (`docs/spec/system-view.md`) disagree, the spec wins. In particular, the bubble around Halden IV in board 2 and the magnifier in board 6 are rejected. Satellites are reached by focus and the inset's "In orbit" list.
- Mock-ups drawn before 2026-10-16 show selection and hover as a white ring round the marker. The spec uses four corner brackets (HOV-06) instead.
- Colours in the images are the real ones (the palette in `../presentation-sketches.md`, specced as MRK-01 in `docs/spec/system-view.md`). Boards up to 8 use the first build's fonts (Barlow, JetBrains Mono); 13 onwards load IBM Plex from `assets/fonts/`, so they render with the real fonts when opened from the repo.
- Planet images are CSS placeholder gradients, not art direction for renders.

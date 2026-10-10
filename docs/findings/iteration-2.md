# Findings: iteration 2

*Iteration:* 2, Corvane system view as a vector display (commit `5ff0381`). *Written up:* 2026-10-10, after the fact, from the iteration 3 brief and the notes left in the code. Format and types are explained in `docs/README.md`.

---

### F2-01 · Moons drawn individually at whole-system zoom
- **Observation:** all moons were drawn individually at whole-system zoom.
- **Affects:** new · **Type:** spec gap
- **Disposition:** spec amended: satellite grouping, GRP-01 to GRP-08 (from presentation.md, 2026-10-09). **Closed.**

### F2-02 · Moons orbit too fast to read at 1 day/s
- **Observation:** moons orbited too fast to read at 1 day/s.
- **Affects:** new · **Type:** spec gap
- **Disposition:** spec amended: motion rule, MOT-01 to MOT-07, and test periods TEST-02. **Closed.**

### F2-03 · Fully continuous zoom feels odd
- **Observation:** the fully continuous zoom felt odd.
- **Affects:** zoom · **Type:** spec wrong
- **Disposition:** spec amended: stepped zoom with stops and limits, ZOOM-01 to ZOOM-11. **Closed.**

### F2-04 · Inspect simplifications in the build
- **Observation:** noted in `src/hover_inspect.rs`: the leader line is a short straight connector rather than a diagonal; an inspected planet's satellites un-grouped instead of fanning out; the rest of the map was not dimmed; clicking empty space does not close the inset (Esc does).
- **Affects:** INS-01, INS-03 · **Type:** bug (minor deviations)
- **Disposition:** fan-out and dimming superseded by the magnifier (MAG-01, MAG-02). Straight leader line accepted for now. Whether clicking empty space should close the inset is a **spec gap, open**: decide and add to INS-03.

# Findings

Open observations from building and playing the prototype. Each one is waiting for a decision. Once it's settled, the outcome goes into the spec, the code or the to-do list, and the finding is deleted.

Format and types are in `docs/README.md`. New findings take the next free number from F-115.

---

### F-113 · Morrow IX's trader at 4 d is not in dot mode at 1 d/s
- **Observation:** TEST-09 asks for a trader round Morrow IX with a period of about 4 d, in dot mode at 1 d/s. With `tail_secs` at 8 s, anything under 8 d is an exposure ring at 1 d/s, so 4 d can't be a dot. The build uses 9 d: a dot at 1 d/s, at 0.46 of Morrow IX's Hill radius.
- **Affects:** TEST-09 · **Type:** spec wrong
- **Disposition:** open. Option: change TEST-09's period to about 9 d.

### F-114 · Large solid white body discs look bad and pile up
- **Observation:** since MRK-03 and MRK-10, the star, planets and moons are solid white discs 36, 30 and 24 px across. They look heavy, and in the inner system they overlap into a white mass.
- **Affects:** MRK-01, MRK-03, MRK-10 · **Type:** spec wrong
- **Disposition:** open. Options: go back to smaller discs (the old 18, 12 and 6 px); or keep the size but add visual relief, such as a dimmer or outlined disc, shading, or a dark rim so overlapping discs stay apart.

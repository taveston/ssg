# Spec: Ideology profile

*Scope:* how a faction's (and later a character's) ideology and attitudes are stored, changed, labelled and displayed. Simulation rules only; where it appears in the UI is not specced yet. *Source ideas:* `docs/ideas/mechanics/faction-history.md` (sections 1, 2 and 8), `docs/ideas/gdd.md` (4.7), diagram `docs/ideas/mechanics/ideology-triangle.svg`. *Status:* specced, not yet built. Checks are in `docs/validation/ideology.md`.

IDs are stable: never reused or renumbered.

---

## 1. Scores

| ID | Requirement |
|---|---|
| IDEO-01 | A profile holds three ideology scores, **Cohesion**, **Authority** and **Autonomy**, which always sum to exactly 12. |
| IDEO-02 | Scores are stored as integers in tenths (0 to 120 each, summing to 120) and displayed divided by 10 to one decimal place. |
| IDEO-03 | The display form is "Coh 6 · Auth 4 · Auto 2" (one decimal shown only when not a whole number). |
| IDEO-04 | A profile also holds **Outlook** (Insular − to Outward +) and **Militancy** (Pacifist − to Militant +), integers clamped to −6…+6, starting at 0. |

## 2. Labels

| ID | Requirement |
|---|---|
| IDEO-10 | If any score is 7 or more (70 internally), the label is that corner's: Cohesion → **Collectivist**, Authority → **Authoritarian**, Autonomy → **Individualist**. |
| IDEO-11 | Otherwise, if the lowest score is 3 or less (30 internally), the label is the edge opposite that score: Autonomy lowest → **Paternalist**, Cohesion lowest → **Meritocratic**, Authority lowest → **Mutualist**. If two scores tie for lowest at 3 or less, the lowest decides; an exact tie between two lowest scores is unspecified (see open points). |
| IDEO-12 | Otherwise the label is **Pluralist**. |
| IDEO-13 | Each label works as an adjective for a state ("a Paternalist state") and lower-case as a noun for a person ("a paternalist"). |
| IDEO-14 | Outlook and Militancy words: 0 → "neutral"; ±1 to ±2 → "outward-leaning" / "insular-leaning", "militant-leaning" / "pacifist-leaning"; ±3 or more → "strongly outward" / "strongly insular", "strongly militant" / "strongly pacifist". |
| IDEO-15 | Summary line: label, then Outlook word, then Militancy word, comma-separated, e.g. "Meritocratic, outward-leaning, strongly militant". |

## 3. Events

| ID | Requirement |
|---|---|
| IDEO-20 | A single-score event "+x Force" adds x to that score and removes x from the other two in proportion to their current sizes (moving the profile in a straight line towards that corner); "−x Force" does the reverse. |
| IDEO-21 | Rounding: split the change proportionally in tenths, round each share down, then give leftover tenths one at a time to the score with the largest remainder, so the total stays exactly 120. |
| IDEO-22 | Limits: no score goes below 0 or above 120; a negative change is capped at the current score; if both other scores are 0 when one must give points back, the points are split equally between them (leftover tenth by the rule in IDEO-21). |
| IDEO-23 | A two-sided event "A → B x" moves x from score A to score B only, capped at A's current value; the third score is untouched. |
| IDEO-24 | Outlook and Militancy events add their value and clamp to −6…+6. |
| IDEO-25 | Worked example (must hold): Coh 6 · Auth 1 · Auto 5, then +3 Authority, gives Coh 4.4 · Auth 4 · Auto 3.6 (Mutualist → Pluralist). |

## 4. Starting profile from faction history

| ID | Requirement |
|---|---|
| IDEO-30 | A faction picks one option in each of three phases (origins, turmoil, recovery). Each option gives exactly 4 points spread across the three forces, so three options sum to 12; Outlook and Militancy are plain sums of the options from 0. |
| IDEO-31 | The option values are data, loaded from a file, not code. The current values are the tables in `docs/ideas/mechanics/faction-history.md` sections 3 to 5 (placeholders for balancing). |

## Open points (decide before building)

- Tie-break for IDEO-11 when the two lowest scores are equal and both 3 or less, with no score at 7+. It happens on the boundary line between two edge regions: for example Coh 3 · Auth 3 · Auto 6 (reachable from two histories) or, after events, Coh 2.8 · Auth 2.8 · Auto 6.4. Options: a fixed priority order, or Pluralist.
- Whether characters use exactly the same structure (gdd 4.3 says candidate).

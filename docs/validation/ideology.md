# Validation: Ideology profile

Checks for `docs/spec/ideology.md`. Everything here is pure logic, so every check should be a `cargo test` case named after its ID once the profile is built. Record results per iteration as in `system-view.md`.

## Test vectors

| ID | Input | Expected |
|---|---|---|
| IDEO-01, 02 | Any profile after any sequence of events below | Scores are integers 0–120 summing to exactly 120 |
| IDEO-03 | 60 / 40 / 20; 44 / 40 / 36 | "Coh 6 · Auth 4 · Auto 2"; "Coh 4.4 · Auth 4 · Auto 3.6" |
| IDEO-04, 24 | Outlook +5 then +3 event | Outlook +6 (clamped) |
| IDEO-10 | 0 / 1 / 11; 1 / 11 / 0; 12 / 0 / 0; 7 / 0 / 5 | Individualist; Authoritarian; Collectivist; Collectivist |
| IDEO-11 | 5 / 4 / 3; 0 / 6 / 6; 6 / 0 / 6 | Paternalist; Meritocratic; Mutualist |
| IDEO-12 | 4 / 4 / 4; 4.4 / 4 / 3.6 | Pluralist; Pluralist |
| IDEO-11 boundary | 3.1 / 4.9 / 4.0 vs 3.0 / 5.0 / 4.0 | Pluralist vs Meritocratic |
| IDEO-14, 15 | Label Meritocratic, Outlook +1, Militancy +4 | "Meritocratic, outward-leaning, strongly militant" |
| IDEO-20, 21, 25 | 6 / 1 / 5, then +3 Authority | 4.4 / 4 / 3.6 |
| IDEO-21 | 3 / 4 / 5, then +1 Cohesion (shares 4.44 and 5.56 tenths, rounded down to 4 and 5) | 4 / 3.6 / 4.4: the leftover tenth goes to Autonomy, the larger remainder |
| IDEO-22 | 0 / 12 / 0, then −3 Authority | 1.5 / 9 / 1.5 |
| IDEO-22 | 2 / 5 / 5, then −4 Cohesion | Cohesion 0 (change capped at 2); others +1 each |
| IDEO-23 | 4 / 4 / 4, then Autonomy → Authority 2 | 4 / 6 / 2 |
| IDEO-30 | Every combination of the 6 × 6 × 6 history options | Scores sum to 12; label counts match `faction-history.md` section 7 (corners about 18%, edges 13–15%, Pluralist 4%) |

## Results

| Iteration | Result | Notes |
|---|---|---|
| (not built) | – | |

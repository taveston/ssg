# Orphan Worlds: Interface Style (Panels, Windows, Reports)

This doc proposes how the interface around the map looks and behaves: panels, the window model, message traffic, reports, requests, dossiers and the Administration view. The main ideas:

- **One console register everywhere.** The same dark, quiet look as the map, in the spatial views and in Administration. Flavour comes from structure (signal headers, file stamps), not from a change of skin.
- **Figures carry their certainty.** One notation for exact, estimated, ranged, unknown and stale values, used in every panel, report and table.
- **Reports have authors.** Each comes from a named person or office, with the state's own figures underneath. Whether the prose carries the author's bias is a mechanics question, still open.
- **No free-floating windows.** Panels dock in fixed places, the inspection inset included.

The console register is **decided** (2026-10-12); everything else is **candidate**. Mock-up: `mockups/8-ui-style.png` (source `mockups/8-ui-style.html`). Builds on *Presentation & Visual Style* (sections 2 and 4) and *Symbology*. Names, figures and the currency ("cr") in the mock-up are placeholders.

---

## 1. Register

- **Console, not paper (decided 2026-10-12):** the Administration view keeps the dark console look rather than switching to a "paper files" look. Reasons: one set of components to build, long text reads well light-on-dark at these sizes, and moving between map and Administration doesn't jolt. Board 8 of the mock-up compares the two.
- The look is a well-kept government plotting console: near-black ground, translucent panels, hairline borders, small corner radius (3–4 px), no glow, no scan lines, no drop shadows.
- Period flavour comes from text conventions borrowed from military staff work: precedence, date-time groups, FROM/TO/SUBJ headers, file stamps in small mono capitals.

## 2. Type, colour and controls

- **Type:** Barlow for text and labels; JetBrains Mono for figures, dates and stamps (MRK-02). Section headings in small spaced capitals. Body text 14 px, nothing under 11 px. Reading width kept under about 70 characters.
- **Colour has meaning or none.** Colours are reserved:
  - Affiliation colours (SYM-02) only for affiliation: symbols, references and owner lines.
  - Orange for "needs attention" (a shortfall, low readiness, an urgent message). Same orange as unknown contacts; both mean "look at this".
  - **No green for good or red for bad.** Green means neutral and red means hostile. Good and bad are shown by sign and words; only "needs attention" gets a colour.
  - Everything else is off-white text, pale grey-blue secondary text, and hairlines.
- **Buttons:** one primary per panel (filled off-white), secondary (outlined), quiet (text only). An action that can't be undone uses an orange outline and asks once. **Specced** 2026-10-13 as BTN-01, BTN-02 (the first build used orange and blue for buttons, F-117).

## 3. Figures carry their certainty

The same notation everywhere, so the player can always tell measured from guessed. Ties the whole interface to the uncertainty pillar, not just the inset images.

| Kind | Shown as | Example |
|---|---|---|
| Exact | Plain figure | 141 k |
| Estimate | "~" before the figure | ~4 |
| Range | Low–high | 6–8 |
| Unknown | "?" in a dashed chip | ? |
| Stale | Dimmed, with the date it was true | 212 k · 02 FEB |
| Needs attention | Orange | −18 u |

- Skills and traits use it too: skill bars show the range, unknown traits are dashed chips ("? not yet observed"), as in the planetary-view sketch.
- Any figure can show its source and date on hover.

## 4. Window model

- **Fixed docks, no free windows.** Top bar across the top; a right-hand detail panel for long content (as in the planetary view); message traffic docked bottom right; the inspection inset docked at the right of the map (INS). Nothing floats or overlaps by accident.
- **Insets** stay the only map-anchored panels. Pinned insets (INS-03) are the way to compare; **candidate:** a limit of about three pinned, oldest unpinned first.
- **Modal dialogs** only for decisions that can't be undone, and they never stop the clock unless the player pauses.
- Every panel closes with Esc, and the top bar (date, rate, traffic count) is visible in every view, including Administration.

## 5. Panel anatomy

Every panel follows the same order (board 2):

1. **Header:** symbol or portrait, name, one class-and-ownership line, and Focus / Pin / Close where they apply.
2. **Key figures:** a row of label-over-value pairs, figures in mono.
3. **Sections** under small-capital headings, separated by hairlines: lists, tables, text.
4. **Actions** last, primary first.

- **Specced** for the inspection inset (2026-10-13, from F-120): key facts and section headings as INS-09, buttons as INS-02.

- **References:** any ship, station, person or organisation named in text is a reference: its symbol (ships and stations) and name, underlined. Click shows it on the map or opens its file. This is the "links to the map" idea from *Presentation & Visual Style*, section 4.

## 6. Message traffic

- **One tray for everything that arrives:** reports, requests, bids, arrivals, alerts. Docked bottom right on the map; the full list lives in Administration.
- **Precedence sets urgency, not colour:** FLASH (events that pause the clock: hostile action, loss of a ship), IMMEDIATE (orange text), PRIORITY, ROUTINE (dimmed). Replaces the alerts sketched top right in board 1.
- Each item: precedence, subject, author and their post, and a date-time group (DTG). Read items dim.
- **Candidate:** filters by office and by place; auto-pause rules per precedence, set by the player.

## 7. Reports

- **Layout (candidate):** a signal header (PREC, DTG, FROM, TO, SUBJ), then a short body, then the figures it rests on in mono, with their certainty notation.
- **Authored, biased prose (open; a mechanics decision):** the body could be written from the author's point of view and carry their personality and bias (an aggressive commander reads threat into a cold hauler), so the player learns to weigh people against data. Ties into characters with hidden traits (*Characters, Organisations & Events*). The worry is the running clock: at 1 day/s, reports arrive faster than prose can be read, and reading for nuance invites pausing all the time. Options to weigh later: keep bias to a single line or a tag ("Marr assesses: hostile"); put bias only in rare, important reports; or roll routine traffic into digests. Settle with the character and information mechanics.
- **Summary first:** every report and collated return opens with a one-sentence conclusion, then the table (board 7).
- Actions on a report are the decisions it asks for (Authorise / Refuse), plus Show on map and Reply.
- **Candidate:** reports reach the player at once (no signal delay, per the information model), but carry the date of the observation they describe, which may be older.

## 8. Requests

- A request names its author, a reply-by date and explicit options. Each option lists what the state can price (cost, time, who builds it) as figures, and what it can't as the author's claim ("Voss: not enough") or an estimate ("opinion ~ −10").
- "Ask for advice" sends it to the relevant office for a second view, which may disagree with the author.
- "Later" keeps it in traffic until the reply-by date; what happens after that is up to the author.

## 9. Dossiers

- A person's file shows what the state knows: portrait, post, origin; traits seen as chips and traits not yet seen as dashed chips; skills as ranges; ideology label (*Factions & Ideology*, the ideology triangle) with its certainty; and a dated record of dealings with them.
- Entries from old or second-hand sources are dimmed and name their source.
- Organisations get the same file layout.

## 10. The Administration view

- **Layout (candidate):** full-screen 2D. Left: navigation by office. Centre: the document. Right: context. The offices in the mock-up (Cabinet, Treasury, Admiralty, Procurement, Interior, Intelligence; Files: People, Organisations, Records) are placeholders: none of government is defined yet.
- The right column holds the office holder, related items and that office's policy controls.
- **Offices own their reports and their levers:** the Treasury page shows the monthly returns and the tax and toll settings; reports describe, policy panels decide, so the player never edits a number inside a report.
- The top bar stays, with a Map button back to the spatial view.
- **Collated returns:** tables with this month, last month and a six-month sparkline per line; notes link to causes ("no convoy in Feb · C-12").

## 11. Open questions

- Office list and names: depends on how government and ministers are designed.
- Whether reports arrive as separate items or roll up into daily or weekly digests at high time rates.
- How much prose to generate per report, and whether authors' traits colour it, given the running clock (section 7).
- Portrait style (painted, photographic, or a stylised ID-card look) and its art cost.
- Whether the right-hand detail panel in the spatial views and the Administration context column are the same component.
- Currency name and units.
- Accessibility: text scaling, and a high-contrast option for the hairline style.

## Rendering the mock-up

`8-ui-style.html` reuses the symbol rules of `7-symbology.html`. To regenerate the PNG, open it at 1440 px wide and take a full-page screenshot (or use Playwright, pointing the fonts at `assets/fonts/`).

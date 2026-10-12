# Orphan Worlds: design docs

This folder is the single source of truth for the design. Ideas are worked out in `ideas/`, committed rules go in `spec/`, the to-do list says what to build next, and findings record what building and playing turned up. Nothing lives only in a chat or a claude.ai Project.

## Layout

| Path | What goes in it | Rules |
|---|---|---|
| `ideas/` | Thinking: vision, mechanics, back story, presentation, open questions, rejected options | Free-form. Not a basis for code. Settled points are marked **Decided** in place, with a line on why. |
| `ideas/gdd.md` | The overview: vision, pillars, structure, every system in brief, risks, open questions | Start here. |
| `ideas/world/` | Setting, back story, tone, history of the old network | May never become spec; shapes names and flavour. |
| `ideas/mechanics/` | Systems design: ideology and faction history, components and ship assembly, requirements and bids, drives and transit, gates, world generation | Feeds the spec gradually. |
| `ideas/presentation/` | Views and visual style, reference sketches, mock-ups | Mock-ups illustrate; the spec decides behaviour. |
| `ideas/scratch.md` | Half-formed notes | Anything goes. |
| `spec/` | The committed design, written to code from | Every requirement has a stable ID. No open questions inside a requirement. |
| `todo.md` | What to build next: spec changes, fixes and other work | Ordered by priority. Done items are deleted. |
| `findings.md` | Open observations from building and playing | Deleted once decided. |

Status tags in `ideas/`: **Decided** = settled. **Direction** = settled in principle, details open. **Candidate** = under consideration. These describe the idea, not whether it's built: only `spec/` is built from.

## How work flows

1. **Think** in `ideas/`. Mark what gets settled, and why, where it's discussed.
2. **Spec** it: copy the settled rules into `spec/` with IDs, add a **Specced** note in the idea doc, and add an item to `todo.md` under "Spec changes" naming the IDs. A spec change is never committed without its to-do item, in the same commit. This includes existing requirements whose meaning changes, even slightly: clear their ✓ and list them. A rewording that leaves the behaviour the same needs neither.
3. **Build:** you pick items from `todo.md` for a coding session. The session builds them with automated tests, marks the requirements ✓ in the spec's Built column, logs any findings and deletes the finished items.
4. **Play** the build whenever you like, and note anything that looks or feels wrong in `findings.md` (or tell a session to). There's no fixed checklist.
5. **Triage** at the start of a design session: decide each open finding. The outcome goes into the spec (with a to-do item), into `todo.md` under "Fixes", or into `ideas/`, and the finding is deleted.

**Check before committing a spec change** (no test does this): every requirement with an empty Built cell is named on the to-do list (ranges like `IDEO-01 to IDEO-31` count), and every requirement whose meaning changed has its ✓ cleared and is on the list too. Coding sessions run the same check when they finish an item.

## Spec conventions

- IDs are `PREFIX-NN` (for example `ZOOM-13`, `IDEO-20`); each spec file lists its prefixes in its tables. IDs are never reused or renumbered.
- A withdrawn requirement is deleted, and its ID is added to the file's *Retired IDs* line so it's never reused.
- Each requirement is one testable statement. Tunable numbers are named and listed in a table; the code reads them from one config resource.
- **Built** is ✓ once the requirement is in the code, and blank until then.
- A requirement that can't be tested automatically ends with *Manual:* and the reason. Use this rarely: most visual rules can be tested through what drives the picture.
- A requirement about how code is structured (where things live, what imports what, function size, naming, comments, which module reads a key) is guidance for code generation and ends with *Checked in review.* No test reads the source text to enforce it. Tests check behaviour. This is different from *Manual:*, which marks behaviour that can't be tested automatically.
- Each spec file names its source idea docs, and the idea doc says what was specced (a **Specced** note). That two-way link replaces a separate decision log.

## Findings

Each entry in `findings.md`:

```
### F-NNN · Short title
- **Observation:** what you saw or felt, or what the build had to decide
- **Affects:** spec IDs, or "new" · **Type:** see below
- **Disposition:** open, plus the options if there are any
```

Types: **bug** (the build doesn't match the spec), **spec gap** (the spec says nothing), **spec wrong** (the build follows the spec and the result feels bad: the most valuable kind), **idea** (a new thought for `ideas/`), **conflict** (two docs, or a doc and the build, disagree).

When a coding session has to make a choice the spec doesn't cover, it records the choice in a finding. If the choice is fine, triage copies it into the spec and deletes the finding.

## Writing

- Use short, plain sentences. Leave out filler and hedging.
- Open each doc with a few lines saying what it covers and its main conclusions, before any detail.
- Don't keep history in the docs. Git has it. Remove what's no longer true rather than striking it through.

These apply to everything in `docs/` and to `CLAUDE.md`. Older docs are being brought up to this style (see `todo.md`).

## Working with Claude

- In claude.ai, attach this repo (`taveston/ssg`) to the chat; the Project points here rather than holding copies.
- Design sessions edit `ideas/`, `spec/`, `todo.md` and `findings.md` directly and commit; nothing is pasted across by hand.
- Coding sessions (Claude Code) follow `CLAUDE.md` at the repo root.

## Not in the repo yet

Some idea docs cite generator scripts (`scripts/bids/`, `scripts/components/`, `scripts/transit/`) that were run in earlier chat sessions but never committed. Their tables are kept as last generated. Add the scripts here if they turn up, or regenerate them before the figures are specced.

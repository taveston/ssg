# Orphan Worlds: design docs

This folder is the single source of truth for the design. Nothing lives only in a chat or a claude.ai Project; if it matters, it's here.

## Layout

| Folder | What goes in it | Rules |
|---|---|---|
| `ideas/` | Thinking: vision, mechanics, back story, presentation, open questions, rejected options | Free-form. Not a basis for code. Settled points are marked **Decided** in place, with a line on why. |
| `ideas/gdd.md` | The overview: vision, pillars, structure, every system in brief, risks, open questions | Start here. |
| `ideas/world/` | Setting, back story, tone, history of the old network | May never become spec; shapes names and flavour. |
| `ideas/mechanics/` | Systems design: ideology and faction history, components and ship assembly, requirements and bids, drives and transit, world generation | Feeds the spec gradually. |
| `ideas/presentation/` | Views and visual style, reference sketches, mock-ups | Mock-ups illustrate; the spec decides behaviour. |
| `ideas/scratch.md` | Half-formed notes | Anything goes. |
| `spec/` | What we have committed to build, written to code from | Every requirement has a stable ID. No open questions inside a requirement; unknowns go in the file's "Open points" or stay in `ideas/`. |
| `findings/` | One file per prototype iteration: what was observed and what was done about it | Every finding gets a disposition. |
| `validation/` | One checklist per spec file: how to check each ID, and the result per iteration | A failed check becomes a finding. |

### Status tags in `ideas/`

**Decided** = settled. **Direction** = settled in principle, details open. **Candidate** = under consideration. These describe the idea, not whether it is built: only `spec/` is built from.

## Spec conventions

- IDs are `PREFIX-NN` (for example `ZOOM-03`, `IDEO-20`); each spec file lists its prefixes. IDs are never reused or renumbered. A withdrawn requirement stays in place, struck through, with a note.
- Each requirement is one testable statement. Tunable numbers are named and listed in a table; the code reads them from one config resource.
- Each spec file names its source idea docs, and the idea doc says what was specced (a **Specced** note pointing at the IDs). That two-way link replaces a separate decision log.
- Code comments and tests cite IDs (`// GRP-01`, `fn grp_01_hysteresis()`), so a failing test points back to the requirement.

## Findings

One file per iteration, `findings/iteration-N.md`. Each entry:

```
### FN-NN · Short title
- **Observation:** what you saw or felt, or what the build had to decide
- **Affects:** spec IDs, or "new" · **Type:** see below
- **Disposition:** fix code / amend spec (IDs) / park in ideas (doc) / won't fix, plus **Closed** when done
```

Types: **bug** (the build doesn't match the spec), **spec gap** (the spec says nothing), **spec wrong** (the build follows the spec and the result feels bad: the most valuable kind), **idea** (a new thought for `ideas/`), **conflict** (two docs, or a doc and the build, disagree).

## The cycle

1. **Think** in `ideas/`. Mark what gets settled, and why, where it's discussed.
2. **Spec** what the next iteration will build: copy the settled rules into `spec/` with IDs, add a **Specced** note in the idea doc, and write the checks in `validation/`.
3. **Brief:** the milestone section of `CLAUDE.md` lists the spec IDs in scope.
4. **Build** from `spec/` only. Ambiguities the build has to resolve are written up as findings (spec gap), not left in a summary.
5. **Validate:** run `cargo test` and the checklist; record results; every ✗ becomes a finding.
6. **Playtest** and add observations to the iteration's findings file.
7. **Triage** at the start of the next design session: give every open finding a disposition, then back to step 1 or 2.

## Working with Claude

- In claude.ai, attach this repo (`taveston/ssg`) to the chat; the Project points here rather than holding copies.
- Design sessions edit `ideas/`, `spec/`, `findings/` and `validation/` directly and commit; nothing is pasted across by hand.
- Coding sessions (Claude Code) follow `CLAUDE.md` at the repo root.

## Not in the repo yet

Some idea docs cite generator scripts (`scripts/bids/`, `scripts/components/`, `scripts/transit/`) that were run in earlier chat sessions but never committed. Their tables are kept as last generated. Add the scripts here if they turn up, or regenerate them before the figures are specced.

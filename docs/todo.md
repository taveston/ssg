# To do

Work waiting to be built. You choose which items a coding session takes, and the order of the list is the priority.

- **Spec changes:** design sessions add an item whenever they add, change or withdraw requirements. One item per coherent change, naming the IDs.
- **Fixes:** findings triaged as "fix code".
- **Other:** refactors, tooling and doc work that isn't a spec change.

A finished item is deleted (git history keeps the record). Notes indented under an item are hints for whoever builds it.

---

## Spec changes

- [ ] **Test coverage rule** · ARCH-36, and the reworded ARCH-23 and ARCH-30
  - Add an architecture test that finds every built ID in the feature specs (system view, ideology) and checks that a test comment or doc names it, unless the spec marks it *Manual*. It replaces `arch_23_every_built_spec_id_is_cited_in_a_comment`.
  - Write tests for built requirements that have none. Many visual rules can be checked through what drives the picture: projected positions, fold state, pixel sizes, simulated input in a headless app. Mark *Manual* only what truly can't be tested, with the reason.
  - Rename the tests named after spec IDs (`grp_01_…`) so the name says what they check, with the ID in a comment or doc line.
  - Rewrite comments that are only an ID so they explain the rule on their own. Remove comments that point at deleted docs (the validation checklist, iteration history).
- [ ] **Ship and station symbols** · SYM-01 to SYM-08, SYM-13 to SYM-15, TEST-05, and the reworded MRK-01, MRK-04, GRP-06, INS-05, TEST-03
  - Replaces the triangle, diamond and hollow-square markers; delete their code. The `Ownership` enum (own, neutral, foreign) gives way to affiliation and command (SYM-01).
  - Reference: mock-up `docs/ideas/presentation/mockups/7-symbology.png` for shapes and proportions. Ignore its installation role icons.
  - Most rules can be tested through what drives the picture: which frame, icon, fill and amplifiers a known-values record produces, and pixel sizes.
- [ ] **Ship groups and condition** · SYM-09, SYM-10, SYM-12, GRP-09, TEST-06
  - Needs the symbols item first.
- [ ] **Velocity leaders** · SYM-11
  - Blocked: ships in transit are not specced yet.
- [ ] **Trackpad pinch** · ZOOM-07
  - Blocked: see F3-01.
- [ ] **Ideology profile** · IDEO-01 to IDEO-31
  - Decide the open points in `docs/spec/ideology.md` first.

## Fixes

None yet.

## Other

- [ ] **Style pass over existing docs:** bring `docs/ideas/` and `docs/spec/` up to the writing guidelines in `docs/README.md` (summary at the top, short plain sentences). A design session job, reviewed before committing, since tightening prose can shift meaning.

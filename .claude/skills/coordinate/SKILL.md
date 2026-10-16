---
name: coordinate
description: Work through docs/todo.md as a coordinator. Parallel worktree agents build one item each; the coordinator merges every item onto master and stays free for design talk, upstream doc pulls and playtests. Use when the user asks to build several to-do items, "work through the todo list", or says /coordinate.
---

# Coordinate a build session

You plan, dispatch, merge and talk. Agents build. Each to-do item ends as a green commit on `master`, so an interrupted session leaves a working checkpoint. Arguments, if given, narrow the scope (for example "only Spec changes" or "items X and Y").

## 1. Plan (in plan mode if the user is in it)

- Read `docs/todo.md`. Skip blocked items, anything under Parked, and items that only touch docs. Say which you skip and why.
- Order by the dependencies the items name ("build after …"). Group them into streams that touch different areas (for example view vs `sim/transfer`, `content`, `crates/orbits`). Run at most 2–3 agents at once.
- Split a very large item into parts run one after another (for example rules → panel → map). Each part marks ✓ only what it built and edits the item to list what's left. Write each part's API summary to a scratch file for the next part.
- Effort per agent, in the plan for the user to approve: **low** for mechanical renames, **medium** for content and small view items, **high** for orbital maths, large UI and wide changes.
- Write the plan, then exit plan mode for approval.

## 2. Dispatch

- Copy `brief.md` (next to this file) into the session scratchpad. Fill in the repo path, and add any new Bevy pitfalls from memory.
- Start each agent with `isolation: "worktree"`, `run_in_background: true` and the approved `effort`. The prompt says: read the brief at <scratch path> first; then gives the item text and spec IDs, what's already on master that it builds on, which parallel agents touch nearby files, and a findings number range (distinct per agent).
- When something an agent depends on lands, tell it with SendMessage (for example "X is on master; rebase before your final commit"). When a pitfall shows up, add it to the brief so later agents get it.

## 3. Merge each finished item

1. In the agent's worktree: `git rebase master`. Resolve conflicts with small Python scripts. Never chain `git add`/commit after a script that might have failed.
2. Renumber its findings so the sequence has no gaps, and keep the "next free number" line in `docs/findings.md` right. Check that the count of `### F-` headings matches the count of Disposition lines.
3. Run `touch crates/orbits/src/*.rs`, then `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.
4. From the repo root: `git merge --ff-only <branch>`. Then `git worktree remove --force <path>`, `git worktree prune` and `git branch -D <branch>`.
5. Check coverage: every spec row with an empty Built cell is named on `docs/todo.md` (ranges count).
6. Start the next item in that stream from the new master. Report to the user in a few lines: what landed, its findings, and what's running.

Typical conflicts:
- `docs/todo.md`: both sides deleted different done items, so drop both.
- The end of `src/headless_tests.rs`: both sides appended a test, so keep both and re-add the closing `}`.
- `docs/findings.md`: keep both sides' findings.
- Spec Built cells: take the newest text, and keep ✓ only where the meaning is unchanged.

## 4. While agents run

- **"Pull latest design changes":** `git fetch`, then merge `origin/master` and resolve conflicts as above. Run the coverage check, slot new items into the plan file, and tell running agents about changes that affect them.
- **Playtest:** follow CLAUDE.md "Seeing the game": rebuild master fresh (`touch src/main.rs` first), then copy the exe and `assets/` into `play/`. Agents never touch `play/`.
- **Observations:** record each as a finding at once (bug, spec wrong, spec gap or idea), under To triage. Ask before fixing even a clear bug. A design direction the user gives goes into the finding's disposition. Change the spec only when asked.
- **Progress:** list what's on master, what's running and what's next.
- Push only when the user asks.

## 5. Finish

When the user asks, run a high-level code review agent once the scheduled work is merged. It applies easy behaviour-preserving fixes, raises findings for spec questions, and adds refactors to todo.md under Other.

## Pitfalls

- Worktrees share `CARGO_TARGET_DIR=<repo>/target`, because a full target is about 26 GB.
  - Every worktree's `orbits` crate has the same name in it, so a stale copy gives "unresolved imports" or doctest failures. Touch its sources and rerun.
  - `target/debug/orphan_worlds.exe` may be another worktree's build.
- A worktree can start from an old commit, so the brief's first step syncs it with master.
- Game runs hang if a shot can't finish. `OW_SHOT` has a `timeout` key, and agents also wrap runs in their own timeout. If a game window lingers, find its parent `cargo run` and close it.
- Plan mode switched on mid-run blocks agents' commits. Commit their finished work for them afterwards.
- On Windows, `git worktree remove` often says "Permission denied". Use `--force`, then prune. Leftover folders under `.claude/worktrees/` are harmless.

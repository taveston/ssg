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
- Effort per agent: **low** for mechanical renames, **medium** for content and small view items, **high** for orbital maths, large UI and wide changes. Weigh risk as well as kind: subtle per-frame state can need high even when the diff is small. Reviews run at medium, or high for large or orbital diffs. Tell the user the level whenever you start an agent; no approval needed.
- Write the plan, then exit plan mode for approval.

## 2. Dispatch

- Before starting agents, look for a running `orphan_worlds.exe` (`Get-Process orphan_worlds`, check `Path`). One started from `target/` is a leftover run, not a playtest: stop it, since it locks the exe and blocks agents' builds and screenshots. Leave one running from `play/` alone. Do the same check whenever an agent reports "Access is denied".
- Keep the PC awake while agents run, or a sleep stalls every build for hours. Start this in the background with PowerShell (`run_in_background`); it changes no settings, keeps the system (not the screen) awake only while it runs, and stops itself after 12 hours:
  ```powershell
  $k = Add-Type -MemberDefinition '[DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint esFlags);' -Name Power -Namespace Win32 -PassThru
  $null = $k::SetThreadExecutionState([uint32]2147483649)  # ES_CONTINUOUS | ES_SYSTEM_REQUIRED
  $end = (Get-Date).AddHours(12); while ((Get-Date) -lt $end) { Start-Sleep -Seconds 60 }
  ```
  Stop that task when the work is finished. A closed laptop lid still sleeps.
- Copy `brief.md` (next to this file) into the session scratchpad. Fill in the repo path, and add any new Bevy pitfalls from memory.
- Start each agent with `isolation: "worktree"`, `run_in_background: true` and the approved `effort`. The prompt says: read the brief at <scratch path> first; then gives the item text and spec IDs, what's already on master that it builds on, and which parallel agents touch nearby files. Agents don't touch `docs/todo.md`, `docs/findings.md` or Built cells, and they number findings `F-NEW-n`: the brief says so.
- When something an agent depends on lands, tell it with SendMessage (for example "X is on master; rebase before your final commit"). When a pitfall shows up, add it to the brief so later agents get it.

## 3. Merge each finished item

0. Review first: start a review agent in the item's worktree (medium effort, high for large or orbital items). It reviews `git diff master...HEAD`, commits simple behaviour-preserving fixes, and reports larger issues. Put larger issues that need no design decision straight onto `docs/todo.md` (bugs under Fixes, clean-up under Other) without asking the user. Only spec questions go to the user, as findings.
1. In the agent's worktree: `git rebase master`. Conflicts should now be in code only. Resolve them with small Python scripts. Chain every step of a merge with `&&`: `set -e` is not reliable in the Bash tool (a failed `git merge --ff-only` once went on to delete the branch). Never chain `git add`/commit after a script that might have failed.
2. Run `touch crates/orbits/src/*.rs`, then `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.
3. From the repo root: `git merge --ff-only <branch>`. Then `git worktree remove --force <path>`, `git worktree prune` and `git branch -D <branch>`.
4. Apply the agent's **Bookkeeping** on master as one commit:
   - Tick the Built cells, and add any *Manual* note and its reason to the spec row.
   - Delete or edit its to-do item.
   - Delete the findings it resolves.
   - Add its new findings under To triage, giving each `F-NEW-n` the next free number, in order. Replace `F-NEW-n` in any code comments it cited.
   - Update the "next free number" line.
5. Check coverage: every spec row with an empty Built cell is named on `docs/todo.md` (ranges count).
6. Start the next item in that stream from the new master. Report to the user in a few lines: what landed, its findings with their real numbers, and what's running.

Typical conflicts:
- The end of a shared test file: both sides appended a test, so keep both and re-add the closing `}`.
- `tunables.rs`: both sides added fields, so keep both.
- Doc conflicts arise only when merging `origin/master` (section 4).

## 4. While agents run

- **New to-do items raised during the session** (from reviews, findings the user settles, or spec changes): slot each into the running plan at once and schedule it like the rest, telling the user its stream and effort. Don't wait to be asked.
- **"Pull latest design changes":** `git fetch`, then merge `origin/master`. Resolve doc conflicts this way:
  - `todo.md`: drop items that are now built.
  - Findings: keep both sides; drop findings origin settled.
  - Spec rows: take origin's text, and keep ✓ only where the meaning is unchanged. Run the coverage check, slot new items into the plan file, and tell running agents about changes that affect them.
- **Playtest:** follow CLAUDE.md "Seeing the game": rebuild master fresh (`touch src/main.rs` first), then copy the exe and `assets/` into `play/`. Agents never touch `play/`.
- **Observations:** record each as a finding at once (bug, spec wrong, spec gap or idea), under To triage. Ask before fixing even a clear bug. A design direction the user gives goes into the finding's disposition. Change the spec only when asked.
- **Progress:** list what's on master, what's running and what's next.
- **"Show the plan":** print the plan file's contents. Don't enter plan mode: that blocks running agents' commits. If plan mode does come on mid-run, commit agents' finished work for them once it's off.
- Push only when the user asks.

## 5. Finish

Stop the keep-awake task from section 2.

When the user asks, run a high-level code review agent once the scheduled work is merged. It applies easy behaviour-preserving fixes, raises findings for spec questions, and adds refactors to todo.md under Other.

## Pitfalls

- Worktrees share `CARGO_TARGET_DIR=<repo>/target`, because a full target is about 26 GB.
  - Every worktree's `orbits` crate has the same name in it, so a stale copy gives "unresolved imports" or doctest failures. Touch its sources and rerun.
  - `target/debug/orphan_worlds.exe` may be another worktree's build.
  - A parallel agent's build can replace the test binary mid-run, so a check may run another branch's tests (a failing test whose name isn't on the branch). Write test output to a file and fail the `&&` chain on a nonzero exit, so a failure stops the merge; rerun before trusting a failure or a pass.
- A worktree can start from an old commit, so the brief's first step syncs it with master.
- Game runs hang if a shot can't finish. `OW_SHOT` has a `timeout` key, and agents also wrap runs in their own timeout. If a game window lingers, find its parent `cargo run` and close it.
- Pushing or pulling `origin/master` moves master under a running merge check, so its fast-forward fails. If only `docs/` changed, rebase and fast-forward without rerunning the checks.
- On Windows, `git worktree remove` often says "Permission denied". Use `--force`, then prune. Leftover folders under `.claude/worktrees/` are harmless; delete them at the end of the session.

## Common rules for every item agent

You are building ONE item from `docs/todo.md` in the Orphan Worlds repo (Rust + Bevy 0.20), in your own git worktree. Other agents may work in parallel in other worktrees, so stay inside your item's scope.

- **First step:** your worktree may start from an old commit. Run `git merge --ff-only master` in it. If that fails, run `git reset --hard master` (your branch has no work yet).
- Read `CLAUDE.md`, `docs/README.md` and your item's spec rows (`docs/spec/`) before coding. Follow CLAUDE.md exactly:
  - Implement the spec IDs as written, with one automated test per requirement whose comment names the ID.
  - Tunables go in `Tunables` (`src/view/tunables.rs`) or `SimTunables`.
  - Comments must stand without the spec.
  - Check Bevy's API in the pinned source under `~/.cargo/registry`, not from memory.
- **Don't edit `docs/todo.md`, `docs/findings.md` or the spec's Built column.** The coordinator applies that bookkeeping on master after merging your branch, so parallel agents don't conflict over those files. Your branch holds code, tests and code comments only, plus any doc change your item explicitly asks for (for example a sentence the user agreed to add to a spec row, or CLAUDE.md). Never edit `docs/spec/` or `docs/ideas/` otherwise.
- Spec ambiguous or wrong on screen: make a sensible choice and write a finding for your report (see below), numbered `F-NEW-1`, `F-NEW-2`, …. The coordinator gives it its real number. If code comments must cite it, write `F-NEW-1` there too; the coordinator replaces it.
- **Build:** ALWAYS `export CARGO_TARGET_DIR=<REPO>/target`, because disk is tight; never build into the worktree's own `target/`. Run `touch crates/orbits/src/*.rs` before each build or test, since another worktree's build can leave a stale `orbits` copy. If cargo waits on a lock, another agent is building: wait.
- Bash: `cargo` isn't on PATH, so prefix commands with `export PATH="$HOME/.cargo/bin:$PATH" &&`. PowerShell also works.
- Before finishing: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` must all pass.
- Commit on your branch at green steps. End messages with the Co-Authored-By line from the system reminder. Don't push, merge or touch `master`. Run `git rebase master` before your final commit.
- Long edit scripts: write them to a scratch file and run with `PYTHONUTF8=1`. Never open a file for writing in the same expression that reads it.
- **Seeing the game:** use `OW_SHOT` (CLAUDE.md "Seeing the game").
  - ALWAYS run the game under a timeout (`timeout 120 cargo run`).
  - Never capture the desktop, and never touch `play/`.
  - "Access is denied" on build means the user is playing: skip screenshots.
- Bevy 0.20 pitfalls:
  - A node carries one `Tint`.
  - `Entity` order is by bits.
  - Any `Query<&Camera>` must filter by `Camera3d`/`OrbitCamera` or `IsDefaultUiCamera`.
  - Every UI `Node` has a `BackgroundColor`; recolour only the slot a part uses.
  - Text in narrow absolute containers needs `LineBreak::NoWrap`.
  - `add_plugins` takes at most 15 entries per tuple.
  - Headless apps using screen, camera or marks plugins need `SidebarPlugin`.
- **Final report (short):** branch, commits, what was built, anything marked *Manual* (with the reason to add to the spec row), and any trouble. If later parts build on your work, add a short API summary. End with a **Bookkeeping** section for the coordinator to apply:
  - **Tick:** the spec IDs now built.
  - **To-do:** delete the item, or the exact edit if part of it remains.
  - **Resolve:** findings your work settles, to delete (or the new text if one only changes).
  - **New findings:** each in full, in the format of `docs/findings.md` (`### F-NEW-n · Title`, then Observation, Affects · Type, Disposition lines).

# T009: Port Commands — Next And Do

## Goal

Port `Commands::next` and `Commands::do_` from `commands.cpp` to `src/commands/next.rs` and
`src/commands/do_cmd.rs`.

## Depends On

- T007

## Spec References

- `spec.main.md` → §5 Command Summary (next, do rows)
- Original C++: `src/commands.cpp` (`Commands::next`, `Commands::do_`)

## Phase: 3

## Critical: true

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY) — add `pub mod next; pub mod do_cmd;`
- `src/commands/next.rs` (CREATE)
- `src/commands/do_cmd.rs` (CREATE) — named `do_cmd`, not `do`, since `do` is a reserved
  keyword in Rust

## Implementation Steps

1. Port `next`: filter to pending tasks with all dependencies done (`all_deps_done` helper,
   shared in `commands/mod.rs`), prioritize critical path → phase → task number, read the
   task's T\*.md file (port `getFirstStep`/`extractGoal` helpers) and print
   `Depends on:`, `Goal:`, `First step:`, plus `Files:`/`Specs:` lines when those sections
   have list items — the C++ `next` prints all of these (see `commands.cpp:577-612`, and
   `tests/e2e/next.mjs` asserts `Files:`/`Specs:`), print "All tasks blocked or complete"
   if none available. A missing T\*.md silently skips the file-derived detail (no warning).
2. Port `do_`: validate task exists, check dependencies — unmet deps without `--force` are
   an **error**: `Unmet dependencies: <id> (<status>), .... Use --force to proceed`, exit 1
   (NOT a warning — see AGENTS.md known-conflicts #5), transition pending → in_progress,
   print the status transition and, if the T\*.md exists, `Now reading <path>...` with
   `Goal:`/`First step:`; "Already in_progress"/"Already done" are errors (exit 1) if not
   currently pending.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] `tests/e2e/next.mjs` (copied from the C++ repo at T015) and `tests/e2e/do.mjs`
      (author in *this* repo's `tests/e2e/`) pass — **verification deferred to T015**
      (AGENTS.md Locked decision §7). Leave this box unchecked at T009 with a Notes line
      `E2E deferred to T015 (per AGENTS.md)`
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

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
   shared in `commands/mod.rs`), prioritize critical path → phase → task number, read goal
   and first implementation step from the task's T*.md file (port `getFirstStep` helper),
   print "All tasks blocked or complete" if none available.
2. Port `do_`: validate task exists, check dependencies (warn unless `--force`), transition
   pending → in_progress, print goal + first step from the T*.md file, "Already in_progress"
   / "Already done" short-circuits if not currently pending.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] `tests/e2e/next.mjs` (already exists in the C++ repo) and `tests/e2e/do.mjs` (create
      if missing) pass once wired in T014
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

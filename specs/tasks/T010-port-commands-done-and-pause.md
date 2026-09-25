# T010: Port Commands — Done And Pause

## Goal

Port `Commands::done` and `Commands::pause` from `commands.cpp` to `src/commands/done.rs`
and `src/commands/pause.rs`.

## Depends On

- T009

## Spec References

- `spec.main.md` → §5 Command Summary (done, pause rows)
- Original C++: `src/commands.cpp` (`Commands::done`, `Commands::pause`)

## Phase: 3

## Critical: false

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY) — add `pub mod done; pub mod pause;`
- `src/commands/done.rs` (CREATE)
- `src/commands/pause.rs` (CREATE)

## Implementation Steps

1. Port `done`: validate task exists, transition to done (allowed from pending *or*
   in_progress), scan for tasks newly unblocked (all dependencies now done) and print them,
   "Already done" short-circuit — an **error** (`error: Task T003 already done`, exit 1).
2. Port `pause`: validate task exists, transition → pending — the C++ version accepts any
   non-pending status (even `done`), rejecting only already-pending — "Already pending"
   short-circuit is an **error** (`error: Task T003 already pending`, exit 1).

## Acceptance Criteria

- [x] `cargo build` succeeds
- [x] `tests/e2e/done.mjs` and `tests/e2e/pause.mjs` pass — **verification deferred to
      T015** (AGENTS.md Locked decision §7). Author both files in *this* repo's
      `tests/e2e/` (never in `../taskpad`); leave this box unchecked at T010 with a Notes
      line `E2E deferred to T015 (per AGENTS.md)`
- [x] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

- E2E deferred to T015 (per AGENTS.md)

- [2026-09-25 19:34] Ported commands done+pause
E2E verified at T015

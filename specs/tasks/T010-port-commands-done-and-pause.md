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

1. Port `done`: validate task exists, transition to done, scan for tasks newly unblocked
   (all dependencies now done) and print them, "Already done" short-circuit.
2. Port `pause`: validate task exists, transition in_progress → pending, "Already pending"
   short-circuit.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] `tests/e2e/done.mjs` and `tests/e2e/pause.mjs` pass once wired in T014 (create if
      missing in the C++ repo's `tests/e2e/`)
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

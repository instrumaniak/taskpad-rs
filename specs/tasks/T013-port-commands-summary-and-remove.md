# T013: Port Commands — Summary And Remove

## Goal

Port `Commands::summary` and `Commands::remove` from `commands.cpp` to
`src/commands/summary.rs` and `src/commands/remove.rs`.

## Depends On

- T007

## Spec References

- `spec.main.md` → §5 Command Summary (summary, remove rows)
- Original C++: `src/commands.cpp` (`Commands::summary`, `Commands::remove`)

## Phase: 3

## Critical: false

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY) — add `pub mod summary; pub mod remove;`
- `src/commands/summary.rs` (CREATE)
- `src/commands/remove.rs` (CREATE)

## Implementation Steps

1. Port `summary`: totals + percentages (pending/in_progress/done), per-phase done/total
   breakdown in phase order, critical path with its own done/in_progress/pending counts.
2. Port `remove`: validate task exists, warn if other tasks depend on it, prompt `[y/N]`
   (read from stdin) unless `--force`, remove the `status.yaml` entry and from
   `critical_path` if present, `--all` additionally deletes the `.md` file via
   `storage::task_file_path` + `std::fs::remove_file`.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] `tests/e2e/summary.mjs` (create if missing) and `tests/e2e/remove.mjs` (already exists
      in the C++ repo) pass once wired in T014, including the interactive-prompt case via
      `runInteractive`
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

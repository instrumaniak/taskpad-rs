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

- [x] `cargo build` succeeds
- [ ] `tests/e2e/summary.mjs` (author in *this* repo's `tests/e2e/`) and
      `tests/e2e/remove.mjs` (copied from the C++ repo at T015) pass, including the
      interactive-prompt case via `runInteractive` — **verification deferred to T015**
      (AGENTS.md Locked decision §7). Leave this box unchecked at T013 with a Notes line
      `E2E deferred to T015 (per AGENTS.md)`
- [x] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

E2E deferred to T015 (per AGENTS.md)

- [2026-09-25 20:15] Ported summary.rs (totals/pct, phase breakdown, critical path) and remove.rs (validation, dependent warning, y/N prompt, --all/--force) with unit tests and authored tests/e2e/summary.mjs

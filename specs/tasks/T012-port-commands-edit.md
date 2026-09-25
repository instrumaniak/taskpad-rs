# T012: Port Commands — Edit

## Goal

Port `Commands::edit` from `commands.cpp` to `src/commands/edit.rs` — the largest single
command after `import`, handling both task-level and project-level metadata edits.

## Depends On

- T007
- T006

## Spec References

- `spec.main.md` → §5 Command Summary (edit row)
- Original C++: `src/commands.cpp` (`Commands::edit`)

## Phase: 3

## Critical: false

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY) — add `pub mod edit;`
- `src/commands/edit.rs` (CREATE)

## Implementation Steps

1. Port the task-level branch: given a task ID, apply whichever of `--status`, `--phase`,
   `--critical`/`--no-critical`, `--depends` were supplied, validating circular dependencies
   if `--depends` changed, and print a confirmation line per changed field.
2. Port the project-level branch: no task ID given — apply `--phases` (parse
   `"N:name,N:name"` into the `BTreeMap`) and/or `--critical-path` (parse comma-separated
   task IDs, validating each exists).
3. Preserve the C++ version's flag-combination semantics exactly — this command's argument
   surface (via `clap` in T014) needs `--critical`/`--no-critical` as two separate boolean
   flags, matching the CLI11 version, not a single tri-state flag.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] `tests/e2e/edit.mjs` covering both task-level and project-level flag combinations
      passes — **verification deferred to T015** (AGENTS.md Locked decision §7). Author it
      in *this* repo's `tests/e2e/` (never in `../taskpad`); leave this box unchecked at
      T012 with a Notes line `E2E deferred to T015 (per AGENTS.md)`
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

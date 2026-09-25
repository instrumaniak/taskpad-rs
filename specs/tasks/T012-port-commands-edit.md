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
- [ ] `tests/e2e/edit.mjs` passes once wired in T014 (create if missing in the C++ repo's
      `tests/e2e/`) covering both task-level and project-level flag combinations
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

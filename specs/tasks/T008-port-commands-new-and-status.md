# T008: Port Commands — New And Status

## Goal

Port `Commands::new_` and `Commands::status` from `commands.cpp` to
`src/commands/new.rs` and `src/commands/status.rs`.

## Depends On

- T007

## Spec References

- `spec.main.md` → §5 Command Summary (new, status rows)
- Original C++: `src/commands.cpp` (`Commands::new_`, `Commands::status`)

## Phase: 3

## Critical: false

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY) — add `pub mod new; pub mod status;`
- `src/commands/new.rs` (CREATE)
- `src/commands/status.rs` (CREATE)

## Implementation Steps

1. Port `new_`: auto-increment task ID via `find_next_task_id` (port from `commands.cpp`'s
   file-scope helper into `commands/mod.rs` since `status`/`next`/`done` etc. need similar
   task-map scanning helpers), kebab-case the name for the filename, write the T*.md
   template via `storage::write_task_file`, add a `status.yaml` entry, validate no circular
   dependencies if `--depends` given, handle duplicate-name warning and empty-name error.
2. Port `status`: group tasks by phase (ascending, using the `BTreeMap<i32, String>` from
   `models::ProjectConfig`), print `[status]` per task, mark `← next (dependencies met)` for
   the task `next` would pick and `← blocked by TXXX` for tasks with an unmet dependency
   (name column padded per the C++ code: `max(1, 20 - name.len())` **bytes** after the
   name), print the progress summary line.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] `tests/e2e/new.mjs` and `tests/e2e/status.mjs` pass — **verification deferred to
      T015** (AGENTS.md Locked decision §7). Author both files in *this* repo's
      `tests/e2e/` (never in `../taskpad`); leave this box unchecked at T008 with a Notes
      line `E2E deferred to T015 (per AGENTS.md)`
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

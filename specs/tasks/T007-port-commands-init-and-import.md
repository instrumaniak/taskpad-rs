# T007: Port Commands — Init And Import

## Goal

Port `Commands::init` and `Commands::import_` from `commands.cpp` to
`src/commands/init.rs` and `src/commands/import.rs`. This is the first command task and the
critical-path gate for every other command task — it establishes the pattern (function
signature shape, error-to-stderr convention) the rest of `commands/*.rs` follows.

## Depends On

- T004
- T005
- T006

## Spec References

- `spec.main.md` → §5 Command Summary (init, import rows), §6 Error Handling
- Original C++: `src/commands.cpp` (`Commands::init`, `Commands::import_`), `src/commands.h`

## Phase: 3

## Critical: true

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY) — add `pub mod init; pub mod import;` and any shared
  helpers these two commands need (check `commands.cpp`'s file-scope statics — most are
  needed by later commands too, e.g. `find_next_task_id`, but only add what's actually used
  so far to avoid dead-code warnings until later tasks land)
- `src/commands/init.rs` (CREATE)
- `src/commands/import.rs` (CREATE)

## Implementation Steps

1. Port `init`: create `.taskpad` via `storage::create_config`, "Already initialized" error
   if one exists already.
2. Port `import_`: scan `task-dir` for `T[0-9]{3}-*.md`, parse `## Status:` (optional,
   defaults to pending)/`## Depends On`/`## Phase:`/`## Critical:` from each via
   `utils::extract_*` functions and a depends-extraction helper (port `extractDepends` from
   `commands.cpp` — it's not in `utils.cpp`, it lives in `commands.cpp` itself), build a
   `StatusFile`, validate no circular dependencies across the discovered set (missing deps
   and cycles are collected and reported as `warning:` lines on stderr — status.yaml is
   still written and the exit code stays 0, per `import.mjs` and AGENTS.md known-conflicts
   #6), write via `storage::write_status_file`. Respect `--force` for overwriting an
   existing `status.yaml`.
3. Establish the command-function signature convention here for the rest of the port:
   `pub fn run(tasks_dir: &str, ...) -> Result<()>`, printing success output directly to
   stdout inside the function and letting `main.rs` handle the `Err` → `error: {msg}` stderr
   conversion (see `spec.main.md` §8 Error Handling Strategy).

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Unit tests cover the pure parts (e.g. dependency-extraction from a Notes-section-style
      string) where practical without a real filesystem
- [ ] `tests/e2e/init.mjs` and `tests/e2e/import.mjs` pass against the release build —
      **verification deferred to T015** (AGENTS.md Locked decision §7: the suite can't run
      before T014 wiring + T015 `helpers.mjs` rewire). As part of this task, author
      `init.mjs` in *this* repo's `tests/e2e/` if missing; `import.mjs` is copied from the
      C++ repo at T015. Leave this box unchecked at T007 with a Notes line
      `E2E deferred to T015 (per AGENTS.md)`
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

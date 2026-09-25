# T019: Fix architecture layering and testability

## Goal

Fix the layering and testability violations the code review found: a circular dependency
between `storage` and `utils`, a purely stringly-typed error type that forced every message to
be retyped at each call site, a wall-clock `date` subprocess making `storage` impure and
untestable, blocking stdin I/O buried inside a library function, an eight-argument `edit::run`
needing a `too_many_arguments` suppression, and five blanket `#![allow(dead_code)]` headers
that defeated the `-D warnings` gate the project relies on.

## Depends On

(None)

## Phase:

6

## Critical: false

## Spec References

- `spec.main.md` → §2 Architecture (intended dependency direction
  `commands → {storage, utils}`, `storage → utils`), §3 Data Model (`TaskpadError`),
  §6 Error Handling, §7 (explicit types for the public API; no panics),
  §8 (single exit point; command modules never call `process::exit`)
- `AGENTS.md` → Locked decision §3 (`Task.id` is `#[serde(skip)]`, re-threaded by
  `storage::read_status_file`)
- Original C++: `src/storage.cpp` (`readTaskDir`), `src/utils.cpp` (`resolveTaskDir`),
  `src/commands.h` (`Commands::edit` signature)

## Files to Create/Modify

- `src/utils.rs` (MODIFY) — `resolve_task_dir` removed; module doc states the leaf invariant
- `src/storage.rs` (MODIFY) — `resolve_task_dir` added; `append_log` takes an injected
  timestamp; blanket allow removed
- `src/models.rs` (MODIFY) — `TaskpadError` constructors; blanket allow removed
- `src/validator.rs` (MODIFY) — blanket allow removed
- `src/commands/mod.rs` (MODIFY) — `load_status` calls `storage::resolve_task_dir`; helpers
  delegate to the new constructors; stale module doc fixed
- `src/commands/log.rs` (MODIFY) — supplies the timestamp
- `src/commands/remove.rs` (MODIFY) — `run` takes an injected confirmation closure
- `src/commands/edit.rs` (MODIFY) — `EditArgs` struct replaces eight positional parameters
- `src/commands/import.rs`, `new.rs` (MODIFY) — updated `resolve_task_dir` call sites
- `src/main.rs` (MODIFY) — builds `EditArgs`; reads stdin and passes the decision

## Implementation Steps

1. Move `resolve_task_dir` from `utils.rs` into `src/storage.rs` beside `read_task_dir`, so
   `utils` no longer depends on `storage` and the intended DAG holds. Preserve the fallback
   semantics exactly: any `read_task_dir` failure yields `"specs/tasks"`.
2. Add `TaskpadError` constructors (`invalid_id`, `task_not_found`, `no_status_yaml`,
   `cannot_write`, `task_file_not_found`) with byte-identical messages, and route the
   `commands/mod.rs` helpers through them. Do not change `Display` output.
3. Change `storage::append_log` to accept the timestamp as a parameter; `log::run` supplies
   `utils::current_timestamp()`. Update the `storage` tests to pass fixed timestamps and
   assert exact strings.
4. Change `remove::run` to take a `confirm: impl FnOnce(&str) -> bool`; move the `read_line`
   into `main.rs`, preserving the non-tty EOF-declines behaviour. Keep `confirmed()` pure and
   testable, and add an injected-decline test.
5. Introduce `EditArgs` and reduce `edit::run` to `(tasks_dir, id, args)`, removing the
   `#[allow(clippy::too_many_arguments)]`. Preserve the `None`-means-absent semantics that
   `main.rs` previously encoded by collapsing `Option<String>` to `""`.
6. Delete the five blanket `#![allow(dead_code)]` headers, fix the warnings that surface
   (scoping targeted allows with comments where the code is genuinely test-only), and correct
   the `commands/mod.rs` module doc, which claimed the module re-exported only four commands
   when it exports all thirteen.

## Constraints

- `resolve_task_dir`'s swallow-everything fallback is deliberate C++ parity (AGENTS.md
  conflicts table #10) — move it, do not "fix" it.
- No user-visible byte, message, or exit-code changes anywhere.
- T017 and T018's fixes must survive untouched, including `remove`'s `T001-.md` formula and
  the shared `commands/mod.rs` helpers.
- No new dependencies; no `unsafe`; no `unwrap()`/`expect()` outside `#[cfg(test)]`.

## Acceptance Criteria

- [x] `src/utils.rs` contains no code reference to `storage` (only doc comments mention it),
      so the `storage ↔ utils` cycle is broken
- [x] `resolve_task_dir` lives in `src/storage.rs` next to `read_task_dir` with identical
      fallback behaviour, and has a unit test
- [x] `TaskpadError` exposes the five constructors; displayed messages are unchanged
- [x] `append_log` takes the timestamp as a parameter, `log::run` supplies it, and the
      `storage` log tests assert exact strings against fixed timestamps
- [x] `remove::run` takes an injected confirmation closure; `stdin().read_line` no longer
      appears in `src/commands/`, and both accept and decline paths are unit-tested without a
      TTY
- [x] `EditArgs` exists, `edit::run` takes it, and the `too_many_arguments` suppression is
      gone
- [x] Zero blanket `#![allow(dead_code)]` headers remain; the `commands/mod.rs` module doc
      lists all thirteen commands
- [x] `cargo build`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` all pass
      (140/140 at task close), and the E2E suite passes 123/123

## Notes

(filled in during/after implementation)

- The crew additionally built the release binary and ran the E2E suite to confirm the
  structural moves changed no asserted output: 123/123 pass.
- `TaskpadError` deliberately keeps its single `Message(String)` variant. Converting to
  structured variants would change the surface tests match on, so the constructors were added
  as a first, output-neutral step instead.
- `validator::validate_task_exists` remains test-only and carries a scoped `allow(dead_code)`
  with a comment rather than being deleted: it documents the existence check the commands
  now perform through the shared constructor.
- Scope held: `Cargo.toml`, `README.md`, `tests/`, and `specs/` were untouched.

- [2026-09-26 03:01] Architecture: broke storage<->utils cycle (resolve_task_dir into storage), TaskpadError constructors, injected clock into append_log, lifted stdin out of remove via confirm closure, EditArgs struct, removed 5 blanket allow(dead_code)

# T011: Port Commands — Deps And Log

## Goal

Port `Commands::deps` and `Commands::log` from `commands.cpp` to `src/commands/deps.rs` and
`src/commands/log.rs`.

## Depends On

- T007

## Spec References

- `spec.main.md` → §5 Command Summary (deps, log rows)
- Original C++: `src/commands.cpp` (`Commands::deps`, `Commands::log`)

## Phase: 3

## Critical: false

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY) — add `pub mod deps; pub mod log;`
- `src/commands/deps.rs` (CREATE)
- `src/commands/log.rs` (CREATE) — named `log`, shadowing nothing problematic, but note
  `log` is also the name of a very common Rust logging crate; if that crate is ever added
  as a dependency later, this module may need a rename (e.g. `log_cmd`) to avoid ambiguity —
  not a concern for this port, since no logging crate is in the dependency list (§8), but
  worth a one-line comment in the module.
- `src/commands/mod.rs` — re-export as needed to avoid a name clash at the call site in
  `cli.rs` if one arises

## Implementation Steps

1. Port `deps`: show what the task depends on (with ✓/✗ per dependency based on that
   dependency's status) and what depends on it (reverse lookup over the whole task map).
2. Port `log`: validate task exists, call `storage::append_log` with a timestamped entry,
   empty-message error.

## Acceptance Criteria

- [x] `cargo build` succeeds
- [x] `tests/e2e/deps.mjs` and `tests/e2e/log.mjs` pass — **verification deferred to T015**
      (AGENTS.md Locked decision §7). Author both files in *this* repo's `tests/e2e/`
      (never in `../taskpad`); leave this box unchecked at T011 with a Notes line
      `E2E deferred to T015 (per AGENTS.md)`
- [x] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

E2E deferred to T015 (per AGENTS.md)

- [2026-09-25 20:07] Port Commands::deps and Commands::log to src/commands/deps.rs and log.rs with unit tests and E2E test files authored for T015
E2E verified at T015

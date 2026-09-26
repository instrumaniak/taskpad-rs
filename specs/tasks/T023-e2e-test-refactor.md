# T023: e2e test refactor

## Goal

Replace all 14 Node.js `.mjs` E2E test files with pure Rust integration tests. The tests call the same binary via `std::process::Command` but are written in idiomatic Rust using `tempfile::tempdir()`, `env!("CARGO_BIN_EXE_taskpad")`, and shared helpers in `tests/common/mod.rs`.

## Depends On

- T022 (code cleanup) — must pass first

## Phase:

6

## Critical:

false

## Spec References

- Locked decision §7: E2E AC deferral for T007–T013
- AGENTS.md: Compatibility mandate (drop-in replacement for C++)
- TRPL Ch. 11-3: Test organization, `tests/common/mod.rs` pattern

## Files to Create/Modify

- `tests/common/mod.rs` — NEW: shared test helpers (`Project` struct, `run()`, `create_project()`, `to_kebab()`, `task_defaults()`, YAML builders)
- `tests/e2e/mod.rs` — NEW: test module declarations
- `tests/e2e/init.rs` — REPLACE `tests/e2e/init.mjs`
- `tests/e2e/new.rs` — REPLACE `tests/e2e/new.mjs`
- `tests/e2e/status.rs` — REPLACE `tests/e2e/status.mjs`
- `tests/e2e/next.rs` — REPLACE `tests/e2e/next.mjs`
- `tests/e2e/do.rs` — REPLACE `tests/e2e/do.mjs`
- `tests/e2e/done.rs` — REPLACE `tests/e2e/done.mjs`
- `tests/e2e/pause.rs` — REPLACE `tests/e2e/pause.mjs`
- `tests/e2e/deps.rs` — REPLACE `tests/e2e/deps.mjs`
- `tests/e2e/log.rs` — REPLACE `tests/e2e/log.mjs`
- `tests/e2e/edit.rs` — REPLACE `tests/e2e/edit.mjs`
- `tests/e2e/summary.rs` — REPLACE `tests/e2e/summary.mjs`
- `tests/e2e/remove.rs` — REPLACE `tests/e2e/remove.mjs`
- `tests/e2e/import.rs` — REPLACE `tests/e2e/import.mjs`
- Delete: `tests/e2e/*.mjs` (all 14), `tests/helpers.mjs`

## Implementation Steps

1. Create `tests/common/mod.rs` with:
   - `Project` struct wrapping `tempfile::TempDir`, with `run()`, `run_interactive()`, `read_file()`, `exists()`, `destroy()` methods
   - `task_defaults(id)`, `to_kebab(s)`, `task_filename(id, name)` helpers
   - `md_content(id, name, depends)`, `md_import_content(id, name, opts)` builders
   - `generate_yaml(tasks)` helper
   - Binary path via `env!("CARGO_BIN_EXE_taskpad")`
2. Create `tests/e2e/mod.rs` — declares all 14 submodules
3. Create each `tests/e2e/<name>.rs` by porting the corresponding `.mjs` test:
   - `node:test` `describe/it/before/after` → `#[test]` with `mod` for grouped tests
   - `assert.match(r.stdout, /regex/)` → `assert!(stdout.contains(...))` or regex via `assert_regex`
   - `p.run('cmd', ...)` → `project.run(&["cmd", ...])`
   - `p.runInteractive(...)` → `project.run_interactive("input\n", &[...])`
   - `p.destroy()` → drop `Project` (auto-cleanup via `Drop`)
4. Delete all 14 `.mjs` files and `helpers.mjs`
5. `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — all must pass
6. `cargo fmt`

## Constraints

- Behavior must not change — tests must verify the same behavior as the `.mjs` tests
- No new dependencies beyond `tempfile` (already a dev-dep) and optionally `assert_cmd` or `regex` if needed
- Each test uses its own temp directory — no shared state between tests
- Tests must be safe for parallel execution (`cargo test` default)
- Must preserve the E2E AC deferral pattern (T007–T013 deferred boxes)

## Acceptance Criteria

- [ ] `cargo build` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes — all Rust E2E tests green
- [ ] All 14 `.mjs` files deleted, replaced by `.rs` files
- [ ] `tests/common/mod.rs` shared helpers work across all test files
- [ ] `tests/e2e/mod.rs` declares all submodules
- [ ] Each `.rs` test file mirrors its corresponding `.mjs` test behavior

## Notes

- Uses `tempfile::tempdir()` (already a dev-dependency per Locked decision §1)
- Binary discovery via `env!("CARGO_BIN_EXE_taskpad")` — Cargo sets this automatically for integration tests
- The `#[cfg(test)]` modules in `src/` remain unchanged; this is purely adding integration tests
- E2E deferred boxes (T007–T013) per Locked decision §7 remain deferred to T015

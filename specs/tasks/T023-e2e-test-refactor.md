# T023: e2e test refactor

## Goal

Replace the 13 Node.js `.mjs` E2E test files with Rust integration tests that drive the
same binary through `std::process::Command`, using `tempfile::TempDir` for isolation and a
shared helper module in `tests/common/mod.rs`. The assertions are a **line-by-line
transcription** of the existing `.mjs` suite — same fixtures, same literal strings, same
exit codes — so the suite's value as the port's compatibility check is preserved.

Layout follows the convention used by ripgrep (`tests/tests.rs` declaring one `mod` per
feature area plus a shared helper module) and fd (`tests/testenv/mod.rs` + `mod testenv;`):
**one test target, one module per command area, std-only assertions.** No new
dev-dependencies — `spec.main.md` §8's crate list is closed (Locked decision §1).

## Depends On

- T022 (code cleanup) — must pass first

## Phase:

6

## Critical:

false

## Spec References

- `specs/spec.testing.md` §3 — the E2E tier this task replaces (rewritten by this task)
- AGENTS.md: Compatibility mandate (drop-in replacement for the C++ binary)
- AGENTS.md Locked decision §1 — the crate list is closed: no `regex`, no `assert_cmd`
- TRPL Ch. 11-3 — `tests/common/mod.rs` shared-module pattern (also fd's `tests/testenv/`)
- Ground truth for every assertion: this repo's `tests/e2e/*.mjs` and `tests/helpers.mjs`
  as they stand at the start of this task (git history), plus `../taskpad/tests/e2e/*.mjs`
  (read-only) for the three files that came from the C++ repo

## Files to Create/Modify

- `tests/common/mod.rs` — NEW: shared E2E helpers (fd's `testenv` pattern)
- `tests/e2e/main.rs` — NEW: the test target root. **Must be `main.rs`, not `mod.rs`** —
  Cargo only auto-discovers `tests/*.rs` and `tests/*/main.rs`, and a `tests/e2e/mod.rs`
  is silently ignored (0 tests, no warning). Contents: the crate-level clippy `allow`,
  `#[path = "../common/mod.rs"] mod common;`, and the 13 module declarations.
- `tests/e2e/init.rs` — REPLACE `tests/e2e/init.mjs`
- `tests/e2e/new.rs` — REPLACE `tests/e2e/new.mjs`
- `tests/e2e/status.rs` — REPLACE `tests/e2e/status.mjs`
- `tests/e2e/next.rs` — REPLACE `tests/e2e/next.mjs`
- `tests/e2e/do_cmd.rs` — REPLACE `tests/e2e/do.mjs`. **Not `do.rs`** — `do` is a Rust
  keyword, so `mod do;` is a compile error. Matches `src/commands/do_cmd.rs`.
- `tests/e2e/done.rs` — REPLACE `tests/e2e/done.mjs`
- `tests/e2e/pause.rs` — REPLACE `tests/e2e/pause.mjs`
- `tests/e2e/deps.rs` — REPLACE `tests/e2e/deps.mjs`
- `tests/e2e/log.rs` — REPLACE `tests/e2e/log.mjs`
- `tests/e2e/edit.rs` — REPLACE `tests/e2e/edit.mjs`
- `tests/e2e/summary.rs` — REPLACE `tests/e2e/summary.mjs`
- `tests/e2e/remove.rs` — REPLACE `tests/e2e/remove.mjs`
- `tests/e2e/import.rs` — REPLACE `tests/e2e/import.mjs`
- Delete: `tests/e2e/*.mjs` (all 13) and `tests/helpers.mjs`
- `specs/spec.main.md` — MODIFY: lines ~367 (E2E assertion paths), ~539 (crate layout),
  ~557 (build/test commands), ~576 (§9 Testing), and the §8 crate-dependency table
  (record that the E2E tier adds no dev-dependency beyond `tempfile`)
- `specs/spec.testing.md` — MODIFY: §1 overview table + run command, §3 (retitled and
  rewritten for the Rust tier), §4 running tests, §5 CI
- `AGENTS.md` — MODIFY: the task-loop step 5 E2E verification bullet only
- `CHANGELOG.md` — MODIFY: the line advertising the `node --test` E2E suite

**No `Cargo.toml` change.** `tempfile` is already a dev-dependency (Locked decision §1).

## Implementation Steps

1. Create `tests/common/mod.rs` (module-level `#![allow(dead_code)]` — a shared helper
   library's API is wider than any single consumer, and an unused helper is otherwise a
   `dead_code` **error** under the `-D warnings` verification step):
   - `pub struct Project { _tmp: TempDir, root: PathBuf }` — `TempDir` drop replaces the
     `.mjs` `destroy()`; no explicit teardown method.
   - `run(&self, args: &[&str]) -> Output` via `Command::output()` with
     `.current_dir(&self.root)`. Note `output()` gives the child a **null stdin**, so a
     prompt in a non-interactive call gets EOF instead of hanging — a latent hang in the
     `.mjs` version, which inherited node's stdin.
   - `run_interactive(&self, input: &str, args: &[&str]) -> Output` — `Stdio::piped()`,
     write `input`, `wait_with_output()`.
   - `pub struct Output { stdout: String, stderr: String, code: Option<i32> }` — both
     streams `trim()`ed, mirroring `helpers.mjs`'s object so assertions transcribe 1:1.
   - FS accessors the suite actually needs: `path()` (the `.mjs` `resolve()`), `read()`,
     `write()` (`summary.mjs` overwrites `.taskpad` with malformed bytes), `exists()`,
     `read_dir_sorted()` (**must sort** — `fs::read_dir` order is arbitrary), `remove()`
     (`init.mjs` unlinks `.taskpad`), `set_mode()` (`new.mjs` chmods the task dir to 0o555).
   - Resolve the binary in this order: `TASKPAD_BIN`, runtime `CARGO_BIN_EXE_taskpad`, then
     compile-time `env!("CARGO_BIN_EXE_taskpad")`. Canonicalize a relative `TASKPAD_BIN`
     while the test process is still in Cargo's package-root working directory, before
     child commands change directory to the temp project. This makes
     `TASKPAD_BIN=../taskpad/taskpad cargo test --test e2e` work as documented and keeps
     the C++ parity cross-check reproducible.
   - Fixture builders: `task_defaults(id)`, `to_kebab(s)`, `task_filename(id, name)`,
     `md_content(id, name, depends)`, `md_import_content(id, name, opts)`, `generate_yaml(tasks)`.
   - Two distinct fixture types, not one: `TaskSpec` for the `tasks`/`statusYaml` modes
     (`phase: u32` defaulting to 0) and `ImportSpec` for the `importTasks` mode, where
     `phase`/`critical`/`status` must be `Option<_>` because `helpers.mjs:66-74` emits the
     `## Phase:` / `## Critical:` / `## Status:` lines only when the value is defined.
     Also support the `tasks: ['T001', { id: 'T002', depends: [...] }]` shorthand the
     suite mixes (`remove.mjs:9,95-96`).
   - Preserve the semantics of all 186 `assert.match` / `assert.doesNotMatch` calls without
     a regex crate. Use literal substring assertions for literal patterns, exact whole-line
     assertions for anchored patterns, ordered same-line fragment assertions for `.*`, and
     cross-line ordered-fragment assertions for unbounded `[\s\S]*`. Add a bounded
     cross-line helper for `[\s\S]{0,n}` that enforces the maximum gap; do not replace it
     with an unbounded search. Handle the remaining pattern forms explicitly with std-only
     predicates: ASCII case-insensitive matching for `/i`, ASCII digit validation for `\d+`
     and the log timestamp, non-whitespace checks for `\S`, and exact literal-line or
     line-prefix checks for anchored and dynamically constructed patterns such as the
     separator. Negated patterns must negate the equivalent full
     predicate, not just a substring check. Keep these helpers in `tests/common/mod.rs` and
     use plain `assert!` / `assert_eq!` for the 118 `assert.equal` calls.
2. Create `tests/e2e/main.rs`:
   - `#![allow(clippy::unwrap_used, clippy::expect_used)]` — `Cargo.toml`'s
     `[lints.clippy] deny` **does** apply to integration-test targets, so without this the
     helpers' `expect` calls fail the clippy verification step. This is the integration-test
     counterpart of `src/main.rs`'s `#![cfg_attr(test, allow(...))]`, and matches the
     "no `unwrap()` outside test code" rule.
   - `#[path = "../common/mod.rs"] mod common;` — required; a plain `mod common;` looks for
     `tests/e2e/common.rs` and will not find `tests/common/mod.rs`.
   - `mod` declarations for the 13 command areas (`do_cmd`, not `do`).
3. Create each `tests/e2e/<name>.rs`, porting the corresponding `.mjs` file. Mapping rules:
   - **Split** an `it` into its own `#[test]` with a fresh `Project` when it only reads an
     unmutated fixture (all 3 `it`s in `status.mjs` "basic", all 3 in `deps.mjs`, all 4 in
     `summary.mjs` "happy path", both in `next.mjs`). Rust aborts a test at its first
     failed assert, so merging these would lose the per-case failure isolation `node:test`
     gave us.
   - **Merge** into a single `#[test]` only where a later `it` depends on an earlier one's
     side effect: `init.mjs` (unlink → init → re-init), `new.mjs` "basic" (T002 then T003
     numbering), `done.mjs` "basic", the 6 multi-`it` `import.mjs` describes, and the 13
     multi-`it` `edit.mjs` describes.
   - `describe`/`before` → a comment header on each test (`// taskpad <cmd> — <describe name>`)
     plus the fixture setup at the top of the test body. No `mod`-per-group nesting: it
     buys nothing and obscures the test names.
   - In the `new.mjs` "unwritable directory" test, restore the 0o755 mode **immediately
     after** `run()` and before any assert that can panic — `TempDir`'s `Drop` ignores
     deletion errors, so a panicking assert between chmod and restore would silently leak
     the directory.
   - Expected result: roughly 100–110 `#[test]` fns (from 80 `describe` / 123 `it` blocks).
4. Delete all 13 `.mjs` files and `tests/helpers.mjs`. They stay in git history, and
   `../taskpad/tests/e2e/` remains the read-only C++ reference.
5. Update the docs listed under **Files to Create/Modify** (`spec.main.md`,
   `spec.testing.md`, `AGENTS.md` task-loop step 5, `CHANGELOG.md`).
6. `cargo build`, then `cargo clippy --all-targets -- -D warnings`, then `cargo test` —
   all must pass. The E2E tier is now part of `cargo test`, so there is no separate
   `node --test` step and no `cargo build --release` prerequisite.
7. `cargo fmt`.

## Constraints

- **Behavior must not change.** The ported tests must assert exactly what the `.mjs` tests
  assert — same fixtures, same literal strings, same exit codes. If a ported assertion
  needs weakening to pass, that is a bug in the Rust port, not a test fix
  (`spec.testing.md` §1).
- **No new dependencies.** No `regex`, no `assert_cmd`, no `predicates` — `spec.main.md` §8's
  crate list is closed (Locked decision §1). `tempfile` is the only dev-dependency. Note that
  `assert_cmd` is the ecosystem default for CLI E2E, but its `.assert()` runs the binary
  eagerly and its predicates match untrimmed bytes — both wrong for a suite that runs once,
  asserts many times, and compares trimmed output. ripgrep, fd and fulgur-cli all hand-roll
  the same helpers instead.
- Each test owns its temp directory; no shared state between tests; safe for `cargo test`'s
  default parallel execution.
- Assertions must keep the `.mjs` suite's *trim-then-compare* semantics.

## Acceptance Criteria

- [ ] `tests/e2e/main.rs` exists and is the test target root (verified: `cargo test` lists
      the `e2e` target and runs the ported tests — a `tests/e2e/mod.rs` is silently ignored)
- [ ] `tests/common/mod.rs` provides the shared helpers and is reachable from the test
      target via `#[path]`; no duplicated helper logic across the 13 test files
- [ ] All 13 `.mjs` files and `tests/helpers.mjs` are deleted, replaced by `.rs` files
- [ ] Every one of the 13 `.mjs` files has a corresponding `.rs` module (`do.mjs` →
      `do_cmd.rs`), with no coverage dropped: all 123 `it` blocks' assertions are present
- [ ] Every test creates its own temp dir; `cargo test` is green when run in parallel
- [ ] `cargo build` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes — the 179 pre-existing unit tests plus the new E2E tier
- [ ] `cargo fmt` produces no diff
- [ ] `specs/spec.main.md`, `specs/spec.testing.md`, `AGENTS.md` (task-loop step 5) and
      `CHANGELOG.md` no longer reference the Node.js E2E tier
- [ ] The C++ parity cross-check still works: `TASKPAD_BIN=../taskpad/taskpad cargo test
      --test e2e` runs, and reproduces the single documented divergence (`edit --phase abc`)
      rather than erroring on a missing binary or resolving the relative path from the temp
      project directory

## Notes

- **This is a mechanical port, not a redesign.** Resist the urge to "improve" assertions,
  add cases, or restructure the fixture builder API beyond what the transcription needs.
- The `.mjs` suite was verified at T015 against both binaries (123 pass against Rust, 122
  against C++ — the one failure being the documented `edit --phase abc` divergence). Those
  results are the baseline this task must reproduce.
- The pre-existing baseline at the start of this task: `cargo test` 179 passed;
  `node --test tests/e2e/*.mjs` 123 pass / 0 fail across 80 suites.
- Do **not** use the `TASKPAD_BIN` override to skip a failing test. It exists so the C++
  cross-check remains reproducible, not as an escape hatch.

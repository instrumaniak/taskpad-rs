# taskpad-rs — Agent Instructions

Rust port of `taskpad` (C++). **Byte-level drop-in replacement**: same `.taskpad` config,
`status.yaml` schema, `T*.md` template, CLI flags, stdout/stderr wording, exit codes.
The one overt CLI difference is the binary name (`taskpad-rs` vs `taskpad`) plus a
`--version` / `-v` flag — see conflicts-table row 21.

## Source of truth (when unclear, check in this order)

1. **Locked decisions + "Known spec ↔ C++ conflicts" below** — settled 2026-09-25, don't re-litigate.
2. `specs/spec.main.md` / `specs/spec.testing.md` — the behavioral contract.
3. `../taskpad/src/*.cpp`, `../taskpad/src/*.h` — literal C++ implementation (read-only, never edit).
4. `../taskpad/tests/e2e/*.mjs` — literal expected stdout/stderr/exit codes.

C++ wins over spec prose for every user-visible byte (ruled 2026-09-25). A conflict not in the
table below: stop and report, don't silently pick one.

## Build & verify

```bash
cargo build
cargo clippy --all-targets -- -D warnings
cargo test          # unit tier + tests/e2e (Rust integration tier)
cargo fmt --check
```

- E2E alone: `cargo test --test e2e`; one area: `cargo test --test e2e edit`.
- The E2E assertions are transcribed from the C++ `node:test` suite and assert C++ literals.
  A failing assertion is a bug in the port, not a test to weaken.
- Lints: `unsafe_code = "forbid"` crate-wide; `clippy::unwrap_used`/`expect_used` denied
  outside `#[cfg(test)]` (the crate-root `#![cfg_attr(test, allow(...))]` in `src/main.rs`
  and the `#![allow(...)]` atop `tests/e2e/main.rs` are the deliberate carve-outs).

## Format invariants (easy to get wrong)

- YAML: `serde-saphyr` 1.x only. `serde_yaml` and forks are banned (RUSTSEC-2025-0068).
- No `regex` crate — port the C++ manual string scanners.
- `Task.id` is `#[serde(skip)]`: never serialized; `storage::read_status_file` re-threads the
  `status.yaml` map key into it after deserialization.
- `write_status_file` emits exactly what C++ `YAML::Dump` emits: no comments, empty `depends`
  as `depends: ~` (never `[]`), `phases`/`critical_path` omitted when empty, **no trailing
  newline** at EOF, key order `tasks` → `phases` → `critical_path`, per task
  `name` → `status` → `depends` → `phase` → `critical` (serde derive order).
- The reader is more tolerant: header comments, `depends: []`, flow style `depends: [T001]`,
  quoted/plain `name`, missing fields (defaults `""` / `pending` / `[]` / `0` / `false`).
  Fixture-style YAML in `tests/common/mod.rs` must parse but is not writer output.
- `.taskpad` bytes: exactly `# taskpad project config\ntask-dir: <path>\n`.
- `T*.md` template: the literal block in `spec.main.md` §4 (includes `## Phase:` and
  `## Critical:`, single trailing newline, no `## Status:` line).
- Writers are atomic: stage to a temp file beside the target, then `rename`.

## Task tracking

State lives in `specs/tasks/status.yaml`, managed via the binary itself:

```bash
taskpad-rs --tasks-dir specs/tasks status
taskpad-rs --tasks-dir specs/tasks next
```

(`.taskpad` at the repo root already points at `specs/tasks`, so bare `taskpad-rs …` works
from the root; the explicit `--tasks-dir` is unambiguous when cwd differs.)

## House rules

- Never edit `../taskpad` — read-only reference.
- Never edit `specs/spec.main.md`, `specs/spec.testing.md`, or a task file's acceptance
  criteria to make something easier to pass. Flag it instead.
- No `unsafe`, no `unwrap()`/`expect()` outside test code.
- `cargo fmt` before every commit. One logical change per commit.
- Deliberate divergences from C++ behavior go in the README "Intentional hardening
  deviations" table and the conflicts table below — never silent.

## Known spec ↔ C++ conflicts

Found during planning (2026-09-25), resolved as **C++ wins**. Spec corrections already
applied to `specs/*.md`; this table is the record so stale phrasing in task files or git
history doesn't cause confusion. A new conflict: stop and report.

| # | What | Spec said (before) | C++ ground truth |
|---|------|--------------------|------------------|
| 1 | `status.yaml` header comments | §4 schema showed `# taskpad status file …` + blank line | none written at all |
| 2 | Empty `depends` | `depends: []` | `depends: ~` (YAML null) |
| 3 | `.taskpad` comment | `# .taskpad — taskpad project config` | `# taskpad project config` |
| 4 | T\*.md template | §4 example omitted `## Phase:`/`## Critical:` | both present |
| 5 | `do` unmet deps | §5/§6: *warning*, exit 0 | **error**, exit 1: `Unmet dependencies: T002 (pending). Use --force to proceed` |
| 6 | `import` validation | §6: circular → error, exit 1 | `warning: N issue(s) found` + indented lines, **still writes status.yaml**, exit 0 |
| 7 | Circular-dep message | `Circular dependency detected: T003 → T005 → T003` | `… T003 → ... → T005`, or `… T003 depends on itself` |
| 8 | Invalid task ID message | `… (see Task ID Format in Section 3)` | `Invalid task ID format. Expected TXXX (see Task ID Format)` |
| 9 | T\*.md missing | `warning: Task file … not found. Creating template` (exists nowhere in C++) | `next`/`do` silently skip file-derived detail; `log` errors `Task file <path> not found` |
| 10 | `.taskpad` missing | command-level `error: Not initialized…` | string exists in `read_task_dir` but `resolve_task_dir` swallows it, falls back to `specs/tasks` |
| 11 | Invalid task-dir path | `error: Task directory 'xyz' not found…` | no such check; value used as-is, fails later with `No status.yaml found. …` |
| 12 | Directory not writable | `Cannot write to specs/tasks/. Check permissions` | `Cannot write to <file path>. Check permissions` (full file path) |
| 13 | `status` next marker | `← next` | `← next (dependencies met)` |
| 14 | Malformed `status.yaml` / `.taskpad` | one message each | two each: structural (`… Expected YAML mapping` / `… with 'task-dir'`) and parser exception (`… : <what()>`) |
| 15 | Subcommand count | T014 said "twelve" | **thirteen**: init, import, new, status, next, do, done, pause, deps, log, edit, summary, remove |
| 16 | `pause` scope | `in_progress` → pending | any non-pending → pending, **including done** — only already-pending rejected |
| 17 | `helpers.mjs` build (obsolete — transcribed to Rust in T023) | `ensureBuilt()` runs `make`, shared build flag | must build this repo with a repo-specific build flag |
| 18 | Wrong-shaped `tasks` node (e.g. `tasks: 5`) | locked reader contract doesn't cover it; Rust reader errors, exit 1 | C++ tolerates (empty project, exit 0). **Accepted deviation** 2026-09-25: keep strict Rust error |
| 19 | Whitespace-only `new` name | not specified | C++ accepts, writes `T002.md` that `import` can't see. **Accepted deviation** 2026-09-26: Rust rejects with `Task name cannot be empty` |
| 20 | Invalid CLI-arg rejection | spec silent | C++/CLI11: `The following argument was not expected: X` + `Run with --help…`, exit **109**; Rust/clap: `error: unexpected argument 'X' found` + usage, exit **2**. **Accepted deviation** 2026-09-26: inherent to clap replacing CLI11 |
| 21 | Binary name & `--version` | C++ binary is `taskpad`, no version flag | **Intentional divergence** 2026-10-04: binary renamed `taskpad-rs`; clap command name and `--help` usage updated to match; `--version` / `-v` prints `taskpad-rs 0.1.0`, exit 0. Everything else (messages, subcommands, exit codes) unchanged |

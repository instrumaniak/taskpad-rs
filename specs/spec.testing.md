# taskpad (Rust) — Testing Specification

## 1. Overview

Two-tier testing approach, unchanged in spirit from the C++ version — only the unit-test
tier changes framework:

| Tier | Framework | Target | Speed |
|------|-----------|--------|-------|
| Unit | Rust `#[test]` (built into `cargo test`) | Individual functions (in-process) | ~100ms |
| E2E  | Rust integration tests (`tests/e2e/`) — **transcribed from the C++ repo's `node:test` suite** | CLI binary as black box | ~2-3s |

**Run all tests:** `cargo test`

The E2E tier is the single most important compatibility check for this port: those tests
assert on the literal stdout/stderr/exit-code of the compiled binary and know nothing about
what language produced it. They were originally copied verbatim from the C++ repo's
`node:test` suite, so their expected strings *are* the C++ strings; T023 transcribes them
into Rust (`tests/e2e/*.rs`) with the assertions unchanged, which keeps that property.
Treat any E2E test that needs its *assertions* changed to pass as a bug in the
Rust port, not a spec update — the one exception is a case where the original C++ behavior
is discovered to violate its own spec (`spec.main.md`), which should be fixed in both, not
silently diverged on.

---

## 2. Unit Tests (Rust `#[test]`)

### Framework

Built into the toolchain — no external test crate required for the baseline port:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_task_id() {
        assert_eq!(format_task_id(1), "T001");
        assert_eq!(format_task_id(42), "T042");
        assert_eq!(format_task_id(999), "T999");
    }
}
```

### Placement

Unlike the C++ version's separate `tests/test_models.cpp` / `test_storage.cpp` /
`test_utils.cpp` / `test_validator.cpp` files, Rust convention is to put unit tests **in the
same file as the code they test**, inside a `#[cfg(test)] mod tests` block at the bottom of
the file. Follow that convention here — it's not a deviation worth debating, it's how the
`cargo test` / rust-analyzer tooling expects tests to live.

| Module | Coverage (mirrors the C++ `test_*.cpp` files 1:1 in scope) |
|---|---|
| `models.rs` | `Status` ↔ string round-trip; `Task`/`ProjectConfig`/`StatusFile` construction; serde round-trip through the YAML crate chosen in `spec.main.md` §8 |
| `utils.rs` | `to_kebab_case`, `parse_task_id`, `format_task_id`, `normalize_path`, `current_timestamp` (format only, not exact value), `extract_phase`, `extract_critical`, `extract_section_list_items` |
| `storage.rs` | config create/read/exists round-trip; `status.yaml` read/write round-trip; task-file path construction; template write; log append (including "no `## Notes` section yet" case) |
| `validator.rs` | valid/invalid task ID and status strings; circular dependency detection (direct and transitive); task-exists / depends-exist checks |
| `commands/*.rs` | Cover pure decision logic that doesn't require a real filesystem where practical (e.g. "which task does `next` pick given this task map", "is this task unblocked") as unit tests; behavior that's inherently about file/stdout side effects is covered by the E2E tier instead, not duplicated here. |

### Filesystem-touching tests

The C++ spec states unit tests "operate on in-memory data structures" with "no filesystem
side effects" for `models`/`utils`/`validator`, but `storage.cpp`'s own doctest suite
(`test_storage.cpp`) *does* touch the real filesystem (it creates and removes directories
under `tests/data/`) — the two C++ documents are slightly inconsistent with each other here,
and the Rust port should resolve that rather than copy the inconsistency forward:

- `models.rs`, `utils.rs`, `validator.rs` unit tests: **pure, in-memory, no filesystem
  access.**
- `storage.rs` unit tests: filesystem access is inherent to what's being tested (it's an I/O
  module). Use the [`tempfile`](https://crates.io/crates/tempfile) crate's `tempdir()` for
  an isolated, auto-cleaned-up directory per test, rather than a shared `tests/data/`
  fixture directory that tests must remember to clean up manually (the C++ version's
  `createDir`/`removeDir` pattern). This removes a whole category of test-pollution bugs the
  manual approach is exposed to.

### Requirements

- All public (`pub`) functions must have at least one test.
- Edge cases must be tested: empty input, invalid IDs, missing files, boundary values
  (task ID 0, 999, 1000).
- Zero test interdependence — each `#[test]` fn is self-contained and can run in any order
  (Rust's test runner parallelizes by default; a test that depends on execution order is a
  bug).

---

## 3. E2E Tests (Rust integration tests)

### Layout

One test target, one module per command area, and a shared helper module — the convention
used by ripgrep (`tests/tests.rs` declaring one `mod` per feature area) and fd
(`tests/testenv/mod.rs` + `mod testenv;`):

```
tests/
├── common/mod.rs     # shared helpers: Project, Output, fixture builders, assertion helpers
└── e2e/
    ├── main.rs       # the test target root
    └── <command>.rs  # one module per command area
```

**The target root must be named `main.rs`, not `mod.rs`.** Cargo auto-discovers only
`tests/*.rs` and `tests/*/main.rs`; a `tests/e2e/mod.rs` is silently ignored — `cargo test`
reports zero tests and prints no warning. Its contents are the crate-level clippy `allow`
(below), `#[path = "../common/mod.rs"] mod common;` (a plain `mod common;` looks for
`tests/e2e/common.rs` and will not find `tests/common/mod.rs`), and the module declarations.
`do.mjs` becomes `do_cmd.rs` because `do` is a Rust keyword.

### The shared harness: `tests/common/mod.rs`

`Project` wraps a `tempfile::TempDir` (its `Drop` replaces the old harness's `destroy()`)
plus the project root, and exposes `run(&[&str])` / `run_interactive(input, &[&str])` built
on `std::process::Command` with `.current_dir(root)` and `.output()`, plus filesystem
accessors (`path`, `read`, `write`, `exists`, `read_dir_sorted`, `remove`, `set_mode`).
`Output { stdout, stderr, code }` returns both streams **trimmed**, matching what the
`node:test` harness did — that trim is why assertions transcribe directly.

Two fixture spec types, not one: a `TaskSpec` for the `status.yaml`-shaped fixtures, and an
`ImportSpec` for the "import these `T*.md` files" fixtures, where `phase`/`critical`/`status`
are optional because the generated markdown only carries those sections when the caller sets
them.

### Assertions

No test framework crate (see `spec.main.md` §8 for the ruling and the reasoning). The suite's
186 `assert.match` / `assert.doesNotMatch` calls are translated without a regex crate using
helpers in `tests/common/mod.rs`:

| Helper | Covers |
|---|---|
| Literal substring assertion | literal substring patterns |
| `assert_line(out, line)` | exact whole-line (`^…$`, `m`-flag) patterns |
| Ordered-fragment assertions | same-line `.*` and unbounded cross-line `[\s\S]*` patterns |
| Bounded cross-line assertion | `[\s\S]{0,n}` patterns; enforce the maximum gap |
| Explicit std-only predicates | `/i`, ASCII digit/date and non-whitespace patterns, anchored line prefixes, and dynamically constructed exact lines |

The 118 `assert.equal` calls map onto plain `assert_eq!`. Negated regex assertions must
negate the equivalent full predicate; do not reduce them to substring negation when the
original pattern had additional semantics.

### Cargo/lint interaction

Two things the harness must get right, both of which fail loudly if omitted:

- `Cargo.toml`'s `[lints.clippy] unwrap_used`/`expect_used = "deny"` **does** apply to
  integration-test targets, so `tests/e2e/main.rs` needs
  `#![allow(clippy::unwrap_used, clippy::expect_used)]` — the counterpart of `src/main.rs`'s
  `#![cfg_attr(test, allow(...))]`, and consistent with "no `unwrap()` outside test code".
- An unused helper in `tests/common/mod.rs` is a `dead_code` **error** under
  `clippy -- -D warnings`, so that module carries a module-level `#![allow(dead_code)]`: a
  shared helper library's API is legitimately wider than any single consumer.

### Binary discovery

`env!("CARGO_BIN_EXE_taskpad")` gives the debug binary Cargo just built, so the tier no
longer needs `cargo build --release` first. Resolve an explicit `TASKPAD_BIN` override
before the runtime and compile-time Cargo paths. Canonicalize a relative override while the
test process is in the package root, before child commands change directory to each temp
project. This keeps the C++ parity cross-check reproducible:
`TASKPAD_BIN=../taskpad/taskpad cargo test --test e2e` should reproduce the single documented
divergence (`edit --phase abc`, where the C++ binary aborts on an uncaught `std::stoi`
exception) and pass everything else.

### Transcription rule

These tests were originally copied verbatim from the C++ repo, so their expected strings are
the C++ strings; the Rust port transcribes them with the assertions unchanged. If a ported
assertion needs changing to pass, that's a signal the Rust command implementation has drifted
from the C++ output format — fix the implementation, not the test (see §1). `../taskpad` stays
read-only; the `.mjs` originals are recoverable from this repo's git history.

### Test Files

One module per command area under `tests/e2e/`: `init`, `import`, `new`, `status`, `next`,
`do_cmd`, `done`, `pause`, `deps`, `log`, `edit`, `summary`, `remove`. (The C++ repo ships
only `import`, `next` and `remove` under `tests/e2e/` — the rest were gaps in the C++ repo
too, per its own `spec.testing.md`, filled in **in this repo only**; never write into
`../taskpad`.)

### Test granularity

Each `#[test]` builds its own `Project` and is independent, so the suite is safe under
`cargo test`'s parallel execution. When transcribing, split a multi-`it` `describe` into
separate tests wherever each case only reads an unmutated fixture, and merge into one test
only where a later case genuinely depends on an earlier one's side effect (e.g. `init`'s
unlink → init → re-init sequence). Splitting matters because Rust aborts a test at its first
failed assertion, so merging read-only cases would lose the per-case failure reporting the
`node:test` suite had.

### Coverage Requirements

Unchanged from the C++ spec:

1. **Happy path** — basic usage produces expected stdout/stderr and exit code 0
2. **Flag combinations** — every documented flag is exercised
3. **Error messages** — every error/warning/info message from `spec.main.md` §6 has a test
   that triggers it
4. **Prompt workflows** — interactive confirmation (`y`/`N`) tested via `run_interactive`
5. **Non-zero exit codes** — error conditions exit non-zero; prompts do not
6. **File system effects** — files created, modified, or deleted as specified

---

## 4. Running Tests

```bash
# Everything (both tiers)
cargo test

# E2E tier only
cargo test --test e2e

# A single command area's tests
cargo test --test e2e edit

# Lints, which the harness depends on (§3)
cargo clippy --all-targets -- -D warnings
```

### Requirements

- A stable Rust toolchain (`rustup show` to check).
- **No Node.js requirement** — the E2E tier is Rust. CI images need a Rust toolchain and
  nothing else, which is a net simplification versus the C++ version's Node 18+ requirement.
- No separate build step: `cargo test` builds the binary the E2E tier drives, via
  `CARGO_BIN_EXE_taskpad`. The tier exercises the **debug** binary; `cargo build --release`
  is only needed for distribution, not for testing.

---

## 5. CI Integration

CI should run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
`cargo test`. No system package installation step is needed before it (unlike the C++
version's `apt install libcli11-dev libyaml-cpp-dev ...`) and no Node.js runtime — just a
Rust toolchain in the CI image.

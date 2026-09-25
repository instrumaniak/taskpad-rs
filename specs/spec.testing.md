# taskpad (Rust) — Testing Specification

## 1. Overview

Two-tier testing approach, unchanged in spirit from the C++ version — only the unit-test
tier changes framework:

| Tier | Framework | Target | Speed |
|------|-----------|--------|-------|
| Unit | Rust `#[test]` (built into `cargo test`) | Individual functions (in-process) | ~100ms |
| E2E  | node:test (Node.js) — **reused unmodified from the C++ repo** | CLI binary as black box | ~2-3s |

**Run all tests:** `cargo test && node --test tests/e2e/*.mjs`

The E2E tier is the single most important compatibility check for this port: those tests
assert on the literal stdout/stderr/exit-code of the compiled binary and know nothing about
what language produced it. If they pass against the Rust binary without modification (aside
from pointing at the new binary path — see §3), the port is behaviorally correct by
definition. Treat any E2E test that needs its *assertions* changed to pass as a bug in the
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

## 3. E2E Tests (node:test) — Reused Unmodified

### What carries over as-is

`tests/helpers.mjs` and every file under `tests/e2e/` from the C++ repo. They spawn a
compiled binary and assert on its stdout/stderr/exit code — they have no dependency on the
implementation language.

### The one required change

`tests/helpers.mjs` currently does this:

```js
const BINARY = path.join(ROOT, 'taskpad');
// ...
function ensureBuilt() {
  if (fs.existsSync(BUILD_FLAG)) return;
  execSync('make', { stdio: 'inherit', cwd: ROOT });
  fs.writeFileSync(BUILD_FLAG, '');
}
```

For the Rust binary, update just these two spots:

```js
const BINARY = path.join(ROOT, 'target', 'release', 'taskpad');
// ...
function ensureBuilt() {
  if (fs.existsSync(BUILD_FLAG)) return;
  execSync('cargo build --release', { stdio: 'inherit', cwd: ROOT });
  fs.writeFileSync(BUILD_FLAG, '');
}
```

Nothing else in `helpers.mjs` or any `tests/e2e/*.mjs` file should need to change. If a
change beyond this turns out to be necessary to make an E2E test pass, that's a signal the
Rust command implementation has drifted from the C++ output format — fix the implementation,
not the test (see §1).

### Test Files (unchanged list from the C++ spec)

One file per command area under `tests/e2e/`: `init.mjs`, `import.mjs`, `new.mjs`,
`status.mjs`, `next.mjs`, `do.mjs`, `done.mjs`, `pause.mjs`, `deps.mjs`, `log.mjs`,
`edit.mjs`, `summary.mjs`, `remove.mjs`. (The C++ repo currently ships `import.mjs`,
`next.mjs`, `remove.mjs` under `tests/e2e/` — the remaining files are gaps that exist in the
C++ repo too, per its own `spec.testing.md`; fill them in for whichever command they're
missing for as that command's port task is completed, for both binaries' benefit.)

### Coverage Requirements

Unchanged from the C++ spec:

1. **Happy path** — basic usage produces expected stdout/stderr and exit code 0
2. **Flag combinations** — every documented flag is exercised
3. **Error messages** — every error/warning/info message from `spec.main.md` §6 has a test
   that triggers it
4. **Prompt workflows** — interactive confirmation (`y`/`N`) tested via `runInteractive`
5. **Non-zero exit codes** — error conditions exit non-zero; prompts do not
6. **File system effects** — files created, modified, or deleted as specified

---

## 4. Running Tests

```bash
# Unit tests only (cargo test, ~100ms-few seconds depending on suite size)
cargo test

# E2E tests only (node:test, ~2-3s) — requires a release build first
cargo build --release
node --test tests/e2e/*.mjs

# Everything
cargo test && cargo build --release && node --test tests/e2e/*.mjs
```

### Requirements

- A stable Rust toolchain (`rustup show` to check).
- Node.js 18+ (for `node:test`) — same requirement as the C++ version, unchanged.
- No `npm install` required — zero Node.js dependencies, same as before.

---

## 5. CI Integration

CI should run the full command from §4. No system package installation step is needed
before it (unlike the C++ version's `apt install libcli11-dev libyaml-cpp-dev ...`) beyond
whatever the CI image needs to have a Rust toolchain and Node.js 18+ available — this is a
net simplification of the CI setup, worth noting in the port's README/CI config.

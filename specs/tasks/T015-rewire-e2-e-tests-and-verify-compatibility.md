# T015: Rewire E2E Tests And Verify Compatibility

## Goal

Point the existing E2E test suite at the Rust binary, fill in any command's `.mjs` file that
doesn't yet exist, and get the full suite green — this is the acceptance gate for the whole
port.

## Depends On

- T014

## Spec References

- `spec.testing.md` → §3 E2E Tests (the required `helpers.mjs` change), §1 Overview (why a
  passing E2E suite is the compatibility proof)
- Original C++: `tests/helpers.mjs`, `tests/e2e/*.mjs`

## Phase: 5

## Critical: true

## Files to Create/Modify

- `tests/helpers.mjs` (MODIFY) — update `BINARY` path and `ensureBuilt()` per
  `spec.testing.md` §3
- `tests/e2e/*.mjs` (MODIFY only if a genuine output-format bug is found; CREATE any file
  for a command that doesn't have one yet — cross-check against the list in
  `spec.testing.md` §3)

## Implementation Steps

1. Copy `tests/helpers.mjs` and `tests/e2e/` from the C++ repo into this repo unchanged,
   then apply the edits `spec.testing.md` §3 specifies — three of them: `BINARY` path,
   `ensureBuilt()`, **and** the repo-specific `BUILD_FLAG` rename (AGENTS.md Locked
   decision §8 — the original `$TMPDIR` flag is shared with the C++ suite on this machine).
2. Run `cargo build --release && node --test tests/e2e/*.mjs`.
3. For every failure: diagnose whether it's (a) a genuine Rust-side output/behavior bug —
   fix the command implementation, not the test — or (b) a case where the C++ binary itself
   doesn't match its own `spec.main.md` — flag this explicitly in this task's Notes section
   rather than silently changing the test, since it may warrant a fix in the C++ repo too.
4. Fill in any missing `tests/e2e/*.mjs` file for a command that doesn't have E2E coverage
   yet in the C++ repo (per the gap list already called out in `spec.testing.md` §3), using
   the Coverage Requirements checklist from `spec.testing.md` §3.
5. Run `cargo test` for the full unit-test suite alongside the E2E run.
6. Once the suite is green: the seven task files T007–T013 each have a
   `tests/e2e/*.mjs` acceptance box left unchecked under the pre-approved deferral
   (AGENTS.md Locked decision §7). Check those boxes now, and append
   `E2E verified at T015` to each of those task files' Notes sections.

## Acceptance Criteria

- [x] `node --test tests/e2e/*.mjs` passes with zero failures
- [x] `cargo test` passes with zero failures
- [x] Every command has at least one E2E test file
- [x] Any discovered C++-vs-spec discrepancy is documented in this task's Notes, not
      silently papered over
- [x] The deferred `tests/e2e` boxes in T007–T013 are checked and annotated
      `E2E verified at T015` (AGENTS.md Locked decision §7, step 6 above)

## Notes

- [2026-09-25 21:19] Rewired helpers.mjs (3 edits), copied import/next/remove e2e, fixed writer seq-indent + new warning .md parity, filled coverage gaps, suite green 123/123 (cargo test 133/133), T007-T013 E2E boxes checked

**T015 implementation notes**

**Files:** `tests/helpers.mjs` (copied from C++ repo + exactly the three
`spec.testing.md` §3 edits: `BINARY` → `target/release/taskpad`, `ensureBuilt()` →
`cargo build --release`, `BUILD_FLAG` → `.taskpad-rs-e2e-built` per Locked decision §8);
`tests/e2e/{import,next,remove}.mjs` copied unchanged from the C++ repo (13-command set
complete); `tests/e2e/{init,new,status,summary}.mjs` modified (see failures below);
`tests/e2e/next.mjs` + `import.mjs` coverage additions; Rust fixes in
`src/storage.rs` (writer) and `src/commands/new.rs` (warning text).

**Verification:** `cargo build` / `cargo clippy --all-targets -- -D warnings` /
`cargo test` (133 passed, 0 failed) all green; `node --test tests/e2e/*.mjs` →
**123 pass / 0 fail** against the Rust release binary. Cross-check: the identical suite
run against the C++ binary → 122/123 (the single failure is the already-documented T012
divergence below), proving the tests encode C++ behavior, not Rust-specific behavior.

**Failures found and their diagnoses:**

1. `new.mjs` asserted flow-style `depends: [T001]` — **test bug vs ground truth**: both
   binaries' writers emit block style. Verified with the C++ binary
   (`depends:` → `      - T001`); test now pins those exact bytes.
2. **Rust writer bug (fixed, `src/storage.rs`):** block-sequence items were emitted at
   the parent key's indent (serde-saphyr compact default) where yaml-cpp `YAML::Dump`
   indents them one step deeper — affects both `depends:` (6 spaces) and
   `critical_path:` (2 spaces). Fixed with `ser_options! { compact_list_indent: false }`;
   output verified byte-identical to the C++ writer (Locked decision §4).
3. `init.mjs` tests 1/3 failed — **test setup/invocation bugs vs ground truth**:
   `createProject` pre-creates `.taskpad`, so success-path `init` needs a config-free
   dir (C++ verified: exits 1 with `Already initialized` otherwise); and `init <dir>`
   positional is rejected by **both** binaries (C++ CLI11: "not expected", exit 109;
   clap: "unexpected argument", exit 2) — the custom dir comes from the global
   `--tasks-dir` flag (both verified byte-identical). Tests now unlink `.taskpad` and use
   `--tasks-dir my-tasks init`.
4. `status.mjs` "prints phase name when configured" read `status.yaml` instead of stdout
   and the fixture had no `phases:` config — **test bug**; rewrote to configure
   `phases:` and assert `Phase 0: Foundation` / `Phase 1: Build` on stdout (both
   binaries verified identical).
5. **Rust bug (fixed, `src/commands/new.rs`):** duplicate-name warning omitted the `.md`
   suffix (`T001-first-task` vs C++/spec `T001-first-task.md` — spec.main.md §6 row
   agrees with the C++ source). Caught by a new E2E case.

**C++-vs-spec discrepancies (documented, tests NOT silently changed):**

- `edit --phase abc`: C++ aborts with an uncaught `std::invalid_argument` from
  `std::stoi` (no message, non-zero signal exit) where `spec.main.md` §6 requires
  `error: {msg}` + exit 1 and forbids panics; Rust prints
  `error: Invalid phase. Must be a non-negative integer` (exit 1). This was already
  documented in T012's Notes; `edit.mjs` asserts the spec behavior, so it fails against
  the C++ binary in the cross-check run — the divergence is real and remains flagged,
  not papered over.
- **Newly discovered, NOT fixed (flagged for human decision):** a syntactically valid
  `status.yaml` with a wrong-shaped node (e.g. `tasks: 5`) is *tolerated* by the C++
  reader (`storage.cpp:115` skips non-map `tasks` → empty project, `Progress: 0/0`,
  exit 0) but *rejected* by the serde reader (`Invalid status.yaml format: <msg>`,
  exit 1). This sits outside the locked reader contract (AGENTS.md §4 enumerates the
  accepted inputs and does not list wrong-shaped nodes); full parity would need
  yaml-cpp-style per-node leniency across `tasks`/`phases`/`critical_path`/per-task
  fields. No E2E test pins either behavior; needs a human ruling before changing the
  reader or adding a test.
- `status.yaml` *syntax-error* messages: both binaries emit
  `Invalid status.yaml format: <parser message>` with parser-specific wording
  (yaml-cpp vs serde-saphyr) — spec §6 leaves the tail as `<parser message>`, so the
  new E2E case asserts only the stable prefix.
- `.taskpad` malformed messages (`Invalid .taskpad format…`) are unreachable through
  any command in **both** binaries — `resolve_task_dir`/`resolveTaskDir` swallows the
  error and falls back to `specs/tasks` (AGENTS.md conflict #10 mechanism). The E2E
  case asserts the observable fallback (missing-`status.yaml` error); the strings
  themselves are unit-tested in `storage.rs`.

**Coverage gaps filled (spec.testing.md §3 checklist):** duplicate-name warning;
`new --phase`/`--critical` flags; `status` `← next (dependencies met)` marker;
`warning: N issue(s) found` + status.yaml-still-written FS effect on import;
full `status.yaml already exists. Use --force to overwrite` message; `Cannot write to
<path>. Check permissions` (chmod 555 case); `status.yaml` parser-error variant;
malformed `.taskpad` fallback; invalid task-dir path; `next` with missing T*.md
(silent detail omission, no template creation, stdout-only, exit 0). Every documented
flag is now exercised somewhere in the suite; `remove`'s y/N/empty prompt paths were
already covered. Every §6 message row is either E2E-tested or, where unreachable
through the CLI in both binaries, documented above with its unit-test coverage.

**Left imperfect (flagged, out of T015 scope):** T007/T008/T009 still have unchecked
non-E2E boxes (`cargo build` / `cargo clippy` / unit-test rows) from their original
sessions — per instruction only the `tests/e2e` boxes were checked at T015. They are
true today (green toolchain runs above) but were not ticked here.

- **Task-file rename:** `T015-rewire-e2e-tests-and-verify-compatibility.md` →
  `T015-rewire-e2-e-tests-and-verify-compatibility.md`. Both binaries' kebab rule
  (`to_kebab_case`: uppercase letters get a preceding `-`) computes
  `rewire-e2-e-tests-…` from the status.yaml name `Rewire E2E Tests …`, so the
  planner-created `e2e` filename was unreachable — `log` errored
  `Task file … not found` and `next` silently dropped T015's file-derived detail.
  Verified the C++ binary computes the identical `e2-e` path (not a Rust divergence);
  rename is data-level only, no spec/task text changed.

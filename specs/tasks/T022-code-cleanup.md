# T022: code cleanup

## Goal

Remove dead code, rename the `log` module to avoid shadowing the common Rust `log` crate, and clean up C++ parity markers now that the Rust port is independent. The behavior is unchanged — this is purely structural cleanup.

## Depends On

(None)

## Phase:

6

## Critical:

false

## Spec References

- Locked decision §10: session granularity (one task per session)
- AGENTS.md: Non-negotiables (behavior-preserving changes only)

## Files to Create/Modify

- `src/validator.rs` — remove `validate_task_exists` (dead code, `#[allow(dead_code)]`)
- `src/commands/mod.rs` — remove `count_status` (dead code), move `ERR_INVALID_ID` to `#[cfg(test)]`, remove `#[allow(dead_code)]`
- `src/storage.rs` — remove `config_exists` (dead code, `#[allow(dead_code)]`)
- `src/commands/log.rs` → `src/commands/log_cmd.rs` — rename module
- `src/commands/mod.rs` — update re-export from `log` to `log_cmd`
- `src/main.rs` — update dispatch from `commands::log::run` to `commands::log_cmd::run`
- All source files — trim `// Matching C++ ...` and `// C++ parity` markers from doc comments (behavior unchanged)

## Implementation Steps

1. Remove `#[allow(dead_code)] pub(crate) fn validate_task_exists(...)` from `src/validator.rs`
2. Remove `#[allow(dead_code)] pub(crate) fn count_status(...)` from `src/commands/mod.rs`; move `pub(crate) const ERR_INVALID_ID` inside `#[cfg(test)]`
3. Remove `#[allow(dead_code)] pub(crate) fn config_exists(...)` from `src/storage.rs`
4. Rename `src/commands/log.rs` to `src/commands/log_cmd.rs`
5. Update `src/commands/mod.rs`: change `mod log;` to `mod log_cmd;`, update `pub(crate) mod log;` to `pub(crate) mod log_cmd;`
6. Update `src/main.rs`: change `commands::log::run` to `commands::log_cmd::run`
7. Trim C++ parity markers from doc comments across all source files (remove `// Matching C++ ...`, `// C++ parity`, `// port of` leading `//` markers where they're pure historical references)
8. `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — all must pass
9. `cargo fmt`

## Constraints

- Behavior must not change — only structural cleanup
- No new dependencies
- No API changes (all `pub(crate)` items stay `pub(crate)`)
- Must follow the AGENTS.md session granularity rule: one task per session

## Acceptance Criteria

- [x] `cargo build` passes with no errors
- [x] `cargo clippy --all-targets -- -D warnings` passes with no warnings
- [x] `cargo test` passes with all tests green
- [x] `validate_task_exists`, `count_status`, `config_exists` are fully removed
- [x] `ERR_INVALID_ID` is `#[cfg(test)]` only
- [x] `commands/log.rs` renamed to `commands/log_cmd.rs` with all references updated
- [x] C++ parity markers trimmed from doc comments

## Notes

No E2E deferral applies to this task: T022 has no `tests/e2e/*.mjs` acceptance criterion,
so Locked decision §7 (T007–T013 only) does not apply. T015 is already `done` and the
full E2E suite is green here (123/123).

- [2026-09-26 18:25] Initial implementation: removed dead code (`validate_task_exists`,
  `count_status`, `config_exists`), moved `ERR_INVALID_ID` into the test module, renamed
  `commands/log.rs` → `commands/log_cmd.rs`, trimmed `Matching C++ …` / `C++ parity` /
  `Rust port of` markers.
- [2026-09-26 18:46] Review follow-up (see below). All 7 ACs re-verified and checked.

### Review follow-up — defects found in `91d1512` and fixed

A review of the initial commit found the mechanical refactor itself sound (all three dead
functions gone, rename complete, zero `#[allow(dead_code)]` left, no behavior change) but
found three classes of defect around it.

**1. Ten doc comments were mangled, not trimmed.** The marker sweep deleted whole lines from
the middle of sentences, leaving broken prose:

| File | Defect |
|------|--------|
| `src/models.rs` | `/// # Lossy on rewrite///` — stray `///` spliced into the heading |
| `src/models.rs` | `…normalises that field. Kept as` / `erroring instead…` — sentence cut in half |
| `src/storage.rs` | `//! # Symlinks are followed, never rejected//!` — stray `//!` in the heading |
| `src/commands/do_cmd.rs` | `/// untouched —` — dangling line above `run` |
| `src/commands/do_cmd.rs` | `// produces the same empty-list message; left as is.` — orphaned clause |
| `src/commands/init.rs` | `/// the current directory via \`createConfig(".", dir)\`).` — orphaned fragment |
| `src/commands/init.rs` | `/// \`run\` fixes the root to \`"."\` for` / `root so tests can…` — nonsensical |
| `src/commands/status.rs` | `// Deliberate` / `// exact bytes of every…` — nonsensical |
| `src/commands/summary.rs` | `/// precision 1; \`"0.0%"\` when \`total == 0\`).` — orphaned fragment |
| `src/commands/edit.rs` | stray bare `///` before `run` |
| `src/commands/import.rs` | leftover `(matching C++ \`Commands::import_\`).` duplicating the line above |

All eleven repaired. Two were actively misleading (`do_cmd.rs`, `init.rs`): they had cut the
*reason* the code is shaped that way.

**2. The sweep over-reached past its mandate.** Step 7 said to drop markers "where they're
pure historical references." These were Rust-specific soundness rationale with no C++
content, and were restored:

- `extract_depends` — the `windows(4)` / char-boundary note
- `get_first_step` — the `char_indices` note
- `skip_blank_lines` — the raw-byte-walk note
- the test comments explaining *why* each multibyte assertion exists
- the `pick_next_task` ordering comments

The resulting policy, applied consistently: the bare `Matching C++ \`Commands::x\`.`
one-liners and the `Rust port of …` module headers are gone (0 remain), while the ~90
remaining `C++` mentions are all substantive — they record *why* the Rust differs
(byte-vs-char scanning, `YAML::Dump` null emission, `commands.cpp:480` byte padding,
the measured `{:.1}` rounding parity, the T020 deliberate divergences). Those are the
ones worth keeping now that the port is independent.

**3. `status.yaml` was written wrong.** The `T022`/`T023` entries were hand-added with
T021's `phase: 6`. Re-deriving them with the real binary (`taskpad import` on a scratch
copy) shows the committed values were wrong on three fields:

| | committed | `import` derives |
|---|---|---|
| T022 `phase` | `6` | `1` |
| T023 `phase` | `6` | `2` |
| T023 `depends` | `~` | `[T022]` |

Both match what the task files declare (`## Phase: 1` / `## Phase: 2`, and T023's
`## Depends On` → `- T022 (code cleanup) — must pass first`). T023's missing dependency
was the substantive one: it would have let `tp next` offer T023 while T022 was still
open. `status.yaml` now matches `import` exactly apart from `status`, which `import`
resets by design. Names were also re-derived to the title case `import` produces
(`Code Cleanup`, `E2e Test Refactor`) so a future `import` is a no-op.

**Also fixed:**
- Removed `test_require_task_id_uses_const` — fully redundant with the pre-existing
  `test_require_task_id`, which asserts `err.to_string() == ERR_INVALID_ID` on the next
  line. `ERR_INVALID_ID` was already test-only, so no new test was needed to keep it live.
  Test count 180 → 179.
- The `## Notes` line `E2E deferred to T015 (per AGENTS.md)` was copy-paste boilerplate
  from a T007–T013 task file and has been replaced (see the correction at the top).

### Process deviations in the original commit (recorded, not repeated)

- `tp do T022` was never run — `status.yaml` went `pending` → `done` in one step.
- `tp log` was never run, so no work log existed.
- All 7 ACs were left `[ ]` while `status.yaml` said `done`, which AGENTS.md's
  non-negotiables forbid. Now checked — each was independently re-verified first, not
  assumed from the original commit's claim.
- The commit bundled `specs/tasks/T023-e2e-test-refactor.md` and the `T022`/`T023`
  `status.yaml` rows, which are outside this task's declared file list.
- The `pending` → `done` flip was left uncommitted, so the working tree was dirty.

### Verification after these fixes

| Gate | Result |
|---|---|
| `cargo build` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 179 passed, 0 failed |
| `cargo fmt --check` | clean |
| `node --test tests/e2e/*.mjs` | 123 passed, 0 failed |

Behavior is unchanged from the original T022 commit: the only functional edits in the whole
task are the three dead-function removals, the `ERR_INVALID_ID` move, and the module
rename. Everything else is comments, tests, and task metadata.

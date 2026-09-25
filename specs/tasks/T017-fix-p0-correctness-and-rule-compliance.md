# T017: Fix P0 correctness and rule compliance

## Goal

Clear the highest-severity findings from the post-port code review: one silent-wrong-value
bug (`parse_task_id` integer overflow), one C++-parity divergence in `remove`'s task-filename
construction, one command that ignored its own parameter and swallowed I/O errors (`init`),
one command that silently destroyed a recoverable `status.yaml` (`new`), five `unwrap()`s in
production code (an AGENTS.md non-negotiable violation), and two tests that gave false
coverage.

## Depends On

(None)

## Phase:

6

## Critical: false

## Spec References

- `AGENTS.md` → "Non-negotiables" (no `unwrap()`/`expect()` outside test code; never mark a
  task done with an unchecked acceptance criterion)
- `spec.main.md` → §5 Command Summary (`remove` behaviour), §6 Error Handling
- `spec.main.md` → §7 (no panics; explicit types for the public API)
- Original C++: `src/utils.cpp` (`parseTaskId`), `src/commands.cpp` (`remove`, `init`),
  `src/storage.cpp` (`writeStatusFile`)

## Files to Create/Modify

- `src/utils.rs` (MODIFY) — `parse_task_id` accumulator
- `src/commands/remove.rs` (MODIFY) — task-filename construction for empty names
- `src/commands/init.rs` (MODIFY) — parameter handling, `create_dir_all` error propagation,
  tests
- `src/commands/new.rs` (MODIFY) — corrupt-`status.yaml` handling
- `src/commands/status.rs` (MODIFY) — two production `unwrap()`s
- `src/commands/next.rs` (MODIFY) — three production `unwrap()`s
- `src/commands/do_cmd.rs` (MODIFY) — stub test with no assertions
- `src/commands/summary.rs` (MODIFY) — rounding-tie parity comment + regression test

## Implementation Steps

1. `parse_task_id`: replace the wrapping `i32` accumulation with an `i64` accumulator using
   `checked_mul`/`checked_add`, saturating at `i32::MAX`. Keep the stop-at-non-digit
   semantics C++ `std::stoi` relies on. Add an overflow test.
2. `remove`: build the filename inline as `{dir}/{id}-{kebab}.md` to match
   `commands.cpp:1128-1129`, so an empty name yields `T001-.md` (not `T001.md`). Add a test.
3. `init`: extract `run_in(project_root, tasks_dir)`; keep `run(tasks_dir)` as a
   C++-parity wrapper that fixes the root at `"."`. Replace
   `create_dir_all(...).ok()` with error propagation via `TaskpadError::cannot_write`.
   Rewrite both tests to drive `run_in` against a tempdir instead of the process CWD.
4. `new`: only treat a *missing* `status.yaml` as "start an empty project"; propagate every
   other read/parse error so a corrupt file is never overwritten. Add a test asserting the
   corrupt file survives byte-for-byte.
5. Replace the five production `unwrap()`s with `let ... else` / `if let` / `Ordering::Equal`,
   each producing a `Task ... not found` error on the (unreachable) miss path.
6. `do_cmd`: replace the stub test that built fixtures and asserted nothing with a real
   `run()`-based test for the unmet-dependency error, plus a `--force` success case.
7. `summary`: prove the `{:.1}` formatting is byte-identical to C++ `std::fixed` +
   `setprecision(1)` on exact ties using scratch programs against the toolchain's
   libstdc++; leave the implementation unchanged and pin the tie cases with a test.

## Constraints

- C++ source + E2E tests remain ground truth for every user-visible byte, message, and exit
  code. Where a fix would have changed a byte, stop and report instead of guessing.
- Do not add dependencies. No `unsafe`, no `unwrap()`/`expect()` outside `#[cfg(test)]`.
- Stay inside the file list above; other crews were working in parallel.

## Acceptance Criteria

- [x] `parse_task_id` no longer overflows; a key like `T9999999999` saturates instead of
      wrapping to a wrong ID, covered by a unit test
- [x] `remove` builds `T001-.md` (not `T001.md`) for an empty task name, matching
      `commands.cpp:1128-1129`, covered by a unit test
- [x] `init` respects its task-dir parameter and propagates `create_dir_all` failures as
      `Cannot write to <path>. Check permissions`; both its tests exercise a tempdir
- [x] `new` against a corrupt `status.yaml` returns an error and leaves the file
      byte-for-byte unchanged
- [x] Zero `unwrap()`/`expect()` in production code (script-verified across `src/`, stopping
      at each `#[cfg(test)]` marker)
- [x] The `do_cmd` unmet-dependency test asserts real output rather than nothing
- [x] `summary` percentage formatting is proven byte-identical to C++ on exact ties
- [x] `cargo build`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` all pass
      (138/138 at task close)

## Notes

(filled in during/after implementation)

- **Overflow semantics choice:** the `-> i32` signature cannot return an error without
  forcing edits to `commands/mod.rs` (owned by a parallel crew), so the accumulator
  saturates at `i32::MAX` rather than returning the `0` sentinel. Saturation preserves
  ordering semantics (absurdly large IDs sort last) and eliminates the wrap. Changing it
  to map overflow to `0` is a one-line follow-up.
- **Latent adjacent issue, left for T020:** with a saturated `i32::MAX` key present,
  `find_next_task_id` would compute `MAX + 1`. C++ throws there instead. Requires a
  hand-written `status.yaml` key to reach.
- **Rounding:** the crew compiled scratch programs against the toolchain's libstdc++ and
  confirmed `std::fixed`/`setprecision(1)` and Rust's `{:.1}` agree on tie values
  (e.g. `6.25` → `"6.2"` in both), so the implementation was left untouched and only a
  documenting comment plus a pinning test were added. The C++ binary at
  `~/.local/bin/taskpad` no longer exists on this machine — the proof used libstdc++
  directly.
- **Scope held:** the crew touched only the 8 files in its list; `specs/tasks/status.yaml`
  was modified by the human's task-tracking setup, not by the crew.

- [2026-09-26 03:01] P0 correctness: parse_task_id overflow saturates, remove empty-name T001-.md, init respects tasks_dir + propagates mkdir errors, new preserves corrupt status.yaml, 5 prod unwraps removed, do_cmd stub test made real, summary tie parity pinned

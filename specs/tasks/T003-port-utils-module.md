# T003: Port Utils Module

## Goal

Port `utils.h`/`utils.cpp` to `src/utils.rs`: kebab-case conversion, task ID parsing and
formatting, path normalization, timestamp formatting, and the T*.md metadata-extraction
helpers (`extract_phase`, `extract_critical`, `extract_section_list_items`).

## Depends On

- T001

## Spec References

- `spec.main.md` → §4 T*.md Template (the `## Phase:`/`## Critical:` metadata lines these
  functions parse)
- Original C++: `src/utils.h`, `src/utils.cpp`

## Phase: 1

## Critical: false

## Files to Create/Modify

- `src/utils.rs` (MODIFY) — `to_kebab_case`, `parse_task_id`, `format_task_id`,
  `normalize_path`, `current_timestamp`, `split`/`trim` (only if not just using Rust's
  built-in `str::split`/`str::trim` directly at call sites — prefer the std methods over
  porting these two verbatim), `resolve_task_dir`, `extract_phase`, `extract_critical`,
  `extract_section_list_items`

## Implementation Steps

1. Port `to_kebab_case`, `parse_task_id`, `format_task_id`, `normalize_path`,
   `current_timestamp` as close-to-mechanical translations of the C++ logic — these are
   pure string functions with no tricky edge cases beyond what the C++ version already
   handles (see its unit tests for the exact edge-case list to preserve).
2. Port `extract_phase`, `extract_critical`, `extract_section_list_items` — these do manual
   substring scanning in C++; port that directly using Rust string methods. Do **not** add
   a `regex` crate dependency (the `spec.main.md` §8 crate list is closed — see AGENTS.md
   Locked decision §1). Behavior parity matters more than implementation style here, since
   `import`'s correctness depends on these exactly matching.
3. `resolve_task_dir` calls into `storage::read_task_dir` in the C++ version (a
   utils→storage dependency, mirrored by storage→utils for `normalize_path` etc.). This is
   fine within a single crate — implement the function signature now; if `storage.rs` isn't
   populated yet (T004 hasn't run), stub its call target or implement `resolve_task_dir`
   last within this task, after confirming `storage.rs`'s relevant function exists or has an
   agreed signature.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Unit tests port the full case list from the C++ `test_utils.cpp` (kebab-casing edge
      cases, task ID boundary values 0/999/1000, phase/critical extraction with missing
      sections, case-insensitive `## Critical:` parsing)
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

- [2026-09-25 18:36] Port Utils Module

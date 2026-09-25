# T006: Port Validator Module

## Goal

Port `validator.h`/`validator.cpp` to `src/validator.rs`: task ID/status format validation
and circular-dependency detection.

## Depends On

- T002

## Spec References

- `spec.main.md` → §6 Validation Rules
- Original C++: `src/validator.h`, `src/validator.cpp`

## Phase: 2

## Critical: true

## Files to Create/Modify

- `src/validator.rs` (MODIFY) — `is_valid_task_id`, `is_valid_status`,
  `validate_circular_dependencies`, `validate_task_exists`, `validate_depends_exist`

## Implementation Steps

1. Port `is_valid_task_id`/`is_valid_status` — straightforward pattern/set checks.
2. Port `validate_circular_dependencies` — depth-first walk from each proposed dependency
   looking for a path back to the task being validated. The C++ version uses an explicit
   stack + visited-set; a direct translation (`Vec` as stack, `HashSet<String>` visited) is
   fine, or a recursive version if preferred — behavior (which cycles are detected) and the
   exact error message strings must match `validator.cpp`: `Circular dependency detected:
   <id> depends on itself` (self-cycle) and `Circular dependency detected: <id> → ... →
   <dep>` (path found back — note the literal ` → ... → ` with spaces; it is *not* a fully
   joined path like `T003 → T005 → T003`, despite older spec wording — see AGENTS.md
   known-conflicts #7).
3. Port `validate_task_exists`/`validate_depends_exist`.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Unit tests port the full case list from the C++ `test_validator.cpp`: valid/invalid ID
      formats (including boundary values), valid/invalid status strings, direct
      self-dependency cycle, transitive cycle (A→B→C→A), non-cyclic shared dependencies
      (should NOT be flagged), task-exists/depends-exist happy and error paths
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

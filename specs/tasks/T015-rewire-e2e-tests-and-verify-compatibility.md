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
   then apply exactly the two edits `spec.testing.md` §3 specifies.
2. Run `cargo build --release && node --test tests/e2e/*.mjs`.
3. For every failure: diagnose whether it's (a) a genuine Rust-side output/behavior bug —
   fix the command implementation, not the test — or (b) a case where the C++ binary itself
   doesn't match its own `spec.main.md` — flag this explicitly in this task's Notes section
   rather than silently changing the test, since it may warrant a fix in the C++ repo too.
4. Fill in any missing `tests/e2e/*.mjs` file for a command that doesn't have E2E coverage
   yet in the C++ repo (per the gap list already called out in `spec.testing.md` §3), using
   the Coverage Requirements checklist from `spec.testing.md` §3.
5. Run `cargo test` for the full unit-test suite alongside the E2E run.

## Acceptance Criteria

- [ ] `node --test tests/e2e/*.mjs` passes with zero failures
- [ ] `cargo test` passes with zero failures
- [ ] Every command has at least one E2E test file
- [ ] Any discovered C++-vs-spec discrepancy is documented in this task's Notes, not
      silently papered over

## Notes

(filled in during/after implementation)

# T012: Port Commands — Edit

## Goal

Port `Commands::edit` from `commands.cpp` to `src/commands/edit.rs` — the largest single
command after `import`, handling both task-level and project-level metadata edits.

## Depends On

- T007
- T006

## Spec References

- `spec.main.md` → §5 Command Summary (edit row)
- Original C++: `src/commands.cpp` (`Commands::edit`)

## Phase: 3

## Critical: false

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY) — add `pub mod edit;`
- `src/commands/edit.rs` (CREATE)

## Implementation Steps

1. Port the task-level branch: given a task ID, apply whichever of `--status`, `--phase`,
   `--critical`/`--no-critical`, `--depends` were supplied, validating circular dependencies
   if `--depends` changed, and print a confirmation line per changed field.
2. Port the project-level branch: no task ID given — apply `--phases` (parse
   `"N:name,N:name"` into the `BTreeMap`) and/or `--critical-path` (parse comma-separated
   task IDs, validating each exists).
3. Preserve the C++ version's flag-combination semantics exactly — this command's argument
   surface (via `clap` in T014) needs `--critical`/`--no-critical` as two separate boolean
   flags, matching the CLI11 version, not a single tri-state flag.

## Acceptance Criteria

- [x] `cargo build` succeeds
- [ ] `tests/e2e/edit.mjs` covering both task-level and project-level flag combinations
      passes — **verification deferred to T015** (AGENTS.md Locked decision §7). Author it
      in *this* repo's `tests/e2e/` (never in `../taskpad`); leave this box unchecked at
      T012 with a Notes line `E2E deferred to T015 (per AGENTS.md)`
- [x] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

E2E deferred to T015 (per AGENTS.md)

- `tests/e2e/edit.mjs` authored in this repo's `tests/e2e/` (the C++ suite has no
  edit.mjs); `node --check` passes, full run deferred as above.
- `cargo test`: 105 passed / 0 failed (23 of them new `commands::edit` unit tests).
- **Divergences from C++ (not in AGENTS.md's conflicts table)** — C++ aborts with an
  uncaught `std::invalid_argument` (exit 134) where `std::stoi` gets a non-integer;
  `spec.main.md` §6 forbids panics, so the port errors cleanly instead:
  - `edit <id> --phase abc` → `error: Invalid phase. Must be a non-negative integer`
    (C++: `terminate called after throwing an instance of 'std::invalid_argument'`).
    Out-of-`i32` values (e.g. `--phase 9999999999`) hit the same path
    (C++: `std::out_of_range` abort).
  - `edit --phases "abc:Name"` → the pair with a non-numeric key is skipped and the
    remaining pairs still apply (C++: same abort; skipping mirrors C++'s own skip of
    pairs that don't split into exactly two `:`-parts).
- **T014 wiring note:** `run()` takes `phase: &str` (not a typed int) so
  `Phase must be non-negative` stays a command-level error; the clap arg will need
  `allow_negative_numbers` so `--phase -1` reaches it (CLI11 accepts the raw value).
  `--critical` / `--no-critical` are two separate `bool`s, mirroring `cli.cpp`'s
  collapse (`critSet = critical || no_critical; critVal = critical`).

- [2026-09-25 20:14] Ported Commands::edit to src/commands/edit.rs (task-level + project-level branches, 23 unit tests) and authored tests/e2e/edit.mjs

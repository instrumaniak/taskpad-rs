# T004: Port Storage Module — Config And status.yaml

## Goal

Port the `.taskpad` config and `status.yaml` read/write logic from `storage.cpp` to
`src/storage.rs`, using `serde` derives on the `models.rs` structs instead of hand-building
a YAML node tree.

## Depends On

- T002
- T003

## Spec References

- `spec.main.md` → §4 File Formats (status.yaml Schema, .taskpad Config), §8 Crate
  Dependencies (YAML crate choice)
- Original C++: `src/storage.h`, `src/storage.cpp` (the `readTaskDir`/`createConfig`/
  `configExists`/`readStatusFile`/`writeStatusFile` functions specifically — task files and
  log come in T005)

## Phase: 2

## Critical: true

## Files to Create/Modify

- `src/storage.rs` (MODIFY) — `read_task_dir`, `create_config`, `config_exists`,
  `read_status_file`, `write_status_file`

## Implementation Steps

1. Port `read_task_dir`/`create_config`/`config_exists` for the `.taskpad` file — this is a
   single `task-dir: <path>` YAML mapping, trivial with a one-field struct + serde.
2. Port `read_status_file`/`write_status_file` for `status.yaml` — **this is the task where
   the biggest simplification over the C++ version happens**: replace the C++ version's
   manual `YAML::Node` field-by-field construction with `serde::Serialize`/`Deserialize` on
   `StatusFile` directly (deserialize/serialize the whole struct in one call). Confirm the
   output byte-format (key ordering, quoting of string values, `~` vs `[]` for empty
   sequences) is close enough to the C++ output that existing `status.yaml` files parse
   correctly and newly-written ones don't look alarmingly different to a human reading them
   — exact byte-for-byte emitter output parity is not required (only round-trip
   compatibility is), but gratuitous divergence should be avoided.
3. Preserve the exact error message strings from the C++ version for each failure path (see
   `spec.main.md` §6 Edge Cases) — `read_task_dir` failing because `.taskpad` doesn't exist,
   `read_status_file` failing because `status.yaml` doesn't exist or is malformed, etc.

## Constraints

- Must round-trip: a `status.yaml` written by the C++ binary must be read correctly by
  `read_status_file`, and a `status.yaml` written by `write_status_file` must be read
  correctly by the C++ binary. Verify this manually against a fixture file from the
  original repo's `specs/tasks/status.yaml` as a smoke check before marking this task done.

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Unit tests (using `tempfile::tempdir()` per `spec.testing.md` §2) cover: config
      create/read/exists round-trip, re-creating an existing config fails, `status.yaml`
      read/write round-trip including `phases`/`critical_path`, missing-file error messages
      match spec §6 exactly
- [ ] A `status.yaml` fixture copied from the original C++ repo parses successfully
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

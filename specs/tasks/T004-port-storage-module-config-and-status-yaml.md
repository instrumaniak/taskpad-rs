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
   single `task-dir: <path>` YAML mapping, trivial with a one-field struct + serde. The
   written file must be exactly `# taskpad project config\ntask-dir: <path>\n` (comment
   line from the C++ `YAML::Comment`, one trailing LF, no inline comment — see
   `spec.main.md` §4 and AGENTS.md Locked decision §5).
2. Port `read_status_file`/`write_status_file` for `status.yaml` — **this is the task where
   the biggest simplification over the C++ version happens**: replace the C++ version's
   manual `YAML::Node` field-by-field construction with `serde::Serialize`/`Deserialize` on
   `StatusFile` directly (deserialize/serialize the whole struct in one call). The writer
   must still match the C++ emitter's byte format per `spec.main.md` §4's writer facts and
   AGENTS.md Locked decision §4: no comments, empty `depends` as `depends: ~` (never `[]`),
   `phases`/`critical_path` omitted when empty, no trailing newline at EOF, fixed key
   order (serde derive order gives this). The *reader* must tolerate everything §4's
   reader-tolerance list covers (header comments, `[]`, flow style, quoted/plain names) —
   `tests/helpers.mjs` fixtures and C++-written files use different styles, and both must
   parse. Note the trap: `#[serde(default)] Vec<String>` does **not** cover an explicit
   `depends: ~`/`null` — handle null explicitly.
3. Preserve the exact error message strings from the C++ version for each failure path (see
   `spec.main.md` §6 Edge Cases — note there are *two* malformed messages for each file:
   structural `Invalid status.yaml format. Expected YAML mapping` and parser-exception
   `Invalid status.yaml format: <what>`, plus the `.taskpad` pair) — `read_task_dir`
   failing because `.taskpad` doesn't exist, `read_status_file` failing because
   `status.yaml` doesn't exist or is malformed, etc. Write failures use
   `Cannot write to <full file path>. Check permissions`.

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

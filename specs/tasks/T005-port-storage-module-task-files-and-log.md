# T005: Port Storage Module — Task Files And Log

## Goal

Port the T*.md file read/write/template and `## Notes` log-append logic from `storage.cpp`
to `src/storage.rs`.

## Depends On

- T004

## Spec References

- `spec.main.md` → §4 T*.md Template
- Original C++: `src/storage.cpp` (`taskFilePath`, `readTaskFile`, `writeTaskFile`,
  `appendLog`)

## Phase: 2

## Critical: false

## Files to Create/Modify

- `src/storage.rs` (MODIFY) — `task_file_path`, `read_task_file`, `write_task_file`,
  `append_log`

## Implementation Steps

1. Port `task_file_path` — builds `<dir>/<id>-<kebab-name>.md`, falling back to `<id>.md` if
   the kebab-cased name is empty. Uses `utils::to_kebab_case`.
2. Port `read_task_file`/`write_task_file` — `write_task_file` embeds the exact template
   string from `spec.main.md` §4 (Goal/Depends On/**Phase**/**Critical**/Spec References/
   Files to Create-Modify/Implementation Steps/Constraints/Acceptance Criteria/Notes
   sections, no `## Status:` line, single trailing newline). Copy the literal template
   block from the spec — it now matches `storage.cpp`'s `writeTaskFile` byte-for-byte,
   including the `## Phase:`/`## Critical:` placeholder sections — rather than
   reconstructing it from memory: string drift here breaks `import` parsing of
   freshly-created task files.
3. Port `append_log` — finds or creates a `## Notes` section and inserts a
   `- [YYYY-MM-DD HH:MM] <message>` line, matching the C++ version's section-boundary
   logic (insert before the next `## ` heading, or at end of file if `## Notes` is the last
   section).

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Unit tests cover: file-not-found error, template content matches spec §4 exactly,
      log append creates `## Notes` when absent, log append inserts before the next section
      when `## Notes` isn't the last section, timestamp format is `YYYY-MM-DD HH:MM`
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

- [2026-09-25 18:47] Ported task files + log append

# T018: Deduplicate command boilerplate and error strings

## Goal

Collapse the duplication the code review found across the thirteen command modules: an
identical ~15-line "validate ID → resolve dir → read status → look up task" prologue in
seven files, two error literals retyped in seven places, next-task ranking implemented twice
with different algorithms, three separate spellings of the blocker/dependents scan, the same
Goal/First-step/Files/Specs detail block written twice, a three-line status count repeated in
three files, and whole-`Task` clones taken only to read one field.

## Depends On

(None)

## Phase:

6

## Critical: false

## Spec References

- `spec.main.md` → §2 Architecture (module layout; `commands/` may host shared helpers),
  §5 Command Summary (`status`, `next`, `do`, `done`, `deps` output), §6 Error Format
- `AGENTS.md` → Locked decision §2 (C++ wins for every user-visible byte)
- Original C++: `src/commands.cpp` (the seven prologue sites), `src/commands.h`

## Files to Create/Modify

- `src/commands/mod.rs` (MODIFY — additive: new shared helpers + their tests)
- `src/commands/do_cmd.rs`, `done.rs`, `pause.rs`, `deps.rs`, `log.rs`, `remove.rs`,
  `edit.rs` (MODIFY — prologue → `load_status` / shared constructors)
- `src/commands/status.rs`, `next.rs` (MODIFY — single `pick_next_task`)
- `src/commands/summary.rs`, `import.rs` (MODIFY — single `count_all`)
- Per-command test modules that duplicated `mod.rs` coverage (MODIFY — duplicates removed)

## Implementation Steps

1. Add to `commands/mod.rs`: `ERR_INVALID_ID` + `invalid_task_id_error()` +
   `require_task_id()`, a `task_not_found(id)` constructor, `load_status(tasks_dir)` (resolve
   + read), `unmet_deps(task, tasks)`, `dependents_of(tasks, id)`, `pick_next_task(tasks)`
   (critical desc → phase asc → ID asc), `count_all(tasks) -> Counts`, and
   `goal_lines`/`task_detail_lines` for the shared detail block.
2. `pick_next_task` becomes the single canonical ranking implementation, replacing the
   pairwise-best tracker in `status` and the collect-then-`sort_by` in `next`. Gate it on
   `all_deps_done` so dangling-dependency behaviour is unchanged.
3. Replace each command's prologue with the helpers. Keep `edit`'s read-before-validate
   order — that ordering is C++ parity, not an accident.
4. Replace the seven hand-typed `"Invalid task ID format…"` and `format!("Task {id} not
   found")` call sites with the shared const/constructor. Output must be byte-identical.
5. Share the detail block between `next` (indented, Goal + First step + Files + Specs) and
   `do` (unindented, Goal + First step subset) without changing either's output.
6. Remove per-command test copies of the comparator and detail-block coverage now living in
   `mod.rs`; keep the genuinely command-specific tests.
7. Narrow whole-`Task` clones to the fields actually read.

## Constraints

- Zero user-visible byte changes: same stdout, stderr, and exit codes. Same prompt text, same
  column widths, same symbols.
- `remove`'s `T001-.md` filename formula from T017 must survive untouched.
- T017's checked `parse_task_id`, `new`'s corrupt-file propagation, and the removal of the
  production `unwrap()`s must all survive untouched.
- No new dependencies; no `unsafe`; no `unwrap()`/`expect()` outside `#[cfg(test)]`.

## Acceptance Criteria

- [x] `ERR_INVALID_ID` and a task-not-found constructor exist in `commands/mod.rs`, and the
      per-command copies exist only inside `#[cfg(test)]` assertion modules
- [x] All seven prologue sites call `load_status` instead of repeating resolve + read
- [x] `status` and `next` both rank through the single `pick_next_task`; the second
      comparator implementation and the two in-test copies are gone
- [x] `unmet_deps` and `dependents_of` each have one implementation shared by their 2–3
      former call sites
- [x] `count_all` replaces the repeated three-call `count_status` blocks in `status`,
      `summary`, and `import`
- [x] The Goal/First-step/Files/Specs block is produced by one shared helper, with `next`'s
      indented and `do`'s unindented rendering preserved
- [x] Differential check against the C++ binary over 19 commands and 9 error paths reports
      identical stdout, stderr, exit codes, and byte-identical resulting file trees
- [x] `cargo build`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` all pass
      (139/139 at task close)

## Notes

(filled in during/after implementation)

- The crew ran an extra differential pass against the C++ binary (19 commands + 9 error
  paths, comparing stdout, stderr, exit code, and the resulting file trees) beyond the
  requested verification. All identical apart from expected absolute-directory echoes.
- `edit`'s duplicate task lookup (`contains_key` followed by `get_mut`) was left as-is: it is
  single-threaded and harmless, and the surrounding read-before-validate ordering is
  deliberate C++ parity.
- Test dedupe was kept deliberately minimal — a shared `tests/common` fixture crate was
  considered and skipped as too much churn for the benefit.
- The working tree already contained uncommitted T017 state when this task started; `git
  diff -w` confirmed the `cargo fmt` run added no churn to those files.

- [2026-09-26 03:01] Deduplicated command prologue/unmet_deps/dependents_of/pick_next_task/count_all/detail-block into commands/mod.rs with shared error constructors; removed per-command test copies

# T020: Harden robustness and security

## Goal

Close the robustness and security gaps the code review found: non-atomic writes that can leave
a truncated `status.yaml` (total project loss) on a crash or a full disk; TOCTOU windows and
swallowed I/O errors that report success on failure; read errors collapsed into
"not found" messages that send users down the wrong path; and manual byte-index scanners that
panic on a slice that is not on a character boundary. Also record the deviations the hardening
introduces, and restore the writer parity a verification pass caught.

## Depends On

(None)

## Phase:

6

## Critical: false

## Spec References

- `AGENTS.md` → Locked decision §2 (C++ wins for every user-visible byte) and §4
  (`status.yaml` writer parity), and the known-conflicts table (new rows 19 and 20 were added
  by this task's human rulings)
- `spec.main.md` → §4 (`.taskpad` / `status.yaml` / `T*.md` formats), §5 (`new`, `import`),
  §6 Error Handling and Edge Cases, §7 (no panics)
- Original C++: `src/storage.cpp` (`writeStatusFile`, `writeTaskFile`, `readStatusFile`),
  `src/commands.cpp` (`new`, `import`, `remove`)

## Files to Create/Modify

- `src/storage.rs` (MODIFY) — atomic writes, `create_new` config, read-error split, path
  containment, module docs
- `src/models.rs` (MODIFY) — `TaskpadError::cannot_read`; `se_tasks`/`de_tasks` writer-parity
  fix; doc note on lossy status coercion
- `src/commands/new.rs` (MODIFY) — whitespace-only name rejection
- `src/commands/remove.rs` (MODIFY) — report deletion only on success
- `src/commands/import.rs` (MODIFY) — abort on unreadable `status.yaml`; panic-free filename
  slicing
- `src/commands/mod.rs` (MODIFY) — UTF-8-safe `extract_depends`/`get_first_step`/section
  scanning, module docs
- `src/utils.rs` (MODIFY) — UTF-8-safe `extract_phase`/`extract_critical`/`trim`/
  `extract_section_list_items`
- `src/commands/status.rs`, `do_cmd.rs`, `log.rs` (MODIFY) — documentation-only parity notes

## Implementation Steps

1. **Atomic writes.** Route `write_status_file`, `write_task_file`, `create_config`, and
   `append_log` through one `atomic_write` helper: create a uniquely named temp file in the
   *same* directory with `create_new`, write, then rename over the target. Clean the temp file
   up on failure. Use only `std::fs` — `tempfile` is a dev-dependency and no new dependency may
   be added. Name temps `.taskpad-tmp-<pid>-<n>` (dot-prefixed, no extension) so a stranded
   temp can never be scanned as a `T*.md` by `import`. Map every failure to the existing
   `Cannot write to <target>. Check permissions`, never naming the temp path.
2. **TOCTOU.** `create_config` reserves the destination with
   `OpenOptions::new().write(true).create_new(true)`. `remove` prints `Deleted <file>` only when
   `fs::remove_file` returned `Ok` (the result was previously discarded with `let _`).
3. **Read-error split.** `read_task_dir` and `read_status_file` currently map *any* read
   failure to `Not initialized…` / `No status.yaml found…`. Split `ErrorKind::NotFound` (keep
   the existing message) from everything else — permission denied, invalid UTF-8, is-a-directory
   — which surfaces as a new `Cannot read <path>: <os reason>` via `TaskpadError::cannot_read`.
4. **Whitespace-only names.** `new` rejected only `is_empty()`, so `"   "` produced a task file
   named `T001.md` that `import` then cannot see. Reject via `trim().is_empty()`. **Ruled by the
   human as a deliberate deviation from C++** — see conflicts table row 19.
5. **UTF-8-safe scanners.** `extract_depends`, `get_first_step`, `extract_goal`,
   `extract_phase`, `extract_critical`, `extract_section_list_items`, and `utils::trim` all
   indexed `&str` by raw byte offsets and could panic on a non-char-boundary slice. Rewrite
   using `str::get`, `as_bytes().windows(n)`, `char_indices`, and `strip_prefix`, keeping ASCII
   behaviour byte-for-byte identical. Add multibyte regression tests (`é`, `→`, NBSP, emoji)
   in the relevant sections, including one that asserts no panic.
6. **`import` data loss.** `import` swallowed *every* `read_status_file` error and then
   overwrote `status.yaml`. Now only the missing-file case is tolerated; anything else aborts
   before any output or write. Add `strip_prefix`/`strip_suffix` in place of `&fname[..4]` /
   `&fname[5..len-3]`.
7. **Path containment.** Add a `debug_assert` plus tests proving `task_file_path` cannot escape
   the task directory for hostile names (`../../etc/passwd`, `..`, `a/b`). Do **not** reject any
   `task-dir` value — C++ uses it as-is, and the kebab-case filter already strips `/` and `.`.
8. **Writer parity (found by verification, not by the original review).** A differential run
   against the C++ binary showed an empty project wrote `tasks: {}` where C++ writes `tasks: ~`
   (C++ `writeStatusFile` default-constructs a Null `tasks` node and assigns it unconditionally;
   `YAML::Dump` renders Null as `~`). Fix with `serialize_with`/`deserialize_with` on
   `StatusFile::tasks`, emitting `NullableTilde(None)` when empty, and accept `~`, `null`, `{}`,
   and an absent key on read.
9. **Documentation-only notes** for the five C++-parity behaviours that look like bugs but are
   not: symlinks are followed; unknown `status` values coerce to `pending` and are lossy on
   rewrite; the status column pads by bytes so CJK names misalign; `Unmet dependencies: .`
   appears with an empty list when every dependency is missing; task names and log messages are
   printed verbatim without ANSI sanitisation.

## Constraints

- No new dependencies. No `unsafe`. No `unwrap()`/`expect()` outside `#[cfg(test)]`.
- `resolve_task_dir`'s swallow-everything fallback stays as-is — that is conflicts-table #10
  C++ parity, and the improved `read_task_dir` message is only visible to direct callers.
- Items 3 and 4 are the only intentional user-visible behaviour changes; everything else must
  be byte-identical to the previous release.

## Acceptance Criteria

- [x] `status.yaml` and `T*.md` writes go through a temp-file-plus-rename path; a test asserts
      no temp files are left behind, and the writer's output bytes are unchanged for
      non-empty projects
- [x] `create_config` uses `create_new`, so an existing `.taskpad` is never clobbered
- [x] `remove` prints `Deleted <file>` only when the unlink succeeded
- [x] A missing `status.yaml` still yields `No status.yaml found…`; a permission-denied,
      non-UTF-8, or is-a-directory `status.yaml` yields `Cannot read <path>: <reason>` instead
- [x] `taskpad new "   "` is rejected with `Task name cannot be empty`; a normal name is
      unaffected. **Human-ruled deviation from C++ — conflicts table row 19**
- [x] Multibyte UTF-8 in the `Depends On` / `Phase:` / `Critical:` sections no longer panics;
      regression tests cover `é`, `→`, NBSP, and emoji
- [x] `import` against an unreadable or corrupt `status.yaml` aborts and leaves the file
      byte-for-byte unchanged; `import --force` on a readable file still overwrites
- [x] `task_file_path` is proven (test + `debug_assert`) not to escape the task directory
- [x] An empty project writes exactly `tasks: ~` with no trailing newline, byte-identical to
      the C++ binary; a non-empty project is unchanged; the reader accepts `~`, `null`, `{}`,
      and an absent `tasks` key
- [x] The five C++-parity behaviours are documented in code rather than left looking like
      bugs
- [x] `cargo build`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` all pass
      (182/182 at task close), and the E2E suite passes 123/123

## Notes

(filled in during/after implementation)

- **A second `utils` panic was found and fixed while satisfying criterion 5:**
  `utils::trim` computed its end offset as `rfind(..) + 1`, which is mid-codepoint for any
  trailing multibyte character. Since `extract_goal`/`get_first_step` return through `trim`,
  the required multibyte tests panicked until it was fixed. Fixed with
  `char_indices().rev()`; its Unicode-aware whitespace semantics are unchanged, since making
  it ASCII-only would change printed bytes.
- **Deliberate parity byte change, non-ASCII only:** `extract_phase`/`extract_critical` now
  skip only *ASCII* whitespace, matching C++'s C-locale `isspace`, instead of
  `char::is_whitespace`. `## Phase:\u{a0}3` therefore returns `0` where the previous code
  returned `3`. ASCII input is byte-for-byte identical and no E2E test covers it.
- **Conflicts table rows 19 and 20** were added to `AGENTS.md` per explicit human rulings:
  row 19 keeps the whitespace-only-name rejection; row 20 records that clap's
  argument-rejection wording and exit code 2 differ from CLI11's wording and exit code 109,
  which is inherent to the locked T014 decision to use clap. Both agree on which invocations
  are valid, so this only affects already-invalid input.
- **`import` in an empty task dir never reaches the writer** — it returns early with
  `No T*.md files found in …`, so the `tasks: ~` divergence was reachable only via `remove` of
  the last task. The earlier review's claim that `import` triggered it was wrong.
- **`tests/e2e/summary.mjs` uses `tasks: {}` as an input fixture** and was deliberately left
  untouched: it now doubles as a regression guard that hand-written `{}` files still parse.
- **Flagged, not fixed:** atomic writes swap the inode, so a pre-existing file's mode is
  replaced by `0o666 & ~umask`, and a read-only *file* in a writable directory is now
  replaceable where `fs::write` previously failed. No test or E2E case depends on either
  behaviour. No `fsync` is performed (matching C++).
- **Flagged, still open:** names whose kebab-case form is empty for reasons other than
  whitespace (e.g. `"!!!"`, `"---"`) still yield a file that `import` cannot see.
- **Flagged, out of scope:** `read_task_file`/`append_log`'s read still collapse any error to
  `Task file <path> not found`.
- **Verification note:** the E2E harness's build flag lives in `$TMPDIR`, so a stale
  `.taskpad-rs-e2e-built` makes `ensureBuilt()` skip the build and silently test an old
  binary. `rm -f /tmp/.taskpad-rs-e2e-built` before any E2E run.
- **Stale node caveat:** `node --test tests/e2e/` (directory argument) errors on node 22.14
  independently of this code; the glob form `node --test tests/e2e/*.mjs` is the working
  invocation.

- [2026-09-26 03:01] Hardening: atomic temp+rename writes, create_new config, NotFound vs Cannot read split, remove prints Deleted only on success, whitespace-only new names rejected, UTF-8-safe scanners, import aborts on unreadable status.yaml, containment tests
- [2026-09-26 03:01] Parity fix found by C++/Rust differential harness: empty project now writes 'tasks: ~' not 'tasks: {}' (custom se_tasks/de_tasks); reader accepts ~, null, {} and absent

# taskpad-rs — Agent Instructions

This is a Rust port of `taskpad`, a C++ CLI tool. The C++ original lives at `../taskpad`
(sibling directory). **Treat `../taskpad` as read-only reference material — never edit
anything inside it**, including its `tests/e2e/` directory. Several task files say
"create if missing in the C++ repo's `tests/e2e/`" — they mean *this* repo's copy; see
Locked decision §7.

## Compatibility mandate

The Rust binary must be a drop-in replacement for the C++ one: same `.taskpad` config
format, same `status.yaml` schema, same `T*.md` template, same CLI flags, same stdout/stderr
wording, same exit codes. This is not a "new tool inspired by taskpad" — it is taskpad,
rebuilt. When in doubt about exact behavior, the C++ source and its E2E tests are the
ground truth, not intuition about what "seems right."

## Source of truth hierarchy

When something is unclear, check in this order:

1. **"Locked decisions" and "Known spec ↔ C++ conflicts" below** — settled at the planning
   session (2026-09-25) with the human. Do not re-litigate them.
2. `specs/spec.main.md` and `specs/spec.testing.md` — the behavioral contract. Both were
   corrected on 2026-09-25 where they contradicted the C++ source (see conflicts table).
3. `../taskpad/src/*.cpp` / `../taskpad/src/*.h` — the literal C++ implementation
4. `../taskpad/tests/e2e/*.mjs` — the literal expected stdout/stderr for each command

If you hit a conflict **not** listed in the known-conflicts table, **stop and report it**
rather than picking one silently — see "When stuck" below.

## Locked decisions

All approved by the human during the planning session (2026-09-25). Implement as written.

1. **YAML crate: `serde-saphyr` (1.x)**, declared in `Cargo.toml` at T001. `serde_yaml` and
   its forks stay banned (RUSTSEC-2025-0068 territory). Also at T001: add `tempfile` as a
   **dev-dependency** — T004's ACs and `spec.testing.md` §2 require `tempfile::tempdir()`,
   and `Cargo.toml` is in T001's file list, so that's where it lands. **No `regex` crate** —
   `spec.main.md` §8's crate list is closed; port the C++ manual string scanners.
2. **Conflict ruling: C++ source + E2E tests win** over spec prose for every user-visible
   byte, message, and exit code. The specs were corrected to match; the table below records
   what was wrong so old phrasing doesn't cause confusion later.
3. **`Task.id` is kept** as a struct field with `#[serde(skip)]`: never serialized;
   `storage::read_status_file` re-threads the `status.yaml` map key into it after
   deserialization. Document in `models.rs`'s module doc comment (T002) and cross-reference
   it from `storage.rs` (T004) — together those satisfy spec §3 and T002 step 3.
4. **`status.yaml` writer parity** — `write_status_file` emits exactly what the C++
   `YAML::Dump` emits: no comments; empty `depends` as `depends: ~` (never `[]`);
   `phases`/`critical_path` keys omitted entirely when empty; **no trailing newline at
   EOF**; key order `tasks` → `phases` → `critical_path`, and per task `name` → `status` →
   `depends` → `phase` → `critical` (serde derive order gives this). The *reader* must
   accept everything the C++ reader accepts: header comments, `depends: []`, flow style
   `depends: [T001]`, quoted/plain `name` scalars, missing optional fields (defaults:
   name `""`, status `pending`, depends `[]`, phase `0`, critical `false`). Note:
   `tests/helpers.mjs` fixtures are written in a *different* style than the C++ writer
   output — both must parse.
5. **`.taskpad` bytes:** exactly `# taskpad project config\ntask-dir: <path>\n` (single
   comment line, no inline comment, one trailing LF).
6. **T\*.md template:** the literal block in `spec.main.md` §4 — includes the `## Phase:`
   and `## Critical:` placeholder sections, single trailing newline, no `## Status:` line.
7. **E2E AC deferral for T007–T013** (human-approved exception to the "never mark done
   with an unchecked criterion" rule, for these boxes only): the `tests/e2e/*.mjs`
   acceptance boxes in those seven task files cannot be verified before T014 (CLI wiring)
   plus T015 (`helpers.mjs` rewire). Per task:
   - implement + unit-test, and author/copy the named `.mjs` file(s) into **this** repo's
     `tests/e2e/` as part of the task;
   - leave **only** the E2E box unchecked, add a Notes line
     `E2E deferred to T015 (per AGENTS.md)`, then `tp done` is allowed;
   - at T015, once the suite is green, check those deferred boxes in T007–T013 and append
     `E2E verified at T015` to each of their Notes.
   Every other acceptance box follows the normal rule: never check what you haven't run.
8. **`tests/helpers.mjs` needs three edits, not two** (see `spec.testing.md` §3): `BINARY`,
   `ensureBuilt()`, **and `BUILD_FLAG`** — the original `$TMPDIR/.taskpad-e2e-built` flag is
   shared with the C++ repo's suite on this machine; a stale flag makes `ensureBuilt()`
   skip the build entirely and the Rust binary never gets compiled. Rename it repo-specific
   (e.g. `.taskpad-rs-e2e-built`).
9. **`cargo install --path .` (T016) stays gated:** ask the human for explicit go-ahead
   before running it. If declined, leave that AC box unchecked and say so in the report.
10. **Session granularity: strictly one task per session**, then stop and report — no
    batching (including the structurally-similar command tasks) unless the human explicitly
    asks for it mid-session.

## Known spec ↔ C++ conflicts

Found during planning (2026-09-25), resolved as **C++ wins**. Spec corrections have
already been applied to `specs/*.md`; this table is the record so a stale phrasing in a
task file or in git history doesn't cause confusion. A conflict you discover that is *not*
in this table is a stop-and-report event (see "When stuck").

| # | What | Spec said (before) | C++ ground truth |
|---|------|--------------------|------------------|
| 1 | `status.yaml` header comments | §4 schema showed `# taskpad status file …` + blank line | none written at all — `writeStatusFile` dumps the node tree only |
| 2 | Empty `depends` | `depends: []` | `depends: ~` (YAML null) |
| 3 | `.taskpad` comment | `# .taskpad — taskpad project config` | `# taskpad project config` (`storage.cpp:62`) |
| 4 | T\*.md template | §4 example omitted `## Phase:`/`## Critical:` | both present (`storage.cpp:226-267`) |
| 5 | `do` unmet deps | §5/§6: *warning*, exit 0 | **error**, exit 1: `Unmet dependencies: T002 (pending). Use --force to proceed` (`commands.cpp:656`) |
| 6 | `import` validation (missing deps, cycles) | §6: circular → error, exit 1 | `warning: N issue(s) found` + indented lines on stderr, **still writes status.yaml**, exit 0 (`commands.cpp:289-318`; asserted by `tests/e2e/import.mjs`) |
| 7 | Circular-dep message | `Circular dependency detected: T003 → T005 → T003` | `Circular dependency detected: T003 → ... → T005` (literal `→ ... →`, not a fully joined path), or `… T003 depends on itself` for self-cycles (`validator.cpp:30,45`) |
| 8 | Invalid task ID message | `… (see Task ID Format in Section 3)` | `Invalid task ID format. Expected TXXX (see Task ID Format)` — no "in Section 3" (`commands.cpp:620` etc.) |
| 9 | T\*.md missing | `warning: Task file … not found. Creating template` — this string exists nowhere in C++ | `next`/`do` silently skip file-derived detail (Goal/First step/Files/Specs); `log` errors `Task file <path> not found` |
| 10 | `.taskpad` missing | `error: Not initialized. Run 'taskpad init' first` as a command-level error | the string exists in `read_task_dir` (`storage.cpp:37`) but `resolve_task_dir` swallows the failure and falls back to `specs/tasks` (`utils.cpp:93-97`) — no command ever surfaces it |
| 11 | Invalid task-dir path | `error: Task directory 'xyz' not found. Check task-dir in .taskpad` | no such check or string — the value is used as-is and typically fails later with `No status.yaml found. …` |
| 12 | Directory not writable | `Cannot write to specs/tasks/. Check permissions` (directory) | `Cannot write to <file path>. Check permissions` — the full file path being written (`storage.cpp:71,196,271,317`) |
| 13 | `status` next marker | `← next` | `← next (dependencies met)` (`commands.cpp:488`) |
| 14 | Malformed `status.yaml` / `.taskpad` | one message each | two each: structural (`… Expected YAML mapping`, and for `.taskpad` `… with 'task-dir'`) and parser exception (`… : <what()>`) — `storage.cpp:44,49,103,145` |
| 15 | Subcommand count | T014 said "twelve" | **thirteen**: init, import, new, status, next, do, done, pause, deps, log, edit, summary, remove |
| 16 | `pause` scope | §5: in_progress → pending | any non-pending → pending, **including done** — only already-pending is rejected (`commands.cpp:760`) |
| 17 | `helpers.mjs` build | `ensureBuilt()` runs `make`, shared build flag | must run `cargo build --release` with a repo-specific build flag (Locked decision §8) |

Also corrected while editing: `spec.main.md` §6 gained missing rows (malformed `.taskpad`
dual messages, `already pending`), §8 now names `serde-saphyr` + `tempfile`, and §4's
writer/reader notes were added.

## Task tracking (read this before doing anything else)

Task state lives in `specs/tasks/status.yaml`, managed via the `taskpad` CLI itself. The
C++ `taskpad` is already built and installed at `~/.local/bin/taskpad`, on `PATH`. Until
`T014` (CLI wiring) is done, **this repo has no working `taskpad` binary of its own** — use
that installed C++ one for all task tracking:

```bash
alias tp='taskpad --tasks-dir specs/tasks'
tp status
tp next
```

A `.taskpad` config (`task-dir: specs/tasks`) already exists in this repo's root, so bare
`taskpad …` also works from the repo root; the alias's explicit `--tasks-dir` keeps things
unambiguous when the working directory differs — prefer the alias. The sibling `../taskpad`
repo has its own separate `.taskpad`; they never interact.

Once `T014`'s acceptance criteria are met, **switch to the self-built binary explicitly by
path** — `./target/release/taskpad --tasks-dir specs/tasks` — for `T015`/`T016`, rather than
relying on the bare `taskpad` command. The installed C++ binary owns the `taskpad` name on
`PATH`; do not run `cargo install --path .` (which would install to `~/.cargo/bin/taskpad`
and create a second, possibly-conflicting `taskpad` on `PATH` depending on `PATH` ordering)
until the human explicitly decides it's time to replace the installed C++ version (Locked
decision §9). Call the switch to the self-built binary out explicitly when it happens —
it's the first real proof the port works.

## The task loop

For each task:

1. `tp next` — respects critical path → phase → task number ordering. Don't cherry-pick a
   task out of order without a reason, and say what the reason was if you do.
2. Read the full task file (`specs/tasks/TXXX-*.md`), then read every file listed under its
   **Spec References** section — especially the named C++ source file(s) — before writing
   any code. Also re-check "Locked decisions" and the conflicts table above for anything
   that task touches.
3. `tp do TXXX`
4. Implement exactly what's listed under **Files to Create/Modify**. No unrelated
   refactors, no drive-by fixes to other modules, no scope creep into a later task's files —
   if you notice something else that needs fixing, note it, don't fix it inline.
5. Verify, in order, stopping at the first failure and fixing before continuing:
   - `cargo build`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - any E2E file named in the task's acceptance criteria (`node --test tests/e2e/<name>.mjs`)
     — only applicable from T015 onward, since the E2E suite needs the wired release binary
     and the rewired `helpers.mjs`. For T007–T013 the E2E boxes follow the deferral process
     (Locked decision §7); for those tasks, also make sure the named `.mjs` file exists in
     this repo's `tests/e2e/` before finishing.
6. Only check off an acceptance criteria box once it's genuinely true — not "should pass,"
   actually run and confirmed. Sole exception: the deferred T007–T013 E2E boxes, handled
   per Locked decision §7.
7. `tp log TXXX "<one-line summary of what was implemented>"`
8. `tp done TXXX`
9. `cargo fmt`, then `git add -A && git commit -m "TXXX: <task name>"` — one commit per
   task, never batch multiple tasks into one commit, never commit a task that failed its
   own verification step.
10. `tp status` to confirm state, then **stop** — see Session Granularity below.

## Non-negotiables

- Never edit `../taskpad` (reference only) — including creating the missing `.mjs` files
  there; E2E files live only in this repo.
- Never edit `specs/spec.main.md`, `specs/spec.testing.md`, or a task file's acceptance
  criteria **to make something easier to pass**. If a criterion seems wrong, flag it in your
  summary — don't edit around it. (The 2026-09-25 spec corrections were human-authorized
  accuracy fixes that make the port *stricter*, not easier; don't redo or revert them.)
- Stay inside the current task's declared file list (exception: adding `tempfile` to
  `Cargo.toml` belongs to T001 per Locked decision §1).
- No `unsafe`. No `unwrap()`/`expect()` outside test code (see `spec.main.md` §7).
- `cargo fmt` before every commit.
- Never mark a task `done` with a failing `cargo build`/`clippy`/`test`, or an unchecked
  acceptance criterion you didn't actually verify (except the deferred E2E boxes per
  Locked decision §7).

## Session granularity

**One task, then stop and report back** — even though `tp next` would happily hand you the
next one immediately, and even for the structurally similar command tasks (T008–T013).
This is the human's explicit choice (Locked decision §10); batching happens only if the
human asks for it mid-session. The one-task discipline matters most on the critical path
(T001, T002, T004, T006, T007, T009, T014, T015) since mistakes there compound into
everything built on top of them.

## When stuck

If a task's instructions are ambiguous, conflict with `spec.main.md`, conflict with the
C++ source, or conflict with anything above: **first check the Locked decisions and the
known-conflicts table** — most planning-time questions are already settled there. If it's
still unresolved: **stop, state exactly what's blocking and cite the specific spec section
or source file/line involved, and wait for direction.** Do not guess and proceed, and do
not silently pick whichever interpretation is easier to implement.

## Reporting back

At the end of a task, summarize: which task completed, what verification was run and its
result, anything flagged as a concern (new spec/C++ discrepancies, acceptance criteria that
seemed off, scope you deliberately left out), and what `tp next` would pick up next.

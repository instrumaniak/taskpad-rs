# taskpad-rs — Agent Instructions

This is a Rust port of `taskpad`, a C++ CLI tool. The C++ original lives at `../taskpad`
(sibling directory). **Treat `../taskpad` as read-only reference material — never edit
anything inside it.**

## Compatibility mandate

The Rust binary must be a drop-in replacement for the C++ one: same `.taskpad` config
format, same `status.yaml` schema, same `T*.md` template, same CLI flags, same stdout/stderr
wording, same exit codes. This is not a "new tool inspired by taskpad" — it is taskpad,
rebuilt. When in doubt about exact behavior, the C++ source and its E2E tests are the
ground truth, not intuition about what "seems right."

## Source of truth hierarchy

When something is unclear, check in this order:

1. `specs/spec.main.md` and `specs/spec.testing.md` — the behavioral contract
2. `../taskpad/src/*.cpp` / `../taskpad/src/*.h` — the literal C++ implementation
3. `../taskpad/tests/e2e/*.mjs` — the literal expected stdout/stderr for each command

If the spec and the C++ source ever disagree with each other, **stop and report the
conflict** rather than picking one silently — see "When stuck" below.

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

(`--tasks-dir specs/tasks` is required every invocation unless `taskpad init` has been run
in this repo to write a `.taskpad` config — either is fine, but if `taskpad init` is run
here, double-check its `.taskpad` doesn't get confused with the sibling `../taskpad` repo's
own `.taskpad` — they're separate directories so this is a non-issue in practice, just don't
assume `.taskpad` is absent because you're "in the taskpad project.")

Once `T014`'s acceptance criteria are met, **switch to the self-built binary explicitly by
path** — `./target/release/taskpad --tasks-dir specs/tasks` — for `T015`/`T016`, rather than
relying on the bare `taskpad` command. The installed C++ binary owns the `taskpad` name on
`PATH`; do not run `cargo install --path .` (which would install to `~/.cargo/bin/taskpad`
and create a second, possibly-conflicting `taskpad` on `PATH` depending on `PATH` ordering)
until the human explicitly decides it's time to replace the installed C++ version. Call the
switch to the self-built binary out explicitly when it happens — it's the first real proof
the port works.

## The task loop

For each task:

1. `tp next` — respects critical path → phase → task number ordering. Don't cherry-pick a
   task out of order without a reason, and say what the reason was if you do.
2. Read the full task file (`specs/tasks/TXXX-*.md`), then read every file listed under its
   **Spec References** section — especially the named C++ source file(s) — before writing
   any code.
3. `tp do TXXX`
4. Implement exactly what's listed under **Files to Create/Modify**. No unrelated
   refactors, no drive-by fixes to other modules, no scope creep into a later task's files —
   if you notice something else that needs fixing, note it, don't fix it inline.
5. Verify, in order, stopping at the first failure and fixing before continuing:
   - `cargo build`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - any E2E file named in the task's acceptance criteria (`node --test tests/e2e/<name>.mjs`),
     once a debug/release binary exists to point them at
6. Only check off an acceptance criteria box once it's genuinely true — not "should pass,"
   actually run and confirmed.
7. `tp log TXXX "<one-line summary of what was implemented>"`
8. `tp done TXXX`
9. `git add -A && git commit -m "TXXX: <task name>"` — one commit per task, never batch
   multiple tasks into one commit, never commit a task that failed its own verification step.
10. `tp status` to confirm state, then **stop** — see Session Granularity below for whether
    to continue to the next task or hand back for review.

## Non-negotiables

- Never edit `../taskpad` (reference only).
- Never edit `specs/spec.main.md`, `specs/spec.testing.md`, or a task file's acceptance
  criteria to make something easier to pass. If a criterion seems wrong, flag it in your
  summary — don't edit around it.
- Stay inside the current task's declared file list.
- No `unsafe`. No `unwrap()`/`expect()` outside test code (see `spec.main.md` §7).
- `cargo fmt` before every commit.
- Never mark a task `done` with a failing `cargo build`/`clippy`/`test`, or an unchecked
  acceptance criterion you didn't actually verify.

## Session granularity

Default: **one task, then stop and report back** — even though `tp next` would happily hand
you the next one immediately. This matters most for the critical-path tasks (T001, T002,
T004, T006, T007, T009, T014, T015) since mistakes there compound into everything built on
top of them.

Exception: once a pattern is established by the first task in a batch of structurally
similar, lower-risk tasks (e.g. `T007` establishes the command-module pattern that
`T008`/`T010`/`T011`/`T012`/`T013` all repeat), those may run back-to-back in one session
without stopping in between — still one commit per task, still full verification per task,
just without a human checkpoint between each.

## When stuck

If a task's instructions are ambiguous, or conflict with `spec.main.md`, or conflict with
what the C++ source actually does: **stop, state exactly what's blocking and cite the
specific spec section or source file/line involved, and wait for direction.** Do not guess
and proceed, and do not silently pick whichever interpretation is easier to implement.

## Reporting back

At the end of a task (or a batch, per Session Granularity), summarize: which task(s)
completed, what verification was run and its result, anything flagged as a concern (spec/C++
discrepancies, acceptance criteria that seemed off, scope you deliberately left out), and
what `tp next` would pick up next.

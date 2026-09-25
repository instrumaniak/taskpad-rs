# T014: Wire CLI And Main Entry Point

## Goal

Port `cli.cpp`/`main.cpp` to `src/cli.rs`/`src/main.rs`, wiring all thirteen subcommands
via `clap`'s derive API and dispatching to the command modules built in T007–T013. This is
the integration point where the Rust binary first becomes fully runnable end-to-end.

## Depends On

- T008
- T009
- T010
- T011
- T012
- T013

## Spec References

- `spec.main.md` → §5 CLI Commands (all rows, especially the `--tasks-dir` resolution order
  and `--depends`/repeatable-flag semantics), §8 Crate Layout, Error Handling Strategy
- Original C++: `src/cli.h`, `src/cli.cpp`, `src/main.cpp`

## Phase: 4

## Critical: true

## Files to Create/Modify

- `src/cli.rs` (MODIFY) — `clap::Parser` top-level struct with global `--tasks-dir`, and a
  `clap::Subcommand` enum with one variant per command (mirroring the C++ `CLI::App*`
  subcommands), each carrying its own args/flags per `spec.main.md` §5
- `src/main.rs` (MODIFY) — parse args, resolve `tasks_dir` via
  `utils::resolve_task_dir` (§5 Global Flags resolution order), match on the subcommand enum
  and call the matching `commands::<name>::run(...)`, convert any `Err(TaskpadError)` to
  `eprintln!("error: {msg}")` + `std::process::exit(1)`

## Implementation Steps

1. Define the `clap::Subcommand` enum with all thirteen variants (init, import, new,
   status, next, do, done, pause, deps, log, edit, summary, remove — the original "twelve"
   was a typo; 13 matches `cli.cpp` and `spec.main.md` §5), matching each command's exact
   flags from `spec.main.md` §5 (repeatable `--depends` as `Vec<String>` with
   `#[arg(long)]`; `--force`/`--critical`/`--no-critical`/`--all` as `bool` flags via
   `#[arg(long)]`; positional `id`/`name`/`message` args as required `String`s where the C++
   version marks them `->required()`).
2. Wire `main.rs`: parse, resolve tasks-dir, `match` on subcommand, call into
   `commands::*::run`, single centralized error-to-stderr conversion (see
   `spec.main.md` §8 Error Handling Strategy — this must be the *only* place that formats
   `error: {msg}` and calls `process::exit`, not scattered across command modules).
3. Double check the `do` subcommand's clap variant name doesn't collide with the `do`
   keyword — the enum variant can be named `Do` (fine, it's an identifier position clap
   handles via `#[command(name = "do")]` regardless of the Rust-side variant/module name).

## Acceptance Criteria

- [x] `cargo build --release` succeeds and produces a working `target/release/taskpad`
      binary
- [x] `taskpad --help` lists all thirteen subcommands with descriptions matching the C++
      version's `CLI::App` descriptions (note: clap's help layout and bad-flag exit codes
      differ from CLI11's — no E2E test pins those, so don't imitate CLI11's nonstandard
      codes like 106/109)
- [x] Manually running through Appendix A's example workflow (`init`, `new`, `next`, `do`,
      `done`, `status`) against a scratch directory produces output matching the C++
      binary's output for the same sequence
- [x] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

- [2026-09-25] `src/cli.rs`: clap derive with global `--tasks-dir`, a 13-variant
  `Command` enum carrying the exact `cli.cpp` help strings/flags, `Do` variant with
  `#[command(name = "do")]`, `allow_negative_numbers` on both `--phase` flags (CLI11
  accepts `--phase -1`; `edit --phase` stays a `String` so `abc` reaches the command-level
  error), `disable_help_subcommand` so help lists exactly the thirteen subcommands.
  `src/main.rs`: parse → `utils::resolve_task_dir` → dispatch → the single centralized
  `eprintln!("error: {e}")` + `exit(1)` (only site that formats `error:`).
- Verified: release build ✓; clippy `-D warnings` ✓; `cargo test` 132 passed ✓; top-level
  and per-subcommand `--help` diffs vs the C++ binary show layout-only differences — all
  13 names/descriptions/flag help strings match. Appendix A workflow plus three extra
  scenario groups (re-import/`--force`, the edit/remove/log flag surface incl. `[y/N]`
  prompts and circular-dep warning, and 14 error paths) run against both binaries into
  separate scratch dirs: every stdout/stderr/exit code byte-identical (timestamps
  normalized); all written files identical except the byte-gap flagged below.
- FLAG (pre-existing, out of T014 scope): `src/commands/status.rs:74` computes
  `20 - task.name.len()` in `usize` — underflows for names ≥ 20 bytes, then
  `" ".repeat(...)` panics with `capacity overflow` (exit 101) on this repo's own
  `status.yaml`. C++ clamps in signed arithmetic (`commands.cpp:480-482` → padding 1).
  Must be fixed before T015: the self-built binary's `status` panics on this repo's own
  task names.
- FLAG (T004 scope): `storage::write_status_file` emits block sequences at the key's
  indent (`depends:` + 4-space `    - T001`) where yaml-cpp emits `      - T001` — a
  byte-parity gap vs Locked decision §4 for non-empty `depends`/`critical_path`. No
  current E2E regex pins the indent (all loose), but note `tests/e2e/new.mjs:36` asserts
  flow style `depends: [T001]`, which fails against the **C++** binary itself —
  T008-authored test bug (E2E deferred), will fail at T015 regardless of Rust code.
- FLAG (deliberate, bad-flag area): clap accepts global `--tasks-dir` after the
  subcommand (CLI11 rejects, exit 109), and rejects CLI11's single-occurrence
  space-separated `--depends T001 T002` form (spec §5 mandates the repeatable form
  `--depends T001 --depends T002`, which works).

- [2026-09-25 20:44] Wire 13-subcommand clap CLI in cli.rs + centralized dispatch/error handling in main.rs; C++-parity verified on help, Appendix A, and 14 error paths

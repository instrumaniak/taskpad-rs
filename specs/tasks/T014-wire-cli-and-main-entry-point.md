# T014: Wire CLI And Main Entry Point

## Goal

Port `cli.cpp`/`main.cpp` to `src/cli.rs`/`src/main.rs`, wiring all twelve subcommands via
`clap`'s derive API and dispatching to the command modules built in T007–T013. This is the
integration point where the Rust binary first becomes fully runnable end-to-end.

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
  `utils::resolve_task_dir` (§4 resolution order), match on the subcommand enum and call the
  matching `commands::<name>::run(...)`, convert any `Err(TaskpadError)` to
  `eprintln!("error: {msg}")` + `std::process::exit(1)`

## Implementation Steps

1. Define the `clap::Subcommand` enum with all twelve variants, matching each command's
   exact flags from `spec.main.md` §5 (repeatable `--depends` as `Vec<String>` with
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

- [ ] `cargo build --release` succeeds and produces a working `target/release/taskpad`
      binary
- [ ] `taskpad --help` lists all twelve subcommands with descriptions matching the C++
      version's `CLI::App` descriptions
- [ ] Manually running through Appendix A's example workflow (`init`, `new`, `next`, `do`,
      `done`, `status`) against a scratch directory produces output matching the C++
      binary's output for the same sequence
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

(filled in during/after implementation)

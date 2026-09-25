# taskpad — Task Management CLI

taskpad is a lightweight, deterministic task management CLI tool designed for AI-assisted development workflows. It provides a single source of truth for task status while keeping task descriptions as plain Markdown files.

This repository is the **Rust port** of the original [C++ taskpad](https://github.com/instrumaniak/taskpad) — a drop-in replacement with the same `.taskpad` config, `status.yaml` schema, `T*.md` templates, CLI flags, output wording, and exit codes.

See [Main Spec](specs/spec.main.md) for full details and [Testing Spec](specs/spec.testing.md) for the testing setup.

## Usage

```bash
taskpad init                          # create .taskpad + specs/tasks/status.yaml
taskpad import                        # build status.yaml from existing T*.md files (--force to overwrite)
taskpad new "Project Setup" --phase 1 --critical --depends T001
taskpad status                        # all tasks with their status
taskpad next                          # next task whose dependencies are met
taskpad do T001                       # start a task (--force skips the dependency check)
taskpad done T001                     # mark a task complete
taskpad pause T001                    # revert a task to pending
taskpad deps T001                     # show dependency information
taskpad log T001 "one-line summary"   # append a log entry
taskpad edit T001 --status done       # project-level: --phases, --critical-path (omit the ID)
taskpad summary                       # overall progress statistics
taskpad remove T001 --all             # --force skips the confirmation prompt
```

Global flag: `--tasks-dir <TEXT>` overrides the task directory (otherwise `task-dir` is read from the `.taskpad` config). Run `taskpad --help` or `taskpad <command> --help` for the full flag list.

## Requirements

- **Rust (MSRV 1.89)** on stable. The floor is set by `serde-saphyr 1.3`, which declares
  `rust-version = "1.89"`; edition 2024 alone only needs 1.85. There are no system
  packages and no C++ toolchain — everything comes from crates.io.
- **Node.js 18+** — *only* to run the E2E suite, which is `node --test` + `node:test`
  (developed and verified on Node 22). The `taskpad` binary itself has no Node dependency.

## Building from Source

```bash
# Clone repository
git clone https://github.com/instrumaniak/taskpad-rs.git
cd taskpad-rs

# Build
cargo build --release
# Binary at target/release/taskpad

# [optional] Run all tests (unit + E2E)
cargo test
node --test tests/e2e/*.mjs

# Install (to ~/.cargo/bin — ensure that's on PATH)
cargo install --path .

# check
taskpad --help

# clean up
cargo clean
```

## Verification

Run from the repository root, in this order:

```bash
cargo build
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --check
```

Then the E2E suite (needs Node.js 18+; the Rust port's E2E tests are `node --test` files, not
C++ binaries — 80 suites / 123 assertions across the 13 subcommands):

```bash
cargo build --release   # must come first — see the note below
node --test tests/e2e/*.mjs
```

> **Build the release binary before the E2E run.** `tests/helpers.mjs` writes a build-flag file
> to the OS temp dir (`.taskpad-rs-e2e-built`) and skips `cargo build --release` whenever that
> file exists. If it is stale, the suite silently tests the *previous* binary. Delete
> `$(node -p "require('os').tmpdir()")/.taskpad-rs-e2e-built` to force a rebuild.

`Cargo.toml` denies `clippy::unwrap_used`/`clippy::expect_used` and forbids `unsafe_code` crate-wide; the single
`#![cfg_attr(test, allow(...))]` at the top of `src/main.rs` is what keeps the `#[cfg(test)]`
modules legal. `clippy::pedantic` is deliberately *not* enabled.

## C++ parity

The C++ `taskpad` is the ground truth for every user-visible byte: config format, `status.yaml`
schema (including `depends: ~` for an empty list and the absence of a trailing newline), the
`T*.md` template, all thirteen subcommands, all stdout/stderr wording, and all exit codes.
When this README, the code, and the C++ disagree, the C++ wins.

- `specs/spec.main.md` — the behavioral contract.
- `AGENTS.md` → **"Known spec ↔ C++ conflicts"** — the 18 discrepancies that were found and ruled
  on (empty `depends` as `~` vs `[]`, `do` treating unmet deps as an error rather than a warning,
  `pause` also reverting `done` tasks, the exact circular-dependency wording, and so on). Read
  this before concluding that a behavior here is a bug.

### Intentional hardening deviations

Beyond byte-for-byte parity, the port added a small number of **deliberate divergences** — cases
where the C++ behavior is lossy or unsafe and reporting it correctly was judged worth a
different message. All are localized, documented at the code site, and covered by tests.

| Deviation | What C++ does | What this port does |
| --- | --- | --- |
| **Atomic writes** — `status.yaml`, `.taskpad`, `T*.md` | Writes in place; a reader (or a crash) can observe a truncated file. | Every writer stages content in a temp file beside the target and `rename`s it over. Readers see either the old file or the complete new one. Error messages are unchanged (`Cannot write to <path>. Check permissions`). |
| **Read errors vs. missing files** | Maps *every* read failure onto its "not found" message. | Only `ErrorKind::NotFound` produces `Not initialized. Run 'taskpad init' first` / `No status.yaml found. …`. Any other failure reports `Cannot read <path>: <reason>` (e.g. permission denied, a directory in the file's place, invalid UTF-8). |
| **Whitespace-only task names** | `taskpad new "   "` succeeds; the file is written as `T001.md`, which `taskpad import` then never picks up (it derives ids from `T<NNN>-<name>.md`). | Rejected with the existing `Task name cannot be empty` message — no new user-visible string. |
| **`import` on an unreadable `status.yaml`** | Cannot distinguish "absent" from "unreadable", so it starts from empty and overwrites the file it failed to read. | Aborts and reports the real error, leaving the file on disk untouched. Only a genuinely absent `status.yaml` is treated as "no project yet". |
| **ASCII-only whitespace scanning** in `extract_phase` / `extract_critical` | Byte scanner; a multibyte char between `## Phase:` and the number ends the scan (yields `0`/`false`). | Same result, reached by scanning bytes instead of char boundaries, so an NBSP or emoji can never panic a mid-codepoint slice. |
| **Strict `status.yaml` shape** | A `tasks:` node of the wrong type (e.g. `tasks: 5`) is tolerated and treated as an empty project. | Rejected with `Invalid status.yaml format. Expected YAML mapping` (exit 1). Accepted divergence — see `AGENTS.md` conflicts table, row 18. |

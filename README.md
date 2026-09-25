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

## Building from Source

Requires a stable Rust toolchain ([rustup](https://rustup.rs)). No system packages are needed — the Rust port has no C++ toolchain and no external library dependencies; everything comes from crates.io. Node.js is only needed to run the E2E test suite.

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

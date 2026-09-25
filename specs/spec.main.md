# taskpad (Rust) — Main Specification

> This is a **rewrite spec**, not a from-scratch spec. taskpad already exists as a C++17
> binary (see the [original C++ spec](https://github.com/instrumaniak/taskpad/blob/main/specs/spec.main.md)
> for reference). This document governs a **Rust port** of that same tool.
>
> **Compatibility invariant:** Sections 1–6 below (Overview, Architecture, Data Model, File
> Formats, CLI Commands, Error Handling) describe *user-visible behavior* — the on-disk
> formats and CLI contract — and MUST NOT change from the C++ version. A project that used
> the C++ binary must be able to switch to the Rust binary with zero changes to its
> `.taskpad`, `status.yaml`, or `T*.md` files, and zero changes to muscle-memory command
> usage. Only Sections 7–8 (Coding Style, Implementation Details) differ, because those are
> about how the Rust binary is built, not what it does.

## 1. Overview

### Purpose

taskpad is a lightweight, deterministic task management CLI tool designed for AI-assisted
development workflows. It provides a single source of truth for task status while keeping
task descriptions as plain Markdown files.

### Design Principles

- **Single source of truth**: All task metadata lives in one file (`status.yaml`). No
  duplication across files.
- **No frontmatter**: Task description files (`T*.md`) are pure Markdown with no
  YAML/JSON frontmatter.
- **Human-friendly**: Clear output, predictable behavior, no crashes on edge cases.
- **Deterministic**: Same inputs always produce same outputs. No randomness, no side effects.
- **Zero privileges**: Installation requires no root/sudo access.
- **Minimal config**: `.taskpad` stores only project-level settings (task directory path).

### Use Cases

- Solo developers tracking implementation tasks
- Small teams tracking implementation tasks
- Projects with sequential task execution (one task at a time)
- Any workflow where task status needs to be machine-readable and human-editable

---

## 2. Architecture

### File Structure in Target Project

```
project-root/
├── .taskpad                     # Project config (YAML, created by init)
├── specs/
│   └── tasks/
│       ├── status.yaml          # Single source of truth (all metadata)
│       ├── T001-project-setup.md  # Task description (plain markdown)
│       ├── T002-asset-acquisition.md
│       └── ...
└── ...
```

### .taskpad Config File

A minimal YAML config at the project root, created by `taskpad init`:

```yaml
# .taskpad — taskpad project config
task-dir: specs/tasks    # Path to task files directory (default: specs/tasks)
```

- Minimal by design: only `task-dir` for now; future settings go here
- Task directory resolution order is defined in [Global Flags](#global-flags)

### How CLI Interacts with Files

```
User
    │
    ▼
┌─────────────────────┐
│  taskpad CLI        │
│  (reads/writes)     │
└─────────┬───────────┘
          │
          ├──► .taskpad        (config: task-dir)
          │
          ├──► status.yaml    (metadata: status, deps, phases)
          │
          └──► T*.md          (descriptions: goal, steps, criteria)
```

### Data Flow

1. `taskpad init` — Creates `.taskpad` config file
2. `taskpad import` — Scans `task-dir` for existing T*.md files, creates `status.yaml`
3. `taskpad new <name>` — Creates `T*.md` file + adds entry to `status.yaml`
4. `taskpad do <id>` — Updates `status.yaml` (pending → in_progress)
5. `taskpad done <id>` — Updates `status.yaml` (in_progress → done)
6. `taskpad status` — Reads `status.yaml`, displays formatted table
7. `taskpad next` — Reads `status.yaml`, finds next unblocked task

---

## 3. Data Model

### Status Enum

Rust equivalent of the C++ `enum class Status`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pending,
    InProgress,
    Done,
}
```

**Status mapping (unchanged from C++ version):**

| Enum Variant  | YAML Value    | Display          |
|---------------|---------------|-------------------|
| `Pending`     | `pending`     | `[pending]`       |
| `InProgress`  | `in_progress` | `[in_progress]`   |
| `Done`        | `done`        | `[done]`          |

Implement `Display`/`FromStr` (or plain `status_to_string`/`string_to_status` functions,
matching the C++ naming if a 1:1 port is preferred) rather than deriving `serde`'s default
enum representation directly — `status.yaml` uses lowercase/snake_case values
(`in_progress`, not `InProgress`), which needs an explicit `#[serde(rename = "...")]` per
variant or a manual `Serialize`/`Deserialize` impl either way.

### Task ID Format

Task IDs follow the pattern `T[0-9]{3}` (T001 through T999, zero-padded 3-digit numbers).
This pattern is used in:
- File names: `T001-project-setup.md`
- `status.yaml` keys: `T001:`
- Dependency references: `[T001, T002]`

### Task Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(skip)] // id is the map key in StatusFile.tasks, not a field in the YAML node itself
    pub id: String,
    pub name: String,
    pub status: Status,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub phase: i32,
    #[serde(default)]
    pub critical: bool,
}
```

Note the C++ version stores `id` as a field on `Task` even though it's redundant with the
`status.yaml` map key — the Rust port should decide once, early (in the storage module task),
whether to keep that redundancy for parity with the C++ struct or drop `id` from `Task` and
thread it separately as the map key. Either is acceptable; document the choice in
`storage.rs`'s module doc comment since it affects every command that touches `Task`.

### ProjectConfig Struct

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub phases: BTreeMap<i32, String>,       // ordered, unlike a HashMap — status/summary output must be phase-ordered
    #[serde(default)]
    pub critical_path: Vec<String>,
}
```

Use `BTreeMap`, not `HashMap`, for `phases` — `taskpad status` and `taskpad summary` must
print phases in ascending numeric order, and `BTreeMap` gives that for free without a sort
step. The C++ version uses `std::map<int, std::string>` for the same reason.

### Error / Result Type

The C++ version uses a hand-rolled `Result<T>` struct (`value` + `error` string) because
C++ has no built-in tagged union ergonomic enough for this. **Rust already has this** —
use `std::result::Result<T, E>` directly rather than porting the wrapper struct. Recommended
shape:

```rust
#[derive(Debug, thiserror::Error)]
pub enum TaskpadError {
    #[error("{0}")]
    Message(String), // escape hatch for the many ad-hoc error strings in the C++ version
    // add structured variants opportunistically during the port (e.g. TaskNotFound(String))
    // where a caller needs to branch on the error kind, not just print it.
}

pub type Result<T> = std::result::Result<T, TaskpadError>;
```

Every C++ function returning `Result<void>` becomes a Rust function returning
`Result<()>`. This is a direct, mechanical mapping — no behavior changes, just idiom.

---

## 4. File Formats

*(Unchanged from the C++ version — this is the compatibility contract. A file written by
the C++ binary must be readable by the Rust binary and vice versa.)*

### status.yaml Schema

```yaml
# taskpad status file — single source of truth
# Generated by taskpad init or taskpad new

tasks:
  T001:
    name: "Project Setup"
    status: pending          # pending | in_progress | done
    depends: []              # List of task IDs this depends on
    phase: 0                 # Phase number (non-negative integer)
    critical: true           # Part of critical path

  T002:
    name: "Asset Acquisition"
    status: pending
    depends:
      - T001
    phase: 0
    critical: false

# Project metadata
phases:
  0: "Scaffolding"
  1: "Foundation Types"

critical_path:
  - T001
  - T003
```

### T*.md Template

```markdown
# T001: Project Setup

## Goal

Set up project structure, directory tree, and a stub entry point.

## Depends On

(None)

## Spec References

- `spec.main.md` → relevant section(s)

## Files to Create/Modify

- `src/main.rs` (NEW or MODIFY)

## Implementation Steps

1. Create the directory tree under the project root
2. ...

## Constraints

- ...

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo run -- status` produces expected output

## Notes

( filled in during/after implementation )
```

`taskpad import` additionally recognizes two optional metadata lines inside a T*.md file,
which is how phase/critical get picked up without a `status.yaml` entry existing yet:

```markdown
## Phase: 2

## Critical: true
```

### Directory Structure Conventions

- Task files: `TXXX-kebab-case-name.md` (e.g., `T001-project-setup.md`)
- Status file: `status.yaml` (always in same directory as task files)
- Task IDs: see [Task ID Format](#task-id-format) in Section 3

---

## 5. CLI Commands

*(Behavior unchanged from the C++ version — every example below must produce byte-identical
stdout from the Rust binary.)*

### Global Flags

| Flag | Description |
|------|-------------|
| `--tasks-dir <path>` | Override `task-dir` from `.taskpad` config |
| `--help` | Show help for command |

Task directory resolution order:
1. `--tasks-dir` flag (if provided)
2. `task-dir` from `.taskpad` config
3. Fallback: `specs/tasks/`

**Note:** CLI flag uses plural `tasks-dir`, config key uses singular `task-dir`.

### Command Summary

| Command | Behavior |
|---|---|
| `taskpad init` | Creates `.taskpad` config. Errors "Already initialized" if one exists. Does NOT create `status.yaml`. |
| `taskpad import [--force]` | Scans `task-dir` for `T[0-9]{3}-*.md`, parses `## Status:`/`## Depends On`/`## Phase:`/`## Critical:`, builds `status.yaml`. Errors if `status.yaml` exists, unless `--force`. Validates no circular dependencies. |
| `taskpad new <name> [--depends TXXX]... [--phase N] [--critical]` | Auto-increments task ID, kebab-cases the name for the filename, writes the T*.md template (no `## Status:` line), adds a `status.yaml` entry. Validates no circular dependencies. |
| `taskpad status` | Groups tasks by phase, shows `[status]`, marks `← next` / `← blocked by TXXX`, shows a progress summary line. |
| `taskpad next` | Finds pending tasks with all dependencies done; prioritizes critical path → phase → task number; reads goal + first implementation step from the T*.md file; "All tasks blocked or complete" if none. |
| `taskpad do <id> [--force]` | pending → in_progress. Warns (not errors) on unmet deps unless `--force`. "Already in_progress"/"Already done" if not pending. |
| `taskpad done <id>` | in_progress or pending → done. Lists newly-unblocked tasks. "Already done" if already done. |
| `taskpad pause <id>` | in_progress → pending. "Already pending" if already pending. |
| `taskpad deps <id>` | Shows what the task depends on (✓/✗ per dependency) and what depends on it. |
| `taskpad log <id> <message>` | Appends `- [YYYY-MM-DD HH:MM] <message>` to the `## Notes` section of the T*.md file (creating the section if absent). |
| `taskpad edit [<id>] --status/--phase/--critical/--no-critical/--depends...` (task-level) `--phases/--critical-path` (project-level) | Updates the relevant `status.yaml` field(s). Task-level requires `<id>`; project-level does not. Validates circular deps when `--depends` changes. |
| `taskpad summary` | Totals + percentages, per-phase done/total, critical path status. |
| `taskpad remove <id> [--all] [--force]` | Removes the `status.yaml` entry (and from `critical_path` if present). Prompts `[y/N]` unless `--force`. `--all` also deletes the `.md` file. Warns if other tasks depend on it. |

For exact output formatting of every command (column widths, arrows, symbols, punctuation),
treat the C++ source (`src/commands.cpp`) and the E2E test assertions
(`tests/e2e/*.mjs`) as the source of truth — they encode the literal strings this spec
paraphrases. Reproducing that output byte-for-byte is what makes the E2E suite reusable
unmodified against the Rust binary; see `spec.testing.md`.

---

## 6. Error Handling

### Error Format

```
error: <message>       # Fatal — command cannot proceed, exit code 1
warning: <message>     # Non-fatal — may affect results, exit code 0
info: <message>        # Informational — no action needed, exit code 0
```

No panics, no stack traces, no `unwrap()`-induced crashes reaching the user. Every error
path in a command handler must resolve to a `Result::Err(TaskpadError)` printed as
`error: {msg}` to stderr with exit code 1 — never a Rust panic.

### Edge Cases

| Case | Behavior |
|------|----------|
| Task ID not found | `error: Task T999 not found` |
| Invalid task ID format | `error: Invalid task ID format. Expected TXXX (see Task ID Format in Section 3)` |
| Task already in_progress | `error: Task T003 already in_progress` |
| Task already done | `error: Task T003 already done` |
| Dependencies not met | `warning: Task T003 depends on T002 (pending). Use --force to proceed` |
| Circular dependency | `error: Circular dependency detected: T003 → T005 → T003` |
| `.taskpad` missing | `error: Not initialized. Run 'taskpad init' first` |
| `status.yaml` missing | `error: No status.yaml found. Run 'taskpad import' or 'taskpad new' first` |
| `status.yaml` malformed | `error: Invalid status.yaml format. Expected YAML mapping` |
| T*.md missing | `warning: Task file T003-core-types.md not found. Creating template` |
| No tasks available | `info: All tasks blocked or complete` |
| Directory not writable | `error: Cannot write to specs/tasks/. Check permissions` |
| Empty task name | `error: Task name cannot be empty` |
| Duplicate task name | `warning: Task with similar name exists: T005-layout-system.md` |
| `.taskpad` already exists | `error: Already initialized. Remove .taskpad to re-initialize` |
| `status.yaml` already exists (import) | `error: status.yaml already exists. Use --force to overwrite` |
| Invalid task-dir path | `error: Task directory 'xyz' not found. Check task-dir in .taskpad` |

### Validation Rules

- Task IDs must match pattern `T[0-9]{3}`
- Task names must be non-empty
- Dependencies must reference existing task IDs
- No circular dependencies allowed (validated on `import`, `edit --depends`, `new --depends`)
- Status values must be one of: `pending`, `in_progress`, `done`
- Phase numbers must be non-negative integers

---

## 7. Coding Style (Rust)

This replaces the C++ version's coding-style section — the Rust rewrite should read as
idiomatic Rust, not a transliteration of C++ idioms.

### Formatting & Linting

- `cargo fmt` (default settings) is the formatter of record — no manual formatting debates.
- `cargo clippy --all-targets -- -D warnings` must be clean before a task is considered done.
- No `unsafe` blocks anywhere in this codebase — there is no scenario in a YAML-and-Markdown
  CLI tool that needs it.

### Naming Conventions (standard Rust, not the C++ table)

| Construct | Convention | Example |
|---|---|---|
| Modules/files | `snake_case` | `storage.rs`, `commands/mod.rs` |
| Functions/methods | `snake_case` | `find_next_task()`, `to_kebab_case()` |
| Types/structs/enums | `PascalCase` | `Task`, `ProjectConfig`, `TaskpadError` |
| Enum variants | `PascalCase` | `Status::InProgress` |
| Variables/params | `snake_case` | `task_id`, `status` |
| Constants | `SCREAMING_SNAKE_CASE` | `DEFAULT_TASK_DIR` |
| Booleans | `is_`/`has_`/`can_` prefix | `is_done`, `has_circular_dep` |

### Error Handling

- No `panic!`, `unwrap()`, or `expect()` in library/command code reachable from user input.
  `unwrap()`/`expect()` are acceptable only in tests, or on invariants that are genuinely
  unreachable (document why at the call site with a comment if so).
- Public functions return `Result<T, TaskpadError>` (see §3). Bubble errors with `?` rather
  than manual matching where possible.
- All errors printed as `error: <message>` to stderr, exactly as in the C++ version.

### Type Inference

Unlike the C++ spec's "Explicit Types Rule" (which forbids `auto` outside a few cases),
**idiomatic Rust leans the other way**: let type inference do its job for local bindings
(`let tasks = HashMap::new();`), and reserve explicit types for function signatures, struct
fields, and anywhere inference would be ambiguous or the type is part of the public API.
This is a deliberate divergence from the C++ spec, not an oversight — porting the C++ rule
literally would fight the language.

### Module Organization

- One `.rs` file per C++ `.h`/`.cpp` pair, same name (`models.rs`, `storage.rs`, `utils.rs`,
  `validator.rs`), unless a module grows large enough to become its own directory with a
  `mod.rs` (this applies to `commands/`, see §8).
- `pub` only what other modules actually need; keep helper functions private (`fn`, not
  `pub fn`) exactly as the C++ version uses anonymous-namespace/`static` helpers in
  `commands.cpp`.
- No `use foo::*` glob imports outside test modules.

### Anti-patterns

| Forbidden | Use instead |
|---|---|
| `unwrap()`/`expect()` on user-input-derived `Result`/`Option` outside tests | `?` propagation, or explicit `match`/`if let` with a `TaskpadError` |
| `unsafe` | not needed anywhere in this codebase |
| Stringly-typed status/error kinds where a caller branches on them | `enum` variants |
| `use module::*` outside `#[cfg(test)]` | explicit imports |

---

## 8. Implementation Details (Rust)

This replaces the C++ version's Implementation Details section (Makefile, CLI11/yaml-cpp,
`src/*.cpp` layout) with the Rust equivalent.

### Crate Dependencies

| Crate | Purpose | Notes |
|---|---|---|
| `clap` (`derive` feature) | CLI argument parsing | Replaces CLI11. Subcommands as a `#[derive(Subcommand)]` enum; `--depends` (repeatable) as `Vec<String>` with `#[arg(long)]`. |
| `serde` (`derive` feature) | Struct ↔ YAML mapping | Replaces the manual `YAML::Node` tree-building in the C++ `storage.cpp` — derive `Serialize`/`Deserialize` on `Task`/`StatusFile`/`ProjectConfig` instead. |
| a maintained YAML crate — evaluate `serde-saphyr` or `noyalib` at implementation time | YAML parse/emit | **Do not use `serde_yaml`** — it and its direct forks are unmaintained/deprecated (RUSTSEC-2025-0068 territory). Check crates.io for the current maintained recommendation before pinning a version, since this space has been in flux. |
| `thiserror` | Error enum boilerplate | Backs `TaskpadError` (§3). |
| — (std only) | File I/O, path handling | `std::fs`, `std::path::{Path, PathBuf}` — no need for a filesystem crate beyond std for this tool's needs. |

Deliberately **not** using: `nlohmann-json`'s Rust equivalent (`serde_json`) — the C++
version lists `nlohmann-json3-dev` as a dependency but never actually uses it (confirmed:
no `#include <nlohmann/json.hpp>` anywhere in `src/`). Don't carry over an unused dependency.

### Crate Layout

```
taskpad/
├── Cargo.toml
├── src/
│   ├── main.rs            # ~5 lines: parse CLI, dispatch, map Result<()> to process::exit code
│   ├── cli.rs              # clap derive definitions (subcommands, flags)
│   ├── models.rs            # Status, Task, ProjectConfig, StatusFile, TaskpadError, Result
│   ├── storage.rs           # .taskpad + status.yaml + T*.md read/write, log append
│   ├── utils.rs             # kebab-case, task-id parse/format, path normalize, timestamp,
│   │                        # T*.md metadata extraction (extract_phase, extract_critical, ...)
│   ├── validator.rs         # ID/status validation, circular-dependency detection
│   └── commands/
│       ├── mod.rs           # re-exports + shared helpers (e.g. find_next_task_id, all_deps_done)
│       ├── init.rs
│       ├── import.rs
│       ├── new.rs
│       ├── status.rs
│       ├── next.rs
│       ├── do_cmd.rs        # `do` is a Rust keyword — module can't be named `do`
│       ├── done.rs
│       ├── pause.rs
│       ├── deps.rs
│       ├── log.rs
│       ├── edit.rs
│       ├── summary.rs
│       └── remove.rs
└── tests/
    ├── helpers.mjs           # unchanged from the C++ repo, except the built-binary path
    └── e2e/                  # unchanged from the C++ repo
```

This is a deliberate split of the C++ version's single 1,189-line `commands.cpp` into one
module per subcommand — each C++ `Commands::xxx` static method becomes a `pub fn run(...)`
in its own file. `commands/mod.rs` holds only what genuinely needs to be shared across two
or more command modules (the private helper functions currently sitting at the top of
`commands.cpp`: `find_next_task_id`, `all_deps_done`, `kebab_to_title`, etc.) — don't let it
become a dumping ground.

### Build & Install

Cargo replaces the Makefile entirely — no custom build rules are needed for a project this
shape:

```bash
cargo build --release        # replaces `make`
cargo test                   # replaces `make test` (runs #[test] fns across all modules)
node --test tests/e2e/*.mjs  # replaces `make e2e-test` — unchanged
cargo install --path .       # replaces `make install` (installs to ~/.cargo/bin by default)
```

No `make check` equivalent needs to be hand-written if a `check` alias or a tiny justfile
target is added later — the two commands above cover it. Not required for the port itself.

### Error Handling Strategy

- All fallible functions return `Result<T, TaskpadError>` (§3) — no panics reach the user.
- `main.rs` is the single place that converts an `Err` into an `error: {msg}` stderr line
  and a non-zero exit code; command modules never call `process::exit` themselves.

---

## 9. Testing

Two-tier approach, unchanged in spirit from the C++ version:
- **Unit tests**: Rust `#[test]` (in-process, fast)
- **E2E tests**: Node.js `node:test` (binary-as-black-box via CLI) — **reused unmodified**
  from the C++ repo

See [specs/spec.testing.md](spec.testing.md) for the complete testing specification.

---

## 10. Distribution

### Installation

User-level installation, no privileges required — same principle as the C++ version, via
`cargo install` instead of a Makefile `install` target.

```bash
cd /path/to/taskpad
cargo install --path .
# Binary installed to ~/.cargo/bin/taskpad (ensure that's on PATH)
```

### Dependencies

None beyond a stable Rust toolchain (`rustup` / `cargo`) — no system packages to
`apt install`, which is itself a small usability improvement over the C++ version's
`libcli11-dev`/`libyaml-cpp-dev`/etc. requirement. Worth calling out in the port's README.

### Versioning

Follow semantic versioning: `MAJOR.MINOR.PATCH`, tracked in `Cargo.toml`'s `version` field.
Start the Rust port at `1.0.0` to signal behavioral parity with the C++ `1.0.0`, not `0.1.0`
— it is a port, not a new tool.

---

## Appendix A: Example Workflow

Identical to the C++ version — these are user-facing command sequences, not implementation
details:

```bash
# Fresh project
cd /path/to/my-project
taskpad init
taskpad new "Project Setup"
taskpad next
taskpad do T001
taskpad done T001
taskpad status

# Existing project with T*.md files already present
cd /path/to/existing-project
taskpad init
taskpad import
taskpad next
```

## Appendix B: File Naming Conventions

- Task files: `TXXX-kebab-case-name.md` (e.g., `T001-project-setup.md`)
- Status file: `status.yaml` (always lowercase)
- Maximum 999 tasks per project

## Appendix C: YAML Schema Validation

Unchanged from the C++ version — see §4 for the full schema. Validation rules are listed in
§6.

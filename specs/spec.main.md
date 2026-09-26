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

A minimal YAML config at the project root, created by `taskpad init`. The writer emits
exactly these bytes (one comment line, no inline comment, single trailing LF):

```yaml
# taskpad project config
task-dir: specs/tasks
```

(`specs/tasks` shown is the default value; there is no inline comment in the real file.)

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
`status.yaml` map key. **Decision (locked):** keep `id` as a field with `#[serde(skip)]` —
it is never serialized; `storage::read_status_file` re-threads the `status.yaml` map key
into `id` after deserialization, so every command sees the same `Task` shape as the C++
struct. Document this in `models.rs`'s module doc comment (T002), with a cross-reference
from `storage.rs`'s module doc comment (T004) — it affects every command that touches
`Task`, and a bare `Task` (not embedded in `StatusFile`) will deserialize with `id == ""`.

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

This is what the binaries **write** (byte-level, from the C++ `YAML::Dump` emitter —
the Rust writer must match):

```yaml
tasks:
  T001:
    name: Project Setup
    status: pending
    depends: ~
    phase: 0
    critical: true
  T002:
    name: Asset Acquisition
    status: pending
    depends:
      - T001
    phase: 0
    critical: false
phases:
  0: Scaffolding
critical_path:
  - T001
```

Field semantics are defined in §3 (Status Enum, Task Struct, ProjectConfig Struct).
Writer facts, byte level:

- No header comments, no inline comments, no blank lines anywhere.
- Empty `depends` is emitted as `~` (YAML null), never `[]`.
- `phases` and `critical_path` keys are omitted entirely when empty.
- No trailing newline at EOF (the C++ emitter adds none — a reader tolerant of both).
- Key order is fixed: `tasks` → `phases` → `critical_path`; per task `name` → `status` →
  `depends` → `phase` → `critical`. Task keys are lexicographic (safe: zero-padded IDs).
- `name` scalars are plain unless the emitter must quote them (e.g. an interior `&`).

Reader tolerance (both binaries must accept everything below):

- Header comments — e.g. `# taskpad status file`, which the E2E fixtures in
  `tests/common/mod.rs` write and older copies of this spec showed.
- `depends: []`, `depends: ~`, and flow style `depends: [T001]`.
- Quoted or plain `name` values; missing optional fields (defaults: name `""`, status
  `pending`, depends `[]`, phase `0`, critical `false`).
- A non-mapping root → `error: Invalid status.yaml format. Expected YAML mapping` (§6).

### T*.md Template

What `taskpad new` writes — the exact bytes from the C++ `writeTaskFile`
(`storage.cpp:226-267`), with `<ID>`/`<Name>` substituted. Single trailing newline;
no `## Status:` line (status lives only in `status.yaml`):

```markdown
# <ID>: <Name>

## Goal

(Describe the goal)

## Depends On

(None)

## Phase:

(Add phase number here)

## Critical:

(Add critical flag here)

## Spec References

- (Add spec references here)

## Files to Create/Modify

- (Add files here)

## Implementation Steps

1. (Add steps here)

## Constraints

- (Add constraints here)

## Acceptance Criteria

- [ ] (Add criteria here)

## Notes

(filled in during/after implementation)
```

`taskpad import` additionally recognizes optional metadata lines inside a T*.md file,
which is how status/phase/critical get picked up without a `status.yaml` entry existing
yet:

```markdown
## Status: done

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
| `taskpad import [--force]` | Scans `task-dir` for `T[0-9]{3}-*.md`, parses `## Status:`/`## Depends On`/`## Phase:`/`## Critical:`, builds `status.yaml`. Errors if `status.yaml` exists, unless `--force`. Validation problems (missing deps, cycles) are printed as `warning:` lines on stderr, status.yaml is **still written**, exit code 0. |
| `taskpad new <name> [--depends TXXX]... [--phase N] [--critical]` | Auto-increments task ID, kebab-cases the name for the filename, writes the T*.md template (no `## Status:` line), adds a `status.yaml` entry. Validates no circular dependencies. |
| `taskpad status` | Groups tasks by phase, shows `[status]`, marks `← next (dependencies met)` / `← blocked by TXXX`, shows a progress summary line. |
| `taskpad next` | Finds pending tasks with all dependencies done; prioritizes critical path → phase → task number; reads the T*.md for `Goal:`/`First step:` plus `Depends on:`, `Files:`, `Specs:` lines when present; "All tasks blocked or complete" if none. |
| `taskpad do <id> [--force]` | pending → in_progress. Unmet dependencies without `--force` are an **error** (exit 1): `Unmet dependencies: T002 (pending). Use --force to proceed`. "Already in_progress"/"Already done" also errors if not pending. Prints the status transition, then `Now reading …`/`Goal:`/`First step:` detail if the T*.md exists (silently omitted if not). |
| `taskpad done <id>` | in_progress or pending → done. Lists newly-unblocked tasks. "Already done" if already done. |
| `taskpad pause <id>` | any non-pending status → pending (the C++ version only rejects *already pending*; even `done` may be paused back to `pending`). "Already pending" is an error. |
| `taskpad deps <id>` | Shows what the task depends on (✓/✗ per dependency) and what depends on it. |
| `taskpad log <id> <message>` | Appends `- [YYYY-MM-DD HH:MM] <message>` to the `## Notes` section of the T*.md file (creating the section if absent). |
| `taskpad edit [<id>] --status/--phase/--critical/--no-critical/--depends...` (task-level) `--phases/--critical-path` (project-level) | Updates the relevant `status.yaml` field(s). Task-level requires `<id>`; project-level does not. Validates circular deps when `--depends` changes. |
| `taskpad summary` | Totals + percentages, per-phase done/total, critical path status. |
| `taskpad remove <id> [--all] [--force]` | Removes the `status.yaml` entry (and from `critical_path` if present). Prompts `[y/N]` unless `--force`. `--all` also deletes the `.md` file. Warns if other tasks depend on it. |

For exact output formatting of every command (column widths, arrows, symbols, punctuation),
treat the C++ source (`src/commands.cpp`) and the E2E test assertions
(`tests/e2e/*.rs`) as the source of truth — they encode the literal strings this spec
paraphrases. Reproducing that output byte-for-byte is what lets the E2E suite verify the
Rust binary; see `spec.testing.md`.

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
| Invalid task ID format | `error: Invalid task ID format. Expected TXXX (see Task ID Format)` |
| Task already in_progress | `error: Task T003 already in_progress` |
| Task already done | `error: Task T003 already done` |
| Task already pending (`pause`) | `error: Task T003 already pending` |
| Dependencies not met (`do` without `--force`) | `error: Unmet dependencies: T002 (pending). Use --force to proceed`, exit 1 |
| Circular dependency | `error: Circular dependency detected: T003 → ... → T005` (literal `→ ... →`), or `error: Circular dependency detected: T003 depends on itself` for self-cycles. On `import` instead: `warning: N issue(s) found` + indented issue lines on stderr, status.yaml still written, exit 0. `new --depends`/`edit --depends` treat it as an error (exit 1). |
| `.taskpad` missing | `read_task_dir` returns `error: Not initialized. Run 'taskpad init' first`, but command flows reach it only through `resolve_task_dir`, which silently falls back to `specs/tasks/` — so outside a project the first error seen is the missing-`status.yaml` error below, not this one |
| `.taskpad` malformed | `error: Invalid .taskpad format. Expected YAML mapping with 'task-dir'` (not a mapping / no `task-dir` key) or `error: Invalid .taskpad format: <parser message>` (parse failure) |
| `status.yaml` missing | `error: No status.yaml found. Run 'taskpad import' or 'taskpad new' first` |
| `status.yaml` malformed | `error: Invalid status.yaml format. Expected YAML mapping` (non-mapping root) or `error: Invalid status.yaml format: <parser message>` (parse failure) |
| T*.md missing | `next`/`do` silently omit the file-derived detail (`Goal:`/`First step:`/`Files:`/`Specs:`) — no warning, no template creation; `log` fails with `error: Task file <path> not found` |
| No tasks available | `info: All tasks blocked or complete` |
| Directory/file not writable | `error: Cannot write to <path>. Check permissions` where `<path>` is the full file path being written (`.taskpad`, `status.yaml`, or the T\*.md) |
| Empty task name | `error: Task name cannot be empty` |
| Duplicate task name | `warning: Task with similar name exists: T005-layout-system.md` |
| Empty log message | `error: Log message cannot be empty` |
| `.taskpad` already exists | `error: Already initialized. Remove .taskpad to re-initialize` |
| `status.yaml` already exists (import) | `error: status.yaml already exists. Use --force to overwrite` |
| Invalid task-dir path | no directory-existence check happens — the configured/passed value is used as-is and typically fails later with `error: No status.yaml found. …` |

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
| `serde-saphyr` (1.x) | YAML parse/emit | **Chosen at the planning session (locked — see AGENTS.md)**; replaces yaml-cpp. serde-compatible, maintained, panic-free parsing. **Do not use `serde_yaml`** — it and its direct forks are unmaintained/deprecated (RUSTSEC-2025-0068 territory). |
| `thiserror` | Error enum boilerplate | Backs `TaskpadError` (§3). |
| `tempfile` (**dev-dependency** only) | Isolated temp dirs | Required by `spec.testing.md` §2 for `storage.rs` unit tests, and by §3 for the E2E tier's per-test project fixtures. Added to `Cargo.toml` in T001. |
| — (std only) | File I/O, path handling | `std::fs`, `std::path::{Path, PathBuf}` — no need for a filesystem crate beyond std for this tool's needs. |
| — (std only) | E2E process + assertion layer | `std::process::Command`, `env!("CARGO_BIN_EXE_taskpad")`, and the three assertion helpers in `tests/common/mod.rs`. The E2E tier deliberately adds **no** dev-dependency: see the note below. |

The crate list above is closed: no `regex` (port the C++ manual scanners — see AGENTS.md
Locked decision §1), no other additions without a human ruling.

**The E2E tier uses no test framework crate** (ruled 2026-09-26, T023). `assert_cmd` +
`predicates` is the common ecosystem default for CLI integration tests, and ripgrep, fd and
bat all appear in that ecosystem — but `assert_cmd`'s `.assert()` runs the binary *eagerly*
(these tests run once and then assert several things, including reading `status.yaml`
back), and its predicates match *untrimmed* bytes (this suite compares trimmed output).
Adopting it would mean bypassing its main API and hand-writing a trim predicate anyway, on
top of 8 transitive dependencies. ripgrep, fd and fulgur-cli all hand-roll the equivalent
helpers over `Command::output()` instead, which is what `tests/common/mod.rs` does. `regex`
stays banned; the 14 cross-line assertions in the suite are covered by a
`contains_after`-style helper.

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
    ├── common/mod.rs         # shared E2E helpers (Project, Output, fixture builders, asserts)
    └── e2e/                  # one module per command area, transcribed from the C++ suite
        ├── main.rs           # the test target root — `mod` declarations + shared-helper include
        ├── init.rs
        ├── import.rs
        ├── new.rs
        ├── status.rs
        ├── next.rs
        ├── do_cmd.rs
        ├── done.rs
        ├── pause.rs
        ├── deps.rs
        ├── log.rs
        ├── edit.rs
        ├── summary.rs
        └── remove.rs
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
cargo test                   # replaces `make test` + `make e2e-test` — runs both the
                             # in-crate #[test] fns and the tests/e2e/ integration tier
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
- **E2E tests**: Rust integration tests under `tests/e2e/` (binary-as-black-box via
  `std::process::Command`), transcribed assertion-for-assertion from the C++ repo's
  `node:test` suite — the literals they assert are the C++ literals

Both tiers run under a single `cargo test`. See
[specs/spec.testing.md](spec.testing.md) for the complete testing specification.

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

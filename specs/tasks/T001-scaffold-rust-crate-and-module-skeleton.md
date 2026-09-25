# T001: Scaffold Rust Crate And Module Skeleton

## Goal

Create the Rust binary crate skeleton with the module layout and dependencies defined in
`spec.main.md` §8, so every later task has a compiling (if empty) target to build against.

## Depends On

(None)

## Spec References

- `spec.main.md` → §8 Crate Dependencies, Crate Layout, Build & Install

## Phase: 0

## Critical: true

## Files to Create/Modify

- `Cargo.toml` (CREATE)
- `src/main.rs` (CREATE) — stub, prints nothing yet or a placeholder
- `src/cli.rs` (CREATE) — empty module
- `src/models.rs` (CREATE) — empty module
- `src/storage.rs` (CREATE) — empty module
- `src/utils.rs` (CREATE) — empty module
- `src/validator.rs` (CREATE) — empty module
- `src/commands/mod.rs` (CREATE) — empty module
- `.gitignore` (CREATE) — exclude `/target`

## Implementation Steps

1. Run `cargo init --name taskpad` (or hand-write `Cargo.toml` if starting from an existing
   directory) to produce the crate skeleton.
2. Add dependencies to `Cargo.toml`: `clap` (derive feature), `serde` (derive feature),
   `serde-saphyr` (the YAML crate chosen per `spec.main.md` §8 — do not use `serde_yaml`),
   and `thiserror`; plus `tempfile` as a **dev-dependency** (T004's unit tests and
   `spec.testing.md` §2 require `tempfile::tempdir()`, and `Cargo.toml` is in this task's
   file list — see AGENTS.md Locked decision §1). No other crates: the §8 crate list is
   closed (no `regex`, no `serde_json`).
3. Create empty `mod` declarations in `main.rs` for `cli`, `models`, `storage`, `utils`,
   `validator`, `commands` so the crate compiles with `cargo build` even before any logic
   is ported.
4. Create the `src/commands/` directory with an empty `mod.rs` (the per-command files land
   there in later tasks — do not create them yet).
5. Add `.gitignore` excluding `/target`.

## Constraints

- Do not port any actual logic in this task — it is scaffolding only. Every module should
  compile empty.
- Pin dependency versions loosely (e.g. `clap = "4"`) rather than exact patch versions,
  matching normal Rust practice, unless a specific version is needed to avoid a known issue.

## Acceptance Criteria

- [ ] `cargo build` succeeds with zero warnings
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds
- [ ] Directory structure matches `spec.main.md` §8's Crate Layout

## Notes

(filled in during/after implementation)

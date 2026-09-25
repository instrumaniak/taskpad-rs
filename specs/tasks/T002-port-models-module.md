# T002: Port Models Module

## Goal

Port `models.h`/`models.cpp` to `src/models.rs`: the `Status` enum, `Task`, `ProjectConfig`,
`StatusFile` structs, and the `TaskpadError`/`Result` types that replace the C++ `Result<T>`
wrapper.

## Depends On

- T001

## Spec References

- `spec.main.md` → §3 Data Model (Status Enum, Task Struct, ProjectConfig Struct, Error /
  Result Type)
- Original C++: `src/models.h`, `src/models.cpp`

## Phase: 1

## Critical: true

## Files to Create/Modify

- `src/models.rs` (MODIFY) — implement `Status`, `Task`, `ProjectConfig`, `StatusFile`,
  `TaskpadError`, `Result<T>` type alias

## Implementation Steps

1. Define `Status` as a Rust enum with `Serialize`/`Deserialize` mapping to
   `pending`/`in_progress`/`done` (see spec §3 — needs explicit rename handling, not the
   derive default).
2. Define `Task`, `ProjectConfig` (using `BTreeMap<i32, String>` for `phases` — see spec §3
   for why), and `StatusFile` structs with `serde` derives.
3. Document `Task.id` per spec §3's locked decision (keep the field, `#[serde(skip)]`,
   re-threaded from the status.yaml map key by `storage::read_status_file`) in `models.rs`'s
   module doc comment, with a note that `storage.rs` cross-references it (T004).
4. Define `TaskpadError` (via `thiserror`) and the crate's `Result<T>` alias.
5. Port `status_to_string`/`string_to_status` equivalents (or rely on serde's rename
   attributes plus a `Display` impl if that fully covers the C++ functions' call sites —
   check `commands.cpp` usage before deciding which).

## Acceptance Criteria

- [ ] `cargo build` succeeds
- [ ] Unit tests (`#[cfg(test)] mod tests` in `models.rs`) cover: `Status` string round-trip
      for all three variants; a `StatusFile` serde round-trip through `serde-saphyr` produces
      the same struct back — including `Task.id`, which the storage-style map-key
      re-threading restores. (A bare `Task` round-trip drops `id` since it is
      `#[serde(skip)]`; for that case compare all the other fields.)
- [ ] `cargo clippy --all-targets -- -D warnings` succeeds

## Notes

- Step 5 resolved as `Display`/`FromStr` (no separate `status_to_string`/`string_to_status` fns): `commands.cpp` uses the bare strings in message composition (e.g. `blockers += dep + " (" + statusToString(...) + ")"`), while the bracketed terminal rendering `[pending]` comes from a *separate* `statusColor()` static helper that belongs in `commands/mod.rs` (T007+), not `models`. So `Display` = bare string (= `statusToString` parity), and the spec §3 "Display" column describes terminal output, not the `Display` impl.
- Byte parity for `depends`: absent / `~` / `null` / `[]` all deserialize to an empty vec; an empty vec serializes as `depends: ~` via `serde_saphyr::NullableTilde`, matching the C++ emitter (Locked decision §4). This had to live in `models.rs` (the field attributes) because T004's file list is `storage.rs` only.
- Accepted reader divergence (flagged, not a spec↔C++ conflict): C++ silently ignores a `tasks:`/`phases:` node of the wrong *shape* (returns empty, success); serde surfaces `Invalid status.yaml format: …`. Only reachable with hand-corrupted files; none of the E2E fixtures exercise it.
- `#![allow(dead_code)]` at the module top: the public API here (`Status`, `Task`, `ProjectConfig`, `StatusFile`, `TaskpadError`, `Result`) is consumed by later modules (T004+) and by tests, so it is dead only during this task's build.
- `phases` uses `BTreeMap<i32, String>` as spec §3 requires, giving ordered phase output.

- [2026-09-25 18:27] Ported models.h/cpp to src/models.rs: Status (serde rename + custom Deserialize for unknown→Pending), Task with id#[serde(skip)] and null-tolerant depends, ProjectConfig(BTreeMap), StatusFile(flatten config), TaskpadError/Result; 10 unit tests; build + clippy + test green

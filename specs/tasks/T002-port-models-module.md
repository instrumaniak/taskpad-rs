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

(filled in during/after implementation)

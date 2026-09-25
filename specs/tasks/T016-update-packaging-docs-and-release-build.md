# T016: Update Packaging, Docs, And Release Build

## Goal

Replace the Makefile-based build/install instructions with Cargo equivalents in the README,
and do a final release-build sanity pass.

## Depends On

- T015

## Spec References

- `spec.main.md` → §10 Distribution
- Original C++: `README.md`, `Makefile`

## Phase: 5

## Critical: false

## Files to Create/Modify

- `README.md` (MODIFY) — replace the "Building from Source" section's `apt install`/`make`
  instructions with `cargo build --release`/`cargo install --path .`, per
  `spec.main.md` §10
- `Cargo.toml` (MODIFY) — set `version = "1.0.0"` per §10's versioning note, fill in
  `description`/`repository`/`license` metadata fields
- `Makefile` (DELETE) — no longer needed once Cargo fully replaces it; keep only if the
  team wants a `make check` convenience alias, which is optional per `spec.main.md` §8

## Implementation Steps

1. Update `README.md`'s build/install/test instructions to match `spec.main.md` §8/§10
   exactly (`cargo build --release`, `cargo test`, `node --test tests/e2e/*.mjs`,
   `cargo install --path .`).
2. Fill in `Cargo.toml` package metadata.
3. Run a final `cargo build --release` from a clean `cargo clean` state to confirm there's
   no leftover dependency on anything from the scaffold stage that only worked by accident
   (stale `target/` artifacts, etc.).
4. Decide whether to remove the `Makefile` outright or keep a thin one — either is fine,
   document the choice in this task's Notes.

## Acceptance Criteria

- [ ] `cargo clean && cargo build --release` succeeds from scratch
- [ ] README's build/install instructions are accurate and match what a fresh clone
      actually needs (no stale `apt install` references)
- [ ] `cargo install --path .` succeeds and `taskpad --help` runs from the installed binary

## Notes

(filled in during/after implementation)

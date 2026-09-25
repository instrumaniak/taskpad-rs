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

- `README.md` (CREATE — this repo has no README yet; copy the C++ repo's `README.md` as
  the starting point, then replace the "Building from Source" section's
  `apt install`/`make` instructions with `cargo build --release`/`cargo install --path .`,
  per `spec.main.md` §10 — do not copy `../taskpad`'s Makefile usage or apt dependencies;
  also note the Rust port needs no system packages)
- `Cargo.toml` (MODIFY) — set `version = "1.0.0"` per §10's versioning note, fill in
  `description`/`license` metadata fields (`repository`: no URL exists for this Rust port —
  leave it out unless the human provides one)

(There is no `Makefile` in this repo to delete — Cargo has been the only build system since
T001; the C++ repo's Makefile is reference material only and stays untouched.)

## Implementation Steps

1. Create `README.md` from the C++ repo's README as the base, then update its
   build/install/test instructions to match `spec.main.md` §8/§10 exactly
   (`cargo build --release`, `cargo test`, `node --test tests/e2e/*.mjs`,
   `cargo install --path .`) and strip the C++ toolchain requirements
   (`apt install libcli11-dev libyaml-cpp-dev …`, `make`).
2. Fill in `Cargo.toml` package metadata.
3. Run a final `cargo build --release` from a clean `cargo clean` state to confirm there's
   no leftover dependency on anything from the scaffold stage that only worked by accident
   (stale `target/` artifacts, etc.).
4. Confirm nothing in the README/docs references a `Makefile` (this repo has never had one
   — the C++ repo's Makefile is read-only reference material). Note the outcome in Notes.

## Acceptance Criteria

- [ ] `cargo clean && cargo build --release` succeeds from scratch
- [ ] README's build/install instructions are accurate and match what a fresh clone
      actually needs (no stale `apt install` references)
- [ ] `cargo install --path .` succeeds and `taskpad --help` runs from the installed binary
      — **do not run this until the human gives explicit go-ahead** (AGENTS.md Locked
      decision §9: it installs a second `taskpad` on `PATH` alongside the C++ one). If not
      approved, leave this box unchecked and say so in the report.

## Notes

(filled in during/after implementation)

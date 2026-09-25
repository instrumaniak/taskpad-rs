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

- [x] `cargo clean && cargo build --release` succeeds from scratch
- [x] README's build/install instructions are accurate and match what a fresh clone
      actually needs (no stale `apt install` references)
- [x] `cargo install --path .` succeeds and `taskpad --help` runs from the installed binary
      — **do not run this until the human gives explicit go-ahead** (AGENTS.md Locked
      decision §9: it installs a second `taskpad` on `PATH` alongside the C++ one). If not
      approved, leave this box unchecked and say so in the report.
      *(Approved by the human for T016 — gate lifted; ran with the explicit path
      `~/.cargo/bin/taskpad --help`, all thirteen subcommands listed.)*

## Notes

(filled in during/after implementation)

- **README.md (CREATE)**: based on the C++ repo's upstream `README.md`
  (fetched from `github.com/instrumaniak/taskpad` — the local sibling checkout's README is
  stale at 5 lines and lacks the "Building from Source (Linux/Ubuntu)" section; the local
  checkout was never modified). The intro paragraph is kept verbatim; the User Guide link
  was dropped (this repo has no `docs/taskpad.md`) in favour of links to
  `specs/spec.main.md` / `specs/spec.testing.md`, plus a port note linking the original
  C++ repo. The `apt install libcli11-dev libyaml-cpp-dev … doctest-dev` / `make` /
  `make check|test|e2e-test|install|clean` block was replaced with
  `cargo build --release`, `cargo test`, `node --test tests/e2e/*.mjs`,
  `cargo install --path .`, `cargo clean`, and the `~/.local/bin` PATH-setup snippet was
  replaced by the `~/.cargo/bin` note (no shell-profile edits recommended). Added a Usage
  section listing all thirteen subcommands and their flags, taken from
  `taskpad --help` (ground truth). No system packages / no C++ toolchain called out per
  `spec.main.md` §8/§10.
- **README grep (implementation step 4)**: `grep -inE "make|apt|Makefile" README.md` →
  **no matches** (exit 1). Nothing in the README references `make`, `Makefile`, or
  `apt install`.
- **Cargo.toml**: `version = "1.0.0"` and `description` were already set (T001);
  added `repository = "https://github.com/instrumaniak/taskpad-rs"` (human-provided).
  **`license` field deliberately omitted**: the reference C++ repo declares no license
  anywhere — no `LICENSE`/`COPYING` file locally or on GitHub (raw 404), nothing in its
  README, Makefile, sources, or specs beyond one aspirational `├── LICENSE` line in its
  spec's example tree. Per human instruction, none was invented; flagged for the report.
- **Verification**: `cargo clean && cargo build --release` from scratch → success, zero
  warnings; `cargo clippy --all-targets -- -D warnings` → clean; `cargo test` → 133/133
  pass; `cargo fmt --check` → clean; `node --test tests/e2e/*.mjs` → 123/123 pass (after
  clearing the stale `$TMPDIR/.taskpad-rs-e2e-built` build flag).
- **Install (Locked decision §9 gate lifted by human)**: `cargo install --path .` →
  installed `taskpad v1.0.0` to `~/.cargo/bin/taskpad`; `~/.cargo/bin/taskpad --help`
  lists all thirteen subcommands. PATH report: `which taskpad` →
  `/home/raziur/.local/bin/taskpad` (the C++ binary still wins) because `~/.local/bin`
  (PATH index 2) precedes `~/.cargo/bin` (index 6). PATH/shell profiles untouched; the
  C++ binary was not removed or replaced.

- [2026-09-25 23:52] README from C++ upstream with cargo build/test/install instructions, Cargo.toml repository metadata, clean-scratch release build, cargo install verified

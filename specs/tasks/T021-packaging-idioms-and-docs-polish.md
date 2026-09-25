# T021: Packaging idioms and docs polish

## Goal

Close the community-practices gaps the code review found: no lint enforcement in `Cargo.toml`
(the project's own "no `unwrap()` outside tests" rule was tribal rather than machine-checked),
missing package metadata, no MSRV, a binary crate whose entire surface was declared `pub` with
no `lib.rs`, a test fixture resolved relative to the process CWD, and a README that documented
neither how to verify the port nor where it deliberately parts ways with the C++ original.

## Depends On

(None)

## Phase:

6

## Critical: false

## Spec References

- `spec.main.md` → §8 (Dependencies and crate layout — the closed crate list, no new
  dependencies), §10 Distribution (versioning, install)
- `AGENTS.md` → "Non-negotiables" (no `unsafe`; no `unwrap()` outside tests; `cargo fmt` before
  every commit)
- Original C++: `README.md`, `Makefile` (packaging reference only)

## Files to Create/Modify

- `Cargo.toml` (MODIFY) — `rust-version`, `keywords`, `categories`, `[[bin]]`, `[lints]`
- `src/main.rs` (MODIFY) — single crate-level test-lint allowance
- `src/**/*.rs` (MODIFY) — mechanical `pub` → `pub(crate)` across 113 sites
- `src/storage.rs` (MODIFY) — `CARGO_MANIFEST_DIR`-anchored fixture path
- `README.md` (MODIFY) — Requirements/MSRV, Verification, C++ parity, deviations table
- `CHANGELOG.md` (CREATE — the repo had none and `version = "1.0.0"` implies releases)
- `.gitignore` (MODIFY) — Rust noise patterns

## Implementation Steps

1. Add `rust-version`, `keywords`, `categories`, and an explicit `[[bin]] name = "taskpad"`.
   Determine the MSRV from the declared `rust-version` of every locked dependency and take the
   maximum, rather than guessing from edition 2024.
2. Add a `[lints]` table: `[lints.rust] unsafe_code = "forbid"` and `[lints.clippy]
   unwrap_used = "deny"`, `expect_used = "deny"`. Cover the test modules with **one** line —
   `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]` at the top of
   `src/main.rs` — rather than a per-module allow. Do not enable `pedantic`.
3. Mechanically convert `pub` to `pub(crate)` throughout `src/`. Check first whether clap's
   derive needs `pub`; it generates impls in the same module, so it should not.
4. Anchor the C++ fixture test path to `env!("CARGO_MANIFEST_DIR")` so `cargo test
   --manifest-path` works from any working directory.
5. Rewrite the README's verification and parity sections: exact commands, Node prerequisite,
   MSRV, a pointer to `spec.main.md` and the AGENTS.md conflicts table, and an explicit table
   of the intentional hardening deviations (atomic writes, the `Cannot read` error split,
   whitespace-only name rejection, `import` aborting on an unreadable `status.yaml`, ASCII-only
   whitespace scanning).
6. Add a `CHANGELOG.md` and extend `.gitignore`.

## Constraints

- **No new dependencies.** `spec.main.md` §8's crate list is closed.
- The `pub` → `pub(crate)` pass must not be forced through if it breaks anything: revert that
  one item and report it rather than weakening the lints to accommodate it.
- No `unsafe`; no `unwrap()`/`expect()` outside `#[cfg(test)]`.
- `specs/` and the task files are out of scope.

## Acceptance Criteria

- [x] `Cargo.toml` declares `rust-version`, `keywords`, `categories`, and an explicit
      `[[bin]] name = "taskpad"`
- [x] `[lints.rust] unsafe_code = "forbid"` and `[lints.clippy] unwrap_used`/`expect_used` =
      `"deny"` are set, and the lints are demonstrated to actually fail the build
- [x] Exactly one line in `src/main.rs` allows those two clippy lints under `cfg(test)`
- [x] Zero bare `pub` items remain in `src/`; all 113 are `pub(crate)`
- [x] No `LICENSE` file was invented — the upstream C++ repository declares no license
- [x] `cargo test` passes when invoked from a different working directory via
      `--manifest-path`
- [x] README documents Requirements/MSRV, the exact verification commands, a C++ parity
      pointer, and the intentional deviations
- [x] `CHANGELOG.md` exists and `.gitignore` covers the new temp-file pattern
- [x] `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (167/167 at task
      close), `cargo fmt --check`, and `node --test tests/e2e/*.mjs` (123/123) all pass

## Notes

(filled in during/after implementation)

- **MSRV = 1.89**, derived from `serde-saphyr 1.3.0`, which declares `rust-version = "1.89"`.
  Every other locked dependency declares a lower floor (`clap`/`clap_builder`/`clap_derive`/
  `getrandom` 1.85, `thiserror` 1.77, `granit-parser` 1.81, `syn`/`quote`/`proc-macro2`/
  `serde_derive` 1.71, `serde` 1.56), so 1.89 is the maximum — not edition 2024 (1.85).
  **Caveat: unverified.** Only stable 1.96.1 is installed on this machine, so the value comes
  from the declared fields rather than an actual 1.89 build.
- **No LICENSE, deliberately.** The upstream C++ repository ships none: no `LICENSE`/`COPYING`
  file locally or on GitHub, nothing in its README, Makefile, sources, or specs beyond one
  aspirational `├── LICENSE` line in its spec's example tree. Rather than invent one, the
  `license` key is absent, a comment in `Cargo.toml` records why, and `CHANGELOG.md` lists it
  under known limitations. **`cargo publish` will warn until a license is chosen.** It may be
  worth adding one upstream and mirroring it here.
- **The `pub` → `pub(crate)` pass needed no exceptions** — clap's derive generates impls in the
  same module, so `pub(crate)` fields suffice. The crew sanity-checked that both new lints
  genuinely bite (an injected `unsafe` block and an injected non-test `unwrap()` each failed
  the build) and then reverted the probes.
- **The E2E build flag lives in `os.tmpdir()`**, so the repo itself stays clean; only
  `.taskpad-tmp-*` was added to `.gitignore` for the atomic-write temps of T020.
- `pedantic` was deliberately not enabled: it would flood the build with warnings and is not
  part of the project's stated bar.
- Scope held: `specs/` and the task files were untouched.

- [2026-09-26 03:01] Packaging: rust-version/lints/keywords/[[bin]] in Cargo.toml, unwrap_used+expect_used+unsafe_code denied, pub->pub(crate), CARGO_MANIFEST_DIR fixture, README verification+parity+deviations, CHANGELOG, .gitignore

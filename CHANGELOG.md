# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0]

First release of the Rust port. The goal is drop-in compatibility with the C++
[`taskpad`](https://github.com/instrumaniak/taskpad) — same `.taskpad` config, same
`status.yaml` schema, same `T*.md` template, same thirteen subcommands, same stdout/stderr
wording, same exit codes.

### Added

- `init`, `import`, `new`, `status`, `next`, `do`, `done`, `pause`, `deps`, `log`, `edit`,
  `summary`, `remove`, plus the global `--tasks-dir` override.
- Byte-compatible `status.yaml` writer: `depends: ~` for an empty list, `phases` /
  `critical_path` omitted when empty, key order `tasks` → `phases` → `critical_path`, and no
  trailing newline.
- A tolerant `status.yaml` reader that also accepts header comments, `depends: []`, flow-style
  `depends: [T001]`, quoted or plain `name`, and omitted optional fields.
- `T*.md` template and `.taskpad` config bytes matching the C++ emitters exactly.
- Unit tests in-crate and a Rust integration-test E2E tier under `tests/e2e/`,
  transcribed assertion-for-assertion from the C++ repo's `node:test` suite.
- `Cargo.toml` packaging metadata (`rust-version` 1.89, `readme`, `keywords`, `categories`,
  explicit `[[bin]]`) and crate-wide lints: `unsafe_code = "forbid"`,
  `clippy::unwrap_used`/`clippy::expect_used` denied outside `#[cfg(test)]`.

### Security / robustness

Hardening deviations from the C++ behavior, all documented in the README and at the code site:

- All `status.yaml` / `.taskpad` / `T*.md` writes are atomic (temp file + `rename`).
- Read failures are distinguished from missing files: only `ErrorKind::NotFound` reports
  "not initialized" / "No status.yaml found"; anything else reports
  `Cannot read <path>: <reason>`.
- Whitespace-only task names are rejected instead of creating a `T001.md` that `import` can
  never see.
- `import` aborts on an unreadable `status.yaml` rather than overwriting it.
- `extract_phase` / `extract_critical` scan bytes, so a multibyte character can never panic
  a mid-codepoint slice.

### Known limitations

- No `license` field: the upstream C++ repository ships no LICENSE file, so there is none to
  mirror. Set a license before publishing to crates.io.
- `tasks:` with a non-mapping value is rejected here but tolerated by the C++ binary (accepted
  divergence, `AGENTS.md` conflicts table row 18).

#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for the `taskpad-rs` binary, driven as a black box through
//! `std::process::Command`.
//!
//! One test target, one module per command area, with the shared harness in
//! `tests/common/mod.rs` — the layout ripgrep uses (`tests/tests.rs` plus one
//! `mod` per feature area) and fd uses (`tests/testenv/mod.rs`).
//!
//! Every assertion here was transcribed from the C++ repo's `node:test` suite,
//! so the expected strings are the C++ strings. If one of these tests needs its
//! assertion changed in order to pass, that is a bug in the Rust port, not a
//! test to fix — see `specs/spec.testing.md` §1.
//!
//! The `#[path]` attribute is required: a plain `mod common;` would look for
//! `tests/e2e/common.rs` and never find `tests/common/mod.rs`. The file must be
//! named `main.rs` and not `mod.rs` because Cargo only auto-discovers
//! `tests/*.rs` and `tests/*/main.rs` — a `tests/e2e/mod.rs` is silently
//! ignored, yielding zero tests and no warning.
//!
//! The `clippy` allows above are required, not optional: `Cargo.toml`'s
//! `[lints.clippy]` `deny` applies to integration-test targets too. This is the
//! test-side counterpart of `src/main.rs`'s `#![cfg_attr(test, allow(...))]`, and
//! it is consistent with the "no `unwrap()` outside test code" rule — this *is*
//! test code.

#[path = "../common/mod.rs"]
mod common;

mod deps;
mod do_cmd;
mod done;
mod edit;
mod import;
mod init;
mod log;
mod new;
mod next;
mod pause;
mod remove;
mod status;
mod summary;

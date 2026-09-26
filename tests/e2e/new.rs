//! Transcription of `tests/e2e/new.mjs`.
//!
//! Every literal below is a C++ literal — the `.mjs` suite was copied verbatim
//! from the C++ repo, so the assertions are transcribed unchanged. The
//! `depends:` one is the load-bearing comment from the `.mjs`: the C++
//! `YAML::Dump` writer emits *block* style, sequence items indented one step
//! under the key, never flow style `[T001]`.

use super::common::*;

/// The `status.yaml` every `before()` hook in `new.mjs` wrote — byte-identical
/// in all six `describe` blocks, so it lives here rather than being copied six
/// times.
const FIRST_TASK_DONE: &str = "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: done\n    depends: []\n    phase: 0\n    critical: false\n";

// --- taskpad new — basic ---------------------------------------------------
// Merged: `it` 2 ("updates status.yaml with the new task") reads the
// status.yaml `it` 1 writes, and `it` 3 ("prints Updated status.yaml") needs
// T002 to already exist for the T003 numbering.

#[test]
fn creates_a_new_task_file() {
    // taskpad new — basic
    let p = Project::new(Fixture::StatusYaml(FIRST_TASK_DONE));

    // it: "creates a new task file"
    let r = p.run(&["new", "Second Task", "--depends", "T001"]);
    contains(&r.stdout, "Created T002-second-task.md");
    assert!(p.exists("tasks/T002-second-task.md"));
    assert_eq!(r.code, Some(0));

    // it: "updates status.yaml with the new task"
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "T002");
    contains(&yaml, "Second Task");
    contains(&yaml, "status: pending");
    // Ground truth (C++ YAML::Dump verified): block style, sequence items
    // indented one step under the `depends:` key — not flow style `[T001]`.
    // The original regex is a *literal* here: `depends:\n` + six spaces +
    // `- T001`, so it pins the indentation exactly.
    contains(&yaml, "depends:\n      - T001");

    // it: "prints Updated status.yaml"
    let r = p.run(&["new", "Third Task"]);
    contains(&r.stdout, "Updated status.yaml");
    assert_eq!(r.code, Some(0));
}

// --- taskpad new — empty name error ----------------------------------------

#[test]
fn returns_error_for_empty_task_name() {
    // taskpad new — empty name error
    let p = Project::new(Fixture::StatusYaml(FIRST_TASK_DONE));

    // The `.mjs` passed an empty string as a real argv element, not a
    // missing argument.
    let r = p.run(&["new", "", "--depends", "T001"]);
    contains(&r.stderr, "Task name cannot be empty");
    assert_eq!(r.code, Some(1));
}

// --- taskpad new — invalid dependency --------------------------------------

#[test]
fn returns_error_for_non_existent_dependency() {
    // taskpad new — invalid dependency
    let p = Project::new(Fixture::StatusYaml(FIRST_TASK_DONE));

    let r = p.run(&["new", "Task", "--depends", "T999"]);
    contains(&r.stderr, "Dependency T999 not found");
    assert_eq!(r.code, Some(1));
}

// --- taskpad new — duplicate name warning ----------------------------------

#[test]
fn warns_with_the_existing_file_name_and_still_creates() {
    // taskpad new — duplicate name warning
    let p = Project::new(Fixture::StatusYaml(FIRST_TASK_DONE));

    let r = p.run(&["new", "First Task"]);
    contains(
        &r.stderr,
        "warning: Task with similar name exists: T001-first-task.md",
    );
    contains(&r.stdout, "Created T002-first-task.md");
    assert_eq!(r.code, Some(0));
    assert!(p.exists("tasks/T002-first-task.md"));
}

// --- taskpad new — phase and critical flags --------------------------------

#[test]
fn writes_phase_and_critical_into_status_yaml() {
    // taskpad new — phase and critical flags
    let p = Project::new(Fixture::StatusYaml(FIRST_TASK_DONE));

    let r = p.run(&["new", "Phased Task", "--phase", "2", "--critical"]);
    contains(&r.stdout, "Created T002-phased-task.md");
    contains(&r.stdout, "Updated status.yaml");
    assert_eq!(r.stderr, "");
    assert_eq!(r.code, Some(0));

    let yaml = p.read("tasks/status.yaml");
    // Both patterns are `/T002:[\s\S]*…/`, so each is an ordered subsequence
    // over the whole file.
    has_ordered(&yaml, &["T002:", "phase: 2"]);
    has_ordered(&yaml, &["T002:", "critical: true"]);
}

// --- taskpad new — unwritable directory ------------------------------------

#[test]
#[cfg(unix)]
fn errors_when_the_task_file_cannot_be_written() {
    // taskpad new — unwritable directory
    let p = Project::new(Fixture::StatusYaml(FIRST_TASK_DONE));

    p.set_mode("tasks", 0o555);
    // Self-skip when the mode is not enforced for us — `chmod` does not stop
    // root, so as root the write would succeed and the assertions below would be
    // meaningless. Same guard as `src/storage.rs`'s unreadable-file test.
    let probe = p.path("tasks/.perm-probe");
    if std::fs::write(&probe, "").is_ok() {
        let _ = std::fs::remove_file(&probe);
        p.set_mode("tasks", 0o755);
        return;
    }

    let r = p.run(&["new", "Second Task"]);
    // Restore the mode *before* the first assertion. The `.mjs` got this from a
    // try/finally; Rust has no equivalent, and `TempDir`'s `Drop` ignores
    // deletion errors — so a panicking assert in between would silently leak a
    // read-only directory into the temp dir.
    p.set_mode("tasks", 0o755);

    has_line_ordered(&r.stderr, &["error: Cannot write to", "Check permissions"]);
    assert_eq!(r.stdout, "");
    assert_eq!(r.code, Some(1));
    assert!(!p.exists("tasks/T002-second-task.md"));
}

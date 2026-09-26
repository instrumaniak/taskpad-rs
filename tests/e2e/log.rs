//! Rust port of `tests/e2e/log.mjs`.
//!
//! Every assertion here is a literal the `.mjs` suite copied from the C++ repo,
//! so it is transcribed unchanged — see the shared brief.

use super::common::*;

/// The `before()` fixture of the "basic", "repeated entries", "empty message
/// error", "task not found" and "invalid task id" describes: one pending task
/// with its `T*.md` file.
fn single_task_project() -> Project {
    Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]))
}

#[test]
fn appends_a_timestamped_entry_and_reports_the_file() {
    let p = single_task_project();
    let r = p.run(&["log", "T001", "worked on the thing"]);
    assert_eq!(r.stdout, "Logged to tasks/T001-first-task.md");
    assert_eq!(r.code, Some(0));

    let content = p.read("tasks/T001-first-task.md");
    has_log_entry(&content, "worked on the thing");
    // helpers.mjs fixture has no ## Notes section — log must create it
    contains(&content, "## Notes");
    assert_eq!(content.matches("## Notes").count(), 1);
}

#[test]
fn appends_under_the_existing_notes_section_in_order() {
    let p = single_task_project();
    // The describe's `before()` seeded the first entry.
    p.run(&["log", "T001", "first entry"]);

    let r = p.run(&["log", "T001", "second entry"]);
    assert_eq!(r.code, Some(0));

    let content = p.read("tasks/T001-first-task.md");
    // `.mjs` `content.indexOf('first entry') < content.indexOf('second entry')`.
    // `find` returns `usize` here, so a missing fragment is a huge offset
    // rather than the -1 the JS comparison relied on.
    assert!(content.find("first entry") < content.find("second entry"));
    assert_eq!(content.matches("## Notes").count(), 1);
    has_log_entry(&content, "second entry");
}

#[test]
fn returns_error_for_an_empty_log_message() {
    let p = single_task_project();
    let r = p.run(&["log", "T001", ""]);
    contains(&r.stderr, "Log message cannot be empty");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_when_task_does_not_exist() {
    let p = single_task_project();
    let r = p.run(&["log", "T999", "a message"]);
    contains(&r.stderr, "Task T999 not found");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_for_invalid_task_id_format() {
    let p = Project::new(Fixture::Empty);
    let r = p.run(&["log", "BAD", "a message"]);
    contains(&r.stderr, "Invalid task ID format");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_when_the_task_md_file_does_not_exist() {
    // `statusYaml` only: T001 is in the project but has no `T*.md` file.
    let p = Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: pending\n    depends: []\n    phase: 0\n    critical: false\n",
    ));
    let r = p.run(&["log", "T001", "a message"]);
    contains(&r.stderr, "Task file tasks/T001-first-task.md not found");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_when_no_status_yaml_exists() {
    let p = Project::new(Fixture::Empty);
    let r = p.run(&["log", "T001", "a message"]);
    contains(&r.stderr, "No status.yaml found");
    assert_eq!(r.code, Some(1));
}

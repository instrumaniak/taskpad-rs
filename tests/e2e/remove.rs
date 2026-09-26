//! Transcription of `tests/e2e/remove.mjs`.
//!
//! The `.mjs` shared one project per `describe` and mutated it across the `it`
//! cases, but each case only touches its own task id, so every case becomes a
//! self-contained `#[test]` with its own `Project`. The two "does nothing on …"
//! cases created their *own* extra project mid-test (`p2` / `p3`); here each is
//! simply the whole test.
//!
//! Note the argument-order flip for the prompt cases: `helpers.mjs`'s
//! `runInteractive(...all)` popped the input off the *end* of the variadic list,
//! while the Rust harness takes the input first.

use super::common::*;

/// The `.mjs` fixtures built with `createProject({ tasks: ['T001', 'T002'] })`;
/// `helpers.mjs` derives the names (`Task 1`, `Task 2`), so the files on disk
/// are `T001-task-1.md` / `T002-task-2.md`.
fn two_tasks() -> Project {
    Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001"),
        TaskSpec::new("T002"),
    ]))
}

#[test]
fn removes_task_from_status_yaml_with_force_keeps_md() {
    // taskpad remove
    let p = two_tasks();
    let r = p.run(&["remove", "T001", "--force"]);

    contains(&r.stdout, "Removed T001");
    contains(&r.stdout, "Updated status.yaml");
    not_contains(&r.stdout, "Deleted");

    let s = p.run(&["status"]);
    not_contains(&s.stdout, "T001");

    assert!(p.exists("tasks/T001-task-1.md"));
}

#[test]
fn all_force_removes_from_status_yaml_and_deletes_md() {
    // taskpad remove
    let p = two_tasks();
    let r = p.run(&["remove", "T002", "--all", "--force"]);

    contains(&r.stdout, "Removed T002");
    contains(&r.stdout, "Deleted T002-task-2.md");

    assert!(!p.exists("tasks/T002-task-2.md"));

    let s = p.run(&["status"]);
    not_contains(&s.stdout, "T002");
}

#[test]
fn removes_on_y_response() {
    // taskpad remove — confirmation prompt
    let p = two_tasks();
    let r = p.run_interactive("y\n", &["remove", "T001"]);

    contains(&r.stdout, "Are you sure");
    contains(&r.stdout, "Removed T001");

    let s = p.run(&["status"]);
    not_contains(&s.stdout, "T001");
}

#[test]
fn all_removes_md_on_y_confirmation() {
    // taskpad remove — confirmation prompt
    let p = two_tasks();
    let r = p.run_interactive("y\n", &["remove", "T002", "--all"]);

    contains(&r.stdout, "Are you sure");
    contains(&r.stdout, "Deleted");
    assert!(!p.exists("tasks/T002-task-2.md"));

    let s = p.run(&["status"]);
    not_contains(&s.stdout, "T002");
}

#[test]
fn does_nothing_on_n_response() {
    // taskpad remove — confirmation prompt
    let p = Project::new(Fixture::Tasks(vec![TaskSpec::new("T003")]));
    let r = p.run_interactive("N\n", &["remove", "T003"]);

    contains(&r.stdout, "Are you sure");

    let s = p.run(&["status"]);
    contains(&s.stdout, "T003");

    assert!(p.exists("tasks/T003-task-3.md"));
}

#[test]
fn does_nothing_on_empty_response() {
    // taskpad remove — confirmation prompt
    let p = Project::new(Fixture::Tasks(vec![TaskSpec::new("T004")]));
    let r = p.run_interactive("\n", &["remove", "T004"]);

    contains(&r.stdout, "Are you sure");

    let s = p.run(&["status"]);
    contains(&s.stdout, "T004");
}

#[test]
fn warns_about_dependents_before_removing() {
    // taskpad remove — dependent warning
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001"),
        TaskSpec::new("T002").depends(&["T001"]),
    ]));
    let r = p.run(&["remove", "T001", "--force"]);

    contains(&r.stderr, "depend on T001");
    contains(&r.stdout, "Removed T001");
}

#[test]
fn returns_error_for_nonexistent_task() {
    // taskpad remove — errors
    let p = Project::new(Fixture::Tasks(vec![TaskSpec::new("T001")]));
    let r = p.run(&["remove", "T999"]);

    contains(&r.stderr, "T999 not found");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_for_invalid_task_id_format() {
    // taskpad remove — errors
    let p = Project::new(Fixture::Tasks(vec![TaskSpec::new("T001")]));
    let r = p.run(&["remove", "abc"]);

    contains(&r.stderr, "Invalid task ID format");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_when_no_tasks_directory_exists() {
    // taskpad remove — no status.yaml
    let p = Project::new(Fixture::Empty);
    let r = p.run(&["remove", "T001", "--force"]);

    contains(&r.stderr, "No status.yaml found");
    assert_eq!(r.code, Some(1));
}

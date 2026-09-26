//! Rust port of `tests/e2e/done.mjs`.
//!
//! Every assertion here is a literal the `.mjs` suite copied from the C++ repo,
//! so it is transcribed unchanged — see the shared brief.

use super::common::*;

/// The `before()` fixture of the "basic" and "unblocked tasks display" describes:
/// T002 depends on T001.
fn pair_project() -> Project {
    Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task").depends(&["T001"]),
    ]))
}

/// The `before()` fixture of the "already-done error", "task not found" and
/// "invalid task id" describes: a single pending task.
fn single_task_project() -> Project {
    Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]))
}

#[test]
fn marks_a_pending_task_as_done() {
    let p = pair_project();
    let r = p.run(&["done", "T001"]);
    contains(&r.stdout, "✓ T001 marked as done");
    assert_eq!(r.code, Some(0));

    // `.mjs` it 2, 'persists the transition to status.yaml'. It reads the file
    // the run above rewrote, so it is merged into this test rather than split
    // out — on its own it would be asserting against the unmutated fixture.
    let yaml = p.read("tasks/status.yaml");
    has_ordered(&yaml, &["T001:", "status: done"]);
}

#[test]
fn returns_error_when_task_is_already_done() {
    let p = single_task_project();
    // The describe's `before()` reached the already-done state here.
    p.run(&["done", "T001"]);

    let r = p.run(&["done", "T001"]);
    contains(&r.stderr, "already done");
    assert_eq!(r.code, Some(1));
}

#[test]
fn marks_an_in_progress_task_as_done() {
    let p = Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: in_progress\n    depends: []\n    phase: 0\n    critical: false\n",
    ));
    let r = p.run(&["done", "T001"]);
    contains(&r.stdout, "✓ T001 marked as done");
    assert_eq!(r.code, Some(0));
}

#[test]
fn shows_unblocked_tasks_after_marking_a_dependency_done() {
    let p = pair_project();
    let r = p.run(&["done", "T001"]);
    contains(&r.stdout, "✓ T001 marked as done");
    contains(&r.stdout, "Unblocked tasks:");
    has_line_ordered(&r.stdout, &["T002", "Second Task", "[pending]"]);
    assert_eq!(r.code, Some(0));
}

#[test]
fn returns_error_when_task_does_not_exist() {
    let p = single_task_project();
    let r = p.run(&["done", "T999"]);
    contains(&r.stderr, "Task T999 not found");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_for_invalid_task_id_format() {
    let p = Project::new(Fixture::Empty);
    let r = p.run(&["done", "BAD"]);
    contains(&r.stderr, "Invalid task ID format");
    assert_eq!(r.code, Some(1));
}

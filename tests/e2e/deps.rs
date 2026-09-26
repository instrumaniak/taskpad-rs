//! Rust port of `tests/e2e/deps.mjs`.
//!
//! Every assertion here is a literal the `.mjs` suite copied from the C++ repo,
//! so it is transcribed unchanged — see the shared brief. The four
//! whole-stdout comparisons are written as normal strings with `\n` escapes
//! rather than raw multi-line literals, which `cargo fmt` would reindent.

use super::common::*;

/// The `before()` fixture of the "dependencies and dependents" describe: a
/// done → pending → pending chain.
fn chain_project() -> Project {
    Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task").status("done"),
        TaskSpec::new("T002").name("Second Task").depends(&["T001"]),
        TaskSpec::new("T003").name("Third Task").depends(&["T002"]),
    ]))
}

/// The `before()` fixture of the "no deps and no dependents" and "task not
/// found" describes: one pending task with no edges.
fn single_task_project() -> Project {
    Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]))
}

#[test]
fn shows_done_dependency_with_check_and_reverse_lookup() {
    let p = chain_project();
    let r = p.run(&["deps", "T002"]);
    // The id column is padded to 5 and the name column to 12, hence the two
    // spaces after `T001` and after `First Task`.
    assert_eq!(
        r.stdout,
        "T002 depends on:\n  T001  First Task  [done] ✓\n\nTasks waiting on T002:\n  T003  Third Task  [pending]"
    );
    assert_eq!(r.code, Some(0));
}

#[test]
fn shows_pending_dependency_with_cross_and_no_dependents() {
    let p = chain_project();
    let r = p.run(&["deps", "T003"]);
    assert_eq!(
        r.stdout,
        "T003 depends on:\n  T002  Second Task  [pending] ✗\n\nTasks waiting on T003:\n  (none)"
    );
    assert_eq!(r.code, Some(0));
}

#[test]
fn shows_dependents_with_no_dependency_list() {
    let p = chain_project();
    let r = p.run(&["deps", "T001"]);
    assert_eq!(
        r.stdout,
        "T001 depends on:\n  (none)\n\nTasks waiting on T001:\n  T002  Second Task  [pending]"
    );
    assert_eq!(r.code, Some(0));
}

#[test]
fn prints_none_for_both_sections() {
    let p = single_task_project();
    let r = p.run(&["deps", "T001"]);
    assert_eq!(
        r.stdout,
        "T001 depends on:\n  (none)\n\nTasks waiting on T001:\n  (none)"
    );
    assert_eq!(r.code, Some(0));
}

#[test]
fn prints_the_bare_dependency_id_without_a_status_marker() {
    // T001 depends on T999, which is not in `status.yaml` at all.
    let p = Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: pending\n    depends: [T999]\n    phase: 0\n    critical: false\n",
    ));
    let r = p.run(&["deps", "T001"]);
    // No `[status]` and no `✓`/`✗` marker: the id is printed on its own.
    assert_eq!(
        r.stdout,
        "T001 depends on:\n  T999\n\nTasks waiting on T001:\n  (none)"
    );
    assert_eq!(r.code, Some(0));
}

#[test]
fn returns_error_when_task_does_not_exist() {
    let p = single_task_project();
    let r = p.run(&["deps", "T999"]);
    contains(&r.stderr, "Task T999 not found");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_for_invalid_task_id_format() {
    let p = Project::new(Fixture::Empty);
    let r = p.run(&["deps", "BAD"]);
    contains(&r.stderr, "Invalid task ID format");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_error_when_no_status_yaml_exists() {
    let p = Project::new(Fixture::Empty);
    let r = p.run(&["deps", "T001"]);
    contains(&r.stderr, "No status.yaml found");
    assert_eq!(r.code, Some(1));
}

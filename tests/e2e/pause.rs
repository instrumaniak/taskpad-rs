use super::common::*;

// taskpad pause — basic
#[test]
fn pauses_an_in_progress_task() {
    let p = Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: in_progress\n    depends: []\n    phase: 0\n    critical: false\n",
    ));

    let r = p.run(&["pause", "T001"]);
    contains(&r.stdout, "Paused T001 — First Task");
    has_line_ordered(&r.stdout, &["Status changed:", "in_progress", "pending"]);
    assert_eq!(r.code, Some(0));
}

// taskpad pause — done to pending
#[test]
fn pauses_a_done_task_back_to_pending() {
    let p = Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: done\n    depends: []\n    phase: 0\n    critical: false\n",
    ));

    let r = p.run(&["pause", "T001"]);
    contains(&r.stdout, "Paused T001 — First Task");
    has_line_ordered(&r.stdout, &["Status changed:", "done", "pending"]);
    assert_eq!(r.code, Some(0));
}

// taskpad pause — already-pending error
#[test]
fn returns_error_when_task_is_already_pending() {
    let p = Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: pending\n    depends: []\n    phase: 0\n    critical: false\n",
    ));

    let r = p.run(&["pause", "T001"]);
    contains(&r.stderr, "already pending");
    assert_eq!(r.code, Some(1));
}

// taskpad pause — task not found
#[test]
fn returns_error_when_task_does_not_exist() {
    let p = Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: in_progress\n    depends: []\n    phase: 0\n    critical: false\n",
    ));

    let r = p.run(&["pause", "T999"]);
    contains(&r.stderr, "Task T999 not found");
    assert_eq!(r.code, Some(1));
}

// taskpad pause — invalid task id
#[test]
fn returns_error_for_invalid_task_id_format() {
    let p = Project::new(Fixture::Empty);

    let r = p.run(&["pause", "BAD"]);
    contains(&r.stderr, "Invalid task ID format");
    assert_eq!(r.code, Some(1));
}

use super::common::*;

// taskpad do — basic
#[test]
fn starts_a_pending_task_and_changes_status_to_in_progress() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task").depends(&["T001"]),
    ]));

    let r = p.run(&["do", "T002", "--force"]);
    contains(&r.stdout, "Started T002 — Second Task");
    has_line_ordered(&r.stdout, &["Status changed:", "pending", "in_progress"]);
}

// taskpad do — already in_progress error
#[test]
fn returns_error_when_task_is_already_in_progress() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));
    p.run(&["do", "T001", "--force"]);

    let r = p.run(&["do", "T001"]);
    contains(&r.stderr, "already in_progress");
    assert_eq!(r.code, Some(1));
}

// taskpad do — unmet dependencies error
#[test]
fn returns_error_without_force_when_deps_are_not_met() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task").depends(&["T001"]),
    ]));

    let r = p.run(&["do", "T002"]);
    contains(&r.stderr, "Unmet dependencies");
    assert_eq!(r.code, Some(1));
}

// taskpad do — --force bypasses dependency check
#[test]
fn succeeds_with_force_even_when_deps_are_not_met() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task").depends(&["T001"]),
    ]));

    let r = p.run(&["do", "T002", "--force"]);
    contains(&r.stdout, "Started T002 — Second Task");
    assert_eq!(r.code, Some(0));
}

// taskpad do — invalid task id
#[test]
fn returns_error_for_invalid_task_id_format() {
    let p = Project::new(Fixture::Empty);

    let r = p.run(&["do", "BAD", "--force"]);
    contains(&r.stderr, "Invalid task ID format");
    assert_eq!(r.code, Some(1));
}

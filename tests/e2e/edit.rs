//! `tests/e2e/edit.mjs` — the `taskpad edit` E2E slice.
//!
//! 15 `describe` blocks / 31 `it` cases transcribed into 20 `#[test]` fns per
//! the T023 split/merge decision: 7 describes merged (each later `it` reads
//! state an earlier one wrote, or is the negative counterpart of a failing run
//! the test itself performs), 13 split so every fn owns a fresh `Project` and
//! aborts at its own first failed assertion.

use super::common::*;

// ---------------------------------------------------------------------------
// task-level status change  (2 `it` merged: it2 reads the yaml it1 wrote)
// ---------------------------------------------------------------------------

/// Merged `it` 1 + 2: "updates the status and prints one confirmation line" and
/// "persists the new status to status.yaml".
#[test]
fn updates_the_status_and_prints_one_confirmation_line() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task"),
        TaskSpec::new("T003").name("Third Task"),
    ]));

    let r = p.run(&["edit", "T001", "--status", "in_progress"]);
    assert_eq!(r.stdout, "Updated T001 status: in_progress");
    assert_eq!(r.stderr, "");
    assert_eq!(r.code, Some(0));

    let yaml = p.read("tasks/status.yaml");
    has_ordered(&yaml, &["T001:", "status: in_progress"]);
}

// ---------------------------------------------------------------------------
// combined task-level flags  (2 `it` merged: it2 reads the yaml it1 wrote)
// ---------------------------------------------------------------------------

/// Merged `it` 1 + 2: "applies status, depends, phase, and critical in one
/// command" and "persists every changed field to status.yaml".
#[test]
fn applies_status_depends_phase_and_critical_in_one_command() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task"),
        TaskSpec::new("T003").name("Third Task"),
    ]));

    let r = p.run(&[
        "edit",
        "T001",
        "--status",
        "done",
        "--phase",
        "2",
        "--critical",
        "--depends",
        "T002",
    ]);
    assert_eq!(
        r.stdout,
        "Updated T001 status: done depends: T002 phase: 2 critical: true"
    );
    assert_eq!(r.code, Some(0));

    let yaml = p.read("tasks/status.yaml");
    has_ordered(&yaml, &["T001:", "status: done"]);
    has_ordered_bounded_tail(&yaml, "T001:", "depends:", "T002", 20);
    has_ordered(&yaml, &["T001:", "phase: 2"]);
    has_ordered(&yaml, &["T001:", "critical: true"]);
}

// ---------------------------------------------------------------------------
// repeated --depends
// ---------------------------------------------------------------------------

#[test]
fn replaces_depends_with_every_supplied_id_joined_with_comma_space() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task"),
        TaskSpec::new("T003").name("Third Task"),
    ]));

    let r = p.run(&["edit", "T001", "--depends", "T002", "--depends", "T003"]);
    assert_eq!(r.stdout, "Updated T001 depends: T002, T003");
    assert_eq!(r.code, Some(0));
}

// ---------------------------------------------------------------------------
// --critical and --no-critical  (3 `it` merged: sequential true -> false -> true)
// ---------------------------------------------------------------------------

/// Merged `it` 1 + 2 + 3: the three runs are a sequential toggle of the same
/// field, so each one's `status.yaml` read depends on the one before it.
#[test]
fn critical_marks_the_task_critical() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    // it1: --critical marks the task critical
    let r = p.run(&["edit", "T001", "--critical"]);
    assert_eq!(r.stdout, "Updated T001 critical: true");
    assert_eq!(r.code, Some(0));
    contains(&p.read("tasks/status.yaml"), "critical: true");

    // it2: --no-critical unsets it
    let r = p.run(&["edit", "T001", "--no-critical"]);
    assert_eq!(r.stdout, "Updated T001 critical: false");
    assert_eq!(r.code, Some(0));
    contains(&p.read("tasks/status.yaml"), "critical: false");

    // it3: both flags together behave like --critical (cli.cpp: critVal = editCritical)
    let r = p.run(&["edit", "T001", "--critical", "--no-critical"]);
    assert_eq!(r.stdout, "Updated T001 critical: true");
    assert_eq!(r.code, Some(0));
    contains(&p.read("tasks/status.yaml"), "critical: true");
}

// ---------------------------------------------------------------------------
// invalid status  (2 `it` merged: it2 is the negative counterpart of it1)
// ---------------------------------------------------------------------------

/// Merged `it` 1 + 2: "rejects an unknown status value" and "leaves status.yaml
/// untouched on failure". Split off, the second assertion could never fail —
/// it re-reads a fixture nothing ever wrote.
#[test]
fn rejects_an_unknown_status_value() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit", "T001", "--status", "bogus"]);
    contains(
        &r.stderr,
        "Invalid status. Must be: pending, in_progress, or done",
    );
    assert_eq!(r.code, Some(1));

    let yaml = p.read("tasks/status.yaml");
    has_ordered(&yaml, &["T001:", "status: pending"]);
}

// ---------------------------------------------------------------------------
// invalid task id
// ---------------------------------------------------------------------------

#[test]
fn returns_error_for_invalid_task_id_format() {
    // edit reads status.yaml before validating the id (commands.cpp:889
    // runs readStatusFile ahead of the isValidTaskId check), so the
    // fixture needs a status.yaml for the id error to surface.
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit", "BAD", "--status", "done"]);
    contains(
        &r.stderr,
        "Invalid task ID format. Expected TXXX (see Task ID Format)",
    );
    assert_eq!(r.code, Some(1));
}

// ---------------------------------------------------------------------------
// task not found
// ---------------------------------------------------------------------------

#[test]
fn returns_error_when_task_does_not_exist() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit", "T999", "--status", "done"]);
    contains(&r.stderr, "Task T999 not found");
    assert_eq!(r.code, Some(1));
}

// ---------------------------------------------------------------------------
// no changes specified  (2 `it` split: two independent messages, no mutation)
// ---------------------------------------------------------------------------

#[test]
fn errors_when_a_task_id_is_given_without_any_task_level_flag() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit", "T001"]);
    contains(
        &r.stderr,
        "No changes specified. Use --status, --phase, --critical, or --depends",
    );
    assert_eq!(r.code, Some(1));
}

#[test]
fn project_level_flags_are_ignored_in_the_task_level_branch() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit", "T001", "--phases", "0:Scaffolding"]);
    contains(
        &r.stderr,
        "No changes specified. Use --status, --phase, --critical, or --depends",
    );
    assert_eq!(r.code, Some(1));
}

// ---------------------------------------------------------------------------
// phase validation  (2 `it` split: two independent messages)
// ---------------------------------------------------------------------------

#[test]
fn rejects_a_negative_phase() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit", "T001", "--phase", "-1"]);
    contains(&r.stderr, "Phase must be non-negative");
    assert_eq!(r.code, Some(1));
}

// The C++ binary aborts (uncaught std::invalid_argument from std::stoi)
// on a non-numeric phase; the port turns that into a clean error instead
// of panicking — see spec.main.md §6 "No panics" and the T012 task Notes.
#[test]
fn rejects_a_non_numeric_phase() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit", "T001", "--phase", "abc"]);
    contains(&r.stderr, "Invalid phase. Must be a non-negative integer");
    assert_eq!(r.code, Some(1));
}

// ---------------------------------------------------------------------------
// dependency errors  (4 `it` merged: the three failing runs + "no write on
// failure", which split off would be a tautology)
// ---------------------------------------------------------------------------

/// Merged `it` 1 + 2 + 3 + 4: three rejected `--depends` values followed by
/// the assertion that none of them wrote the file.
#[test]
fn rejects_a_dependency_that_does_not_exist() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task").depends(&["T001"]),
    ]));

    // it1: a dependency that does not exist
    let r = p.run(&["edit", "T002", "--depends", "T999"]);
    contains(&r.stderr, "Dependency T999 not found");
    assert_eq!(r.code, Some(1));

    // it2: a self-dependency
    let r = p.run(&["edit", "T001", "--depends", "T001"]);
    contains(
        &r.stderr,
        "Circular dependency detected: T001 depends on itself",
    );
    assert_eq!(r.code, Some(1));

    // it3: a transitive cycle with the literal `→ ... →` separator
    // (not a fully joined path). T002 already depends on T001, so
    // T001 -> T002 would close the loop.
    let r = p.run(&["edit", "T001", "--depends", "T002"]);
    contains(
        &r.stderr,
        "Circular dependency detected: T001 \u{2192} ... \u{2192} T002",
    );
    assert_eq!(r.code, Some(1));

    // it4: no write happened. The helpers.mjs fixture style uses `depends: []`,
    // which a failed edit must leave exactly as it was.
    let yaml = p.read("tasks/status.yaml");
    has_ordered(&yaml, &["T001:", "depends: []"]);
}

// ---------------------------------------------------------------------------
// project-level phases  (3 `it` merged: it2 reads it1's write, it3 re-runs)
// ---------------------------------------------------------------------------

/// Merged `it` 1 + 2 + 3: the first run writes the mapping it2 reads, and the
/// second run replaces it in place.
#[test]
fn replaces_the_phase_mapping_and_confirms() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task"),
    ]));

    // it1: replaces the phase mapping and confirms
    let r = p.run(&["edit", "--phases", "0:Scaffolding,1:Foundation"]);
    assert_eq!(r.stdout, "Updated phases mapping");
    assert_eq!(r.code, Some(0));

    // it2: persists both phase entries to status.yaml
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "phases:");
    contains(&yaml, "Scaffolding");
    contains(&yaml, "Foundation");

    // it3: replaces rather than merges on a second run
    let r = p.run(&["edit", "--phases", "7:Only"]);
    assert_eq!(r.stdout, "Updated phases mapping");
    assert_eq!(r.code, Some(0));
    let yaml = p.read("tasks/status.yaml");
    has_ordered_within(&yaml, &["phases:", "7: Only"], 40);
    not_contains(&yaml, "Scaffolding");
}

// ---------------------------------------------------------------------------
// project-level critical path  (2 `it` split: it2 is standalone)
// ---------------------------------------------------------------------------

#[test]
fn sets_the_critical_path_and_confirms() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task"),
    ]));

    let r = p.run(&["edit", "--critical-path", "T001,T002"]);
    assert_eq!(r.stdout, "Updated critical path");
    assert_eq!(r.code, Some(0));
    let yaml = p.read("tasks/status.yaml");
    has_ordered_within(&yaml, &["critical_path:", "T001"], 40);
    has_ordered_within(&yaml, &["critical_path:", "T002"], 60);
}

#[test]
fn rejects_an_id_that_is_not_a_task() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task"),
    ]));

    let r = p.run(&["edit", "--critical-path", "T999"]);
    contains(&r.stderr, "Task T999 in critical path not found");
    assert_eq!(r.code, Some(1));
}

// ---------------------------------------------------------------------------
// project-level combined flags  (2 `it` merged: it2 reads it1's write)
// ---------------------------------------------------------------------------

/// Merged `it` 1 + 2: "prints one confirmation line per supplied flag" and
/// "persists both to status.yaml".
#[test]
fn prints_one_confirmation_line_per_supplied_flag() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
        TaskSpec::new("T002").name("Second Task"),
    ]));

    let r = p.run(&[
        "edit",
        "--phases",
        "0:Scaffolding",
        "--critical-path",
        "T001,T002",
    ]);
    assert_eq!(r.stdout, "Updated phases mapping\nUpdated critical path");
    assert_eq!(r.code, Some(0));

    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "Scaffolding");
    contains(&yaml, "critical_path:");
}

// ---------------------------------------------------------------------------
// no project-level changes  (2 `it` split: two independent messages)
// ---------------------------------------------------------------------------

#[test]
fn errors_when_no_flags_are_given_at_all() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit"]);
    contains(
        &r.stderr,
        "No project-level changes specified. Use --phases or --critical-path",
    );
    assert_eq!(r.code, Some(1));
}

#[test]
fn task_level_flags_are_ignored_in_the_project_level_branch() {
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("First Task"),
    ]));

    let r = p.run(&["edit", "--status", "done"]);
    contains(
        &r.stderr,
        "No project-level changes specified. Use --phases or --critical-path",
    );
    assert_eq!(r.code, Some(1));
}

// ---------------------------------------------------------------------------
// missing status.yaml  (2 `it` split: two independent messages)
// ---------------------------------------------------------------------------

#[test]
fn task_level_branch_fails_before_validating_the_id() {
    let p = Project::new(Fixture::Empty);

    let r = p.run(&["edit", "T001", "--status", "done"]);
    contains(
        &r.stderr,
        "No status.yaml found. Run 'taskpad import' or 'taskpad new' first",
    );
    assert_eq!(r.code, Some(1));
}

#[test]
fn project_level_branch_fails_the_same_way() {
    let p = Project::new(Fixture::Empty);

    let r = p.run(&["edit", "--phases", "0:Scaffolding"]);
    contains(
        &r.stderr,
        "No status.yaml found. Run 'taskpad import' or 'taskpad new' first",
    );
    assert_eq!(r.code, Some(1));
}

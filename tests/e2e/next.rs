use super::common::*;

// taskpad next
#[test]
fn shows_next_task_when_one_is_available() {
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("First Task"),
        ImportSpec::new("T002")
            .name("Second Task")
            .depends(&["T001"]),
    ]));
    // T001 has no deps, T002 depends on T001
    // Import, then mark T001 as done to unblock T002
    p.run(&["import"]);
    p.run(&["do", "T001", "--force"]);
    p.run(&["done", "T001"]);

    let r = p.run(&["next"]);
    contains(&r.stdout, "Next:");
    contains(&r.stdout, "T002");
}

// taskpad next
#[test]
fn reads_files_and_specs_from_t_md_via_extract_section_list_items() {
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("First Task"),
        ImportSpec::new("T002")
            .name("Second Task")
            .depends(&["T001"]),
    ]));
    // T001 has no deps, T002 depends on T001
    // Import, then mark T001 as done to unblock T002
    p.run(&["import"]);
    p.run(&["do", "T001", "--force"]);
    p.run(&["done", "T001"]);

    let r = p.run(&["next"]);
    contains(&r.stdout, "Files:");
    contains(&r.stdout, "src/second-task.cpp");
    contains(&r.stdout, "Specs:");
    contains(&r.stdout, "specs/spec1.md");
}

// taskpad next — all blocked or complete
#[test]
fn shows_info_when_all_tasks_blocked_or_complete() {
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001")
            .name("Blocked Task")
            .depends(&["T999"]),
    ]));
    p.run(&["import"]);

    let r = p.run(&["next"]);
    contains(&r.stdout, "All tasks blocked or complete");
}

// taskpad next — priority order
#[test]
fn prioritizes_critical_path_over_phase_number() {
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001")
            .name("Phase 1 Non-Critical")
            .phase(1)
            .critical(false),
        ImportSpec::new("T002")
            .name("Phase 0 Critical")
            .phase(0)
            .critical(true),
        ImportSpec::new("T003")
            .name("Phase 0 Non-Critical")
            .phase(0)
            .critical(false),
    ]));
    p.run(&["import"]);

    let r = p.run(&["next"]);
    contains(&r.stdout, "T002");
    contains(&r.stdout, "Phase 0 Critical");
}

// taskpad next — missing T*.md file
#[test]
fn silently_omits_file_derived_detail_and_creates_no_template() {
    // statusYaml mode writes only status.yaml — no T*.md files exist.
    let p = Project::new(Fixture::StatusYaml(
        "tasks:\n  T001:\n    name: \"Fileless Task\"\n    status: pending\n    depends: []\n    phase: 0\n    critical: false\n",
    ));

    let r = p.run(&["next"]);
    contains(&r.stdout, "Next:");
    contains(&r.stdout, "T001");
    contains(&r.stdout, "Fileless Task");
    not_contains(&r.stdout, "Goal:");
    not_contains(&r.stdout, "First step:");
    assert_eq!(r.stderr, "");
    assert_eq!(r.code, Some(0));
    // No template file is created for the missing T*.md (spec.main.md §6).
    assert_eq!(p.read_dir_sorted("tasks"), vec!["status.yaml"]);
}

//! Transcription of `tests/e2e/import.mjs`.
//!
//! Every literal below is a C++ literal — the `.mjs` suite was copied verbatim
//! from the C++ repo, so the assertions are transcribed unchanged. The two
//! cases that assert `exit 0` *with* a stderr warning (a circular dependency, a
//! dependency that was not found) are the C++ ground truth recorded in
//! `AGENTS.md`'s conflicts table #6: `import` reports validation issues as
//! `warning: N issue(s) found`, still writes `status.yaml`, and still exits 0.

use super::common::*;

// --- taskpad import — basic ------------------------------------------------
// Merged: `it` 2 ("status.yaml contains both tasks with defaults") reads the
// very status.yaml `it` 1's `import` writes, so splitting it would leave it
// asserting on an empty project.

#[test]
fn creates_status_yaml_from_existing_t_md_files() {
    // taskpad import — basic
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("First Task"),
        ImportSpec::new("T002")
            .name("Second Task")
            .depends(&["T001"]),
    ]));

    // it: "creates status.yaml from existing T*.md files"
    let r = p.run(&["import"]);
    contains(&r.stdout, "Created status.yaml");
    contains(&r.stdout, "Found 2 task files");
    assert!(p.exists("tasks/status.yaml"));

    // it: "status.yaml contains both tasks with defaults"
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "T001");
    contains(&yaml, "T002");
    contains(&yaml, "First Task");
    contains(&yaml, "Second Task");
    contains(&yaml, "phase: 0");
    contains(&yaml, "critical: false");
}

// --- taskpad import — --force overwrites -----------------------------------
// Merged: `it` 2 ("succeeds with --force") needs the status.yaml `it` 1's
// first `import` creates; on a fresh project `--force` would not be exercised
// at all, since the refusal *is* what creates the file.

#[test]
fn fails_without_force_when_status_yaml_exists() {
    // taskpad import — --force overwrites
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("Force Task"),
    ]));

    // it: "fails without --force when status.yaml exists"
    p.run(&["import"]);
    let r = p.run(&["import"]);
    contains(
        &r.stderr,
        "status.yaml already exists. Use --force to overwrite",
    );
    assert_eq!(r.code, Some(1));

    // it: "succeeds with --force"
    let r = p.run(&["import", "--force"]);
    contains(&r.stdout, "Overwrote status.yaml");
    assert_eq!(r.code, Some(0));
}

// --- taskpad import — phase parsing ----------------------------------------

#[test]
fn parses_phase_n_correctly() {
    // taskpad import — phase parsing
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("Phase Test").phase(3),
    ]));

    p.run(&["import"]);
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "phase: 3");
}

#[test]
fn defaults_to_0_when_no_phase_header() {
    // taskpad import — phase parsing
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("No Phase"),
    ]));

    p.run(&["import"]);
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "phase: 0");
}

// --- taskpad import — critical parsing -------------------------------------

#[test]
fn parses_critical_true() {
    // taskpad import — critical parsing
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("Critical True").critical(true),
    ]));

    p.run(&["import"]);
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "critical: true");
}

#[test]
fn defaults_to_false_when_no_critical_header() {
    // taskpad import — critical parsing
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("Not Critical"),
    ]));

    p.run(&["import"]);
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "critical: false");
}

// --- taskpad import — status parsing ---------------------------------------

#[test]
fn parses_status_done() {
    // taskpad import — status parsing
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("Done Task").status("done"),
    ]));

    p.run(&["import"]);
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "status: done");
}

#[test]
fn defaults_to_pending_when_no_status_header() {
    // taskpad import — status parsing
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("Pending Task"),
    ]));

    p.run(&["import"]);
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "status: pending");
}

// --- taskpad import — depends parsing --------------------------------------

#[test]
fn parses_depends_on_references() {
    // taskpad import — depends parsing
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("First Task"),
        ImportSpec::new("T002").name("Dep Task").depends(&["T001"]),
    ]));

    p.run(&["import"]);
    let yaml = p.read("tasks/status.yaml");
    // `/T002[\s\S]*depends:\n.*- T001/` — two parts: the ordered subsequence
    // from `T002` to `depends:` spans lines, and the `.*` between `depends:`
    // and `- T001` may NOT cross one, so it has to be the next line.
    // (`has_line_ordered` would be wrong here for exactly that reason.)
    contains(&yaml, "T002");
    has_line_followed_by(&yaml, "depends:", "- T001");
}

// --- taskpad import — missing sections default -----------------------------

#[test]
fn handles_task_with_no_optional_headers() {
    // taskpad import — missing sections default
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("Minimal Task"),
    ]));

    p.run(&["import"]);
    let yaml = p.read("tasks/status.yaml");
    contains(&yaml, "phase: 0");
    contains(&yaml, "critical: false");
    contains(&yaml, "status: pending");
}

// --- taskpad import — errors ------------------------------------------------

#[test]
fn returns_error_for_circular_dependency() {
    // taskpad import — errors
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001").name("Task A").depends(&["T002"]),
        ImportSpec::new("T002").name("Task B").depends(&["T001"]),
    ]));

    let r = p.run(&["import"]);
    has_digits_between(&r.stderr, "warning: ", " issue(s) found");
    contains_ignore_ascii_case(&r.stderr, "circular");
    assert_eq!(r.code, Some(0)); // still writes with warning
    assert!(p.exists("tasks/status.yaml"));
}

#[test]
fn reports_error_when_dependency_not_found() {
    // taskpad import — errors
    let p = Project::new(Fixture::Import(vec![
        ImportSpec::new("T001")
            .name("Missing Dep")
            .depends(&["T999"]),
    ]));

    let r = p.run(&["import"]);
    has_digits_between(&r.stderr, "warning: ", " issue(s) found");
    contains(&r.stderr, "T999");
    assert_eq!(r.code, Some(0)); // still writes with warning
    assert!(p.exists("tasks/status.yaml"));
}

#[test]
fn reports_no_files_when_no_t_md_files_exist() {
    // taskpad import — errors
    let p = Project::new(Fixture::Empty);

    let r = p.run(&["import"]);
    contains(&r.stdout, "No T*.md files found");
    assert_eq!(r.code, Some(0));
}

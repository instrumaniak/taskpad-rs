//! Transcription of `tests/e2e/summary.mjs`.
//!
//! Every literal below is a C++ literal — the `.mjs` suite was copied verbatim
//! from the C++ repo, so the assertions are transcribed unchanged, including
//! the column alignment baked into the summary header (`Total tasks:    4`,
//! `Done:           1 (25.0%)`, …) and the `─` rule under the title.

use super::common::*;

/// The `status.yaml` the `.mjs` "happy path" `before()` hook wrote. The four
/// tests of that describe each re-run `summary` against this same read-only
/// fixture, so it lives in one place; every test still owns a fresh `Project`.
const HAPPY_PATH_YAML: &str = "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: done\n    depends: []\n    phase: 0\n    critical: true\n  T002:\n    name: \"Second Task\"\n    status: in_progress\n    depends: [T001]\n    phase: 1\n    critical: true\n  T003:\n    name: \"Third Task\"\n    status: pending\n    depends: [T002]\n    phase: 1\n    critical: false\n  T004:\n    name: \"Fourth Task\"\n    status: pending\n    depends: [T001]\n    phase: 2\n    critical: true\nphases:\n  0: Setup\n  1: Core\ncritical_path: [T001, T002, T004, T999]\n";

#[test]
fn prints_totals_and_percentages() {
    // taskpad summary — happy path
    let p = Project::new(Fixture::StatusYaml(HAPPY_PATH_YAML));
    let r = p.run(&["summary"]);

    assert_eq!(r.code, Some(0));
    assert_eq!(r.stderr, "");
    has_line(&r.stdout, "Task Summary");
    // `new RegExp('^' + SEPARATOR + '$', 'm')`, SEPARATOR = '─'.repeat(13)
    has_line(&r.stdout, &"\u{2500}".repeat(13));
    has_line(&r.stdout, "Total tasks:    4");
    has_line(&r.stdout, "Done:           1 (25.0%)");
    has_line(&r.stdout, "In progress:    1 (25.0%)");
    has_line(&r.stdout, "Pending:        2 (50.0%)");
}

#[test]
fn prints_per_phase_breakdown_in_phase_order() {
    // taskpad summary — happy path
    let p = Project::new(Fixture::StatusYaml(HAPPY_PATH_YAML));
    let r = p.run(&["summary"]);

    has_line(&r.stdout, "By Phase:");
    has_line(&r.stdout, "  Phase 0 (Setup): 1/1 done");
    has_line(&r.stdout, "  Phase 1 (Core): 0/2 done");
    has_line(&r.stdout, "  Phase 2: 0/1 done");

    // deepEqual over `stdout.split('\n').filter(l => l.startsWith('  Phase '))`
    let phase_lines: Vec<&str> = r
        .stdout
        .lines()
        .filter(|l| l.starts_with("  Phase "))
        .collect();
    assert_eq!(
        phase_lines,
        vec![
            "  Phase 0 (Setup): 1/1 done",
            "  Phase 1 (Core): 0/2 done",
            "  Phase 2: 0/1 done",
        ]
    );
}

#[test]
fn prints_critical_path_with_its_own_status_counts() {
    // taskpad summary — happy path
    let p = Project::new(Fixture::StatusYaml(HAPPY_PATH_YAML));
    let r = p.run(&["summary"]);

    has_line(
        &r.stdout,
        "Critical Path: T001 \u{2192} T002 \u{2192} T004 \u{2192} T999",
    );
    // T999 is not a task, so it counts toward neither status nor totals.
    has_line(&r.stdout, "  Status: 1/4 done, 1 in_progress, 1 pending");
}

#[test]
fn does_not_modify_the_project() {
    // taskpad summary — happy path
    let p = Project::new(Fixture::StatusYaml(HAPPY_PATH_YAML));
    let before_yaml = p.read("tasks/status.yaml");
    p.run(&["summary"]);
    let after_yaml = p.read("tasks/status.yaml");

    assert_eq!(after_yaml, before_yaml);
}

#[test]
fn formats_percentages_with_one_decimal_place() {
    // taskpad summary — percentage rounding
    let p = Project::new(Fixture::Tasks(vec![
        TaskSpec::new("T001").name("Task One").status("done"),
        TaskSpec::new("T002").name("Task Two").status("done"),
        TaskSpec::new("T003").name("Task Three").status("pending"),
    ]));
    let r = p.run(&["summary"]);

    assert_eq!(r.code, Some(0));
    has_line(&r.stdout, "Total tasks:    3");
    has_line(&r.stdout, "Done:           2 (66.7%)");
    has_line(&r.stdout, "Pending:        1 (33.3%)");
}

#[test]
fn prints_zero_totals_and_omits_the_by_phase_section() {
    // taskpad summary — empty task list
    let p = Project::new(Fixture::StatusYaml("tasks: {}\n"));
    let r = p.run(&["summary"]);

    assert_eq!(r.code, Some(0));
    has_line(&r.stdout, "Total tasks:    0");
    has_line(&r.stdout, "Done:           0 (0.0%)");
    has_line(&r.stdout, "In progress:    0 (0.0%)");
    has_line(&r.stdout, "Pending:        0 (0.0%)");
    not_contains(&r.stdout, "By Phase:");
    not_contains(&r.stdout, "Critical Path:");
}

#[test]
fn prints_the_by_phase_header_with_no_rows() {
    // taskpad summary — phases without tasks
    let p = Project::new(Fixture::StatusYaml("phases:\n  0: Setup\n  1: Core\n"));
    let r = p.run(&["summary"]);

    assert_eq!(r.code, Some(0));
    has_line(&r.stdout, "Total tasks:    0");
    has_line(&r.stdout, "Done:           0 (0.0%)");

    // `lines[lines.length - 1]` over the trimmed stdout.
    assert_eq!(r.stdout.lines().last(), Some("By Phase:"));
    not_line_starts_with(&r.stdout, "  Phase ");
}

#[test]
fn returns_the_missing_status_yaml_error() {
    // taskpad summary — no status.yaml
    let p = Project::new(Fixture::Empty);
    let r = p.run(&["summary"]);

    contains(
        &r.stderr,
        "No status.yaml found. Run 'taskpad import' or 'taskpad new' first",
    );
    assert_eq!(r.stdout, "");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_the_structural_status_yaml_error() {
    // taskpad summary — malformed status.yaml
    let p = Project::new(Fixture::StatusYaml("- not\n- a\n- mapping\n"));
    let r = p.run(&["summary"]);

    contains(
        &r.stderr,
        "Invalid status.yaml format. Expected YAML mapping",
    );
    assert_eq!(r.stdout, "");
    assert_eq!(r.code, Some(1));
}

#[test]
fn returns_the_parser_status_yaml_error() {
    // taskpad summary — status.yaml with a YAML syntax error
    let p = Project::new(Fixture::StatusYaml("foo: [unclosed\n"));
    let r = p.run(&["summary"]);

    // The `<parser message>` tail is parser-specific (yaml-cpp vs
    // serde-saphyr wording differs); the stable prefix must match.
    has_nonspace_after(&r.stderr, "Invalid status.yaml format: ");
    assert_eq!(r.stdout, "");
    assert_eq!(r.code, Some(1));
}

#[test]
fn malformed_taskpad_falls_back_to_specs_tasks_missing_status_yaml() {
    // taskpad summary — .taskpad problems
    let p = Project::new(Fixture::Empty);
    // `resolve_task_dir` swallows the config error and falls back to
    // specs/tasks — matching the C++ `resolveTaskDir` (spec.main.md §6).
    p.write(".taskpad", "[1, 2]\n");
    let r = p.run(&["summary"]);

    contains(
        &r.stderr,
        "No status.yaml found. Run 'taskpad import' or 'taskpad new' first",
    );
    assert_eq!(r.code, Some(1));
}

#[test]
fn invalid_task_dir_path_fails_with_missing_status_yaml() {
    // taskpad summary — .taskpad problems
    let p = Project::new(Fixture::Empty);
    p.write(".taskpad", "task-dir: does-not-exist\n");
    let r = p.run(&["summary"]);

    contains(
        &r.stderr,
        "No status.yaml found. Run 'taskpad import' or 'taskpad new' first",
    );
    assert_eq!(r.code, Some(1));
}

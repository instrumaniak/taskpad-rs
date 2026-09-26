//! Rust port of `tests/e2e/status.mjs`.
//!
//! Every assertion here is a literal the `.mjs` suite copied from the C++ repo,
//! so it is transcribed unchanged — see the shared brief.

use super::common::*;

// The `.mjs` seeded these through `before()` and shared one `p` across the
// `it`s. Each ported `#[test]` builds its own `Project` instead, so the three
// `it`s of the "basic" describe no longer share a fixture object.

/// `taskpad status — basic`: T003 is critical and unblocked, so it is the
/// `→ next` pick.
fn basic_project() -> Project {
    Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: done\n    depends: []\n    phase: 0\n    critical: false\n  T002:\n    name: \"Second Task\"\n    status: pending\n    depends: [T001]\n    phase: 0\n    critical: false\n  T003:\n    name: \"Third Task\"\n    status: pending\n    depends: []\n    phase: 0\n    critical: true\n",
    ))
}

/// `taskpad status — blocked task`: T003 waits on T002, which is still pending.
fn blocked_project() -> Project {
    Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"First Task\"\n    status: done\n    depends: []\n    phase: 0\n    critical: false\n  T002:\n    name: \"Second Task\"\n    status: pending\n    depends: [T001]\n    phase: 0\n    critical: false\n  T003:\n    name: \"Third Task\"\n    status: pending\n    depends: [T002]\n    phase: 0\n    critical: false\n",
    ))
}

/// `taskpad status — phase grouping`: two phases with configured names.
fn phase_project() -> Project {
    Project::new(Fixture::StatusYaml(
        "# taskpad status file\n\ntasks:\n  T001:\n    name: \"Alpha\"\n    status: done\n    depends: []\n    phase: 0\n    critical: false\n  T002:\n    name: \"Beta\"\n    status: pending\n    depends: []\n    phase: 1\n    critical: true\n\nphases:\n  0: Foundation\n  1: Build\n",
    ))
}

#[test]
fn prints_progress_line() {
    let p = basic_project();
    let r = p.run(&["status"]);
    contains(&r.stdout, "Progress: 1/3 done");
    assert_eq!(r.code, Some(0));
}

#[test]
fn marks_next_task_with_arrow() {
    let p = basic_project();
    let r = p.run(&["status"]);
    // Two spaces between the id and the name: the id column is padded to 5.
    contains(&r.stdout, "→ T003  Third Task");
    assert_eq!(r.code, Some(0));
}

#[test]
fn marks_the_next_task_with_the_dependencies_met_note() {
    let p = basic_project();
    let r = p.run(&["status"]);
    // C++ prints the `← next (dependencies met)` note, not the bare `← next`
    // the spec prose used to show (AGENTS.md conflicts table #13).
    contains(&r.stdout, "← next (dependencies met)");
    assert_eq!(r.code, Some(0));
}

#[test]
fn shows_blocked_by_for_pending_task_with_unmet_dependency() {
    let p = blocked_project();
    let r = p.run(&["status"]);
    contains(&r.stdout, "blocked by T002");
    assert_eq!(r.code, Some(0));
}

#[test]
fn groups_tasks_by_phase_in_ascending_order() {
    let p = phase_project();
    let r = p.run(&["status"]);
    contains(&r.stdout, "Phase 0");
    contains(&r.stdout, "Phase 1");
    assert_eq!(r.code, Some(0));
}

#[test]
fn prints_phase_name_when_configured() {
    let p = phase_project();
    let r = p.run(&["status"]);
    contains(&r.stdout, "Phase 0: Foundation");
    contains(&r.stdout, "Phase 1: Build");
    assert_eq!(r.code, Some(0));
}

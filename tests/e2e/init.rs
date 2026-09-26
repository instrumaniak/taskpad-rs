use super::common::*;

// taskpad init
//
// The `.mjs`'s `it2` ("reports Already initialized when .taskpad exists") is
// folded into this test rather than split out: it re-runs `init` against the
// `.taskpad` that `it1` just created, so on a fresh project it would have
// nothing to read.
#[test]
fn initializes_a_new_project() {
    let p = Project::new(Fixture::Empty);
    // The helper pre-creates `.taskpad`; `init` must start from a
    // config-free directory (C++ binary verified: exits 1 with
    // "Already initialized" when `.taskpad` exists).
    p.remove(".taskpad");
    let r = p.run(&["init"]);
    contains(&r.stdout, "Initialized taskpad in");
    contains(&r.stdout, "Created .taskpad config");
    contains(&r.stdout, "Ready to add tasks with");
    assert!(p.exists(".taskpad"));
    assert_eq!(r.code, Some(0));

    // reports Already initialized when .taskpad exists
    let r = p.run(&["init"]);
    contains(&r.stderr, "Already initialized");
    assert_eq!(r.code, Some(1));
}

// taskpad init
#[test]
fn initializes_with_a_custom_task_directory() {
    let p = Project::new(Fixture::Empty);
    // Ground truth (C++ verified): `init` takes no positional dir — CLI11
    // rejects `init my-tasks` (exit 109); the custom dir comes from the
    // global `--tasks-dir` flag.
    p.remove(".taskpad");
    let r = p.run(&["--tasks-dir", "my-tasks", "init"]);
    contains(&r.stdout, "Initialized taskpad in my-tasks/");
    assert_eq!(r.code, Some(0));
}

// taskpad init — import after init
#[test]
fn can_import_tasks_after_init() {
    let p = Project::new(Fixture::Empty);
    // The harness pre-creates `.taskpad`, so this `init` is a no-op that
    // exits 1 ("Already initialized"). The `.mjs` never asserted on it —
    // `import` reads the `task-dir` the harness already wrote.
    p.run(&["init"]);
    let r = p.run(&["import"]);
    contains(&r.stdout, "Scanning");
    assert_eq!(r.code, Some(0));
}

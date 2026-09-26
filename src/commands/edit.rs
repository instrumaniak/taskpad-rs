use crate::commands::{load_status, require_task_id, task_not_found};
use crate::models::Result;
use crate::models::StatusFile;
use crate::models::TaskpadError;
use crate::models::string_to_status;
use crate::storage;
use crate::utils;
use crate::validator;
use std::collections::BTreeMap;

/// Bundled task/project edit options for [`run`].
///
/// `None` and `Some("")` both mean "absent" (likewise `None` and
/// `Some([])` for `depends`), preserving the previous `&str` convention
/// where `""` meant absent. `critical` / `no_critical` stay two separate
/// booleans mirroring the CLI11 `--critical` / `--no-critical` flags.
#[derive(Debug, Default)]
pub(crate) struct EditArgs {
    /// Set task status (`pending`|`in_progress`|`done`).
    pub(crate) status: Option<String>,
    /// Replace task dependencies.
    pub(crate) depends: Option<Vec<String>>,
    /// Set phase number.
    pub(crate) phase: Option<String>,
    /// Mark as critical.
    pub(crate) critical: bool,
    /// Unmark critical.
    pub(crate) no_critical: bool,
    /// Replace phase mapping (`"N:name,N:name"`).
    pub(crate) phases: Option<String>,
    /// Replace critical path (comma-separated task IDs).
    pub(crate) critical_path: Option<String>,
}

/// Edit task metadata or project settings.
///
/// With a non-empty `task_id` (task-level branch): applies whichever of
/// `status`, `depends`, `phase`, and `critical`/`no_critical` were
/// supplied, validating dependencies (existence + circularity) when
/// `depends` changes, then prints one confirmation line covering every
/// changed field.
///
/// With an empty `task_id` (project-level branch): applies `phases`
/// (`"N:name,N:name"`, full replacement) and/or `critical_path`
/// (comma-separated task IDs, each validated to exist), printing one
/// confirmation line per applied flag. Task-level flags are ignored in
/// this branch, project-level flags in the other — matching C++.
///
/// `critical` and `no_critical` are kept as two separate booleans (like
/// the CLI11 `--critical` / `--no-critical` flags) rather than a single
/// tri-state; the CLI collapses them the same way `cli.cpp` does:
/// critical is set when either flag is given, and its value is `true`
/// only when `--critical` was the one supplied.
pub(crate) fn run(tasks_dir: &str, task_id: &str, args: EditArgs) -> Result<()> {
    let status = args.status.as_deref().unwrap_or_default();
    let depends: &[String] = args.depends.as_deref().unwrap_or_default();
    let phase = args.phase.as_deref().unwrap_or_default();
    let critical = args.critical;
    let no_critical = args.no_critical;
    let phases = args.phases.as_deref().unwrap_or_default();
    let critical_path = args.critical_path.as_deref().unwrap_or_default();

    let (dir, mut sf) = load_status(tasks_dir)?;

    if task_id.is_empty() {
        return edit_project(&dir, &mut sf, phases, critical_path);
    }

    require_task_id(task_id)?;

    if !sf.tasks.contains_key(task_id) {
        return Err(task_not_found(task_id));
    }

    // Validation runs to completion before any mutation, in the same order
    // as C++ (status → depends → phase), so a failure never writes the file.
    if !status.is_empty() && !validator::is_valid_status(status) {
        return Err(TaskpadError::Message(
            "Invalid status. Must be: pending, in_progress, or done".into(),
        ));
    }

    if !depends.is_empty() {
        validator::validate_depends_exist(depends, &sf.tasks)?;
        validator::validate_circular_dependencies(task_id, depends, &sf.tasks)?;
    }

    let mut phase_num = None;
    if !phase.is_empty() {
        let p = parse_phase_number(phase).ok_or_else(|| {
            TaskpadError::Message("Invalid phase. Must be a non-negative integer".into())
        })?;
        if p < 0 {
            return Err(TaskpadError::Message("Phase must be non-negative".into()));
        }
        phase_num = Some(p);
    }

    let critical_set = critical || no_critical;

    let changed = !status.is_empty() || !depends.is_empty() || phase_num.is_some() || critical_set;
    if !changed {
        return Err(TaskpadError::Message(
            "No changes specified. Use --status, --phase, --critical, or --depends".into(),
        ));
    }

    let task = match sf.tasks.get_mut(task_id) {
        Some(t) => t,
        None => return Err(task_not_found(task_id)),
    };
    if !status.is_empty() {
        task.status = string_to_status(status);
    }
    if !depends.is_empty() {
        task.depends = depends.to_vec();
    }
    if let Some(p) = phase_num {
        task.phase = p;
    }
    if critical_set {
        task.critical = critical;
    }

    storage::write_status_file(&dir, &sf)?;

    let mut line = format!("Updated {task_id}");
    if !status.is_empty() {
        line.push_str(&format!(" status: {status}"));
    }
    if !depends.is_empty() {
        line.push_str(&format!(" depends: {}", depends.join(", ")));
    }
    if !phase.is_empty() {
        line.push_str(&format!(" phase: {phase}"));
    }
    if critical_set {
        line.push_str(&format!(" critical: {}", critical));
    }
    println!("{line}");

    Ok(())
}

/// Project-level branch: no task ID given — replace the phase mapping
/// (`--phases`) and/or the critical path (`--critical-path`), then print
/// one confirmation line per supplied flag.
/// Matching the `taskId.empty()` half of C++ `Commands::edit`.
fn edit_project(dir: &str, sf: &mut StatusFile, phases: &str, critical_path: &str) -> Result<()> {
    let mut changed = false;

    if !phases.is_empty() {
        let mut new_phases: BTreeMap<i32, String> = BTreeMap::new();
        for pair in utils::split(phases, ',') {
            let parts = utils::split(&pair, ':');
            if parts.len() == 2 {
                // C++ runs std::stoi over the key and aborts on a
                // non-numeric one; the port skips the malformed pair
                // instead of panicking (see task Notes).
                if let Some(num) = parse_phase_number(&parts[0]) {
                    new_phases.insert(num, utils::trim(&parts[1]));
                }
            }
        }
        sf.config.phases = new_phases;
        changed = true;
    }

    if !critical_path.is_empty() {
        let cp = utils::split(critical_path, ',');
        for id in &cp {
            if !sf.tasks.contains_key(id) {
                return Err(TaskpadError::Message(format!(
                    "Task {id} in critical path not found"
                )));
            }
        }
        sf.config.critical_path = cp;
        changed = true;
    }

    if !changed {
        return Err(TaskpadError::Message(
            "No project-level changes specified. Use --phases or --critical-path".into(),
        ));
    }

    storage::write_status_file(dir, sf)?;

    if !phases.is_empty() {
        println!("Updated phases mapping");
    }
    if !critical_path.is_empty() {
        println!("Updated critical path");
    }

    Ok(())
}

/// Parse an integer the way `std::stoi` does: skip leading whitespace,
/// accept an optional sign, then take the leading run of decimal digits
/// (anything after the digits is ignored, so `"2abc"` yields `2`).
///
/// Returns `None` when there are no digits at all or the value doesn't
/// fit in `i32` — C++ aborts with an uncaught `std::invalid_argument` /
/// `std::out_of_range` in those cases, which the port converts into a
/// normal error instead of panicking.
fn parse_phase_number(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let num_start = i;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    s[num_start..i].parse::<i32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Status, Task};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            status: Status::Pending,
            depends: vec![],
            phase: 0,
            critical: false,
        }
    }

    /// Create a temp dir with a `status.yaml` containing `tasks` and an
    /// empty `.taskpad`-style layout. Returns (tempdir guard, dir path).
    fn setup_with(tasks: BTreeMap<String, Task>) -> (tempfile::TempDir, String) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap().to_string();
        let sf = StatusFile {
            tasks,
            ..Default::default()
        };
        storage::write_status_file(&root, &sf).unwrap();
        (dir, root)
    }

    fn single_task(id: &str) -> BTreeMap<String, Task> {
        let mut tasks = BTreeMap::new();
        tasks.insert(id.to_string(), make_task(id));
        tasks
    }

    /// Build [`EditArgs`] from the old positional style: `""` means absent
    /// (mirroring the pre-bundle `&str` convention) and `&[]` means absent
    /// for `depends`. `run` normalizes `None` and `Some("")` identically,
    /// so wrapping everything in `Some` still exercises absent-vs-present.
    fn ea(
        status: &str,
        depends: &[String],
        phase: &str,
        critical: bool,
        no_critical: bool,
        phases: &str,
        critical_path: &str,
    ) -> EditArgs {
        EditArgs {
            status: Some(status.to_string()),
            depends: Some(depends.to_vec()),
            phase: Some(phase.to_string()),
            critical,
            no_critical,
            phases: Some(phases.to_string()),
            critical_path: Some(critical_path.to_string()),
        }
    }

    #[test]
    fn task_level_status_change() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(
            &root,
            "T001",
            ea("in_progress", &[], "", false, false, "", ""),
        );
        assert!(result.is_ok());
        let sf = storage::read_status_file(&root).unwrap();
        assert_eq!(sf.tasks["T001"].status, Status::InProgress);
    }

    #[test]
    fn task_level_all_fields() {
        let mut tasks = single_task("T001");
        tasks.insert("T002".to_string(), make_task("T002"));
        let (_guard, root) = setup_with(tasks);

        let depends = vec!["T002".to_string()];
        let result = run(
            &root,
            "T001",
            ea("done", &depends, "2", true, false, "", ""),
        );
        assert!(result.is_ok());

        let sf = storage::read_status_file(&root).unwrap();
        assert_eq!(sf.tasks["T001"].status, Status::Done);
        assert_eq!(sf.tasks["T001"].depends, depends);
        assert_eq!(sf.tasks["T001"].phase, 2);
        assert!(sf.tasks["T001"].critical);
    }

    #[test]
    fn no_critical_flag_unsets_critical() {
        let mut task = make_task("T001");
        task.critical = true;
        let mut tasks = single_task("T001");
        tasks.insert("T001".to_string(), task);
        let (_guard, root) = setup_with(tasks);

        let result = run(&root, "T001", ea("", &[], "", false, true, "", ""));
        assert!(result.is_ok());
        let sf = storage::read_status_file(&root).unwrap();
        assert!(!sf.tasks["T001"].critical);
    }

    #[test]
    fn both_critical_flags_yield_true() {
        // cli.cpp: critSet = editCritical || editNoCritical; critVal = editCritical
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "T001", ea("", &[], "", true, true, "", ""));
        assert!(result.is_ok());
        let sf = storage::read_status_file(&root).unwrap();
        assert!(sf.tasks["T001"].critical);
    }

    #[test]
    fn invalid_status_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "T001", ea("bogus", &[], "", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid status. Must be: pending, in_progress, or done"
        );
    }

    #[test]
    fn invalid_task_id_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "BAD", ea("done", &[], "", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid task ID format. Expected TXXX (see Task ID Format)"
        );
    }

    #[test]
    fn task_not_found_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "T999", ea("done", &[], "", false, false, "", ""));
        assert_eq!(result.unwrap_err().to_string(), "Task T999 not found");
    }

    #[test]
    fn no_changes_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "T001", ea("", &[], "", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "No changes specified. Use --status, --phase, --critical, or --depends"
        );
    }

    #[test]
    fn project_level_flags_ignored_in_task_branch() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(
            &root,
            "T001",
            ea("", &[], "", false, false, "0:Name", "T001"),
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "No changes specified. Use --status, --phase, --critical, or --depends"
        );
    }

    #[test]
    fn negative_phase_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "T001", ea("", &[], "-1", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Phase must be non-negative"
        );
    }

    #[test]
    fn non_numeric_phase_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "T001", ea("", &[], "abc", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid phase. Must be a non-negative integer"
        );
    }

    #[test]
    fn dependency_not_found_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let depends = vec!["T999".to_string()];
        let result = run(&root, "T001", ea("", &depends, "", false, false, "", ""));
        assert_eq!(result.unwrap_err().to_string(), "Dependency T999 not found");
    }

    #[test]
    fn circular_dependency_self_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let depends = vec!["T001".to_string()];
        let result = run(&root, "T001", ea("", &depends, "", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Circular dependency detected: T001 depends on itself"
        );
    }

    #[test]
    fn circular_dependency_transitive_error() {
        let mut tasks = single_task("T001");
        let mut t2 = make_task("T002");
        t2.depends = vec!["T001".to_string()];
        tasks.insert("T002".to_string(), t2);
        let (_guard, root) = setup_with(tasks);

        let depends = vec!["T002".to_string()];
        let result = run(&root, "T001", ea("", &depends, "", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Circular dependency detected: T001 → ... → T002"
        );
    }

    #[test]
    fn failed_validation_does_not_write() {
        let (_guard, root) = setup_with(single_task("T001"));
        let before = std::fs::read_to_string(format!("{root}/status.yaml")).unwrap();
        let result = run(&root, "T001", ea("bogus", &[], "", false, false, "", ""));
        assert!(result.is_err());
        let after = std::fs::read_to_string(format!("{root}/status.yaml")).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn project_phases_and_critical_path() {
        let mut tasks = single_task("T001");
        tasks.insert("T002".to_string(), make_task("T002"));
        let (_guard, root) = setup_with(tasks);

        let result = run(
            &root,
            "",
            ea(
                "",
                &[],
                "",
                false,
                false,
                "0:Scaffolding,1:Foundation",
                "T001,T002",
            ),
        );
        assert!(result.is_ok());

        let sf = storage::read_status_file(&root).unwrap();
        assert_eq!(sf.config.phases.len(), 2);
        assert_eq!(sf.config.phases[&0], "Scaffolding");
        assert_eq!(sf.config.phases[&1], "Foundation");
        assert_eq!(sf.config.critical_path, vec!["T001", "T002"]);
    }

    #[test]
    fn project_phases_replaces_existing_mapping() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "", ea("", &[], "", false, false, "0:Old", ""));
        assert!(result.is_ok());
        let result = run(&root, "", ea("", &[], "", false, false, "1:New", ""));
        assert!(result.is_ok());

        let sf = storage::read_status_file(&root).unwrap();
        assert_eq!(sf.config.phases.len(), 1);
        assert_eq!(sf.config.phases[&1], "New");
    }

    #[test]
    fn project_malformed_phase_pairs_skipped() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(
            &root,
            "",
            ea("", &[], "", false, false, "nocolon,abc:Name,2:Good", ""),
        );
        assert!(result.is_ok());

        let sf = storage::read_status_file(&root).unwrap();
        assert_eq!(sf.config.phases.len(), 1);
        assert_eq!(sf.config.phases[&2], "Good");
    }

    #[test]
    fn project_no_changes_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "", ea("done", &[], "1", true, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "No project-level changes specified. Use --phases or --critical-path"
        );
    }

    #[test]
    fn critical_path_task_not_found_error() {
        let (_guard, root) = setup_with(single_task("T001"));
        let result = run(&root, "", ea("", &[], "", false, false, "", "T001,T999"));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Task T999 in critical path not found"
        );
    }

    #[test]
    fn missing_status_yaml_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let result = run(root, "T001", ea("done", &[], "", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "No status.yaml found. Run 'taskpad import' or 'taskpad new' first"
        );
    }

    #[test]
    fn task_level_branch_reads_status_before_validating_id() {
        // C++ resolves the dir and reads status.yaml before the ID checks,
        // so a missing status.yaml wins over an invalid ID.
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let result = run(root, "BAD", ea("done", &[], "", false, false, "", ""));
        assert_eq!(
            result.unwrap_err().to_string(),
            "No status.yaml found. Run 'taskpad import' or 'taskpad new' first"
        );
    }

    #[test]
    fn parse_phase_number_stoi_semantics() {
        assert_eq!(parse_phase_number("2"), Some(2));
        assert_eq!(parse_phase_number("0"), Some(0));
        assert_eq!(parse_phase_number("-1"), Some(-1));
        assert_eq!(parse_phase_number("+3"), Some(3));
        assert_eq!(parse_phase_number("  42"), Some(42));
        assert_eq!(parse_phase_number("2abc"), Some(2));
        assert_eq!(parse_phase_number("abc"), None);
        assert_eq!(parse_phase_number(""), None);
        assert_eq!(parse_phase_number("-"), None);
        assert_eq!(parse_phase_number("   "), None);
        assert_eq!(parse_phase_number("9999999999"), None);
    }
}

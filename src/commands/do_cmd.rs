use crate::commands::{
    all_deps_done, goal_lines, load_status, require_task_id, task_not_found, unmet_deps,
};
use crate::models::Result;
use crate::models::Status;
use crate::models::TaskpadError;
use crate::models::status_to_string;
use crate::storage;

/// Start working on a task.
///
/// Validates the task ID, checks the task exists and isn't already
/// in_progress or done, verifies dependencies (unless --force),
/// then sets status to InProgress and writes status.yaml.
/// Matching C++ `Commands::do_`.
///
/// The task name is printed verbatim (`Started T001 — <name>`), so ANSI
/// escape sequences or other control characters in `name` reach stdout
/// untouched — C++ parity, see the note in [`crate::commands`].
pub(crate) fn run(tasks_dir: &str, task_id: &str, force: bool) -> Result<()> {
    require_task_id(task_id)?;

    let (dir, mut sf) = load_status(tasks_dir)?;

    let task = match sf.tasks.get(task_id) {
        Some(t) => t.clone(),
        None => return Err(task_not_found(task_id)),
    };

    if task.status == Status::InProgress {
        return Err(TaskpadError::Message(format!(
            "Task {task_id} already in_progress"
        )));
    }
    if task.status == Status::Done {
        return Err(TaskpadError::Message(format!(
            "Task {task_id} already done"
        )));
    }

    if !force && !all_deps_done(&task, &sf.tasks) {
        let mut blockers: Vec<String> = Vec::new();
        for (dep, status) in unmet_deps(&task, &sf.tasks) {
            blockers.push(format!("{dep} ({})", status_to_string(status)));
        }
        // Note the degenerate case: `all_deps_done` is also false when a
        // dependency ID is missing from status.yaml entirely, but
        // `unmet_deps` can only report IDs it can look up. So a task whose
        // every dependency is absent yields an empty list here and the
        // message reads `Unmet dependencies: . Use --force to proceed`.
        // C++ parity — commands.cpp:645-656 walks the same two loops and
        // produces the same empty-list message; left as is.
        return Err(TaskpadError::Message(format!(
            "Unmet dependencies: {}. Use --force to proceed",
            blockers.join(", ")
        )));
    }

    if let Some(entry) = sf.tasks.get_mut(task_id) {
        entry.status = Status::InProgress;
    }

    storage::write_status_file(&dir, &sf)?;

    println!("Started {task_id} \u{2014} {}", task.name);
    println!("Status changed: pending \u{2192} in_progress");

    let path = storage::task_file_path(&dir, task_id, &task.name);
    if let Ok(content) = storage::read_task_file(&path) {
        println!();
        println!("Now reading {}...", path);
        for line in goal_lines(&content) {
            println!("{line}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StatusFile;
    use crate::models::Task;
    use tempfile::tempdir;

    fn make_task(id: &str, status: Status) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            status,
            depends: vec![],
            phase: 0,
            critical: false,
        }
    }

    #[test]
    fn invalid_task_id_error() {
        let result = run(".", "BAD", false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid task ID format")
        );
    }

    #[test]
    fn unmet_deps_error_message() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::Pending));
        // T002 depends on T001 which is still pending -> unmet deps.
        let mut t002 = make_task("T002", Status::Pending);
        t002.depends = vec!["T001".to_string()];
        sf.tasks.insert("T002".to_string(), t002);
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T002", false);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unmet dependencies: T001 (pending). Use --force to proceed"
        );

        // With --force the same call succeeds.
        assert!(run(root, "T002", true).is_ok());
    }
}

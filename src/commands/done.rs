use crate::commands::all_deps_done;
use crate::models::Result;
use crate::models::Status;
use crate::models::TaskpadError;
use crate::storage;
use crate::utils;
use crate::validator;

/// Mark a task as done.
///
/// Validates the task ID, checks the task exists and isn't already
/// done, sets status to Done, then finds and displays tasks that
/// became unblocked by this change.
/// Matching C++ `Commands::done`.
pub fn run(tasks_dir: &str, task_id: &str) -> Result<()> {
    if !validator::is_valid_task_id(task_id) {
        return Err(TaskpadError::Message(
            "Invalid task ID format. Expected TXXX (see Task ID Format)".into(),
        ));
    }

    let dir = utils::resolve_task_dir(tasks_dir);
    let mut sf = storage::read_status_file(&dir)?;

    let task = match sf.tasks.get(task_id) {
        Some(t) => t.clone(),
        None => {
            return Err(TaskpadError::Message(format!("Task {task_id} not found")));
        }
    };

    if task.status == Status::Done {
        return Err(TaskpadError::Message(format!(
            "Task {task_id} already done"
        )));
    }

    if let Some(entry) = sf.tasks.get_mut(task_id) {
        entry.status = Status::Done;
    }
    storage::write_status_file(&dir, &sf)?;

    println!("\u{2713} {} marked as done", task_id);

    // Find newly unblocked tasks
    let mut unblocked: Vec<(String, String)> = Vec::new();
    for (id, t) in &sf.tasks {
        if t.status != Status::Pending {
            continue;
        }
        if !all_deps_done(t, &sf.tasks) {
            continue;
        }
        for dep in &t.depends {
            if dep == task_id {
                unblocked.push((id.clone(), t.name.clone()));
                break;
            }
        }
    }

    if !unblocked.is_empty() {
        println!("\nUnblocked tasks:");
        for (id, name) in &unblocked {
            println!("  {id}  {name}  [pending]");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StatusFile;
    use crate::models::Task;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn make_task(id: &str, status: Status, depends: Vec<String>) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            status,
            depends,
            phase: 0,
            critical: false,
        }
    }

    #[test]
    fn invalid_task_id_error() {
        let result = run(".", "BAD");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid task ID format")
        );
    }

    #[test]
    fn already_done_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::Done, vec![]));
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T001");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already done"));
    }

    #[test]
    fn pending_to_done() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            make_task("T001", Status::Pending, vec![]),
        );
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T001");
        assert!(result.is_ok());

        let sf2 = storage::read_status_file(root).unwrap();
        assert_eq!(sf2.tasks["T001"].status, Status::Done);
    }

    #[test]
    fn in_progress_to_done() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            make_task("T001", Status::InProgress, vec![]),
        );
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T001");
        assert!(result.is_ok());

        let sf2 = storage::read_status_file(root).unwrap();
        assert_eq!(sf2.tasks["T001"].status, Status::Done);
    }

    #[test]
    fn unblocked_list_detection() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        // T002 depends on T001; T001 is pending → T002 is blocked
        // Mark T001 done → T002 should appear in unblocked list
        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::Done, vec![]));
        sf.tasks.insert(
            "T002".to_string(),
            make_task("T002", Status::Pending, vec!["T001".to_string()]),
        );
        storage::write_status_file(root, &sf).unwrap();

        // Now mark T001 as done (it's already done, so test the unblocked logic
        // by using a different setup)
        let result = run(root, "T001");
        // T001 is already done → error
        assert!(result.is_err());

        // Set up fresh: T001 is pending, T002 depends on T001
        let mut sf2 = StatusFile::default();
        sf2.tasks.insert(
            "T001".to_string(),
            make_task("T001", Status::Pending, vec![]),
        );
        sf2.tasks.insert(
            "T002".to_string(),
            make_task("T002", Status::Pending, vec!["T001".to_string()]),
        );
        storage::write_status_file(root, &sf2).unwrap();

        // Mark T001 as done
        let result = run(root, "T001");
        assert!(result.is_ok());

        let sf3 = storage::read_status_file(root).unwrap();
        assert_eq!(sf3.tasks["T001"].status, Status::Done);
        // T002 should now be done (all deps met) but its status is still Pending
        // in the file — it's just that all_deps_done would now return true
        assert_eq!(sf3.tasks["T002"].status, Status::Pending);
    }

    #[test]
    fn all_deps_done_pure() {
        let mut tasks = BTreeMap::new();
        tasks.insert("T001".to_string(), make_task("T001", Status::Done, vec![]));
        tasks.insert(
            "T002".to_string(),
            make_task("T002", Status::Pending, vec!["T001".to_string()]),
        );
        assert!(all_deps_done(&tasks["T002"], &tasks));
    }
}

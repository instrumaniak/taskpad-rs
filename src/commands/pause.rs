use crate::commands::{load_status, require_task_id, task_not_found};
use crate::models::Result;
use crate::models::Status;
use crate::models::TaskpadError;
use crate::models::status_to_string;
use crate::storage;

/// Pause a task, reverting it to pending.
///
/// Validates the task ID, checks the task exists and isn't already
/// pending, records the old status, sets status to Pending, and
/// displays the transition.
pub(crate) fn run(tasks_dir: &str, task_id: &str) -> Result<()> {
    require_task_id(task_id)?;

    let (dir, mut sf) = load_status(tasks_dir)?;

    let task = match sf.tasks.get(task_id) {
        Some(t) => t.clone(),
        None => return Err(task_not_found(task_id)),
    };

    if task.status == Status::Pending {
        return Err(TaskpadError::Message(format!(
            "Task {task_id} already pending"
        )));
    }

    let old_status = status_to_string(task.status);

    if let Some(entry) = sf.tasks.get_mut(task_id) {
        entry.status = Status::Pending;
    }
    storage::write_status_file(&dir, &sf)?;

    println!("Paused {} \u{2014} {}", task_id, task.name);
    println!("Status changed: {} \u{2192} pending", old_status);

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
    fn already_pending_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::Pending));
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T001");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already pending"));
    }

    #[test]
    fn in_progress_to_pending() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::InProgress));
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T001");
        assert!(result.is_ok());

        let sf2 = storage::read_status_file(root).unwrap();
        assert_eq!(sf2.tasks["T001"].status, Status::Pending);
    }

    #[test]
    fn done_to_pending_allowed() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::Done));
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T001");
        assert!(result.is_ok());

        let sf2 = storage::read_status_file(root).unwrap();
        assert_eq!(sf2.tasks["T001"].status, Status::Pending);
    }
}

use crate::commands::{load_status, require_task_id, task_not_found};
use crate::models::Result;
use crate::models::TaskpadError;
use crate::storage;
use crate::utils;

/// Append a timestamped entry to a task's T\*.md `## Notes` section.
///
/// Validates the task ID and that the message is non-empty, checks the
/// task exists in `status.yaml`, then delegates the timestamped append to
/// [`storage::append_log`] (which errors with `Task file <path> not found`
/// when the T\*.md is missing) and reports the file that was written.
/// The timestamp comes from [`utils::current_timestamp()`] here at the
/// command layer so `storage::append_log` stays pure and testable.
///
/// The message is stored verbatim in the `## Notes` entry — only emptiness is
/// rejected, never the contents. ANSI escape sequences, control characters
/// and newlines all survive into the task file, and are re-emitted verbatim
/// by any later `taskpad` command that echoes a T\*.md section.
pub(crate) fn run(tasks_dir: &str, task_id: &str, message: &str) -> Result<()> {
    require_task_id(task_id)?;

    if message.is_empty() {
        return Err(TaskpadError::Message("Log message cannot be empty".into()));
    }

    let (dir, sf) = load_status(tasks_dir)?;

    let task = match sf.tasks.get(task_id) {
        Some(t) => t.clone(),
        None => return Err(task_not_found(task_id)),
    };

    let file_path = storage::task_file_path(&dir, task_id, &task.name);
    storage::append_log(&file_path, message, &utils::current_timestamp())?;

    println!("Logged to {}", file_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Status;
    use crate::models::StatusFile;
    use crate::models::Task;
    use tempfile::tempdir;

    fn make_task(id: &str, name: &str) -> Task {
        Task {
            id: id.to_string(),
            name: name.to_string(),
            status: Status::Pending,
            depends: vec![],
            phase: 0,
            critical: false,
        }
    }

    #[test]
    fn invalid_task_id_error() {
        let result = run(".", "BAD", "message");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid task ID format")
        );
    }

    #[test]
    fn empty_message_error() {
        // Checked before the status file is even read.
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let result = run(root, "T001", "");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Log message cannot be empty"
        );
    }

    #[test]
    fn task_not_found_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", "First"));
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T999", "message");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Task T999 not found");
    }

    #[test]
    fn missing_task_file_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", "First"));
        storage::write_status_file(root, &sf).unwrap();

        // No T001-first.md exists.
        let result = run(root, "T001", "message");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Task file"));
    }

    #[test]
    fn appends_entry_to_notes_section() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", "First"));
        storage::write_status_file(root, &sf).unwrap();

        let file_path = storage::task_file_path(root, "T001", "First");
        storage::write_task_file(&file_path, "T001", "First").unwrap();

        let result = run(root, "T001", "did the thing");
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("did the thing"));
        assert!(content.contains("## Notes"));
    }
}

use crate::models::Result;
use crate::models::TaskpadError;
use crate::storage;
use crate::utils;
use crate::validator;
use std::io::Write;
use std::path::Path;

/// Remove a task from `status.yaml` (and optionally delete its `.md` file).
///
/// Validates the task ID, warns when other tasks depend on the removed
/// task, prompts for confirmation unless `force`, drops the task from the
/// task map and the critical path, and writes `status.yaml` back.
/// Matching C++ `Commands::remove`.
pub fn run(tasks_dir: &str, task_id: &str, remove_all: bool, force: bool) -> Result<()> {
    if !validator::is_valid_task_id(task_id) {
        return Err(TaskpadError::Message(
            "Invalid task ID format. Expected TXXX (see Task ID Format)".into(),
        ));
    }

    let dir = utils::resolve_task_dir(tasks_dir);
    let mut sf = storage::read_status_file(&dir)?;

    let task_name = match sf.tasks.get(task_id) {
        Some(t) => t.name.clone(),
        None => {
            return Err(TaskpadError::Message(format!("Task {task_id} not found")));
        }
    };

    let task_path = storage::task_file_path(&dir, task_id, &task_name);
    let task_file = task_path
        .rsplit('/')
        .next()
        .unwrap_or(task_path.as_str())
        .to_string();

    // Warn about dependents (printed even when the removal is later declined).
    let mut dependents: Vec<String> = Vec::new();
    for (id, task) in &sf.tasks {
        if task.depends.iter().any(|dep| dep == task_id) {
            dependents.push(id.clone());
        }
    }
    if !dependents.is_empty() {
        eprintln!(
            "warning: The following tasks depend on {}: {}",
            task_id,
            dependents.join(" ")
        );
    }

    if !force {
        let prompt = if remove_all {
            format!(
                "Are you sure you want to remove {} and delete {}? [y/N] ",
                task_id, task_file
            )
        } else {
            format!(
                "Are you sure you want to remove {} from status.yaml? [y/N] ",
                task_id
            )
        };
        print!("{}", prompt);
        let _ = std::io::stdout().flush();

        if !read_confirmation() {
            return Ok(());
        }
    }

    sf.tasks.remove(task_id);
    sf.config.critical_path.retain(|id| id != task_id);
    storage::write_status_file(&dir, &sf)?;

    println!("Removed {} \u{2014} {}", task_id, task_name);
    println!("Updated status.yaml");

    if remove_all && Path::new(&task_path).exists() {
        let _ = std::fs::remove_file(&task_path);
        println!("Deleted {}", task_file);
    }

    Ok(())
}

/// Read one line from stdin and return `true` when it starts with `y` or
/// `Y` (matching the C++ `std::getline` + first-character check in
/// `Commands::remove`; an empty line, EOF, or read error declines).
fn read_confirmation() -> bool {
    let mut response = String::new();
    if std::io::stdin().read_line(&mut response).is_err() {
        return false;
    }
    confirmed(&response)
}

/// Return `true` when `response` counts as a "yes" — its first character
/// is `y` or `Y`, matching C++ `response[0] == 'y' || response[0] == 'Y'`.
fn confirmed(response: &str) -> bool {
    response.starts_with('y') || response.starts_with('Y')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Status;
    use crate::models::StatusFile;
    use crate::models::Task;
    use tempfile::tempdir;

    fn make_task(id: &str, depends: Vec<String>) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            status: Status::Pending,
            depends,
            phase: 0,
            critical: false,
        }
    }

    fn setup_project() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap().to_string();
        storage::create_config(&root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();
        dir
    }

    #[test]
    fn confirmed_accepts_yes_variants() {
        assert!(confirmed("y"));
        assert!(confirmed("y\n"));
        assert!(confirmed("Y\n"));
        assert!(confirmed("yes\n"));
    }

    #[test]
    fn confirmed_rejects_no_and_empty() {
        assert!(!confirmed(""));
        assert!(!confirmed("\n"));
        assert!(!confirmed("N\n"));
        assert!(!confirmed("no\n"));
        assert!(!confirmed(" y\n"));
    }

    #[test]
    fn invalid_task_id_error() {
        let result = run(".", "BAD", false, true);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid task ID format. Expected TXXX (see Task ID Format)"
        );
    }

    #[test]
    fn task_not_found_error() {
        let dir = setup_project();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", vec![]));
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T999", false, true);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Task T999 not found");
    }

    #[test]
    fn missing_status_yaml_error() {
        let dir = setup_project();
        let root = dir.path().to_str().unwrap();

        let result = run(root, "T001", false, true);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No status.yaml found. Run 'taskpad import' or 'taskpad new' first"
        );
    }

    #[test]
    fn force_removes_task_and_critical_path_entry() {
        let dir = setup_project();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", vec![]));
        sf.tasks.insert(
            "T002".to_string(),
            make_task("T002", vec!["T001".to_string()]),
        );
        sf.config.critical_path = vec!["T001".to_string(), "T002".to_string()];
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root, "T001", false, true);
        assert!(result.is_ok());

        let sf2 = storage::read_status_file(root).unwrap();
        assert!(!sf2.tasks.contains_key("T001"));
        assert!(sf2.tasks.contains_key("T002"));
        assert_eq!(sf2.config.critical_path, vec!["T002"]);
    }

    #[test]
    fn force_all_deletes_task_file() {
        let dir = setup_project();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        let task = Task {
            id: "T001".to_string(),
            name: "First Task".to_string(),
            status: Status::Pending,
            depends: vec![],
            phase: 0,
            critical: false,
        };
        sf.tasks.insert("T001".to_string(), task.clone());
        storage::write_status_file(root, &sf).unwrap();

        let md_path = storage::task_file_path(root, "T001", "First Task");
        std::fs::write(&md_path, "# T001: First Task\n").unwrap();
        assert!(Path::new(&md_path).exists());

        let result = run(root, "T001", true, true);
        assert!(result.is_ok());

        assert!(!Path::new(&md_path).exists());
        let sf2 = storage::read_status_file(root).unwrap();
        assert!(sf2.tasks.is_empty());
    }

    #[test]
    fn force_all_without_task_file_still_removes_entry() {
        let dir = setup_project();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", vec![]));
        storage::write_status_file(root, &sf).unwrap();

        // No T*.md file exists on disk.
        let result = run(root, "T001", true, true);
        assert!(result.is_ok());

        let sf2 = storage::read_status_file(root).unwrap();
        assert!(sf2.tasks.is_empty());
    }
}

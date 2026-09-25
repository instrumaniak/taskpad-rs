use crate::commands::{dependents_of, load_status, require_task_id, task_not_found};
use crate::models::Result;
use crate::models::Status;
use crate::models::status_to_string;

/// Show a task's dependencies and the tasks waiting on it.
///
/// Prints `TXXX depends on:` with a `✓`/`✗` marker per existing dependency
/// (done → `✓`, otherwise `✗`; unknown dependency IDs are printed bare),
/// then a reverse lookup under `Tasks waiting on TXXX:`.
/// Matching C++ `Commands::deps`.
pub(crate) fn run(tasks_dir: &str, task_id: &str) -> Result<()> {
    require_task_id(task_id)?;

    let (_, sf) = load_status(tasks_dir)?;

    let task = match sf.tasks.get(task_id) {
        Some(t) => t.clone(),
        None => return Err(task_not_found(task_id)),
    };

    println!("{task_id} depends on:");
    if task.depends.is_empty() {
        println!("  (none)");
    } else {
        for dep in &task.depends {
            print!("  {dep}");
            match sf.tasks.get(dep) {
                Some(dep_task) => {
                    print!(
                        "  {}  [{}]",
                        dep_task.name,
                        status_to_string(dep_task.status)
                    );
                    if dep_task.status == Status::Done {
                        println!(" \u{2713}");
                    } else {
                        println!(" \u{2717}");
                    }
                }
                None => println!(),
            }
        }
    }

    let dependents = dependents_of(&sf.tasks, task_id);

    println!();
    println!("Tasks waiting on {task_id}:");
    if dependents.is_empty() {
        println!("  (none)");
    } else {
        for id in dependents {
            if let Some(entry) = sf.tasks.get(id) {
                println!(
                    "  {id}  {}  [{}]",
                    entry.name,
                    status_to_string(entry.status)
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StatusFile;
    use crate::models::Task;
    use crate::storage;
    use tempfile::tempdir;

    fn make_task(id: &str, name: &str, status: Status, depends: Vec<String>) -> Task {
        Task {
            id: id.to_string(),
            name: name.to_string(),
            status,
            depends,
            phase: 0,
            critical: false,
        }
    }

    fn setup(tasks: Vec<Task>) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap().to_string();
        storage::create_config(&root, "tasks").unwrap();

        let mut sf = StatusFile::default();
        for task in tasks {
            sf.tasks.insert(task.id.clone(), task);
        }
        storage::write_status_file(&root, &sf).unwrap();
        dir
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
    fn task_not_found_error() {
        let dir = setup(vec![make_task("T001", "First", Status::Pending, vec![])]);
        let root = dir.path().to_str().unwrap();

        let result = run(root, "T999");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Task T999 not found");
    }

    #[test]
    fn missing_status_yaml_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let result = run(root, "T001");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No status.yaml found")
        );
    }

    #[test]
    fn no_deps_and_no_dependents() {
        let dir = setup(vec![make_task("T001", "First", Status::Pending, vec![])]);
        let root = dir.path().to_str().unwrap();

        assert!(run(root, "T001").is_ok());
    }

    #[test]
    fn deps_and_dependents_display() {
        let dir = setup(vec![
            make_task("T001", "First", Status::Done, vec![]),
            make_task(
                "T002",
                "Second",
                Status::Pending,
                vec!["T001".to_string(), "T999".to_string()],
            ),
            make_task(
                "T003",
                "Third",
                Status::InProgress,
                vec!["T002".to_string()],
            ),
        ]);
        let root = dir.path().to_str().unwrap();

        // T002 has a done dep (✓), a pending-marker unknown dep (bare id),
        // and one dependent (T003).
        assert!(run(root, "T002").is_ok());
        // T003 depends on the pending T002 → ✗ marker branch.
        assert!(run(root, "T003").is_ok());
    }
}

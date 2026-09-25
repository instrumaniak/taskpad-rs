use crate::commands::find_next_task_id;
use crate::models::Result;
use crate::models::Status;
use crate::models::Task;
use crate::models::TaskpadError;
use crate::storage;
use crate::utils;
use crate::validator;

/// Create a new task.
///
/// Validates the name, checks for duplicate names, validates
/// dependencies, detects circular dependencies, writes the T*.md
/// template, and updates `status.yaml`.
/// Matching C++ `Commands::new_`.
pub fn run(
    tasks_dir: &str,
    name: &str,
    depends: Vec<String>,
    phase: i32,
    critical: bool,
) -> Result<()> {
    if name.is_empty() {
        return Err(TaskpadError::Message("Task name cannot be empty".into()));
    }

    let dir = utils::resolve_task_dir(tasks_dir);

    let mut sf = storage::read_status_file(&dir).unwrap_or_default();

    for (id, task) in &sf.tasks {
        if task.name == name {
            eprintln!(
                "warning: Task with similar name exists: {}-{}",
                id,
                utils::to_kebab_case(name)
            );
        }
    }

    for dep in &depends {
        if !validator::is_valid_task_id(dep) {
            return Err(TaskpadError::Message(format!(
                "Invalid dependency ID: {dep}"
            )));
        }
        if !sf.tasks.contains_key(dep) {
            return Err(TaskpadError::Message(format!("Dependency {dep} not found")));
        }
    }

    let next_id = find_next_task_id(&sf.tasks);

    let temp_task = Task {
        id: next_id.clone(),
        depends: depends.clone(),
        ..Default::default()
    };
    sf.tasks.insert(next_id.clone(), temp_task);

    match validator::validate_circular_dependencies(&next_id, &depends, &sf.tasks) {
        Ok(()) => {}
        Err(e) => {
            sf.tasks.remove(&next_id);
            return Err(e);
        }
    }

    sf.tasks.remove(&next_id);

    let path = storage::task_file_path(&dir, &next_id, name);
    storage::write_task_file(&path, &next_id, name)?;

    let task = Task {
        id: next_id.clone(),
        name: name.to_string(),
        status: Status::Pending,
        depends: depends.clone(),
        phase,
        critical,
    };
    sf.tasks.insert(next_id.clone(), task);

    storage::write_status_file(&dir, &sf)?;

    println!("Created {}-{}.md", next_id, utils::to_kebab_case(name));
    println!("Updated status.yaml");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn empty_name_returns_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();
        let result = run(&format!("{}/tasks", root), "", vec![], 0, false);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Task name cannot be empty");
    }
}

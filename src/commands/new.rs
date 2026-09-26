use crate::commands::find_next_task_id;
use crate::models::Result;
use crate::models::Status;
use crate::models::StatusFile;
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
pub(crate) fn run(
    tasks_dir: &str,
    name: &str,
    depends: Vec<String>,
    phase: i32,
    critical: bool,
) -> Result<()> {
    // Reject whitespace-only names, not just empty ones. `to_kebab_case`
    // drops non-alphanumerics, so `"   "` kebabs to `""` and the task file
    // would be written as `T001.md`; `taskpad import` derives both the id
    // and the name from the `T001-<kebab>.md` filename, so such a task is
    // invisible to `import` forever. The C++ only tests `name.empty()`, so
    // rejecting blanks is a deliberate divergence (T020), kept under the
    // existing "Task name cannot be empty" message so no new string is
    // introduced.
    if name.trim().is_empty() {
        return Err(TaskpadError::Message("Task name cannot be empty".into()));
    }

    let dir = storage::resolve_task_dir(tasks_dir);

    let mut sf = match storage::read_status_file(&dir) {
        Ok(sf) => sf,
        // A missing `status.yaml` just means "no project yet", so start
        // from an empty file. Any other error (malformed YAML, wrong
        // shape) must propagate: silently falling back to an empty file
        // would overwrite and destroy the existing `status.yaml` below.
        // `read_status_file` (storage.rs) reports a missing file with
        // exactly this message, so match on it.
        Err(e)
            if e.to_string()
                == "No status.yaml found. Run 'taskpad import' or 'taskpad new' first" =>
        {
            StatusFile::default()
        }
        Err(e) => return Err(e),
    };

    for (id, task) in &sf.tasks {
        if task.name == name {
            eprintln!(
                "warning: Task with similar name exists: {}-{}.md",
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

    #[test]
    fn whitespace_only_name_returns_error_and_writes_nothing() {
        for name in ["   ", " ", "\t", "\n", " \t\n "] {
            let dir = tempdir().unwrap();
            let root = dir.path().to_str().unwrap();
            let tasks_dir = format!("{root}/tasks");
            std::fs::create_dir_all(&tasks_dir).unwrap();

            let result = run(&tasks_dir, name, vec![], 0, false);
            assert!(
                result.is_err(),
                "whitespace-only name {name:?} must be rejected"
            );
            assert_eq!(result.unwrap_err().to_string(), "Task name cannot be empty");

            // Nothing may be created: an unnamed task would land in a
            // kebab-less `T001.md` that `import` can never see.
            let entries: Vec<String> = std::fs::read_dir(&tasks_dir)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            assert!(entries.is_empty(), "name {name:?} created {entries:?}");
        }
    }

    #[test]
    fn normal_name_still_works() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let tasks_dir = format!("{root}/tasks");
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(&tasks_dir).ok();

        let result = run(&tasks_dir, "  Padded Name  ", vec![], 0, false);
        assert!(
            result.is_ok(),
            "got: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Surrounding whitespace is preserved in the name (kebab-case strips
        // it for the filename), exactly as before this check.
        let sf = storage::read_status_file(&tasks_dir).unwrap();
        assert_eq!(sf.tasks["T001"].name, "  Padded Name  ");

        let path = format!("{tasks_dir}/T001-padded-name.md");
        assert!(std::path::Path::new(&path).exists(), "expected {path}");
        let md = std::fs::read_to_string(&path).unwrap();
        assert!(md.starts_with("# T001:   Padded Name  \n"), "got: {md}");
    }

    #[test]
    fn missing_status_yaml_starts_empty_project() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let tasks_dir = format!("{}/tasks", root);
        storage::create_config(root, &tasks_dir).unwrap();
        std::fs::create_dir_all(&tasks_dir).ok();

        let result = run(&tasks_dir, "First Task", vec![], 0, false);
        assert!(result.is_ok());

        let sf = storage::read_status_file(&tasks_dir).unwrap();
        assert!(sf.tasks.contains_key("T001"));
    }

    #[test]
    fn corrupt_status_yaml_returns_error_and_preserves_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let tasks_dir = format!("{}/tasks", root);
        storage::create_config(root, &tasks_dir).unwrap();
        std::fs::create_dir_all(&tasks_dir).ok();

        let status_path = format!("{}/status.yaml", tasks_dir);
        let before = "tasks:\n  T001: [unclosed\n  bad indent:\n:\n";
        std::fs::write(&status_path, before).unwrap();
        // Sanity check: the fixture really is unreadable.
        assert!(storage::read_status_file(&tasks_dir).is_err());

        let result = run(&tasks_dir, "New Task", vec![], 0, false);
        assert!(result.is_err());

        let after = std::fs::read_to_string(&status_path).unwrap();
        assert_eq!(after, before);
    }
}

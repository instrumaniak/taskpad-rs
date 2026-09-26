use crate::models::Result;
use crate::models::TaskpadError;
use crate::storage;

/// Initialize a taskpad project.
///
/// Creates the `.taskpad` config file and the task directory, both under the
/// current directory (`createConfig(".", dir)`).
pub(crate) fn run(tasks_dir: &str) -> Result<()> {
    run_in(".", tasks_dir)
}

/// Initialize a taskpad project under an explicit project root.
///
/// `run` fixes the root to `"."`; this helper exposes the root so tests can
/// exercise initialization inside a temp dir without touching (or depending
/// on) the real working directory.
fn run_in(project_root: &str, tasks_dir: &str) -> Result<()> {
    let dir = if tasks_dir.is_empty() {
        "specs/tasks".to_string()
    } else {
        tasks_dir.to_string()
    };
    storage::create_config(project_root, &dir)?;
    std::fs::create_dir_all(&dir).map_err(|_| {
        TaskpadError::Message(format!("Cannot write to {}. Check permissions", dir))
    })?;
    println!("Initialized taskpad in {}/", dir);
    println!("Created .taskpad config");
    println!("Ready to add tasks with `taskpad new` or `taskpad import`");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn init_creates_config_and_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let task_dir = format!("{}/tasks", root);
        let result = run_in(root, &task_dir);
        assert!(result.is_ok());
        assert!(Path::new(format!("{}/.taskpad", root).as_str()).exists());
        assert!(Path::new(&task_dir).is_dir());
    }

    #[test]
    fn init_already_initialized_returns_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let task_dir = format!("{}/tasks", root);
        // First init populates the temp dir; the second must fail because
        // of that temp `.taskpad` — not because of whatever happens to be
        // in the process working directory.
        run_in(root, &task_dir).unwrap();
        let result = run_in(root, &task_dir);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Already initialized. Remove .taskpad to re-initialize"
        );
    }
}

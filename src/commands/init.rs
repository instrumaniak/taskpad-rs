use crate::models::Result;
use crate::storage;

/// Initialize a taskpad project.
///
/// Creates the `.taskpad` config file and the task directory.
/// Matching C++ `Commands::init`.
pub fn run(tasks_dir: &str) -> Result<()> {
    let dir = if tasks_dir.is_empty() {
        "specs/tasks".to_string()
    } else {
        tasks_dir.to_string()
    };
    storage::create_config(".", &dir)?;
    std::fs::create_dir_all(&dir).ok();
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
        storage::create_config(root, &task_dir).unwrap();
        std::fs::create_dir_all(&task_dir).ok();
        assert!(Path::new(format!("{}/.taskpad", root).as_str()).exists());
        assert!(Path::new(&task_dir).is_dir());
    }

    #[test]
    fn init_already_initialized_returns_error() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let task_dir = format!("{}/tasks", root);
        storage::create_config(root, &task_dir).unwrap();
        let result = run(&task_dir);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Already initialized. Remove .taskpad to re-initialize"
        );
    }
}

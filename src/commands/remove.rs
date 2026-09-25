use crate::commands::{dependents_of, load_status, require_task_id, task_not_found};
use crate::models::Result;
use crate::storage;
use crate::utils;
use std::io::Write;
use std::path::Path;

/// Remove a task from `status.yaml` (and optionally delete its `.md` file).
///
/// Validates the task ID, warns when other tasks depend on the removed
/// task, asks `confirm` for confirmation unless `force`, drops the task
/// from the task map and the critical path, and writes `status.yaml` back.
/// The caller supplies `confirm` (normally a stdin-backed closure from
/// `main.rs`); it receives the prompt text and returns `true` to proceed.
/// Keeping stdin out of here makes the command testable without a tty —
/// an empty line, EOF, or read error must decline (see `confirmed`).
/// Matching C++ `Commands::remove`.
pub(crate) fn run(
    tasks_dir: &str,
    task_id: &str,
    remove_all: bool,
    force: bool,
    confirm: impl FnOnce(&str) -> bool,
) -> Result<()> {
    require_task_id(task_id)?;

    let (dir, mut sf) = load_status(tasks_dir)?;

    let task_name = match sf.tasks.get(task_id) {
        Some(t) => t.name.clone(),
        None => return Err(task_not_found(task_id)),
    };

    // Match C++ `Commands::remove` (commands.cpp) byte-for-byte:
    // `taskId + "-" + toKebabCase(taskName) + ".md"` unconditionally, so an
    // empty task name yields `T001-.md`. (This deliberately differs from
    // `storage::task_file_path`, which collapses the empty-kebab case to
    // `T001.md`; `storage.rs` is outside this fix's scope so the C++
    // formula is spelled out here.)
    let task_path = format!(
        "{}/{}-{}.md",
        utils::normalize_path(&dir),
        task_id,
        utils::to_kebab_case(&task_name)
    );
    let task_file = task_path
        .rsplit('/')
        .next()
        .unwrap_or(task_path.as_str())
        .to_string();

    // Warn about dependents (printed even when the removal is later declined).
    let dependents: Vec<String> = dependents_of(&sf.tasks, task_id)
        .into_iter()
        .cloned()
        .collect();
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

        if !confirm(&prompt) {
            return Ok(());
        }
    }

    sf.tasks.remove(task_id);
    sf.config.critical_path.retain(|id| id != task_id);
    storage::write_status_file(&dir, &sf)?;

    println!("Removed {} \u{2014} {}", task_id, task_name);
    println!("Updated status.yaml");

    if remove_all && Path::new(&task_path).exists() {
        // Only claim the file is gone when it actually is: the previous code
        // discarded the `remove_file` result and printed "Deleted …" even
        // when the unlink failed. Silent on failure, as in the C++ original.
        if std::fs::remove_file(&task_path).is_ok() {
            println!("Deleted {}", task_file);
        }
    }

    Ok(())
}

/// Return `true` when `response` counts as a "yes" — its first character
/// is `y` or `Y`, matching C++ `response[0] == 'y' || response[0] == 'Y'`
/// (an empty line, EOF, or read error therefore declines).
pub(crate) fn confirmed(response: &str) -> bool {
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
        let result = run(".", "BAD", false, true, |_| true);
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

        let result = run(root, "T999", false, true, |_| true);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Task T999 not found");
    }

    #[test]
    fn missing_status_yaml_error() {
        let dir = setup_project();
        let root = dir.path().to_str().unwrap();

        let result = run(root, "T001", false, true, |_| true);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No status.yaml found. Run 'taskpad import' or 'taskpad new' first"
        );
    }

    #[test]
    fn declined_confirmation_keeps_task() {
        let dir = setup_project();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", vec![]));
        storage::write_status_file(root, &sf).unwrap();

        // Injected decline (`|_| false`) stands in for an empty line / EOF
        // on stdin without touching the real stdin.
        let result = run(root, "T001", false, false, |_| false);
        assert!(result.is_ok());

        let sf2 = storage::read_status_file(root).unwrap();
        assert!(sf2.tasks.contains_key("T001"));
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

        let result = run(root, "T001", false, true, |_| true);
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

        let result = run(root, "T001", true, true, |_| true);
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
        let result = run(root, "T001", true, true, |_| true);
        assert!(result.is_ok());

        let sf2 = storage::read_status_file(root).unwrap();
        assert!(sf2.tasks.is_empty());
    }

    #[test]
    fn force_all_with_empty_name_uses_cpp_dash_md_filename() {
        // `taskpad new` rejects empty names, but a hand-written
        // `status.yaml` entry can still have one. C++ builds the filename
        // as `taskId + "-" + toKebabCase(name) + ".md"` unconditionally,
        // i.e. `T001-.md` for an empty name.
        let dir = setup_project();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            Task {
                id: "T001".to_string(),
                name: String::new(),
                status: Status::Pending,
                depends: vec![],
                phase: 0,
                critical: false,
            },
        );
        storage::write_status_file(root, &sf).unwrap();

        let md_path = format!("{}/T001-.md", utils::normalize_path(root));
        std::fs::write(&md_path, "# T001:\n").unwrap();
        assert!(Path::new(&md_path).exists());

        let result = run(root, "T001", true, true, |_| true);
        assert!(result.is_ok());

        assert!(!Path::new(&md_path).exists());
        let sf2 = storage::read_status_file(root).unwrap();
        assert!(sf2.tasks.is_empty());
    }
}

use crate::commands::{count_status, extract_depends, extract_status_line, kebab_to_title};
use crate::models::Result;
use crate::models::Status;
use crate::models::StatusFile;
use crate::models::Task;
use crate::storage;
use crate::utils;
use crate::validator;

/// Import task files into `status.yaml`.
///
/// Scans the task directory for `T*.md` files, parses their headers,
/// validates dependencies, and writes `status.yaml`. Warnings are
/// printed to stderr but the function still returns Ok(()) and writes
/// the file (matching C++ `Commands::import_`).
/// Matching C++ `Commands::import_`.
pub fn run(tasks_dir: &str, force: bool) -> Result<()> {
    let dir = utils::resolve_task_dir(tasks_dir);

    match storage::read_status_file(&dir) {
        Ok(_) if !force => {
            return Err(crate::models::TaskpadError::Message(
                "status.yaml already exists. Use --force to overwrite".into(),
            ));
        }
        Ok(_) => {}
        Err(_) => {}
    }

    println!("Scanning {}/ for T*.md files...", dir);

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => {
            return Err(crate::models::TaskpadError::Message(format!(
                "Cannot scan {dir}"
            )));
        }
    };

    let mut task_files = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = match path.file_name().and_then(|f| f.to_str()) {
            Some(f) => f,
            None => continue,
        };
        if is_task_file(filename) {
            task_files.push(path);
        }
    }

    if task_files.is_empty() {
        println!("No T*.md files found in {}/", dir);
        return Ok(());
    }

    let mut sf = StatusFile::default();
    let mut errors = Vec::new();

    for path in &task_files {
        let fname = match path.file_name().and_then(|f| f.to_str()) {
            Some(f) => f,
            None => {
                errors.push(format!("Invalid filename: {}", path.display()));
                continue;
            }
        };
        let id = &fname[..4];
        let kebab_name = if fname.len() > 5 {
            &fname[5..fname.len() - 3]
        } else {
            ""
        };

        if !crate::validator::is_valid_task_id(id) {
            errors.push(format!("Invalid filename: {}", path.display()));
            continue;
        }

        let content = match storage::read_task_file(path.to_string_lossy().as_ref()) {
            Ok(c) => c,
            Err(e) => {
                errors.push(e.to_string());
                continue;
            }
        };

        let status_str = extract_status_line(&content);
        let status = if !status_str.is_empty() {
            crate::models::string_to_status(&status_str)
        } else {
            Status::Pending
        };
        let task = Task {
            id: id.to_string(),
            name: kebab_to_title(kebab_name),
            status,
            depends: extract_depends(&content),
            phase: crate::utils::extract_phase(&content),
            critical: crate::utils::extract_critical(&content),
        };

        sf.tasks.insert(id.to_string(), task);
    }

    for (id, task) in &sf.tasks {
        for dep in &task.depends {
            if !sf.tasks.contains_key(dep) {
                errors.push(format!("Task {id} depends on {dep} which was not found"));
            }
        }
    }

    for (id, task) in &sf.tasks {
        match validator::validate_circular_dependencies(id, &task.depends, &sf.tasks) {
            Ok(_) => {}
            Err(e) => errors.push(e.to_string()),
        }
    }

    if !errors.is_empty() {
        eprintln!("warning: {} issue(s) found", errors.len());
        for err in &errors {
            eprintln!("  {}", err);
        }
    }

    storage::write_status_file(&dir, &sf)?;

    let done = count_status(&sf.tasks, Status::Done);
    let in_prog = count_status(&sf.tasks, Status::InProgress);
    let pend = count_status(&sf.tasks, Status::Pending);

    println!("Found {} task files", sf.tasks.len());
    if force {
        println!("Overwrote status.yaml with {} tasks", sf.tasks.len());
    } else {
        println!("Created status.yaml with {} tasks", sf.tasks.len());
    }
    println!(
        "Status distribution: {} done, {} in_progress, {} pending",
        done, in_prog, pend
    );

    Ok(())
}

/// Check whether a filename matches the T*.md task-file pattern.
/// Matching C++ `filename[0]=='T' && isdigit(filename[1..4]) && filename[4]=='-' && ends_with(".md")`.
fn is_task_file(filename: &str) -> bool {
    let bytes = filename.as_bytes();
    bytes.len() > 4
        && bytes[0] == b'T'
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && filename.ends_with(".md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_task_file() {
        assert!(is_task_file("T001-name.md"));
        assert!(is_task_file("T999-project.md"));
        assert!(!is_task_file("T00-name.md"));
        assert!(!is_task_file("T001name.md"));
        assert!(!is_task_file("T001-name.txt"));
        assert!(!is_task_file("other.md"));
        assert!(!is_task_file("T001.md"));
    }

    #[test]
    fn test_extract_depends_pure() {
        let content = "## Depends On\n- T001\n- T002\n## Phase:";
        assert_eq!(extract_depends(content), vec!["T001", "T002"]);
    }

    #[test]
    fn test_kebab_to_title_pure() {
        assert_eq!(kebab_to_title("project-setup"), "Project Setup");
    }
}

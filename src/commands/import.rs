use crate::commands::{count_all, extract_depends, extract_status_line, kebab_to_title};
use crate::models::Result;
use crate::models::Status;
use crate::models::StatusFile;
use crate::models::Task;
use crate::models::TaskpadError;
use crate::storage;
use crate::validator;

/// Import task files into `status.yaml`.
///
/// Scans the task directory for `T*.md` files, parses their headers,
/// validates dependencies, and writes `status.yaml`. Warnings are
/// printed to stderr but the function still returns Ok(()) and writes
/// the file (matching C++ `Commands::import_`).
/// Matching C++ `Commands::import_`.
pub(crate) fn run(tasks_dir: &str, force: bool) -> Result<()> {
    let dir = storage::resolve_task_dir(tasks_dir);

    match storage::read_status_file(&dir) {
        // A status.yaml that could be read is the "already initialized"
        // case: only --force proceeds to overwrite it.
        Ok(_) if !force => {
            return Err(TaskpadError::Message(
                "status.yaml already exists. Use --force to overwrite".into(),
            ));
        }
        Ok(_) => {}
        // Only a genuinely absent status.yaml is tolerated here (a project
        // that has never been initialized). A read or parse failure aborts
        // instead: C++ cannot tell the two apart — its reader maps every
        // failure onto its "not found" message — so `taskpad import` there
        // would overwrite a status.yaml it had merely failed to read. The
        // file on disk is left untouched and the real error is reported.
        Err(e) if is_missing_status_file(&e) => {}
        Err(e) => return Err(e),
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
        let id = match fname.get(..4) {
            Some(id) => id,
            // Unreachable: `is_task_file` has already checked that the first
            // five bytes are `T`, three digits and `-`, all ASCII. Handled
            // explicitly so no byte-offset slicing below can ever panic.
            None => {
                errors.push(format!("Invalid filename: {}", path.display()));
                continue;
            }
        };
        // `strip_prefix`/`strip_suffix` instead of `&fname[5..len - 3]`: they
        // cannot split a multibyte character, so a task file with a non-ASCII
        // name (`T001-é.md`) yields `é` rather than a mid-codepoint panic.
        let kebab_name = fname
            .strip_prefix(id)
            .and_then(|rest| rest.strip_prefix('-'))
            .and_then(|rest| rest.strip_suffix(".md"))
            .unwrap_or("");

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

    let counts = count_all(&sf.tasks);
    let done = counts.done;
    let in_prog = counts.in_progress;
    let pend = counts.pending;

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

/// Whether `err` is [`storage::read_status_file`]'s "there is no
/// status.yaml here yet" error, as opposed to a real read/parse failure.
///
/// [`TaskpadError`] carries only the rendered message (it mirrors the C++
/// `Result<T>` value-or-message wrapper, with no structured payload), so the
/// one fixed literal is the discriminator. `read_status_file` raises it from
/// exactly one place — `ErrorKind::NotFound` — and every other failure
/// renders differently (`Cannot read <path>: <os error>` or
/// `Invalid status.yaml format…`), so no two cases can be confused.
fn is_missing_status_file(err: &TaskpadError) -> bool {
    err.to_string() == TaskpadError::no_status_yaml().to_string()
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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_is_task_file() {
        assert!(is_task_file("T001-name.md"));
        assert!(is_task_file("T999-project.md"));
        assert!(!is_task_file("T00-name.md"));
        assert!(!is_task_file("T001name.md"));
        assert!(!is_task_file("T001-name.txt"));
        assert!(!is_task_file("other.md"));
        assert!(!is_task_file("T001.md"));
        // Non-ASCII names are still T*.md files as far as the pattern goes.
        assert!(is_task_file("T001-é.md"));
    }

    #[test]
    fn test_is_missing_status_file_discriminates() {
        assert!(is_missing_status_file(&TaskpadError::no_status_yaml()));
        assert!(!is_missing_status_file(&TaskpadError::Message(
            "Invalid status.yaml format. Expected YAML mapping".into()
        )));
        assert!(!is_missing_status_file(&TaskpadError::Message(
            "Cannot read specs/tasks/status.yaml: Permission denied (os error 13)".into()
        )));
    }

    /// Write a minimal task file and return the tasks dir.
    fn project_with_task_file(body: &str) -> (tempfile::TempDir, String) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap().to_string();
        fs::write(format!("{root}/T001-{body}.md"), body).unwrap();
        (dir, root)
    }

    #[test]
    fn import_creates_status_file_when_none_exists() {
        let (_guard, root) = project_with_task_file("project-setup");
        assert!(run(&root, false).is_ok());
        let yaml = fs::read_to_string(format!("{root}/status.yaml")).unwrap();
        assert!(yaml.contains("T001"), "expected T001 in:\n{yaml}");
    }

    #[test]
    fn import_refuses_to_overwrite_without_force() {
        let (_guard, root) = project_with_task_file("project-setup");
        run(&root, false).unwrap();
        let before = fs::read_to_string(format!("{root}/status.yaml")).unwrap();

        // A second run reads status.yaml fine, so this is the C++
        // "already exists" refusal, unchanged.
        let err = run(&root, false).unwrap_err();
        assert_eq!(
            err.to_string(),
            "status.yaml already exists. Use --force to overwrite"
        );
        assert_eq!(
            fs::read_to_string(format!("{root}/status.yaml")).unwrap(),
            before,
            "a refused import must not touch status.yaml"
        );

        // --force is still allowed to overwrite it.
        assert!(run(&root, true).is_ok());
    }

    #[test]
    fn import_aborts_on_unparseable_status_file_and_preserves_it() {
        let (_guard, root) = project_with_task_file("project-setup");
        let path = format!("{root}/status.yaml");
        // A YAML root that is not a mapping: `read_status_file` reports
        // "Invalid status.yaml format. Expected YAML mapping".
        let corrupt = "this root is not a mapping\n";
        fs::write(&path, corrupt).unwrap();

        for force in [false, true] {
            let err = run(&root, force).unwrap_err();
            assert_eq!(
                err.to_string(),
                "Invalid status.yaml format. Expected YAML mapping",
                "force={force}"
            );
            assert_eq!(
                fs::read(&path).unwrap(),
                corrupt.as_bytes(),
                "status.yaml must be preserved byte-for-byte (force={force})"
            );
        }
    }

    #[test]
    fn import_aborts_on_unreadable_status_file_and_preserves_it() {
        let (_guard, root) = project_with_task_file("project-setup");
        // A directory where status.yaml belongs: the read fails with EISDIR,
        // never ENOENT, so it must not be mistaken for a missing file.
        let path = format!("{root}/status.yaml");
        fs::create_dir(&path).unwrap();

        let err = run(&root, false).unwrap_err();
        assert!(
            err.to_string()
                .starts_with(&format!("Cannot read {path}: ")),
            "expected an accurate read error naming the path, got: {err}"
        );
        assert!(fs::metadata(&path).unwrap().is_dir());
    }
}

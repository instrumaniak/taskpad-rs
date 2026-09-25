#![allow(dead_code)]

//! Rust port of `storage.h` / `storage.cpp` from the C++ taskpad codebase.
//!
//! Handles reading and writing the `.taskpad` project config file and the
//! `status.yaml` task-metadata file. Uses `serde_saphyr` for YAML parsing and
//! serialization, matching the byte-level output of the C++ `YAML::Dump` emitter.

use crate::models::Result;
use crate::models::StatusFile;
use crate::models::TaskpadError;
use crate::utils::{current_timestamp, normalize_path, to_kebab_case};
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

// ---------------------------------------------------------------------------
// .taskpad config
// ---------------------------------------------------------------------------

/// The `.taskpad` project config file, deserialized as a single-field YAML mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskpadConfig {
    #[serde(rename = "task-dir")]
    pub task_dir: String,
}

/// Read the task directory from the `.taskpad` config file at `project_root`.
///
/// Returns the `task-dir` value, or an error if the config file is missing,
/// malformed, or lacks the `task-dir` key.
pub fn read_task_dir(project_root: &str) -> Result<String> {
    let config_path = normalize_path(project_root) + "/.taskpad";
    let contents = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => {
            return Err(TaskpadError::Message(
                "Not initialized. Run 'taskpad init' first".into(),
            ));
        }
    };

    match serde_saphyr::from_str::<TaskpadConfig>(&contents) {
        Ok(cfg) => Ok(cfg.task_dir),
        Err(e) => {
            if !contents
                .lines()
                .any(|l| l.trim_start().starts_with("task-dir:"))
            {
                Err(TaskpadError::Message(
                    "Invalid .taskpad format. Expected YAML mapping with 'task-dir'".into(),
                ))
            } else {
                Err(TaskpadError::Message(format!(
                    "Invalid .taskpad format: {}",
                    e
                )))
            }
        }
    }
}

/// Create the `.taskpad` config file at `project_root` with the given `task_dir`.
///
/// Writes exactly `# taskpad project config\ntask-dir: {task_dir}\n`.
/// Errors if the config file already exists or if writing fails.
pub fn create_config(project_root: &str, task_dir: &str) -> Result<()> {
    let config_path = normalize_path(project_root) + "/.taskpad";
    if Path::new(&config_path).exists() {
        return Err(TaskpadError::Message(
            "Already initialized. Remove .taskpad to re-initialize".into(),
        ));
    }

    let cfg = TaskpadConfig {
        task_dir: task_dir.to_string(),
    };
    let mut yaml =
        serde_saphyr::to_string(&cfg).map_err(|e| TaskpadError::Message(e.to_string()))?;
    // serde_saphyr::to_string appends a trailing \n; strip it so we can
    // prepend the comment line and add exactly one trailing \n.
    yaml = yaml.trim_end_matches('\n').to_string();
    let content = format!("# taskpad project config\n{}\n", yaml);

    match std::fs::write(&config_path, content) {
        Ok(()) => Ok(()),
        Err(_) => Err(TaskpadError::Message(format!(
            "Cannot write to {}. Check permissions",
            config_path
        ))),
    }
}

/// Check whether a `.taskpad` config file exists at `project_root`.
pub fn config_exists(project_root: &str) -> bool {
    Path::new((normalize_path(project_root) + "/.taskpad").as_str()).exists()
}

// ---------------------------------------------------------------------------
// status.yaml
// ---------------------------------------------------------------------------

/// Read the `status.yaml` file from `task_dir` and deserialize it into a
/// [`StatusFile`].
///
/// Re-threads the `Task.id` from the map keys after deserialization.
/// Tolerates header comments, `depends: []`, `depends: ~`, flow-style depends,
/// and quoted/plain name values — all handled by serde-saphyr's lenient parser.
pub fn read_status_file(task_dir: &str) -> Result<StatusFile> {
    let path = normalize_path(task_dir) + "/status.yaml";
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            return Err(TaskpadError::Message(
                "No status.yaml found. Run 'taskpad import' or 'taskpad new' first".into(),
            ));
        }
    };

    match serde_saphyr::from_str::<StatusFile>(&contents) {
        Ok(mut sf) => {
            for (id, task) in sf.tasks.iter_mut() {
                task.id = id.clone();
            }
            Ok(sf)
        }
        Err(e) => {
            if yaml_is_mapping(&contents) {
                Err(TaskpadError::Message(format!(
                    "Invalid status.yaml format: {}",
                    e
                )))
            } else {
                Err(TaskpadError::Message(
                    "Invalid status.yaml format. Expected YAML mapping".into(),
                ))
            }
        }
    }
}

/// Write the [`StatusFile`] to `status.yaml` in `task_dir`.
///
/// Serializes via `serde_saphyr::to_string` (which appends a trailing `\n`),
/// then strips that newline so the output has no trailing newline at EOF,
/// matching the C++ `YAML::Dump` emitter byte-for-byte.
pub fn write_status_file(task_dir: &str, sf: &StatusFile) -> Result<()> {
    let path = normalize_path(task_dir) + "/status.yaml";
    let mut yaml = serde_saphyr::to_string(sf).map_err(|e| TaskpadError::Message(e.to_string()))?;
    // Strip the trailing \n that serde_saphyr::to_string appends,
    // so the output has no trailing newline at EOF.
    yaml = yaml.trim_end_matches('\n').to_string();

    match std::fs::write(&path, yaml) {
        Ok(()) => Ok(()),
        Err(_) => Err(TaskpadError::Message(format!(
            "Cannot write to {}. Check permissions",
            path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Task files (T*.md)
// ---------------------------------------------------------------------------

/// Return the file path for a task's T*.md file.
///
/// `normalize_path(task_dir) + "/" + task_id + "-" + to_kebab_case(task_name) + ".md"`.
/// If `to_kebab_case` yields an empty string, the filename is just
/// `task_id + ".md"`.
pub fn task_file_path(task_dir: &str, task_id: &str, task_name: &str) -> String {
    let dir = normalize_path(task_dir);
    let kebab = to_kebab_case(task_name);
    if kebab.is_empty() {
        format!("{}/{}.md", dir, task_id)
    } else {
        format!("{}/{}-{}.md", dir, task_id, kebab)
    }
}

/// Read the contents of a task file at `path`.
///
/// Returns `Err` with `"Task file {path} not found"` if the file does not exist.
pub fn read_task_file(path: &str) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(c) => Ok(c),
        Err(_) => Err(TaskpadError::Message(format!(
            "Task file {} not found",
            path
        ))),
    }
}

/// Write the task template to `path`.
///
/// Substitutes `<ID>` and `<Name>` in the template.  The template is defined
/// by spec.main.md §4 and must end with exactly one `\n`.
pub fn write_task_file(path: &str, task_id: &str, task_name: &str) -> Result<()> {
    let content = TASK_TEMPLATE
        .replace("<ID>", task_id)
        .replace("<Name>", task_name);
    match std::fs::write(path, content) {
        Ok(()) => Ok(()),
        Err(_) => Err(TaskpadError::Message(format!(
            "Cannot write to {}. Check permissions",
            path
        ))),
    }
}

/// Append a log entry to the `## Notes` section of a task file.
///
/// Entry format: `- [{current_timestamp}] {message}`.
/// If the file has no `## Notes` section, one is created.
pub fn append_log(path: &str, message: &str) -> Result<()> {
    let mut content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return Err(TaskpadError::Message(format!(
                "Task file {} not found",
                path
            )));
        }
    };

    let entry = format!("- [{}] {}", current_timestamp(), message);

    let notes_pos = content
        .rfind("\n## Notes")
        .or_else(|| content.rfind("## Notes"));

    if let Some(notes_pos) = notes_pos {
        let mut insert_pos = notes_pos;
        let eol = content[notes_pos + 2..].find('\n');
        if let Some(eol_offset) = eol {
            insert_pos = notes_pos + 2 + eol_offset + 1;
        }

        let next_section = content[insert_pos..].find("\n## ");
        if let Some(next_offset) = next_section {
            let next_section_pos = insert_pos + next_offset;
            content = format!(
                "{}\n{}{}",
                &content[..next_section_pos],
                entry,
                &content[next_section_pos..]
            );
        } else {
            content.push_str(&format!("\n{}\n", entry));
        }
    } else {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("\n## Notes\n\n{}\n", entry));
    }

    match std::fs::write(path, content) {
        Ok(()) => Ok(()),
        Err(_) => Err(TaskpadError::Message(format!(
            "Cannot write to {}. Check permissions",
            path
        ))),
    }
}

/// The task-file template from spec.main.md §4, with `<ID>` and `<Name>`
/// placeholders.  The string ends with exactly one `\n`.
const TASK_TEMPLATE: &str = "# <ID>: <Name>\n\n## Goal\n\n(Describe the goal)\n\n## Depends On\n\n(None)\n\n## Phase:\n\n(Add phase number here)\n\n## Critical:\n\n(Add critical flag here)\n\n## Spec References\n\n- (Add spec references here)\n\n## Files to Create/Modify\n\n- (Add files here)\n\n## Implementation Steps\n\n1. (Add steps here)\n\n## Constraints\n\n- (Add constraints here)\n\n## Acceptance Criteria\n\n- [ ] (Add criteria here)\n\n## Notes\n\n(filled in during/after implementation)\n";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return `true` if `contents` looks like a YAML mapping root.
///
/// Checks the first non-empty, non-comment line for a `key:` pattern.
/// Used to distinguish "root is not a mapping" from "parse exception"
/// when serde-saphyr deserialization fails.
fn yaml_is_mapping(contents: &str) -> bool {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('-') || trimmed.starts_with('[') || trimmed.starts_with('{') {
            return false;
        }
        if trimmed.contains(':') {
            return true;
        }
        return false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ---- config round-trip ----

    #[test]
    fn config_create_read_round_trip() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        create_config(root, "my-tasks").unwrap();
        let task_dir = read_task_dir(root).unwrap();
        assert_eq!(task_dir, "my-tasks");
    }

    #[test]
    fn config_create_already_exists() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        create_config(root, "my-tasks").unwrap();
        let result = create_config(root, "other");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert_eq!(msg, "Already initialized. Remove .taskpad to re-initialize");
    }

    #[test]
    fn config_exists_false_then_true() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        assert!(!config_exists(root));
        create_config(root, "my-tasks").unwrap();
        assert!(config_exists(root));
    }

    // ---- missing file errors ----

    #[test]
    fn read_task_dir_missing_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        // Don't create .taskpad
        let result = read_task_dir(root);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Not initialized. Run 'taskpad init' first"
        );
    }

    #[test]
    fn read_status_file_missing() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        // Don't create status.yaml
        let result = read_status_file(root);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No status.yaml found. Run 'taskpad import' or 'taskpad new' first"
        );
    }

    // ---- .taskpad bad format ----

    #[test]
    fn read_task_dir_no_task_dir_key() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let config_path = format!("{}/.taskpad", root);
        fs::write(&config_path, "foo: bar\n").unwrap();

        let result = read_task_dir(root);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid .taskpad format. Expected YAML mapping with 'task-dir'"
        );
    }

    // ---- status.yaml round-trip ----

    #[test]
    fn status_yaml_round_trip_with_phases_and_critical_path() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        let t1 = crate::models::Task {
            id: "T001".to_string(),
            name: "Project Setup".to_string(),
            status: crate::models::Status::Pending,
            depends: vec![],
            phase: 0,
            critical: true,
        };
        let t2 = crate::models::Task {
            id: "T002".to_string(),
            name: "Asset Acquisition".to_string(),
            status: crate::models::Status::Pending,
            depends: vec!["T001".to_string()],
            phase: 0,
            critical: false,
        };
        sf.tasks.insert("T001".to_string(), t1);
        sf.tasks.insert("T002".to_string(), t2);
        sf.config.phases.insert(0, "Scaffolding".to_string());
        sf.config.critical_path = vec!["T001".to_string()];

        write_status_file(root, &sf).unwrap();
        let loaded = read_status_file(root).unwrap();

        assert_eq!(loaded.tasks.len(), 2);
        assert_eq!(loaded.tasks["T001"].id, "T001");
        assert_eq!(loaded.tasks["T001"].name, "Project Setup");
        assert!(loaded.tasks["T001"].critical);
        assert_eq!(loaded.tasks["T002"].id, "T002");
        assert_eq!(loaded.tasks["T002"].depends, vec!["T001"]);
        assert_eq!(
            loaded.config.phases.get(&0),
            Some(&"Scaffolding".to_string())
        );
        assert_eq!(loaded.config.critical_path, vec!["T001"]);
    }

    // ---- C++ fixture smoke test ----

    #[test]
    fn read_cpp_fixture_status_yaml() {
        let contents = fs::read_to_string("specs/tasks/status.yaml").unwrap();
        let sf = serde_saphyr::from_str::<StatusFile>(&contents).unwrap();
        assert!(!sf.tasks.is_empty());
        assert!(sf.config.phases.contains_key(&0));
        assert!(!sf.config.critical_path.is_empty());
    }

    // ---- helpers-style status.yaml ----

    #[test]
    fn read_helpers_style_status_yaml() {
        let yaml = "# taskpad status file\n\ntasks:\n  T001:\n    name: \"Test\"\n    status: pending\n    depends: [T001]\n    phase: 0\n    critical: false\n";
        let mut sf = serde_saphyr::from_str::<StatusFile>(yaml).unwrap();
        for (id, task) in sf.tasks.iter_mut() {
            task.id = id.clone();
        }
        assert_eq!(sf.tasks.len(), 1);
        assert_eq!(sf.tasks["T001"].id, "T001");
        assert_eq!(sf.tasks["T001"].depends, vec!["T001"]);
    }

    // ---- write_status_file byte properties ----

    #[test]
    fn write_status_file_no_trailing_newline() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let sf = StatusFile::default();
        write_status_file(root, &sf).unwrap();
        let yaml = fs::read_to_string(format!("{}/status.yaml", root)).unwrap();
        assert!(
            !yaml.ends_with('\n'),
            "write_status_file output must NOT end with \\n"
        );
    }

    #[test]
    fn write_status_file_empty_depends_is_tilde() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        let t = crate::models::Task {
            id: "T001".to_string(),
            name: "NoDeps".to_string(),
            status: crate::models::Status::Pending,
            depends: vec![],
            phase: 0,
            critical: false,
        };
        sf.tasks.insert("T001".to_string(), t);
        write_status_file(root, &sf).unwrap();
        let yaml = fs::read_to_string(format!("{}/status.yaml", root)).unwrap();
        assert!(
            yaml.contains("depends: ~"),
            "empty depends must serialize as `depends: ~`, got:\n{yaml}"
        );
    }

    #[test]
    fn write_status_file_phases_omitted_when_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let sf = StatusFile::default();
        write_status_file(root, &sf).unwrap();
        let yaml = fs::read_to_string(format!("{}/status.yaml", root)).unwrap();
        assert!(
            !yaml.contains("phases"),
            "phases must be omitted when empty"
        );
        assert!(
            !yaml.contains("critical_path"),
            "critical_path must be omitted when empty"
        );
    }

    #[test]
    fn write_status_file_phase_keys_unquoted() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        sf.config.phases.insert(0, "Scaffolding".to_string());
        write_status_file(root, &sf).unwrap();
        let yaml = fs::read_to_string(format!("{}/status.yaml", root)).unwrap();
        assert!(
            yaml.contains("0: Scaffolding"),
            "integer phase keys must be unquoted, got:\n{yaml}"
        );
    }

    // ---- write_config byte-exact ----

    #[test]
    fn create_config_byte_exact() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        create_config(root, "specs/tasks").unwrap();
        let content = fs::read_to_string(format!("{}/.taskpad", root)).unwrap();
        assert_eq!(content, "# taskpad project config\ntask-dir: specs/tasks\n");
    }

    // ---- task_file_path ----

    #[test]
    fn task_file_path_with_name() {
        assert_eq!(
            task_file_path("specs/tasks", "T001", "Project Setup"),
            "specs/tasks/T001-project-setup.md"
        );
    }

    #[test]
    fn task_file_path_empty_name() {
        assert_eq!(
            task_file_path("specs/tasks", "T001", ""),
            "specs/tasks/T001.md"
        );
    }

    // ---- read_task_file ----

    #[test]
    fn read_task_file_missing() {
        let dir = tempdir().unwrap();
        let path = format!("{}/T001-missing.md", dir.path().to_str().unwrap());
        let result = read_task_file(&path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not found"), "got: {}", err_msg);
    }

    #[test]
    fn read_task_file_existing() {
        let dir = tempdir().unwrap();
        let path = format!("{}/T001-test.md", dir.path().to_str().unwrap());
        fs::write(&path, "hello world").unwrap();
        let result = read_task_file(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    // ---- write_task_file ----

    #[test]
    fn write_task_file_exact_content() {
        let dir = tempdir().unwrap();
        let path = format!("{}/T001-test.md", dir.path().to_str().unwrap());
        write_task_file(&path, "T001", "Test Name").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let expected = TASK_TEMPLATE
            .replace("<ID>", "T001")
            .replace("<Name>", "Test Name");
        assert_eq!(content, expected);
        assert!(
            content.ends_with("(filled in during/after implementation)\n"),
            "must end with '(filled in during/after implementation)\\n'"
        );
        assert!(
            !content.contains("## Status:"),
            "must not contain ## Status: line"
        );
    }

    // ---- append_log ----

    #[test]
    fn append_log_no_notes_section() {
        let dir = tempdir().unwrap();
        let path = format!("{}/T001-test.md", dir.path().to_str().unwrap());
        fs::write(&path, "# T001: Test\n").unwrap();
        append_log(&path, "first log").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("## Notes"));
        assert!(content.contains("- ["));
        assert!(content.contains("first log"));
        // The ## Notes section should be created
        assert!(content.contains("\n## Notes\n\n"));
    }

    #[test]
    fn append_log_notes_is_last_section() {
        let dir = tempdir().unwrap();
        let path = format!("{}/T001-test.md", dir.path().to_str().unwrap());
        fs::write(&path, "# T001: Test\n\n## Notes\n\nexisting note\n").unwrap();
        append_log(&path, "second log").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("- ["));
        assert!(content.contains("second log"));
        // The entry should be under ## Notes
        let notes_pos = content.rfind("## Notes").unwrap();
        let after_notes = &content[notes_pos..];
        assert!(after_notes.contains("second log"));
    }

    #[test]
    fn append_log_notes_followed_by_another_section() {
        let dir = tempdir().unwrap();
        let path = format!("{}/T001-test.md", dir.path().to_str().unwrap());
        fs::write(
            &path,
            "# T001: Test\n\n## Notes\n\nexisting note\n\n## Files\n\n- a\n",
        )
        .unwrap();
        append_log(&path, "inserted log").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        // The entry should appear between ## Notes and ## Files
        let notes_idx = content.find("## Notes").unwrap();
        let files_idx = content.find("## Files").unwrap();
        assert!(notes_idx < files_idx);
        assert!(content.contains("inserted log"));
        // inserted_log should be between Notes and Files
        let between = &content[notes_idx..files_idx];
        assert!(between.contains("inserted log"));
    }

    // ---- timestamp format ----

    #[test]
    fn current_timestamp_format() {
        let ts = current_timestamp();
        assert_eq!(ts.len(), 16);
        assert_eq!(ts.chars().nth(4), Some('-'));
        assert_eq!(ts.chars().nth(7), Some('-'));
        assert_eq!(ts.chars().nth(10), Some(' '));
        assert_eq!(ts.chars().nth(13), Some(':'));
    }
}

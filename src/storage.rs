//!
//! Handles reading and writing the `.taskpad` project config file and the
//! `status.yaml` task-metadata file. Uses `serde_saphyr` for YAML parsing and
//! serialization, matching the byte-level output of the C++ `YAML::Dump` emitter.
//!
//! Project-dir resolution (`resolve_task_dir`) lives here next to
//! `read_task_dir` so that `utils` stays leaf-level and never depends on
//! this module (see `utils.rs` module docs).
//!
//! Every writer goes through [`atomic_write`]: content is staged in a temp
//! file beside the target and renamed over it, so a reader never sees a
//! half-written `status.yaml`, `.taskpad`, or T*.md file. Errors keep the
//! C++ message text verbatim; the read side additionally distinguishes
//! `ErrorKind::NotFound` (the C++ "not initialized" / "no status.yaml"
//! messages) from every other read failure, which reports the OS reason.
//!
//! # Symlinks are followed, never rejected
//!
//! Nothing here inspects symlinks. `fs::read_to_string` resolves them, the
//! `rename` in [`atomic_write`] replaces the *link* rather than its target,
//! and a task-dir that is itself a symlink is used as given — the C++ tool
//! behaves the same way (`stat` in `fileExists`, `std::ifstream` in
//! `readTaskFile`, `std::ofstream` in the writers, all without
//! `O_NOFOLLOW`/`is_symlink` checks). So a symlinked `status.yaml` or
//! T*.md is read and written *through* the link, and no path component is
//! validated or re-rooted. This is recorded rather than defended: changing
//! it would change which bytes taskpad reads for a project that uses links.
//!
//! # Path containment
//!
//! [`task_file_path`] is the only place a task-controlled string becomes a
//! path. The task *name* half is sanitized by [`to_kebab_case`] and can
//! never escape the task dir; the task-ID half and the task dir itself are
//! used verbatim, exactly as C++ `taskFilePath` does, so the ID is a
//! documented caller precondition (every command validates it with
//! [`crate::validator::is_valid_task_id`] first). See the `debug_assert!`
//! on that function.

use crate::models::Result;
use crate::models::StatusFile;
use crate::models::TaskpadError;
use crate::utils::{normalize_path, to_kebab_case};
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

// ---------------------------------------------------------------------------
// Atomic writes
// ---------------------------------------------------------------------------

/// Monotonic counter making every temp-file name unique within the process,
/// so two writers in the same process never race for the same temp path.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// How many times [`atomic_write`] retries after a temp-name collision
/// before giving up. A collision requires a leftover temp file from a
/// previous process that was given this same pid, so a handful of attempts
/// is already generous.
const TEMP_NAME_ATTEMPTS: u32 = 8;

/// Build a unique temp-file name for an atomic write.
///
/// The name is dot-prefixed and extension-less so a temp file stranded by a
/// crash is inert: it is not a `T*.md` file (so `taskpad import`, which scans
/// for exactly that shape, skips it) and it is not `status.yaml` or
/// `.taskpad`.
fn temp_file_name() -> String {
    let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(".taskpad-tmp-{}-{}", std::process::id(), seq)
}

/// Return the directory the temp file for `target` must live in.
///
/// The temp file has to be a sibling of the target: `rename` is only atomic
/// within a single filesystem. For a bare relative name like `status.yaml`,
/// `Path::parent` yields an empty path, which `Path::new(".")` stands in for.
fn sibling_dir(target: &Path) -> &Path {
    match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// Write `content` to `path` atomically: a fresh temp file is created in the
/// target's own directory, written in full, and then `rename`d over the
/// target. A concurrent reader therefore observes either the previous file or
/// the complete new one, never a half-written mix, and a failure part-way
/// through leaves the previous file untouched.
///
/// The temp file is removed on every failure path.
///
/// Any failure is reported as [`TaskpadError::cannot_write`] for `path` —
/// never for the temp name — so the user-visible message stays byte-identical
/// to the plain `fs::write` this replaces (`Cannot write to <path>. Check
/// permissions`), including the E2E case of a read-only target directory,
/// where creating the temp file fails first.
///
/// Durability across a power cut is deliberately not fsync-ed, matching the
/// C++ writer, which also only flushes user-space buffers.
fn atomic_write(path: &str, content: &str) -> Result<()> {
    let target = Path::new(path);
    let dir = sibling_dir(target);

    for _ in 0..TEMP_NAME_ATTEMPTS {
        let temp = dir.join(temp_file_name());
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => file,
            // Only a leftover temp file can trigger this; take a fresh name.
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(TaskpadError::cannot_write(path)),
        };
        let written = file
            .write_all(content.as_bytes())
            .and_then(|()| file.flush());
        drop(file);
        if written.is_ok() && fs::rename(&temp, target).is_ok() {
            return Ok(());
        }
        // Either the write or the rename failed: drop the temp file and
        // report the failure against the real target, never the temp name.
        let _ = fs::remove_file(&temp);
        return Err(TaskpadError::cannot_write(path));
    }

    Err(TaskpadError::cannot_write(path))
}

// ---------------------------------------------------------------------------
// .taskpad config
// ---------------------------------------------------------------------------

/// The `.taskpad` project config file, deserialized as a single-field YAML mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskpadConfig {
    #[serde(rename = "task-dir")]
    pub(crate) task_dir: String,
}

/// Read the task directory from the `.taskpad` config file at `project_root`.
///
/// Returns the `task-dir` value, or an error if the config file is missing,
/// malformed, or lacks the `task-dir` key.
///
/// Only a genuinely absent file (`ErrorKind::NotFound`) reports
/// "Not initialized"; any other read failure — permissions, a directory in
/// the file's place, invalid UTF-8 — reports [`TaskpadError::cannot_read`]
/// with the OS reason, so a broken project is never misreported as absent.
pub(crate) fn read_task_dir(project_root: &str) -> Result<String> {
    let config_path = normalize_path(project_root) + "/.taskpad";
    let contents = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(TaskpadError::Message(
                "Not initialized. Run 'taskpad init' first".into(),
            ));
        }
        Err(e) => return Err(TaskpadError::cannot_read(&config_path, &e)),
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
/// Writes exactly `# taskpad project config\ntask-dir: {task_dir}\n`, through
/// the same atomic temp-file-and-rename path as every other writer.
/// Errors if the config file already exists or if writing fails.
pub(crate) fn create_config(project_root: &str, task_dir: &str) -> Result<()> {
    let config_path = normalize_path(project_root) + "/.taskpad";
    let already_initialized =
        || TaskpadError::Message("Already initialized. Remove .taskpad to re-initialize".into());
    if Path::new(&config_path).exists() {
        return Err(already_initialized());
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

    // Reserve the destination name before the rename: `create_new` fails
    // with `AlreadyExists` instead of truncating, so an existing `.taskpad`
    // can never be clobbered even if it appears after the `exists()` check
    // above (the check stays for the friendly error on the normal path).
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config_path)
    {
        Ok(reservation) => drop(reservation),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            return Err(already_initialized());
        }
        Err(_) => return Err(TaskpadError::cannot_write(&config_path)),
    }

    if let Err(err) = atomic_write(&config_path, &content) {
        // Drop the reservation, so a failed init cannot leave an empty
        // `.taskpad` behind that would make the next init claim the project
        // is already initialized.
        let _ = fs::remove_file(&config_path);
        return Err(err);
    }
    Ok(())
}

/// Resolve the task directory: return `tasks_dir` if non-empty, otherwise
/// read from the project config; fall back to `"specs/tasks"` on any
/// `read_task_dir` failure (matching C++ `resolveTaskDir` in `utils.cpp`).
pub(crate) fn resolve_task_dir(tasks_dir: &str) -> String {
    if !tasks_dir.is_empty() {
        return tasks_dir.to_string();
    }
    match read_task_dir(".") {
        Ok(dir) => dir,
        Err(_) => "specs/tasks".to_string(),
    }
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
///
/// Only a genuinely absent file (`ErrorKind::NotFound`) reports
/// [`TaskpadError::no_status_yaml`]; any other read failure — permissions, a
/// directory in the file's place, invalid UTF-8 — reports
/// [`TaskpadError::cannot_read`] with the OS reason, so a corrupt or
/// unreadable project is never mistaken for an uninitialized one. That
/// distinction matters: `commands::new` treats a missing file as "no project
/// yet" and starts from an empty one, but must propagate every other error
/// rather than overwrite a file it could not read.
pub(crate) fn read_status_file(task_dir: &str) -> Result<StatusFile> {
    let path = normalize_path(task_dir) + "/status.yaml";
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(TaskpadError::no_status_yaml());
        }
        Err(e) => return Err(TaskpadError::cannot_read(&path, &e)),
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
/// Serializes via `serde_saphyr::to_string_with_options` (which appends a
/// trailing `\n`), then strips that newline so the output has no trailing
/// newline at EOF, matching the C++ `YAML::Dump` emitter byte-for-byte.
///
/// `compact_list_indent: false` is required for parity: yaml-cpp indents block
/// sequence items one step deeper than their parent mapping key
/// (`depends:` → `      - T001`, `critical_path:` → `  - T001`), whereas
/// serde-saphyr's default compact style would put them at the key's indent.
///
/// The bytes are staged in a temp file and renamed over `status.yaml`
/// ([`atomic_write`]), so a reader never sees a truncated file; the emitted
/// content itself is unchanged.
pub(crate) fn write_status_file(task_dir: &str, sf: &StatusFile) -> Result<()> {
    let path = normalize_path(task_dir) + "/status.yaml";
    let opts = serde_saphyr::ser_options! {
        compact_list_indent: false,
    };
    let mut yaml = serde_saphyr::to_string_with_options(sf, opts)
        .map_err(|e| TaskpadError::Message(e.to_string()))?;
    // Strip the trailing \n that serde_saphyr::to_string appends,
    // so the output has no trailing newline at EOF.
    yaml = yaml.trim_end_matches('\n').to_string();

    atomic_write(&path, &yaml)
}

// ---------------------------------------------------------------------------
// Task files (T*.md)
// ---------------------------------------------------------------------------

/// Return the file path for a task's T*.md file.
///
/// `normalize_path(task_dir) + "/" + task_id + "-" + to_kebab_case(task_name) + ".md"`.
/// If `to_kebab_case` yields an empty string, the filename is just
/// `task_id + ".md"`.
///
/// # Containment
///
/// A hostile **task name** cannot escape `task_dir`: [`to_kebab_case`] keeps
/// only ASCII alphanumerics and `-`, so `../../etc/passwd` becomes
/// `etcpasswd` and the result is `<dir>/T001-etcpasswd.md`. The `debug_assert!`
/// below pins that invariant; the test `task_file_path_never_escapes_the_task_dir`
/// exercises it against real hostile names and a real directory.
///
/// The other two components are **not** sanitized, deliberately:
///
/// - `task_id` is concatenated verbatim (C++ `taskFilePath` does the same).
///   Callers must validate it with [`crate::validator::is_valid_task_id`]
///   first — every command does, at its entry point, via
///   [`crate::commands::require_task_id`].
/// - `task_dir` is used exactly as resolved, never re-rooted, rejected or
///   canonicalized. C++ accepts any string here, so `taskpad --tasks-dir
///   ../../elsewhere` legitimately reads and writes there; that is
///   intentional, not an oversight.
pub(crate) fn task_file_path(task_dir: &str, task_id: &str, task_name: &str) -> String {
    let dir = normalize_path(task_dir);
    let kebab = to_kebab_case(task_name);
    debug_assert!(
        !kebab.contains('/') && !kebab.contains('\\') && !kebab.contains(".."),
        "to_kebab_case must never emit a path separator or a '..' component"
    );
    if kebab.is_empty() {
        format!("{}/{}.md", dir, task_id)
    } else {
        format!("{}/{}-{}.md", dir, task_id, kebab)
    }
}

/// Read the contents of a task file at `path`.
///
/// Returns `Err` with `"Task file {path} not found"` if the file does not exist.
pub(crate) fn read_task_file(path: &str) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(c) => Ok(c),
        Err(_) => Err(TaskpadError::task_file_not_found(path)),
    }
}

/// Write the task template to `path`.
///
/// Substitutes `<ID>` and `<Name>` in the template.  The template is defined
/// by spec.main.md §4 and must end with exactly one `\n`.  Staged in a temp
/// file and renamed over `path` ([`atomic_write`]), so an interrupted write
/// cannot leave a half-written task file behind.
pub(crate) fn write_task_file(path: &str, task_id: &str, task_name: &str) -> Result<()> {
    let content = TASK_TEMPLATE
        .replace("<ID>", task_id)
        .replace("<Name>", task_name);
    atomic_write(path, &content)
}

/// Append a log entry to the `## Notes` section of a task file.
///
/// Entry format: `- [{timestamp}] {message}`. The caller supplies
/// `timestamp` (normally `utils::current_timestamp()`); taking it as a
/// parameter keeps this function pure and its tests deterministic.
/// If the file has no `## Notes` section, one is created.
///
/// The rewritten file is staged in a temp file and renamed over `path`
/// ([`atomic_write`]), so a failed append leaves the original notes intact.
pub(crate) fn append_log(path: &str, message: &str, timestamp: &str) -> Result<()> {
    let mut content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return Err(TaskpadError::task_file_not_found(path));
        }
    };

    let entry = format!("- [{timestamp}] {message}");

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

    atomic_write(path, &content)
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

    // ---- read errors: NotFound vs. everything else ----
    //
    // A file that exists but cannot be read is NOT a missing file: reporting
    // it as one lets `taskpad new` start from an empty project and overwrite
    // a `status.yaml` it merely failed to read.

    #[test]
    fn read_status_file_directory_is_not_reported_as_missing() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        // A directory where status.yaml belongs: read_to_string fails with
        // EISDIR, never ENOENT.
        fs::create_dir(format!("{root}/status.yaml")).unwrap();

        let msg = read_status_file(root).unwrap_err().to_string();
        assert_ne!(
            msg, "No status.yaml found. Run 'taskpad import' or 'taskpad new' first",
            "a directory in place of status.yaml must not report 'No status.yaml found'"
        );
        assert!(
            msg.starts_with(&format!("Cannot read {root}/status.yaml: ")),
            "expected an accurate read error naming the path, got: {msg}"
        );
    }

    #[test]
    fn read_status_file_invalid_utf8_is_not_reported_as_missing() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        // Valid file, invalid UTF-8 contents (read_to_string -> InvalidData).
        fs::write(format!("{root}/status.yaml"), [0xff, 0xfe, 0x00]).unwrap();

        let msg = read_status_file(root).unwrap_err().to_string();
        assert_ne!(
            msg, "No status.yaml found. Run 'taskpad import' or 'taskpad new' first",
            "non-UTF-8 status.yaml must not report 'No status.yaml found'"
        );
        assert!(
            msg.starts_with(&format!("Cannot read {root}/status.yaml: ")),
            "expected an accurate read error naming the path, got: {msg}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn read_status_file_unreadable_file_is_not_reported_as_missing() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let path = format!("{root}/status.yaml");
        fs::write(&path, "tasks:\n").unwrap();
        let original = fs::metadata(&path).unwrap().permissions();
        fs::set_permissions(&path, fs::Permissions::from_mode(original.mode() & !0o222)).unwrap();

        // Self-skip when the mode is not enforced for us (e.g. the suite runs
        // as root): nothing to assert if the file is still readable.
        let msg = if fs::read_to_string(&path).is_ok() {
            fs::set_permissions(&path, original).unwrap();
            return;
        } else {
            let msg = read_status_file(root).unwrap_err().to_string();
            fs::set_permissions(&path, original).unwrap();
            msg
        };

        assert_ne!(
            msg, "No status.yaml found. Run 'taskpad import' or 'taskpad new' first",
            "an unreadable status.yaml must not report 'No status.yaml found'"
        );
        assert!(
            msg.starts_with(&format!("Cannot read {root}/status.yaml: ")),
            "expected an accurate read error naming the path, got: {msg}"
        );
    }

    #[test]
    fn read_task_dir_directory_is_not_reported_as_not_initialized() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        fs::create_dir(format!("{root}/.taskpad")).unwrap();

        let msg = read_task_dir(root).unwrap_err().to_string();
        assert_ne!(
            msg, "Not initialized. Run 'taskpad init' first",
            "a directory in place of .taskpad must not report 'Not initialized'"
        );
        assert!(
            msg.starts_with(&format!("Cannot read {root}/.taskpad: ")),
            "expected an accurate read error naming the path, got: {msg}"
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

    // ---- status.yaml writer byte parity ----
    //
    // These assert the exact bytes of the file on disk, including the absence
    // of a trailing newline, so they are the model-level expectations from
    // `models::tests` checked through the real writer + `atomic_write` path.
    // Every expected string is a literal transcription of the C++ binary's
    // `YAML::Dump` output, captured from `../taskpad/taskpad` and re-checked
    // with `xxd` (Locked decision §4 / ruling #2).

    /// Build a [`Task`] for the writer-parity tests. `id` is irrelevant to
    /// serialization (`#[serde(skip)]`); `read_status_file` re-threads it.
    fn parity_task(
        name: &str,
        status: crate::models::Status,
        depends: &[&str],
    ) -> crate::models::Task {
        crate::models::Task {
            name: name.to_string(),
            status,
            depends: depends.iter().map(|d| d.to_string()).collect(),
            ..Default::default()
        }
    }

    /// Write `sf` into a fresh temp dir and return the raw `status.yaml` bytes.
    fn write_and_read_bytes(sf: &StatusFile) -> String {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        write_status_file(root, sf).unwrap();
        fs::read_to_string(Path::new(root).join("status.yaml")).unwrap()
    }

    #[test]
    fn write_status_file_empty_project_is_exactly_tasks_tilde() {
        // C++ ground truth: `taskpad init && taskpad new "Only Task" &&
        // taskpad remove T001 --force` leaves an 8-byte file reading `tasks: ~`
        // (verified with `xxd`: 74 61 73 6b 73 3a 20 7e) — a Null node dumped
        // by yaml-cpp, because `YAML::Node tasksNode;` is never assigned when
        // the project has no tasks but `root["tasks"] = tasksNode` runs
        // regardless (`storage.cpp:154-172`).
        let yaml = write_and_read_bytes(&StatusFile::default());
        assert_eq!(yaml, "tasks: ~");
        assert!(!yaml.ends_with('\n'), "C++ writes no trailing newline");
    }

    #[test]
    fn write_status_file_single_task_is_exact_bytes() {
        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            parity_task("Only Task", crate::models::Status::Pending, &[]),
        );
        let yaml = write_and_read_bytes(&sf);
        assert_eq!(
            yaml,
            concat!(
                "tasks:\n",
                "  T001:\n",
                "    name: Only Task\n",
                "    status: pending\n",
                "    depends: ~\n",
                "    phase: 0\n",
                "    critical: false",
            )
        );
        assert!(!yaml.ends_with('\n'));
    }

    #[test]
    fn write_status_file_multi_task_is_exact_bytes() {
        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            parity_task("Task One", crate::models::Status::Done, &[]),
        );
        sf.tasks.insert(
            "T002".to_string(),
            parity_task("Task Two", crate::models::Status::InProgress, &["T001"]),
        );
        sf.tasks.insert(
            "T010".to_string(),
            parity_task(
                "Task Ten",
                crate::models::Status::Pending,
                &["T001", "T002"],
            ),
        );
        let yaml = write_and_read_bytes(&sf);
        assert_eq!(
            yaml,
            concat!(
                "tasks:\n",
                "  T001:\n",
                "    name: Task One\n",
                "    status: done\n",
                "    depends: ~\n",
                "    phase: 0\n",
                "    critical: false\n",
                "  T002:\n",
                "    name: Task Two\n",
                "    status: in_progress\n",
                "    depends:\n",
                "      - T001\n",
                "    phase: 0\n",
                "    critical: false\n",
                "  T010:\n",
                "    name: Task Ten\n",
                "    status: pending\n",
                "    depends:\n",
                "      - T001\n",
                "      - T002\n",
                "    phase: 0\n",
                "    critical: false",
            )
        );
        assert!(!yaml.ends_with('\n'));
    }

    #[test]
    fn write_status_file_task_with_empty_depends_is_exact_bytes() {
        // The empty-`depends` null is separate from the empty-`tasks` one: a
        // non-empty project still spells an empty dependency list `~`, and
        // yaml-cpp never emits `[]` for it.
        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            parity_task("No Deps", crate::models::Status::Pending, &[]),
        );
        let yaml = write_and_read_bytes(&sf);
        assert_eq!(
            yaml,
            concat!(
                "tasks:\n",
                "  T001:\n",
                "    name: No Deps\n",
                "    status: pending\n",
                "    depends: ~\n",
                "    phase: 0\n",
                "    critical: false",
            )
        );
        assert!(!yaml.contains("[]"), "C++ never emits an empty flow list");
    }

    #[test]
    fn write_status_file_with_phases_and_critical_path_is_exact_bytes() {
        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            parity_task("Scaffold", crate::models::Status::Done, &[]),
        );
        let mut t2 = parity_task("Build", crate::models::Status::Pending, &["T001"]);
        t2.phase = 1;
        t2.critical = true;
        sf.tasks.insert("T002".to_string(), t2);
        sf.config.phases.insert(0, "Scaffolding".to_string());
        sf.config.phases.insert(1, "Building".to_string());
        sf.config.critical_path = vec!["T001".to_string(), "T002".to_string()];
        let yaml = write_and_read_bytes(&sf);
        assert_eq!(
            yaml,
            concat!(
                "tasks:\n",
                "  T001:\n",
                "    name: Scaffold\n",
                "    status: done\n",
                "    depends: ~\n",
                "    phase: 0\n",
                "    critical: false\n",
                "  T002:\n",
                "    name: Build\n",
                "    status: pending\n",
                "    depends:\n",
                "      - T001\n",
                "    phase: 1\n",
                "    critical: true\n",
                "phases:\n",
                "  0: Scaffolding\n",
                "  1: Building\n",
                "critical_path:\n",
                "  - T001\n",
                "  - T002",
            )
        );
        assert!(!yaml.ends_with('\n'));
    }

    #[test]
    fn read_status_file_accepts_null_and_empty_map_tasks() {
        // `tasks: ~` is what the C++ writer emits once a project is empty, so
        // reading it back must succeed (a round trip, not a parse error) —
        // `tasks: {}` must keep working too. C++ is equally tolerant: its
        // reader guards on `IsDefined() && IsMap()` (`storage.cpp:110`), so a
        // Null `tasks` node is simply skipped.
        for spelling in ["tasks: ~", "tasks: null", "tasks: {}"] {
            let dir = tempdir().unwrap();
            let root = dir.path().to_str().unwrap();
            fs::write(Path::new(root).join("status.yaml"), format!("{spelling}\n")).unwrap();
            let sf = read_status_file(root).unwrap();
            assert!(
                sf.tasks.is_empty(),
                "{spelling:?} did not read back as an empty task map"
            );
            assert!(sf.config.phases.is_empty());
            assert!(sf.config.critical_path.is_empty());
        }
    }

    #[test]
    fn empty_project_write_read_write_is_stable() {
        // Re-writing a project that was read from a C++-written `tasks: ~` must
        // not drift to `{}` (or grow a trailing newline) on the second write.
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let path = Path::new(root).join("status.yaml");
        fs::write(&path, "tasks: ~\n").unwrap();

        let sf = read_status_file(root).unwrap();
        write_status_file(root, &sf).unwrap();
        let first = fs::read_to_string(&path).unwrap();
        assert_eq!(first, "tasks: ~");

        let sf = read_status_file(root).unwrap();
        write_status_file(root, &sf).unwrap();
        let second = fs::read_to_string(&path).unwrap();
        assert_eq!(second, first);
    }

    // ---- C++ fixture smoke test ----

    #[test]
    fn read_cpp_fixture_status_yaml() {
        // `CARGO_MANIFEST_DIR` (not a bare relative path) so the fixture
        // resolves the same way under `cargo test --manifest-path …` from
        // any working directory.
        let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/specs/tasks/status.yaml");
        let contents = fs::read_to_string(fixture).unwrap();
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

    // ---- atomic writes ----

    /// Sorted listing of the names in `dir` (so stray temp files are visible).
    fn dir_entries(dir: &str) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_leave_no_temp_files_behind() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        create_config(root, "tasks").unwrap();
        write_status_file(root, &StatusFile::default()).unwrap();
        let md = format!("{root}/T001-test.md");
        write_task_file(&md, "T001", "Test").unwrap();
        append_log(&md, "a note", "2026-01-02 03:04").unwrap();

        assert_eq!(
            dir_entries(root),
            vec![".taskpad", "T001-test.md", "status.yaml"]
        );
    }

    #[test]
    fn write_status_file_replaces_existing_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            crate::models::Task {
                id: "T001".to_string(),
                name: "First".to_string(),
                status: crate::models::Status::Done,
                depends: vec![],
                phase: 0,
                critical: false,
            },
        );
        write_status_file(root, &sf).unwrap();

        // Overwrite with a different project: no leftover of the old bytes.
        write_status_file(root, &StatusFile::default()).unwrap();

        let yaml = fs::read_to_string(format!("{root}/status.yaml")).unwrap();
        assert!(!yaml.contains("First"), "old content survived: {yaml}");
        assert_eq!(dir_entries(root), vec!["status.yaml"]);
    }

    #[test]
    fn write_status_file_missing_directory_reports_cannot_write() {
        let dir = tempdir().unwrap();
        let missing = format!("{}/nope", dir.path().to_str().unwrap());

        let msg = write_status_file(&missing, &StatusFile::default())
            .unwrap_err()
            .to_string();
        // The message must name the real target, never the temp file.
        assert_eq!(
            msg,
            format!("Cannot write to {missing}/status.yaml. Check permissions")
        );
    }

    #[test]
    fn write_task_file_missing_directory_reports_cannot_write() {
        let dir = tempdir().unwrap();
        let missing = format!("{}/nope/T001-test.md", dir.path().to_str().unwrap());

        let msg = write_task_file(&missing, "T001", "Test")
            .unwrap_err()
            .to_string();
        assert_eq!(msg, format!("Cannot write to {missing}. Check permissions"));
    }

    #[test]
    fn append_log_unwritable_target_reports_cannot_write() {
        let dir = tempdir().unwrap();
        let missing = format!("{}/nope/T001-test.md", dir.path().to_str().unwrap());

        // append_log reads first, so a missing file is a task-file error, not
        // a write error: the file has to exist for the write path to run.
        let msg = append_log(&missing, "note", "2026-01-02 03:04")
            .unwrap_err()
            .to_string();
        assert_eq!(msg, format!("Task file {missing} not found"));
    }

    #[test]
    fn create_config_does_not_clobber_existing_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let path = format!("{root}/.taskpad");

        // Hand-written config with a comment: a truncating write would lose
        // the comment and change task-dir.
        fs::write(&path, "# hand written\ntask-dir: keep-me\n").unwrap();

        let msg = create_config(root, "overwrite").unwrap_err().to_string();
        assert_eq!(msg, "Already initialized. Remove .taskpad to re-initialize");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# hand written\ntask-dir: keep-me\n"
        );
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

    /// Task names that try to escape the task directory, plus a few benign
    /// non-ASCII ones.
    const HOSTILE_NAMES: &[&str] = &[
        "../../etc/passwd",
        "..",
        "a/b",
        "/etc/passwd",
        "..\\..\\windows",
        "T002/../../T003",
        "....//....//x",
        "\u{202e}\u{1f600}",
        "日本語/テスト",
        "",
    ];

    #[test]
    fn task_file_path_never_escapes_the_task_dir() {
        for name in HOSTILE_NAMES {
            let path = task_file_path("specs/tasks", "T001", name);
            assert_eq!(
                Path::new(&path).parent(),
                Some(Path::new("specs/tasks")),
                "name {name:?} produced a path outside the task dir: {path}"
            );
            // The file name is a single component: no separator, no `..`.
            let file_name = Path::new(&path)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("");
            assert!(
                !file_name.is_empty()
                    && !file_name.contains('/')
                    && !file_name.contains('\\')
                    && !file_name.contains(".."),
                "name {name:?} produced a non-atomic file name: {file_name:?}"
            );
        }
    }

    #[test]
    fn task_file_path_containment_holds_on_a_real_directory() {
        // The same guarantee against the filesystem: every hostile name lands
        // inside the task dir, and nothing lands in a sibling directory.
        let base = tempdir().unwrap();
        let root = base.path().join("tasks");
        let outside = base.path().join("outside");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&outside).unwrap();
        let root = root.to_str().unwrap();

        for (index, name) in HOSTILE_NAMES.iter().enumerate() {
            // A distinct ID per iteration, since several hostile names
            // sanitize to the same file name (e.g. "..", "" and "日本語/テスト"
            // all collapse to `T001.md`).
            let id = format!("T{:03}", index + 1);
            let path = task_file_path(root, &id, name);
            assert_eq!(
                Path::new(&path).parent(),
                Some(Path::new(root)),
                "name {name:?} produced a path outside {root}: {path}"
            );
            write_task_file(&path, &id, name).unwrap();
            assert!(Path::new(&path).exists(), "{path} was not created");
            assert_eq!(
                fs::read_dir(root).unwrap().count(),
                index + 1,
                "unexpected number of files in {root} after name {name:?}"
            );
        }

        assert!(
            fs::read_dir(&outside).unwrap().next().is_none(),
            "a hostile task name wrote a file outside the task dir"
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

    // ---- append_log (caller-supplied timestamp; deterministic) ----

    #[test]
    fn append_log_no_notes_section() {
        let dir = tempdir().unwrap();
        let path = format!("{}/T001-test.md", dir.path().to_str().unwrap());
        fs::write(&path, "# T001: Test\n").unwrap();
        append_log(&path, "first log", "2026-01-02 03:04").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("## Notes"));
        assert!(content.contains("- [2026-01-02 03:04] first log"));
        // The ## Notes section should be created
        assert!(content.contains("\n## Notes\n\n"));
    }

    #[test]
    fn append_log_notes_is_last_section() {
        let dir = tempdir().unwrap();
        let path = format!("{}/T001-test.md", dir.path().to_str().unwrap());
        fs::write(&path, "# T001: Test\n\n## Notes\n\nexisting note\n").unwrap();
        append_log(&path, "second log", "2026-01-02 03:04").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("- [2026-01-02 03:04] second log"));
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
        append_log(&path, "inserted log", "2026-01-02 03:04").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        // The entry should appear between ## Notes and ## Files
        let notes_idx = content.find("## Notes").unwrap();
        let files_idx = content.find("## Files").unwrap();
        assert!(notes_idx < files_idx);
        assert!(content.contains("- [2026-01-02 03:04] inserted log"));
        // inserted_log should be between Notes and Files
        let between = &content[notes_idx..files_idx];
        assert!(between.contains("inserted log"));
    }

    // ---- resolve_task_dir ----

    #[test]
    fn resolve_task_dir_prefers_explicit_dir() {
        assert_eq!(resolve_task_dir("my-tasks"), "my-tasks");
        assert_eq!(resolve_task_dir(""), "specs/tasks");
    }
}

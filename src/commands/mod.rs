//! Shared command helpers for the taskpad CLI.
//!
//! This module re-exports the thirteen subcommand modules (`init`, `import`,
//! `new`, `status`, `next`, `do_cmd`, `done`, `pause`, `deps`, `log`, `edit`,
//! `summary`, `remove`) and provides file-scope pure helpers extracted from
//! `commands.cpp` that are shared across multiple commands.
//!
//! # No escaping of file-derived text (C++ parity)
//!
//! Everything this module extracts from a T*.md — the goal, the first
//! implementation step, the Files/Specs backtick items — and every task name
//! or log message a command prints, is emitted **verbatim**. Nothing escapes,
//! strips or validates it, so a task file containing ANSI escape sequences
//! (or any other control characters) has them written straight to stdout and a
//! log message keeps them in the `## Notes` entry. This matches C++ line for
//! line (`std::cout << "Goal: " << goal`, `commands.cpp:673`), and is
//! deliberate: sanitising would change the exact bytes of `taskpad next`,
//! `taskpad do` and `taskpad log` for every project.
//!
//! # The manual scanners are byte scanners
//!
//! `extract_status_line`, `extract_depends`, `extract_goal` and
//! `get_first_step` are ports of byte-oriented C++ loops. They are written
//! against `as_bytes()` / `char_indices()` / `str::get` rather than raw
//! `&str` indexing so that a multibyte character inside a section can never
//! land the cursor mid-codepoint, while ASCII input keeps byte-identical
//! results.

pub(crate) mod deps;
pub(crate) mod do_cmd;
pub(crate) mod done;
pub(crate) mod edit;
pub(crate) mod import;
pub(crate) mod init;
pub(crate) mod log;
pub(crate) mod new;
pub(crate) mod next;
pub(crate) mod pause;
pub(crate) mod remove;
pub(crate) mod status;
pub(crate) mod summary;

use crate::models::Result;
use crate::models::Status;
use crate::models::StatusFile;
use crate::models::Task;
use crate::models::TaskpadError;
use crate::storage;
use crate::utils::{extract_section_list_items, format_task_id, parse_task_id, trim};
use crate::validator;
use std::collections::BTreeMap;

/// Convert a kebab-case string to Title Case, capitalizing the first letter
/// and the letter after each '-'. Matching C++ `kebabToTitle`.
pub(crate) fn kebab_to_title(kebab: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in kebab.chars() {
        if c == '-' {
            result.push(' ');
            capitalize = true;
        } else {
            result.push(if capitalize {
                c.to_ascii_uppercase()
            } else {
                c
            });
            capitalize = false;
        }
    }
    result
}

/// Extract the status string from a T*.md content string.
///
/// Searches for "## Status:", returns the trimmed text up to the next '\n'.
/// Returns "" if "## Status:" is absent or has no newline after it.
/// Matching C++ `extractStatusLine`.
pub(crate) fn extract_status_line(content: &str) -> String {
    let pos = match content.find("## Status:") {
        Some(p) => p,
        None => return String::new(),
    };
    let eol = match content[pos..].find('\n') {
        Some(p) => p + pos,
        None => return String::new(),
    };
    content[pos + 10..eol].trim().to_string()
}

/// Extract dependency task IDs from a T*.md content string.
///
/// Finds the "## Depends On" section (from the '\n' after the header to
/// the next "\n## " marker or EOF), then scans for TXXX tokens with
/// word boundaries. Matching C++ `extractDepends`.
pub(crate) fn extract_depends(content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let pos = match content.find("## Depends On") {
        Some(p) => p,
        None => return result,
    };
    let start = match content[pos..].find('\n') {
        Some(p) => p + pos,
        None => return result,
    };
    let end = section_end(content, start);
    let section = match content.get(start..end) {
        Some(section) => section,
        None => return result,
    };
    // Byte windows, the direct equivalent of C++'s
    // `for (size_t i = 0; i + 4 <= section.size(); ++i)`. `windows(4)` can
    // never run off the end, and `window` is a plain `&[u8]` — so a
    // multibyte character in the section can never be sliced mid-codepoint.
    // (A match could in fact only start at an ASCII `T`, which is always a
    // char boundary, but depending on that invariant is exactly what makes
    // hand-ported byte scanners fragile.)
    let bytes = section.as_bytes();
    for (i, window) in bytes.windows(4).enumerate() {
        if window[0] == b'T'
            && window[1].is_ascii_digit()
            && window[2].is_ascii_digit()
            && window[3].is_ascii_digit()
        {
            let word_boundary_before = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let word_boundary_after = i + 4 >= bytes.len() || !bytes[i + 4].is_ascii_alphanumeric();
            if word_boundary_before && word_boundary_after {
                // The four bytes are `T` plus three ASCII digits, hence
                // always valid UTF-8.
                result.push(String::from_utf8_lossy(window).into_owned());
            }
        }
    }
    result
}

/// Byte offset just past the section that begins at the newline `start`:
/// the next `"\n## "` marker, or the end of `content`.
///
/// `start` is always a char boundary (it comes either from
/// `str::find('\n')` or from [`skip_blank_lines`], which only ever advances
/// across `'\n'` bytes), so the `str::get` here only fails for a nonsense
/// offset from a future caller — in which case the section is reported as
/// empty rather than panicking.
fn section_end(content: &str, start: usize) -> usize {
    match content.get(start + 1..) {
        Some(after_newline) => match after_newline.find("\n## ") {
            Some(offset) => start + 1 + offset,
            None => content.len(),
        },
        None => start,
    }
}

/// Advance `start` across the run of blank lines that follows it.
///
/// The direct equivalent of C++'s
/// `while (start + 1 < content.size() && content[start + 1] == '\n') ++start;`.
/// Reading `as_bytes()` keeps the walk on raw bytes, so it can only ever
/// stop on a `'\n'`, and every offset it produces is a char boundary.
fn skip_blank_lines(content: &str, start: usize) -> usize {
    let bytes = content.as_bytes();
    let mut start = start;
    while bytes.get(start + 1) == Some(&b'\n') {
        start += 1;
    }
    start
}

/// Count tasks in `tasks` whose status equals `status`.
/// Matching C++ `countStatus`.
///
/// Only exercised by unit tests — production call sites tally all three
/// statuses in one pass via [`count_all`] — but kept as the direct port of
/// the C++ helper.
#[allow(dead_code)]
pub(crate) fn count_status(tasks: &BTreeMap<String, Task>, status: Status) -> i32 {
    let mut count = 0;
    for task in tasks.values() {
        if task.status == status {
            count += 1;
        }
    }
    count
}

/// Find the next available task ID by taking the max parsed task ID
/// from `tasks` keys and adding 1. Returns `"T001"` for an empty map.
/// Matching C++ `findNextTaskId`.
pub(crate) fn find_next_task_id(tasks: &BTreeMap<String, Task>) -> String {
    let mut max_id = 0;
    for id in tasks.keys() {
        let num = parse_task_id(id);
        if num > max_id {
            max_id = num;
        }
    }
    format_task_id(max_id + 1)
}

/// Return `true` if every dependency in `task.depends` exists in
/// `tasks` and has status [`Status::Done`].
/// Matching C++ `allDepsDone`.
pub(crate) fn all_deps_done(task: &Task, tasks: &BTreeMap<String, Task>) -> bool {
    for dep in &task.depends {
        match tasks.get(dep) {
            Some(t) if t.status == Status::Done => continue,
            _ => return false,
        }
    }
    true
}

/// Return the bracketed status string for terminal display.
/// Done→`"[done]"`, InProgress→`"[in_progress]"`, Pending→`"[pending]"`.
/// Matching C++ `statusColor`.
pub(crate) fn status_color(status: Status) -> &'static str {
    match status {
        Status::Done => "[done]",
        Status::InProgress => "[in_progress]",
        Status::Pending => "[pending]",
    }
}

/// Extract the goal text from a T*.md content string.
///
/// Searches for "## Goal", starts after its newline, skips blank lines,
/// then ends at the next "\n## " marker or EOF. Returns the trimmed text.
/// Matching C++ `extractGoal`.
pub(crate) fn extract_goal(content: &str) -> String {
    let pos = match content.find("## Goal") {
        Some(p) => p,
        None => return String::new(),
    };
    let start = match content[pos..].find('\n') {
        Some(p) => p + pos,
        None => return String::new(),
    };
    let start = skip_blank_lines(content, start);
    let end = section_end(content, start);
    match content.get(start..end) {
        Some(section) => trim(section),
        None => String::new(),
    }
}

/// Extract the first implementation step from a T*.md content string.
///
/// Searches for "## Implementation Steps", starts after its newline,
/// skips blank lines, then scans for the first "- " list item or
/// a numbered "N." item. Returns the trimmed text, or "" if none found.
/// Matching C++ `getFirstStep`.
pub(crate) fn get_first_step(content: &str) -> String {
    let pos = match content.find("## Implementation Steps") {
        Some(p) => p,
        None => return String::new(),
    };
    let start = match content[pos..].find('\n') {
        Some(p) => p + pos,
        None => return String::new(),
    };
    let start = skip_blank_lines(content, start);
    let end = section_end(content, start);
    let section = match content.get(start..end) {
        Some(section) => section,
        None => return String::new(),
    };

    // Walk char boundaries rather than raw byte offsets. C++'s loop indexes
    // `section[i]` for every `i`, which in Rust would step into the middle
    // of a multibyte character and panic on the following `section[i..]`.
    // Only an ASCII `-` or an ASCII digit can start an item (C++ compares
    // `c == '-'` and calls `isdigit` on the raw byte), so `char_indices`
    // considers exactly the same candidates in the same order.
    let mut chars = section.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let after = chars.peek().map(|&(_, next)| next);
        if c == '-' && after == Some(' ') {
            // `c` is ASCII, so `i + 1` is a char boundary: drop the marker
            // and its single space, then take the rest of that line.
            return match section.get(i + 1..) {
                Some(rest) => first_line(rest.strip_prefix(' ').unwrap_or(rest)),
                None => String::new(),
            };
        }
        if c.is_ascii_digit()
            && after == Some('.')
            && let Some(rest) = section.get(i + 1..)
        {
            let item = first_line(rest.strip_prefix('.').unwrap_or(rest));
            if !item.is_empty() {
                return item;
            }
        }
    }
    String::new()
}

/// The first `\n`-terminated line of `s` (or all of `s` if there is no
/// newline), trimmed — the Rust spelling of C++'s
/// `trim(section.substr(i + 2, lineEnd - i - 2))`.
fn first_line(s: &str) -> String {
    match s.split('\n').next() {
        Some(line) => trim(line),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Shared command boilerplate
// ---------------------------------------------------------------------------

/// Error message for an invalid task ID, matching the C++ `commands.cpp`
/// literal byte-for-byte.
///
/// The runtime path builds this via [`TaskpadError::invalid_id()`] (see
/// [`invalid_task_id_error`]); the const pins the literal for tests.
/// Only referenced by tests outside `cfg(test)` builds, hence the scoped allow.
#[allow(dead_code)]
pub(crate) const ERR_INVALID_ID: &str =
    "Invalid task ID format. Expected TXXX (see Task ID Format)";

/// Build the invalid-task-ID error.
pub(crate) fn invalid_task_id_error() -> TaskpadError {
    TaskpadError::invalid_id()
}

/// Validate `task_id`, returning [`invalid_task_id_error`] on failure.
/// Matching the `isValidTaskId` guard at the top of most C++ commands.
pub(crate) fn require_task_id(task_id: &str) -> Result<()> {
    if validator::is_valid_task_id(task_id) {
        Ok(())
    } else {
        Err(invalid_task_id_error())
    }
}

/// Build the "task not found" error for `id`, matching the C++ literal.
pub(crate) fn task_not_found(id: &str) -> TaskpadError {
    TaskpadError::task_not_found(id)
}

/// Resolve the task directory and read `status.yaml` in one step.
///
/// Returns the resolved dir together with the parsed file, matching the
/// `storage::resolve_task_dir` + `read_status_file` prologue shared by the commands.
pub(crate) fn load_status(tasks_dir: &str) -> Result<(String, StatusFile)> {
    let dir = storage::resolve_task_dir(tasks_dir);
    let sf = storage::read_status_file(&dir)?;
    Ok((dir, sf))
}

/// Dependencies of `task` that are still unmet: each dependency naming an
/// existing task whose status is not [`Status::Done`].
///
/// Unknown dependency IDs are skipped, matching the C++ blocker loops in
/// `Commands::do_` and `Commands::status` (which only format entries they
/// can look up). Pair with [`all_deps_done`] for the gate itself, since a
/// missing dependency also counts as unmet there.
///
/// The two behave differently by design, and the difference is visible in
/// `Commands::do_`'s error text: `all_deps_done` is false when a dependency
/// is absent from `status.yaml`, but this function cannot report it, so a task
/// whose every dependency is missing yields an empty list and the degenerate
/// `Unmet dependencies: . Use --force to proceed`. C++ parity — the same two
/// loops produce the same output (`commands.cpp:645-656`).
pub(crate) fn unmet_deps<'t>(
    task: &'t Task,
    tasks: &BTreeMap<String, Task>,
) -> Vec<(&'t String, Status)> {
    let mut out = Vec::new();
    for dep in &task.depends {
        if let Some(dep_task) = tasks.get(dep)
            && dep_task.status != Status::Done
        {
            out.push((dep, dep_task.status));
        }
    }
    out
}

/// IDs of the tasks depending on `id`, in key order.
/// Matching the reverse-lookup loops in `Commands::deps`/`remove`/`done`.
pub(crate) fn dependents_of<'m>(tasks: &'m BTreeMap<String, Task>, id: &str) -> Vec<&'m String> {
    let mut out = Vec::new();
    for (entry_id, entry) in tasks {
        if entry.depends.iter().any(|d| d == id) {
            out.push(entry_id);
        }
    }
    out
}

/// Pick the next actionable task: pending with all dependencies done,
/// ordered by critical (true first), then phase ascending, then numeric
/// ID ascending. Returns the task ID, or `None` when nothing is actionable.
///
/// Single canonical implementation of the comparator previously duplicated
/// in `Commands::status` (pairwise best-tracking) and `Commands::next`
/// (candidate sort) — both reduce to this minimum. The comparison keys off
/// the map keys, matching `Commands::next`; `Commands::status` compared
/// `task.id` instead, which is equal to the key for every file produced by
/// the reader (it re-threads the map key into `Task.id`).
pub(crate) fn pick_next_task(tasks: &BTreeMap<String, Task>) -> Option<String> {
    let mut best: Option<(&String, &Task)> = None;
    for (id, task) in tasks {
        if task.status != Status::Pending || !all_deps_done(task, tasks) {
            continue;
        }
        let better = match &best {
            None => true,
            Some((best_id, best_task)) => {
                if task.critical != best_task.critical {
                    task.critical
                } else if task.phase != best_task.phase {
                    task.phase < best_task.phase
                } else {
                    parse_task_id(id) < parse_task_id(best_id)
                }
            }
        };
        if better {
            best = Some((id, task));
        }
    }
    best.map(|(id, _)| id.clone())
}

/// Status counts for a task map, tallied in one pass.
/// Matching the `countStatus` triple in `status`/`summary`/`import`.
pub(crate) struct Counts {
    pub(crate) done: i32,
    pub(crate) in_progress: i32,
    pub(crate) pending: i32,
}

/// Count tasks per status in a single pass.
pub(crate) fn count_all(tasks: &BTreeMap<String, Task>) -> Counts {
    let mut counts = Counts {
        done: 0,
        in_progress: 0,
        pending: 0,
    };
    for task in tasks.values() {
        match task.status {
            Status::Done => counts.done += 1,
            Status::InProgress => counts.in_progress += 1,
            Status::Pending => counts.pending += 1,
        }
    }
    counts
}

/// Goal + first-step detail lines (unindented, labels included).
///
/// `Commands::do_` prints exactly these two lines after its
/// `Now reading ...` header; [`task_detail_lines`] extends them with the
/// Files/Specs lines for `Commands::next`.
pub(crate) fn goal_lines(content: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let goal = extract_goal(content);
    if !goal.is_empty() {
        lines.push(format!("Goal: {goal}"));
    }
    let first = get_first_step(content);
    if !first.is_empty() {
        lines.push(format!("First step: {first}"));
    }
    lines
}

/// Full task detail block for `Commands::next`: goal, first step, files,
/// and specs lines (unindented — the caller adds the two-space prefix).
pub(crate) fn task_detail_lines(content: &str) -> Vec<String> {
    let mut lines = goal_lines(content);
    let files = extract_section_list_items(content, "## Files to Create/Modify");
    if !files.is_empty() {
        lines.push(format!("Files: {}", files.join(" ")));
    }
    let specs = extract_section_list_items(content, "## Spec References");
    if !specs.is_empty() {
        lines.push(format!("Specs: {}", specs.join(" ")));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kebab_to_title() {
        assert_eq!(kebab_to_title("project-setup"), "Project Setup");
        assert_eq!(kebab_to_title("core-types"), "Core Types");
        assert_eq!(kebab_to_title("simple"), "Simple");
        assert_eq!(kebab_to_title(""), "");
    }

    #[test]
    fn test_extract_status_line() {
        assert_eq!(extract_status_line("## Status: done"), "");
        assert_eq!(extract_status_line("## Status: done\n## Goal"), "done");
        assert_eq!(extract_status_line("no status here"), "");
        assert_eq!(
            extract_status_line("## Status: in_progress\n## Phase:"),
            "in_progress"
        );
    }

    #[test]
    fn test_extract_depends() {
        let content = "## Depends On\n- T001\n- T002\n## Phase:";
        assert_eq!(extract_depends(content), vec!["T001", "T002"]);
        assert_eq!(extract_depends("no depends"), Vec::<String>::new());
    }

    #[test]
    fn test_count_status() {
        let mut tasks = BTreeMap::new();
        tasks.insert(
            "T001".to_string(),
            Task {
                id: "T001".to_string(),
                name: "A".to_string(),
                status: Status::Done,
                depends: vec![],
                phase: 0,
                critical: false,
            },
        );
        tasks.insert(
            "T002".to_string(),
            Task {
                id: "T002".to_string(),
                name: "B".to_string(),
                status: Status::Pending,
                depends: vec![],
                phase: 0,
                critical: false,
            },
        );
        tasks.insert(
            "T003".to_string(),
            Task {
                id: "T003".to_string(),
                name: "C".to_string(),
                status: Status::Done,
                depends: vec![],
                phase: 0,
                critical: false,
            },
        );
        assert_eq!(count_status(&tasks, Status::Done), 2);
        assert_eq!(count_status(&tasks, Status::Pending), 1);
        assert_eq!(count_status(&tasks, Status::InProgress), 0);
    }

    #[test]
    fn test_extract_goal() {
        let content = "## Goal\n\nThis is the goal\n## Phase:";
        assert_eq!(extract_goal(content), "This is the goal");
        let content = "## Goal\n\n(Describe the goal)\n## Depends On\n\n(None)\n";
        assert_eq!(extract_goal(content), "(Describe the goal)");
        assert_eq!(extract_goal("no goal here"), "");
        let content = "## Goal\n\nGoal text\n\n## Implementation Steps\n\n1. Step one\n";
        assert_eq!(extract_goal(content), "Goal text");
    }

    #[test]
    fn test_get_first_step() {
        let content = "## Implementation Steps\n\n1. First step here\n2. Second step\n";
        assert_eq!(get_first_step(content), "First step here");
        let content = "## Implementation Steps\n\n- First bullet step\n";
        assert_eq!(get_first_step(content), "First bullet step");
        assert_eq!(get_first_step("no steps"), "");
        let content = "## Implementation Steps\n\n\n- Blanked step\n";
        assert_eq!(get_first_step(content), "Blanked step");
    }

    // ---- multibyte content must not panic in the manual scanners ----
    //
    // `get_first_step`'s byte walk used to advance one byte per iteration and
    // then slice `section[i..]`, which panics as soon as a multibyte
    // character appears in the section: the walk lands on a continuation byte
    // and the slice is not on a char boundary.

    /// A 2-byte, a 3-byte and a 4-byte character, plus an ASCII arrow.
    const MULTIBYTE: &str = "é \u{2192} \u{a0} \u{1f600}";

    /// Run `f` and return `true` if it did not panic.
    ///
    /// Wrapping the call keeps a regression panic from aborting the whole
    /// test binary; the `assert!` around it reports it as a normal failure.
    fn no_panic<F, R>(f: F) -> bool
    where
        F: FnOnce() -> R + std::panic::UnwindSafe,
    {
        std::panic::catch_unwind(f).is_ok()
    }

    #[test]
    fn test_extract_depends_multibyte_in_section() {
        // Multibyte characters between the dependency tokens are skipped
        // exactly as the C++ byte scanner skips them.
        let content = format!("## Depends On\n- T001\n- {MULTIBYTE}\n- T002\n## Phase:");
        assert_eq!(extract_depends(&content), vec!["T001", "T002"]);

        // A high byte is not an ASCII alphanumeric, so it counts as a word
        // boundary: `éT001` yields T001, matching `!std::isalnum` on the
        // preceding byte.
        assert_eq!(extract_depends("## Depends On\néT001\n"), vec!["T001"]);

        // A token glued to further digits is still rejected (the byte after
        // the token is alphanumeric).
        assert!(extract_depends("## Depends On\nT0012\n").is_empty());

        assert!(
            no_panic(|| extract_depends(&format!("## Depends On\n{MULTIBYTE} T007\n"))),
            "extract_depends must not panic on multibyte input"
        );
    }

    #[test]
    fn test_extract_goal_multibyte() {
        let content = format!("## Goal\n\nShip {MULTIBYTE} now\n\n## Implementation Steps\n");
        assert_eq!(extract_goal(&content), format!("Ship {MULTIBYTE} now"));
        assert!(
            no_panic(|| extract_goal(&format!("## Goal\n\n{MULTIBYTE}\n"))),
            "extract_goal must not panic on multibyte input"
        );
    }

    #[test]
    fn test_get_first_step_multibyte() {
        // The item after the multibyte run is still found, and its text is
        // preserved verbatim.
        let content = format!("## Implementation Steps\n\n{MULTIBYTE}\n1. Real step\n");
        assert_eq!(get_first_step(&content), "Real step");

        // A bullet item after the multibyte run likewise.
        let content = format!("## Implementation Steps\n\n{MULTIBYTE}\n- Real step\n");
        assert_eq!(get_first_step(&content), "Real step");

        // A multibyte character inside the item text survives untouched.
        let content = format!("## Implementation Steps\n\n1. Déploy {MULTIBYTE} now\n");
        assert_eq!(get_first_step(&content), format!("Déploy {MULTIBYTE} now"));

        assert!(
            no_panic(|| get_first_step(&format!("## Implementation Steps\n\n{MULTIBYTE}\n"))),
            "get_first_step must not panic on multibyte input"
        );
    }

    #[test]
    fn test_task_detail_lines_multibyte() {
        let content = format!(
            "## Goal\n\nShip {MULTIBYTE}\n## Implementation Steps\n\n1. Step\n## Files to Create/Modify\n- `src/é.rs`\n## Spec References\n- `specs/é.md`\n"
        );
        assert_eq!(
            task_detail_lines(&content),
            vec![
                format!("Goal: Ship {MULTIBYTE}"),
                "First step: Step".to_string(),
                "Files: src/é.rs".to_string(),
                "Specs: specs/é.md".to_string(),
            ]
        );
    }

    fn helper_task(
        id: &str,
        status: Status,
        depends: Vec<&str>,
        phase: i32,
        critical: bool,
    ) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            status,
            depends: depends.into_iter().map(str::to_string).collect(),
            phase,
            critical,
        }
    }

    #[test]
    fn test_require_task_id() {
        assert!(require_task_id("T001").is_ok());
        assert!(require_task_id("T999").is_ok());
        let err = require_task_id("BAD").unwrap_err();
        assert_eq!(err.to_string(), ERR_INVALID_ID);
        assert_eq!(
            err.to_string(),
            "Invalid task ID format. Expected TXXX (see Task ID Format)"
        );
        assert_eq!(invalid_task_id_error().to_string(), ERR_INVALID_ID);
    }

    #[test]
    fn test_task_not_found_message() {
        assert_eq!(task_not_found("T042").to_string(), "Task T042 not found");
    }

    #[test]
    fn test_load_status_missing_and_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let err = load_status(root).unwrap_err();
        assert_eq!(
            err.to_string(),
            "No status.yaml found. Run 'taskpad import' or 'taskpad new' first"
        );

        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            helper_task("T001", Status::Pending, vec![], 0, false),
        );
        storage::write_status_file(root, &sf).unwrap();

        let (dir_back, sf_back) = load_status(root).unwrap();
        assert_eq!(dir_back, root);
        assert!(sf_back.tasks.contains_key("T001"));
    }

    #[test]
    fn test_all_deps_done_gate() {
        let mut tasks = BTreeMap::new();
        tasks.insert(
            "T001".to_string(),
            helper_task("T001", Status::Done, vec![], 0, false),
        );
        tasks.insert(
            "T002".to_string(),
            helper_task("T002", Status::Pending, vec!["T001"], 0, false),
        );
        tasks.insert(
            "T003".to_string(),
            helper_task("T003", Status::Pending, vec!["T002"], 0, false),
        );
        tasks.insert(
            "T004".to_string(),
            helper_task("T004", Status::Pending, vec!["T999"], 0, false),
        );
        assert!(all_deps_done(&tasks["T002"], &tasks));
        assert!(!all_deps_done(&tasks["T003"], &tasks));
        assert!(!all_deps_done(&tasks["T004"], &tasks));
    }

    #[test]
    fn test_unmet_deps_skips_done_and_missing() {
        let mut tasks = BTreeMap::new();
        tasks.insert(
            "T001".to_string(),
            helper_task("T001", Status::Done, vec![], 0, false),
        );
        tasks.insert(
            "T002".to_string(),
            helper_task("T002", Status::Pending, vec![], 0, false),
        );
        let task = helper_task(
            "T009",
            Status::Pending,
            vec!["T001", "T002", "T999"],
            0,
            false,
        );
        let unmet = unmet_deps(&task, &tasks);
        assert_eq!(unmet.len(), 1);
        assert_eq!(unmet[0].0, "T002");
        assert_eq!(unmet[0].1, Status::Pending);
    }

    #[test]
    fn test_dependents_of_key_order() {
        let mut tasks = BTreeMap::new();
        tasks.insert(
            "T001".to_string(),
            helper_task("T001", Status::Pending, vec![], 0, false),
        );
        tasks.insert(
            "T002".to_string(),
            helper_task("T002", Status::Pending, vec!["T001"], 0, false),
        );
        tasks.insert(
            "T003".to_string(),
            helper_task("T003", Status::Pending, vec!["T001"], 0, false),
        );
        let deps = dependents_of(&tasks, "T001");
        assert_eq!(deps, vec!["T002", "T003"]);
        assert!(dependents_of(&tasks, "T002").is_empty());
    }

    #[test]
    fn test_pick_next_task_ordering() {
        let mut tasks = BTreeMap::new();
        tasks.insert(
            "T001".to_string(),
            helper_task("T001", Status::Pending, vec![], 1, false),
        );
        tasks.insert(
            "T002".to_string(),
            helper_task("T002", Status::Pending, vec![], 1, true),
        );
        tasks.insert(
            "T003".to_string(),
            helper_task("T003", Status::Pending, vec![], 0, true),
        );
        // Critical first, then lowest phase.
        assert_eq!(pick_next_task(&tasks).as_deref(), Some("T003"));

        // Phase tie broken by numeric ID.
        let mut tied = BTreeMap::new();
        tied.insert(
            "T010".to_string(),
            helper_task("T010", Status::Pending, vec![], 0, false),
        );
        tied.insert(
            "T002".to_string(),
            helper_task("T002", Status::Pending, vec![], 0, false),
        );
        assert_eq!(pick_next_task(&tied).as_deref(), Some("T002"));
    }

    #[test]
    fn test_pick_next_task_skips_blocked_and_non_pending() {
        let mut tasks = BTreeMap::new();
        tasks.insert(
            "T001".to_string(),
            helper_task("T001", Status::Done, vec![], 0, true),
        );
        tasks.insert(
            "T002".to_string(),
            helper_task("T002", Status::InProgress, vec![], 0, true),
        );
        tasks.insert(
            "T003".to_string(),
            helper_task("T003", Status::Pending, vec!["T002"], 0, true),
        );
        tasks.insert(
            "T004".to_string(),
            helper_task("T004", Status::Pending, vec![], 2, false),
        );
        // T001/T002 not pending, T003 blocked — T004 wins by elimination.
        assert_eq!(pick_next_task(&tasks).as_deref(), Some("T004"));

        let empty: BTreeMap<String, Task> = BTreeMap::new();
        assert_eq!(pick_next_task(&empty), None);

        let mut blocked = BTreeMap::new();
        blocked.insert(
            "T001".to_string(),
            helper_task("T001", Status::Pending, vec!["T002"], 0, false),
        );
        blocked.insert(
            "T002".to_string(),
            helper_task("T002", Status::Pending, vec!["T001"], 0, false),
        );
        assert_eq!(pick_next_task(&blocked), None);
    }

    #[test]
    fn test_count_all() {
        let mut tasks = BTreeMap::new();
        tasks.insert(
            "T001".to_string(),
            helper_task("T001", Status::Done, vec![], 0, false),
        );
        tasks.insert(
            "T002".to_string(),
            helper_task("T002", Status::InProgress, vec![], 0, false),
        );
        tasks.insert(
            "T003".to_string(),
            helper_task("T003", Status::Pending, vec![], 0, false),
        );
        let counts = count_all(&tasks);
        assert_eq!(counts.done, 1);
        assert_eq!(counts.in_progress, 1);
        assert_eq!(counts.pending, 1);
        let empty: BTreeMap<String, Task> = BTreeMap::new();
        let counts = count_all(&empty);
        assert_eq!((counts.done, counts.in_progress, counts.pending), (0, 0, 0));
    }

    #[test]
    fn test_goal_lines_subset_of_detail_lines() {
        let content = "## Goal\n\nShip it\n## Implementation Steps\n\n1. Do it\n## Files to Create/Modify\n- `a.rs`\n## Spec References\n- `s.md`\n";
        assert_eq!(
            goal_lines(content),
            vec!["Goal: Ship it", "First step: Do it"]
        );
        assert_eq!(
            task_detail_lines(content),
            vec![
                "Goal: Ship it",
                "First step: Do it",
                "Files: a.rs",
                "Specs: s.md",
            ]
        );
        assert!(goal_lines("no sections").is_empty());
        assert!(task_detail_lines("no sections").is_empty());
    }
}

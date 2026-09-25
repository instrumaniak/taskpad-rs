#![allow(dead_code)]

//! Shared command helpers for the taskpad CLI.
//!
//! This module re-exports the subcommand modules (`init`, `import`, `new`,
//! `status`) and provides file-scope pure helpers extracted from `commands.cpp`
//! that are shared across multiple commands.

pub mod import;
pub mod init;
pub mod new;
pub mod status;

use crate::models::Status;
use crate::models::Task;
use crate::utils::{format_task_id, parse_task_id};
use std::collections::BTreeMap;

/// Convert a kebab-case string to Title Case, capitalizing the first letter
/// and the letter after each '-'. Matching C++ `kebabToTitle`.
pub fn kebab_to_title(kebab: &str) -> String {
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
pub fn extract_status_line(content: &str) -> String {
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
pub fn extract_depends(content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let pos = match content.find("## Depends On") {
        Some(p) => p,
        None => return result,
    };
    let start = match content[pos..].find('\n') {
        Some(p) => p + pos,
        None => return result,
    };
    let end = match content[start + 1..].find("\n## ") {
        Some(p) => p + start + 1,
        None => content.len(),
    };
    let section = &content[start..end];
    for i in 0..section.len() {
        if i + 4 > section.len() {
            break;
        }
        let bytes = section.as_bytes();
        if bytes[i] == b'T'
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
        {
            let word_boundary_before = i == 0 || !section.as_bytes()[i - 1].is_ascii_alphanumeric();
            let word_boundary_after =
                i + 4 >= section.len() || !section.as_bytes()[i + 4].is_ascii_alphanumeric();
            if word_boundary_before && word_boundary_after {
                result.push(section[i..i + 4].to_string());
            }
        }
    }
    result
}

/// Count tasks in `tasks` whose status equals `status`.
/// Matching C++ `countStatus`.
pub fn count_status(tasks: &BTreeMap<String, Task>, status: Status) -> i32 {
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
pub fn find_next_task_id(tasks: &BTreeMap<String, Task>) -> String {
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
pub fn all_deps_done(task: &Task, tasks: &BTreeMap<String, Task>) -> bool {
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
pub fn status_color(status: Status) -> &'static str {
    match status {
        Status::Done => "[done]",
        Status::InProgress => "[in_progress]",
        Status::Pending => "[pending]",
    }
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
}

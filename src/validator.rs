#![allow(dead_code)]

//! Rust port of `validator.h` / `validator.cpp` from the C++ taskpad codebase.
//!
//! Provides task ID validation, status validation, and dependency graph
//! validation (existence checks and circular-dependency detection).

use crate::models::{Result, Task, TaskpadError};
use std::collections::{BTreeMap, HashSet};

// ---------------------------------------------------------------------------
// is_valid_task_id
// ---------------------------------------------------------------------------

/// Check whether a string is a valid task ID, matching C++ `isValidTaskId`.
///
/// A valid ID is exactly 4 characters: `'T'` followed by three digits
/// forming a number in `[1, 999]` (so `"T000"` is invalid).
pub fn is_valid_task_id(id: &str) -> bool {
    if id.len() != 4 {
        return false;
    }
    if id.as_bytes()[0] != b'T' {
        return false;
    }
    for c in id[1..].chars() {
        if !c.is_ascii_digit() {
            return false;
        }
    }
    let num = id[1..].parse::<i32>().unwrap_or(0);
    (1..=999).contains(&num)
}

// ---------------------------------------------------------------------------
// is_valid_status
// ---------------------------------------------------------------------------

/// Check whether a string is a valid task status, matching C++ `isValidStatus`.
///
/// Valid statuses are `"pending"`, `"in_progress"`, and `"done"`.
pub fn is_valid_status(status: &str) -> bool {
    status == "pending" || status == "in_progress" || status == "done"
}

// ---------------------------------------------------------------------------
// validate_task_exists
// ---------------------------------------------------------------------------

/// Check that a task with the given ID exists in `tasks`, matching
/// C++ `validateTaskExists`.
///
/// Returns an error if the task is not found.
pub fn validate_task_exists(id: &str, tasks: &BTreeMap<String, Task>) -> Result<()> {
    if tasks.get(id).is_none() {
        return Err(TaskpadError::Message(format!("Task {id} not found")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// validate_depends_exist
// ---------------------------------------------------------------------------

/// Check that every dependency in `depends` exists in `tasks`, matching
/// C++ `validateDependsExist`.
///
/// Returns an error on the first missing dependency.
pub fn validate_depends_exist(depends: &[String], tasks: &BTreeMap<String, Task>) -> Result<()> {
    for dep in depends {
        if tasks.get(dep).is_none() {
            return Err(TaskpadError::Message(format!("Dependency {dep} not found")));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// validate_circular_dependencies
// ---------------------------------------------------------------------------

/// Check for circular dependencies starting from `task_id` through its
/// `depends` list, matching C++ `validateCircularDependencies`.
///
/// For each dependency in `depends`:
/// - If the dependency equals `task_id`, returns an immediate error
///   (self-dependency).
/// - If the dependency is not present in `tasks`, it is skipped.
/// - Otherwise a DFS is performed from that dependency; if `task_id` is
///   reachable, a circular-dependency error is returned with the literal
///   `→ ... →` separator (not a fully joined path).
///
/// The DFS visited set is recreated per outer dependency, not shared
/// across them.
pub fn validate_circular_dependencies(
    task_id: &str,
    depends: &[String],
    tasks: &BTreeMap<String, Task>,
) -> Result<()> {
    for dep in depends {
        if *dep == task_id {
            return Err(TaskpadError::Message(format!(
                "Circular dependency detected: {task_id} depends on itself"
            )));
        }
        if !tasks.contains_key(dep) {
            continue;
        }
        let mut visited = HashSet::new();
        let mut stack = vec![dep.clone()];
        while let Some(current) = stack.pop() {
            if current == task_id {
                return Err(TaskpadError::Message(format!(
                    "Circular dependency detected: {task_id} → ... → {dep}"
                )));
            }
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(task) = tasks.get(&current) {
                for d in &task.depends {
                    if !visited.contains(d) {
                        stack.push(d.clone());
                    }
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, depends: Vec<String>) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            status: crate::models::Status::Pending,
            depends,
            phase: 0,
            critical: false,
        }
    }

    #[test]
    fn test_is_valid_task_id() {
        assert!(is_valid_task_id("T001"));
        assert!(is_valid_task_id("T999"));
        assert!(is_valid_task_id("T012"));
        assert!(!is_valid_task_id("T000"));
        assert!(!is_valid_task_id(""));
        assert!(!is_valid_task_id("T00"));
        assert!(!is_valid_task_id("T0001"));
        assert!(!is_valid_task_id("abc"));
        assert!(!is_valid_task_id("T01a"));
        assert!(!is_valid_task_id("t001"));
    }

    #[test]
    fn test_is_valid_status() {
        assert!(is_valid_status("pending"));
        assert!(is_valid_status("in_progress"));
        assert!(is_valid_status("done"));
        assert!(!is_valid_status(""));
        assert!(!is_valid_status("unknown"));
        assert!(!is_valid_status("PENDING"));
    }

    #[test]
    fn test_validate_task_exists() {
        let mut tasks = BTreeMap::new();
        tasks.insert("T001".to_string(), make_task("T001", vec![]));
        tasks.insert("T002".to_string(), make_task("T002", vec![]));
        assert!(validate_task_exists("T001", &tasks).is_ok());
        assert!(validate_task_exists("T002", &tasks).is_ok());
        assert!(validate_task_exists("T003", &tasks).is_err());
        assert!(validate_task_exists("", &tasks).is_err());
    }

    #[test]
    fn test_validate_depends_exist() {
        let mut tasks = BTreeMap::new();
        tasks.insert("T001".to_string(), make_task("T001", vec![]));
        assert!(validate_depends_exist(&["T001".to_string()], &tasks).is_ok());
        assert!(validate_depends_exist(&[], &tasks).is_ok());
        assert!(validate_depends_exist(&["T002".to_string()], &tasks).is_err());
        assert!(
            validate_depends_exist(&["T001".to_string(), "T002".to_string()], &tasks,).is_err()
        );
    }

    #[test]
    fn test_validate_circular_dependencies_no_cycle() {
        let mut tasks = BTreeMap::new();
        tasks.insert("T001".to_string(), make_task("T001", vec![]));
        tasks.insert(
            "T002".to_string(),
            make_task("T002", vec!["T001".to_string()]),
        );
        tasks.insert(
            "T003".to_string(),
            make_task("T003", vec!["T002".to_string()]),
        );
        assert!(validate_circular_dependencies("T003", &["T002".to_string()], &tasks).is_ok());
    }

    #[test]
    fn test_validate_circular_dependencies_self() {
        let mut tasks = BTreeMap::new();
        tasks.insert("T001".to_string(), make_task("T001", vec![]));
        let result = validate_circular_dependencies("T001", &["T001".to_string()], &tasks);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Circular dependency")
        );
    }

    #[test]
    fn test_validate_circular_dependencies_transitive() {
        let mut tasks = BTreeMap::new();
        tasks.insert(
            "T001".to_string(),
            make_task("T001", vec!["T002".to_string(), "T003".to_string()]),
        );
        tasks.insert("T002".to_string(), make_task("T002", vec![]));
        tasks.insert(
            "T003".to_string(),
            make_task("T003", vec!["T002".to_string()]),
        );
        assert!(
            validate_circular_dependencies(
                "T001",
                &["T002".to_string(), "T003".to_string()],
                &tasks,
            )
            .is_ok()
        );

        tasks.insert(
            "T003".to_string(),
            make_task("T003", vec!["T001".to_string()]),
        );
        let result = validate_circular_dependencies(
            "T001",
            &["T002".to_string(), "T003".to_string()],
            &tasks,
        );
        assert!(result.is_err());
    }
}

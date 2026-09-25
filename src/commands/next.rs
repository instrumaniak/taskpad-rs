use crate::commands::{all_deps_done, extract_goal, get_first_step};
use crate::models::Result;
use crate::models::Status;
use crate::models::status_to_string;
use crate::storage;
use crate::utils;

/// Find and display the next actionable task.
///
/// Candidates are pending tasks whose dependencies are all done,
/// sorted by (critical desc, phase asc, id asc).
/// Matching C++ `Commands::next`.
pub fn run(tasks_dir: &str) -> Result<()> {
    let dir = utils::resolve_task_dir(tasks_dir);
    let sf = storage::read_status_file(&dir)?;

    let mut candidates: Vec<String> = Vec::new();
    for (id, task) in &sf.tasks {
        if task.status == Status::Pending && all_deps_done(task, &sf.tasks) {
            candidates.push(id.clone());
        }
    }

    if candidates.is_empty() {
        println!("info: All tasks blocked or complete");
        return Ok(());
    }

    candidates.sort_by(|a, b| {
        let ta = sf.tasks.get(a).unwrap();
        let tb = sf.tasks.get(b).unwrap();
        if ta.critical != tb.critical {
            return if ta.critical {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        if ta.phase != tb.phase {
            return ta.phase.cmp(&tb.phase);
        }
        utils::parse_task_id(a).cmp(&utils::parse_task_id(b))
    });

    let best_id = &candidates[0];
    let best = sf.tasks.get(best_id).unwrap();

    println!("Next: {} \u{2014} {}", best.id, best.name);

    if !best.depends.is_empty() {
        print!("  Depends on:");
        for dep in &best.depends {
            if let Some(dep_task) = sf.tasks.get(dep) {
                let status_str = status_to_string(dep_task.status);
                let mark = if dep_task.status == Status::Done {
                    "\u{2713}"
                } else {
                    "\u{2717}"
                };
                print!(" {} [{}] {}", dep, status_str, mark);
            }
        }
        println!();
    }

    let path = storage::task_file_path(&dir, &best.id, &best.name);
    if let Ok(content) = storage::read_task_file(&path) {
        let goal = extract_goal(&content);
        if !goal.is_empty() {
            println!("  Goal: {}", goal);
        }
        let first = get_first_step(&content);
        if !first.is_empty() {
            println!("  First step: {}", first);
        }
        let files = utils::extract_section_list_items(&content, "## Files to Create/Modify");
        if !files.is_empty() {
            print!("  Files:");
            for f in &files {
                print!(" {}", f);
            }
            println!();
        }
        let specs = utils::extract_section_list_items(&content, "## Spec References");
        if !specs.is_empty() {
            print!("  Specs:");
            for s in &specs {
                print!(" {}", s);
            }
            println!();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StatusFile;
    use crate::models::Task;

    fn make_task(
        id: &str,
        status: Status,
        depends: Vec<String>,
        phase: i32,
        critical: bool,
    ) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            status,
            depends,
            phase,
            critical,
        }
    }

    #[test]
    fn candidate_selection_critical_first() {
        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            make_task("T001", Status::Pending, vec![], 1, false),
        );
        sf.tasks.insert(
            "T002".to_string(),
            make_task("T002", Status::Pending, vec![], 1, true),
        );
        let mut candidates: Vec<String> = Vec::new();
        for (id, task) in &sf.tasks {
            if task.status == Status::Pending && all_deps_done(task, &sf.tasks) {
                candidates.push(id.clone());
            }
        }
        candidates.sort_by(|a, b| {
            let ta = sf.tasks.get(a).unwrap();
            let tb = sf.tasks.get(b).unwrap();
            if ta.critical != tb.critical {
                return if ta.critical {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
            ta.phase
                .cmp(&tb.phase)
                .then_with(|| utils::parse_task_id(a).cmp(&utils::parse_task_id(b)))
        });
        assert_eq!(candidates[0], "T002");
    }

    #[test]
    fn all_blocked_shows_message() {
        let mut sf = StatusFile::default();
        sf.tasks.insert(
            "T001".to_string(),
            make_task("T001", Status::Pending, vec!["T002".to_string()], 0, false),
        );
        sf.tasks.insert(
            "T002".to_string(),
            make_task("T002", Status::Pending, vec!["T001".to_string()], 0, false),
        );
        let mut candidates: Vec<String> = Vec::new();
        for (id, task) in &sf.tasks {
            if task.status == Status::Pending && all_deps_done(task, &sf.tasks) {
                candidates.push(id.clone());
            }
        }
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_extract_goal() {
        let content = "## Goal\n\nThis is the goal\n## Phase:";
        assert_eq!(extract_goal(content), "This is the goal");
        assert_eq!(extract_goal("no goal here"), "");
    }

    #[test]
    fn test_get_first_step() {
        let content = "## Implementation Steps\n\n1. First step here\n";
        assert_eq!(get_first_step(content), "First step here");
        assert_eq!(get_first_step("no steps"), "");
    }
}

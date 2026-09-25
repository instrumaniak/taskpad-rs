use crate::commands::{all_deps_done, status_color};
use crate::models::Result;
use crate::models::Status;
use crate::storage;
use crate::utils;
use std::collections::BTreeMap;

/// Display the status of all tasks.
///
/// Groups tasks by phase, marks the next actionable task, shows
/// blocked-by information, and prints a progress summary.
/// Matching C++ `Commands::status`.
pub fn run(tasks_dir: &str) -> Result<()> {
    let dir = utils::resolve_task_dir(tasks_dir);
    let sf = storage::read_status_file(&dir)?;

    let mut next_task_id = String::new();
    for (id, task) in &sf.tasks {
        if task.status != Status::Pending {
            continue;
        }
        if !all_deps_done(task, &sf.tasks) {
            continue;
        }
        if next_task_id.is_empty() {
            next_task_id = id.clone();
        } else {
            let current = task;
            let best = sf.tasks.get(&next_task_id).unwrap();
            let current_better = if current.critical && !best.critical {
                true
            } else if current.critical == best.critical {
                current.phase < best.phase
                    || current.phase == best.phase
                        && utils::parse_task_id(&current.id) < utils::parse_task_id(&best.id)
            } else {
                false
            };
            if current_better {
                next_task_id = id.clone();
            }
        }
    }

    let mut by_phase: BTreeMap<i32, Vec<String>> = BTreeMap::new();
    for (id, task) in &sf.tasks {
        by_phase.entry(task.phase).or_default().push(id.clone());
    }

    for (p_num, ids) in &by_phase {
        let phase_name = sf.config.phases.get(p_num).cloned().unwrap_or_default();

        print!("Phase {}", p_num);
        if !phase_name.is_empty() {
            print!(": {}", phase_name);
        }
        println!();

        let mut sorted_ids = ids.clone();
        sorted_ids.sort();

        for id in &sorted_ids {
            let task = sf.tasks.get(id).unwrap();
            let marker = if task.status == Status::Done {
                "\u{2713}"
            } else if *id == next_task_id {
                "\u{2192}"
            } else {
                " "
            };

            print!("{} {}  {}", marker, id, task.name);

            let padding = std::cmp::max(1, 20usize.saturating_sub(task.name.len()));
            print!("{}", " ".repeat(padding));

            print!("{}", status_color(task.status));

            if task.status == Status::Pending {
                if *id == next_task_id {
                    print!("  \u{2190} next (dependencies met)");
                } else if !all_deps_done(task, &sf.tasks) {
                    let mut blockers = Vec::new();
                    for dep in &task.depends {
                        if let Some(dep_task) = sf.tasks.get(dep)
                            && dep_task.status != Status::Done
                        {
                            blockers.push(dep.clone());
                        }
                    }
                    print!("  \u{2190} blocked by {}", blockers.join(", "));
                }
            }
            println!();
        }
        println!();
    }

    let total = sf.tasks.len() as i32;
    let done = crate::commands::count_status(&sf.tasks, Status::Done);
    let in_prog = crate::commands::count_status(&sf.tasks, Status::InProgress);
    let pend = crate::commands::count_status(&sf.tasks, Status::Pending);

    println!(
        "Progress: {}/{} done, {} in_progress, {} pending",
        done, total, in_prog, pend
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StatusFile;
    use crate::models::Task;
    use tempfile::tempdir;

    #[test]
    fn status_reads_existing_project() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        let t1 = Task {
            id: "T001".to_string(),
            name: "First Task".to_string(),
            status: Status::Done,
            depends: vec![],
            phase: 0,
            critical: false,
        };
        sf.tasks.insert("T001".to_string(), t1);
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root);
        assert!(result.is_ok());
    }

    #[test]
    fn status_handles_long_task_names_without_panic() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        let t1 = Task {
            id: "T001".to_string(),
            name: "A task name that is definitely longer than twenty bytes".to_string(),
            status: Status::Pending,
            depends: vec![],
            phase: 0,
            critical: false,
        };
        sf.tasks.insert("T001".to_string(), t1);
        storage::write_status_file(root, &sf).unwrap();

        let result = run(root);
        assert!(result.is_ok());
    }

    #[test]
    fn status_returns_error_for_missing_status_yaml() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let result = run(root);
        assert!(result.is_err());
    }
}

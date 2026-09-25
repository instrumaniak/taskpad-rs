use crate::commands::{
    all_deps_done, count_all, load_status, pick_next_task, status_color, unmet_deps,
};
use crate::models::Result;
use crate::models::Status;
use std::collections::BTreeMap;

/// Display the status of all tasks.
///
/// Groups tasks by phase, marks the next actionable task, shows
/// blocked-by information, and prints a progress summary.
/// Matching C++ `Commands::status`.
pub(crate) fn run(tasks_dir: &str) -> Result<()> {
    let (_, sf) = load_status(tasks_dir)?;

    let next_task_id = pick_next_task(&sf.tasks).unwrap_or_default();

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
            // Unreachable in practice: `by_phase` is built from
            // `sf.tasks` keys just above. Skip gracefully if violated.
            let Some(task) = sf.tasks.get(id) else {
                continue;
            };
            let marker = if task.status == Status::Done {
                "\u{2713}"
            } else if *id == next_task_id {
                "\u{2192}"
            } else {
                " "
            };

            print!("{} {}  {}", marker, id, task.name);

            // The status column is padded to 20 **bytes**, matching C++
            // `20 - static_cast<int>(t.name.size())` (commands.cpp:480): both
            // are byte counts, not display widths. A name in a wide script
            // (CJK, or an emoji) therefore counts as several columns while
            // occupying more than one terminal cell, and the `[status]`
            // column visibly misaligns for those rows only.
            //
            // Deliberate C++ parity: padding by display width would change the
            // exact bytes of every `taskpad status` line for every project
            // with a non-ASCII task name, so the byte-based width is kept.
            let padding = std::cmp::max(1, 20usize.saturating_sub(task.name.len()));
            print!("{}", " ".repeat(padding));

            print!("{}", status_color(task.status));

            if task.status == Status::Pending {
                if *id == next_task_id {
                    print!("  \u{2190} next (dependencies met)");
                } else if !all_deps_done(task, &sf.tasks) {
                    let blockers: Vec<String> = unmet_deps(task, &sf.tasks)
                        .into_iter()
                        .map(|(dep, _)| dep.clone())
                        .collect();
                    print!("  \u{2190} blocked by {}", blockers.join(", "));
                }
            }
            println!();
        }
        println!();
    }

    let total = sf.tasks.len() as i32;
    let counts = count_all(&sf.tasks);

    println!(
        "Progress: {}/{} done, {} in_progress, {} pending",
        counts.done, total, counts.in_progress, counts.pending
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StatusFile;
    use crate::models::Task;
    use crate::storage;
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

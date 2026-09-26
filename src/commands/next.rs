use crate::commands::{load_status, pick_next_task, task_detail_lines, task_not_found};
use crate::models::Result;
use crate::models::Status;
use crate::models::status_to_string;
use crate::storage;

/// Find and display the next actionable task.
///
/// Candidates are pending tasks whose dependencies are all done,
/// sorted by (critical desc, phase asc, id asc).
pub(crate) fn run(tasks_dir: &str) -> Result<()> {
    let (dir, sf) = load_status(tasks_dir)?;

    let Some(best_id) = pick_next_task(&sf.tasks) else {
        println!("info: All tasks blocked or complete");
        return Ok(());
    };
    // Unreachable in practice: the ID came from `sf.tasks` keys above.
    // Error gracefully instead of panicking.
    let Some(best) = sf.tasks.get(&best_id) else {
        return Err(task_not_found(&best_id));
    };

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
        for line in task_detail_lines(&content) {
            println!("  {line}");
        }
    }

    Ok(())
}

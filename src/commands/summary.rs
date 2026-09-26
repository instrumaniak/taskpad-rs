use crate::commands::{count_all, load_status};
use crate::models::Result;
use crate::models::Status;
use crate::models::StatusFile;
use std::collections::BTreeMap;

/// Show overall progress statistics for the project.
///
/// Prints totals with percentages, a per-phase done/total breakdown in
/// phase order, and the critical path with its own status counts.
pub(crate) fn run(tasks_dir: &str) -> Result<()> {
    let (_, sf) = load_status(tasks_dir)?;

    for line in render(&sf) {
        println!("{}", line);
    }

    Ok(())
}

/// Render the full `summary` output as lines (without trailing newlines),
/// matching C++ `Commands::summary` byte-for-byte.
fn render(sf: &StatusFile) -> Vec<String> {
    let mut lines = Vec::new();

    let total = sf.tasks.len() as i32;
    let counts = count_all(&sf.tasks);
    let done = counts.done;
    let in_prog = counts.in_progress;
    let pend = counts.pending;

    lines.push("Task Summary".to_string());
    lines.push("\u{2500}".repeat(13));
    lines.push(format!("Total tasks:    {}", total));
    lines.push(format!(
        "Done:           {} ({})",
        done,
        print_pct(done, total)
    ));
    lines.push(format!(
        "In progress:    {} ({})",
        in_prog,
        print_pct(in_prog, total)
    ));
    lines.push(format!(
        "Pending:        {} ({})",
        pend,
        print_pct(pend, total)
    ));

    if !sf.config.phases.is_empty() || !sf.tasks.is_empty() {
        lines.push(String::new());
        lines.push("By Phase:".to_string());

        let mut phase_total: BTreeMap<i32, i32> = BTreeMap::new();
        let mut phase_done: BTreeMap<i32, i32> = BTreeMap::new();
        for task in sf.tasks.values() {
            *phase_total.entry(task.phase).or_insert(0) += 1;
            if task.status == Status::Done {
                *phase_done.entry(task.phase).or_insert(0) += 1;
            }
        }

        for (phase, count) in &phase_total {
            let done_in_phase = phase_done.get(phase).copied().unwrap_or(0);
            match sf.config.phases.get(phase) {
                Some(name) => lines.push(format!(
                    "  Phase {} ({}): {}/{} done",
                    phase, name, done_in_phase, count
                )),
                None => lines.push(format!(
                    "  Phase {}: {}/{} done",
                    phase, done_in_phase, count
                )),
            }
        }
    }

    if !sf.config.critical_path.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Critical Path: {}",
            sf.config.critical_path.join(" \u{2192} ")
        ));

        let mut cp_done = 0;
        let mut cp_in_prog = 0;
        let mut cp_pend = 0;
        for id in &sf.config.critical_path {
            if let Some(task) = sf.tasks.get(id) {
                match task.status {
                    Status::Done => cp_done += 1,
                    Status::InProgress => cp_in_prog += 1,
                    Status::Pending => cp_pend += 1,
                }
            }
        }

        lines.push(format!(
            "  Status: {}/{} done, {} in_progress, {} pending",
            cp_done,
            sf.config.critical_path.len(),
            cp_in_prog,
            cp_pend
        ));
    }

    lines
}

/// Format `count / total` as a percentage with one decimal place
/// (`std::fixed`, precision 1; `"0.0%"` when `total == 0`).
///
/// Rounding parity note: Rust's `{:.1}` and libstdc++'s
/// `std::fixed << std::setprecision(1)` were compared directly (scratch
/// programs formatting the same `f64` values, including exact binary
/// ties such as `6.25` and `31.25`): both round ties to even
/// (`6.25 -> "6.2"`, `31.25 -> "31.2"`) and agreed byte-for-byte on
/// every value tried (`6.2%`, `31.2%`, `66.7%`, `33.3%`, `18.8%`,
/// `43.8%`, `56.2%`, `68.8%`, `81.2%`, `93.8%`, `25.0%`, `100.0%`).
/// So no custom half-away emulation is needed — the plain `{:.1}`
/// formatting below already matches the C++ output, and the tests
/// pin the tie cases to guard against regressions.
fn print_pct(count: i32, total: i32) -> String {
    if total == 0 {
        return "0.0%".to_string();
    }
    format!("{:.1}%", count as f64 * 100.0 / total as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Task;
    use crate::storage;
    use tempfile::tempdir;

    fn make_task(id: &str, status: Status, phase: i32) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            status,
            depends: vec![],
            phase,
            critical: false,
        }
    }

    #[test]
    fn print_pct_formats_one_decimal() {
        assert_eq!(print_pct(1, 4), "25.0%");
        assert_eq!(print_pct(2, 3), "66.7%");
        assert_eq!(print_pct(1, 3), "33.3%");
        assert_eq!(print_pct(0, 5), "0.0%");
        assert_eq!(print_pct(5, 5), "100.0%");
    }

    #[test]
    fn print_pct_zero_total_is_zero_point_zero() {
        assert_eq!(print_pct(0, 0), "0.0%");
    }

    #[test]
    fn print_pct_exact_ties_match_cpp_fixed_precision_1() {
        // Exact binary ties: `1/16 = 6.25%`, `5/16 = 31.25%`,
        // `3/16 = 18.75%`. Verified against libstdc++
        // `std::fixed << std::setprecision(1)`, which rounds these ties
        // to even exactly like Rust's `{:.1}` — see the note on
        // `print_pct`. Pinned here so a future "fix" to half-away
        // rounding would fail loudly instead of silently breaking
        // C++ byte parity.
        assert_eq!(print_pct(1, 16), "6.2%");
        assert_eq!(print_pct(5, 16), "31.2%");
        assert_eq!(print_pct(3, 16), "18.8%");
        assert_eq!(print_pct(7, 16), "43.8%");
        assert_eq!(print_pct(9, 16), "56.2%");
    }

    #[test]
    fn render_full_output() {
        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::Done, 0));
        sf.tasks
            .insert("T002".to_string(), make_task("T002", Status::InProgress, 1));
        sf.tasks
            .insert("T003".to_string(), make_task("T003", Status::Pending, 1));
        sf.tasks
            .insert("T004".to_string(), make_task("T004", Status::Pending, 2));
        sf.config.phases.insert(0, "Setup".to_string());
        sf.config.phases.insert(1, "Core".to_string());
        sf.config.critical_path = vec![
            "T001".to_string(),
            "T002".to_string(),
            "T004".to_string(),
            "T999".to_string(),
        ];

        assert_eq!(
            render(&sf),
            vec![
                "Task Summary",
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                "Total tasks:    4",
                "Done:           1 (25.0%)",
                "In progress:    1 (25.0%)",
                "Pending:        2 (50.0%)",
                "",
                "By Phase:",
                "  Phase 0 (Setup): 1/1 done",
                "  Phase 1 (Core): 0/2 done",
                "  Phase 2: 0/1 done",
                "",
                "Critical Path: T001 \u{2192} T002 \u{2192} T004 \u{2192} T999",
                "  Status: 1/4 done, 1 in_progress, 1 pending",
            ]
        );
    }

    #[test]
    fn render_phases_without_tasks_prints_header_only() {
        let mut sf = StatusFile::default();
        sf.config.phases.insert(0, "Setup".to_string());

        let lines = render(&sf);
        assert_eq!(lines[lines.len() - 2], "");
        assert_eq!(lines[lines.len() - 1], "By Phase:");
        assert!(lines.iter().any(|l| l == "Total tasks:    0"));
        assert!(lines.iter().any(|l| l == "Done:           0 (0.0%)"));
    }

    #[test]
    fn render_empty_project_has_no_phase_or_critical_sections() {
        let sf = StatusFile::default();
        let lines = render(&sf);
        assert_eq!(
            lines,
            vec![
                "Task Summary",
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
                "Total tasks:    0",
                "Done:           0 (0.0%)",
                "In progress:    0 (0.0%)",
                "Pending:        0 (0.0%)",
            ]
        );
    }

    #[test]
    fn render_critical_path_empty_omits_section() {
        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::Done, 0));
        let lines = render(&sf);
        assert!(!lines.iter().any(|l| l.starts_with("Critical Path:")));
        assert!(lines.iter().any(|l| l == "  Phase 0: 1/1 done"));
    }

    #[test]
    fn summary_reads_flow_style_critical_path_fixture() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        std::fs::write(
            format!("{}/status.yaml", root),
            "tasks:\n  T001:\n    name: A\n    status: done\n    depends: []\n    phase: 0\n    critical: false\ncritical_path: [T001, T999]\n",
        )
        .unwrap();

        let sf = storage::read_status_file(root).unwrap();
        let lines = render(&sf);
        assert!(lines.contains(&"Critical Path: T001 \u{2192} T999".to_string()));
        assert!(lines.contains(&"  Status: 1/2 done, 0 in_progress, 0 pending".to_string()));
    }

    #[test]
    fn summary_reads_existing_project() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), make_task("T001", Status::Done, 0));
        sf.config.phases.insert(0, "Setup".to_string());
        storage::write_status_file(root, &sf).unwrap();

        assert!(run(root).is_ok());
    }

    #[test]
    fn summary_returns_error_for_missing_status_yaml() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        storage::create_config(root, "tasks").unwrap();
        std::fs::create_dir_all(format!("{}/tasks", root)).ok();

        let result = run(root);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No status.yaml found. Run 'taskpad import' or 'taskpad new' first"
        );
    }
}

//! clap derive definitions for the taskpad CLI — port of `cli.cpp`.
//!
//! [`Cli`] is the top-level parser (global `--tasks-dir` plus one required
//! subcommand); [`Command`] has one variant per subcommand, each carrying
//! exactly the args the C++ `CLI::App` definitions register (spec.main.md
//! §5). Dispatch to `commands::*::run` lives in `main.rs`, which is also the
//! only place that formats `error: {msg}`.

use clap::{Parser, Subcommand};

/// Top-level CLI parser. Matching C++ `runCLI`'s `CLI::App`.
#[derive(Debug, Parser)]
#[command(
    name = "taskpad",
    about = "taskpad - Task Management CLI",
    subcommand_required = true,
    arg_required_else_help = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "TEXT",
        help = "Override task directory"
    )]
    pub tasks_dir: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

/// The thirteen subcommands, in the same order as `cli.cpp` registers them.
#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Initialize task tracking")]
    Init,

    #[command(about = "Import existing task files into taskpad")]
    Import {
        #[arg(long, help = "Overwrite existing status.yaml")]
        force: bool,
    },

    #[command(about = "Create a new task")]
    New {
        #[arg(value_name = "name", help = "Task name")]
        name: String,
        #[arg(
            long,
            value_name = "TEXT",
            help = "Dependency task ID (may be repeated)"
        )]
        depends: Vec<String>,
        #[arg(
            long,
            value_name = "INT",
            allow_negative_numbers = true,
            help = "Phase number"
        )]
        phase: Option<i32>,
        #[arg(long, help = "Mark as critical path")]
        critical: bool,
    },

    #[command(about = "Show all tasks with their status")]
    Status,

    #[command(about = "Show the next task to work on")]
    Next,

    #[command(name = "do", about = "Start working on a task")]
    Do {
        #[arg(value_name = "id", help = "Task ID (e.g. T001)")]
        id: String,
        #[arg(long, help = "Skip dependency check")]
        force: bool,
    },

    #[command(about = "Mark a task as complete")]
    Done {
        #[arg(value_name = "id", help = "Task ID (e.g. T001)")]
        id: String,
    },

    #[command(about = "Pause a task (revert to pending)")]
    Pause {
        #[arg(value_name = "id", help = "Task ID (e.g. T001)")]
        id: String,
    },

    #[command(about = "Show dependency information for a task")]
    Deps {
        #[arg(value_name = "id", help = "Task ID (e.g. T001)")]
        id: String,
    },

    #[command(about = "Append a log entry to a task")]
    Log {
        #[arg(value_name = "id", help = "Task ID (e.g. T001)")]
        id: String,
        #[arg(value_name = "message", help = "Log message")]
        message: String,
    },

    #[command(about = "Edit task metadata or project settings")]
    Edit {
        #[arg(value_name = "id", help = "Task ID (omit for project-level edits)")]
        id: Option<String>,
        #[arg(
            long,
            value_name = "TEXT",
            help = "Set status (pending|in_progress|done)"
        )]
        status: Option<String>,
        #[arg(
            long,
            value_name = "TEXT",
            help = "Set dependency task ID (may be repeated)"
        )]
        depends: Vec<String>,
        #[arg(
            long,
            value_name = "TEXT",
            allow_negative_numbers = true,
            help = "Set phase number"
        )]
        phase: Option<String>,
        #[arg(long, help = "Mark as critical")]
        critical: bool,
        #[arg(long, help = "Unmark critical")]
        no_critical: bool,
        #[arg(
            long,
            value_name = "TEXT",
            help = "Set phase mapping (e.g. '0:Scaffolding,1:Foundation')"
        )]
        phases: Option<String>,
        #[arg(
            long,
            value_name = "TEXT",
            help = "Set critical path (comma-separated task IDs)"
        )]
        critical_path: Option<String>,
    },

    #[command(about = "Show overall progress statistics")]
    Summary,

    #[command(about = "Remove a task")]
    Remove {
        #[arg(value_name = "id", help = "Task ID (e.g. T001)")]
        id: String,
        #[arg(long, help = "Also delete the task markdown file")]
        all: bool,
        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },
}

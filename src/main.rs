mod cli;
mod commands;
mod models;
mod storage;
mod utils;
mod validator;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    let tasks_dir = utils::resolve_task_dir(cli.tasks_dir.as_deref().unwrap_or_default());

    let result = match cli.command {
        cli::Command::Init => commands::init::run(&tasks_dir),
        cli::Command::Import { force } => commands::import::run(&tasks_dir, force),
        cli::Command::New {
            name,
            depends,
            phase,
            critical,
        } => commands::new::run(&tasks_dir, &name, depends, phase.unwrap_or(0), critical),
        cli::Command::Status => commands::status::run(&tasks_dir),
        cli::Command::Next => commands::next::run(&tasks_dir),
        cli::Command::Do { id, force } => commands::do_cmd::run(&tasks_dir, &id, force),
        cli::Command::Done { id } => commands::done::run(&tasks_dir, &id),
        cli::Command::Pause { id } => commands::pause::run(&tasks_dir, &id),
        cli::Command::Deps { id } => commands::deps::run(&tasks_dir, &id),
        cli::Command::Log { id, message } => commands::log::run(&tasks_dir, &id, &message),
        cli::Command::Edit {
            id,
            status,
            depends,
            phase,
            critical,
            no_critical,
            phases,
            critical_path,
        } => commands::edit::run(
            &tasks_dir,
            id.as_deref().unwrap_or_default(),
            status.as_deref().unwrap_or_default(),
            &depends,
            phase.as_deref().unwrap_or_default(),
            critical,
            no_critical,
            phases.as_deref().unwrap_or_default(),
            critical_path.as_deref().unwrap_or_default(),
        ),
        cli::Command::Summary => commands::summary::run(&tasks_dir),
        cli::Command::Remove { id, all, force } => {
            commands::remove::run(&tasks_dir, &id, all, force)
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

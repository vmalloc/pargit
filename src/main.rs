use std::path::PathBuf;

use anyhow::Result;
use commands::{FlowCommand, ReleaseCommand, VersionCommand};
use log::error;
use project::Project;
use structopt::StructOpt;
use utils::ObjectKind;

mod commands;
mod config;
mod project;
mod project_types;
mod release;
mod repo;
mod utils;
mod version_file;

#[derive(StructOpt)]
struct Opts {
    #[structopt(short = "-v", parse(from_occurrences))]
    verbosity: usize,

    #[structopt(short = "-q", parse(from_occurrences))]
    quietness: usize,

    #[structopt(short = "-p", long = "--path", default_value = ".")]
    path: PathBuf,

    #[structopt(subcommand)]
    command: commands::Command,
}

fn entry_point(opts: Opts) -> Result<()> {
    log::debug!("Starting...");

    use commands::Command::*;

    let mut project = Project::new(&opts.path)?;

    match opts.command {
        Configure => project.configure(),
        Release(cmd) => process_release_command(&project, cmd, ObjectKind::Release),
        Hotfix(cmd) => process_release_command(&project, cmd, ObjectKind::Hotfix),
        Feature(cmd) => process_flow_command(&project, ObjectKind::Feature, cmd),
        Bugfix(cmd) => process_flow_command(&project, ObjectKind::Bugfix, cmd),
        commands::Command::Version(VersionCommand::Bump(kind)) => project.bump_version(kind),
        Cleanup => project.pargit_cleanup(),
    }
}

fn process_release_command(
    project: &Project,
    cmd: ReleaseCommand,
    release_kind: ObjectKind,
) -> Result<()> {
    use commands::ReleaseCommand::*;

    match cmd {
        Start { spec } => project.release_start(spec, release_kind).map(drop),
        Publish { name } => project.pargit_publish(release_kind, name),
        ReleaseCommand::Delete { name } => project.pargit_delete(release_kind, name),
        Finish { name } => project.release_finish(name, None, release_kind),
        ReleaseCommand::Version(kind) => project.release_version(kind, release_kind),
    }
}

fn process_flow_command(project: &Project, kind: ObjectKind, cmd: FlowCommand) -> Result<()> {
    match cmd {
        FlowCommand::Delete { name } => project.pargit_delete(kind, name),
        FlowCommand::Start { name } => project.pargit_start(kind, &name),
        FlowCommand::Publish { name } => project.pargit_publish(kind, name),
        FlowCommand::Finish { name } => project.pargit_finish(kind, name, "develop"),
    }
}

fn main() {
    let opts = Opts::from_args();
    env_logger::Builder::new()
        .filter_level(match (opts.verbosity + 2).saturating_sub(opts.quietness) {
            0 => log::LevelFilter::Error,
            1 => log::LevelFilter::Warn,
            2 => log::LevelFilter::Info,
            _ => log::LevelFilter::Debug,
        })
        .format_timestamp(None)
        .format_module_path(false)
        .init();

    if let Err(e) = entry_point(opts) {
        error!("{:?}", e);
        std::process::exit(-1);
    }
}

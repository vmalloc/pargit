use std::path::PathBuf;

use anyhow::{bail, Result};
use commands::{FlowCommand, ReleaseCommand, VersionCommand};
use log::error;
use project::Project;
use structopt::StructOpt;

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
        Release(cmd) => process_release_command(&project, cmd, "release"),
        Hotfix(cmd) => process_release_command(&project, cmd, "hotfix"),
        Feature(cmd) => process_flow_command(&project, "feature", cmd),
        Bugfix(cmd) => process_flow_command(&project, "bugfix", cmd),
        commands::Command::Version(VersionCommand::Bump(kind)) => project.bump_version(kind),
        Cleanup => project.pargit_cleanup(),
    }
}

fn process_release_command(
    project: &Project,
    cmd: ReleaseCommand,
    release_type: &str,
) -> Result<()> {
    use commands::ReleaseCommand::*;
    let start_point = match release_type {
        "release" => &project.config().develop_branch_name,
        "hotfix" => &project.config().master_branch_name,
        _ => bail!("Invalid release type: {:?}", release_type),
    };
    match cmd {
        Start { spec } => project
            .release_start(spec, release_type, start_point)
            .map(drop),
        Publish { name } => project.pargit_publish(release_type, name),
        ReleaseCommand::Delete { name } => project.pargit_delete(release_type, name),
        Finish { name } => project.release_finish(name, None, release_type),
        ReleaseCommand::Version(kind) => {
            project.release_version(kind, release_type, &project.config().develop_branch_name)
        }
    }
}

fn process_flow_command(project: &Project, flow_name: &str, cmd: FlowCommand) -> Result<()> {
    match cmd {
        FlowCommand::Delete { name } => project.pargit_delete(flow_name, name),
        FlowCommand::Start { name } => {
            project.pargit_start(flow_name, &name, &project.config().develop_branch_name)
        }
        FlowCommand::Publish { name } => project.pargit_publish(flow_name, name),
        FlowCommand::Finish { name } => {
            project.pargit_finish(flow_name, name, &project.config().develop_branch_name)
        }
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

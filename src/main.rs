use std::path::PathBuf;

use anyhow::Result;
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

    use commands::{Command::*, ReleaseCommand::*};

    let project = Project::new(&opts.path)?;

    match opts.command {
        Release(Start { spec }) => project.release_start(spec).map(drop),
        Release(Publish { name }) => project.pargit_publish("release", name),
        Release(ReleaseCommand::Delete { name }) => project.pargit_delete("release", name),
        Release(Finish { name }) => project.release_finish(name),
        Release(ReleaseCommand::Version(kind)) => project.release_version(kind),

        Feature(cmd) => process_flow_command(&project, "feature", cmd),
        Bugfix(cmd) => process_flow_command(&project, "bugfix", cmd),
        commands::Command::Version(VersionCommand::Bump(kind)) => project.bump_version(kind),
        Cleanup => project.pargit_cleanup(),
    }
}

fn process_flow_command(project: &Project, flow_name: &str, cmd: FlowCommand) -> Result<()> {
    match cmd {
        FlowCommand::Delete { name } => project.pargit_delete(flow_name, name),
        FlowCommand::Start { name } => project.pargit_start(flow_name, &name, "develop"),
        FlowCommand::Publish { name } => project.pargit_publish(flow_name, name),
        FlowCommand::Finish { name } => project.pargit_finish(flow_name, name, "develop"),
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

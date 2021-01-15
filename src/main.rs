use std::path::PathBuf;

use anyhow::Result;
use commands::{ReleaseCommand, VersionCommand};
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

    use commands::{Command::*, FeatureCommand, ReleaseCommand::*};

    let project = Project::new(&opts.path)?;

    match opts.command {
        Release(Start { spec }) => project.release_start(spec).map(drop),
        Release(Publish { name }) => project.pargit_publish("release", name),
        Release(ReleaseCommand::Delete { name }) => project.pargit_delete("release", name),
        Release(Finish { name }) => project.release_finish(name),
        Release(ReleaseCommand::Version(kind)) => project.release_version(kind),
        Feature(FeatureCommand::Delete { name }) => project.pargit_delete("feature", name),
        Feature(FeatureCommand::Start { name }) => {
            project.pargit_start("feature", &name, "develop")
        }
        Feature(FeatureCommand::Publish { name }) => project.pargit_publish("feature", name),
        Feature(FeatureCommand::Finish { name }) => {
            project.pargit_finish("feature", name, "develop")
        }
        commands::Command::Version(VersionCommand::Bump(kind)) => project.bump_version(kind),
        Cleanup => project.pargit_cleanup(),
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

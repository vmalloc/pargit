use anyhow::Result;
use commands::{ReleaseCommand, VersionCommand};
use log::error;
use structopt::StructOpt;

mod commands;
mod repo;
mod version_management;

#[derive(StructOpt)]
struct Opts {
    #[structopt(short = "-v", parse(from_occurrences))]
    verbosity: usize,

    #[structopt(short = "-p", long = "--path", default_value = ".")]
    path: String,

    #[structopt(subcommand)]
    command: commands::Command,
}

fn entry_point(opts: Opts) -> Result<()> {
    log::debug!("Starting...");

    use commands::{Command::*, FeatureCommand, ReleaseCommand::*};

    let repo = repo::Repository::on_path(opts.path)?;

    match opts.command {
        Release(Start { spec }) => crate::version_management::release_start(&repo, spec),
        Release(Publish { name }) => repo.pargit_publish("release", name),
        Release(ReleaseCommand::Delete { name }) => repo.release_delete(name),
        Release(Finish { name }) => repo.release_finish(name),
        Release(ReleaseCommand::Version(kind)) => {
            crate::version_management::release_version(&repo, kind)
        }
        Feature(FeatureCommand::Delete { name }) => repo.pargit_delete("feature", name),
        Feature(FeatureCommand::Start { name }) => repo.pargit_start("feature", &name, "develop"),
        Feature(FeatureCommand::Publish { name }) => repo.pargit_publish("feature", name),
        Feature(FeatureCommand::Finish { name }) => repo.pargit_finish("feature", name, "develop"),
        commands::Command::Version(VersionCommand::Bump(kind)) => {
            crate::version_management::bump_version(&repo, kind)
        }
    }
}

fn main() {
    let opts = Opts::from_args();
    env_logger::Builder::new()
        .filter_level(match opts.verbosity {
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

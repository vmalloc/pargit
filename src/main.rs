use std::{io::Write, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use commands::{FlowCommand, ReleaseCommand, VersionCommand};
use log::error;
use pargit::Pargit;
use utils::ObjectKind;

mod commands;
mod config;
mod pargit;
mod project_types;
mod release;
mod repo;
mod utils;
mod version_file;

#[derive(Parser)]
#[command(version)]
/// Pargit is a tool for manipulating releases and branching schemes of git-managed projects
struct Opts {
    #[clap(global = true, short = 'v', action=clap::ArgAction::Count)]
    /// increases log verbosity (can be specified multiple times)
    verbosity: u8,

    #[clap(global = true, short = 'q', action=clap::ArgAction::Count)]
    /// reduces log verbosity (can be specified multiple times)
    quietness: u8,

    #[clap(short = 'p', long = "path", default_value = ".")]
    /// Path of the repository to operate on
    path: PathBuf,

    #[clap(subcommand)]
    command: commands::Command,
}

fn entry_point(opts: Opts) -> Result<()> {
    log::debug!("Starting...");

    use commands::Command::*;

    if let Configure = opts.command {
        process_configure_command()
    } else {
        let project = Pargit::new(&opts.path)?;

        match opts.command {
            Configure => Ok(()),
            Release(cmd) => process_release_command(&project, cmd, ObjectKind::Release),
            Hotfix(cmd) => process_release_command(&project, cmd, ObjectKind::Hotfix),
            Feature(cmd) => process_flow_command(&project, ObjectKind::Feature, cmd),
            Bugfix(cmd) => process_flow_command(&project, ObjectKind::Bugfix, cmd),
            commands::Command::Version(VersionCommand::Bump { kind }) => project.bump_version(kind),
            Cleanup => project.pargit_cleanup(),
        }
    }
}

fn process_configure_command() -> Result<()> {
    let filename = ".pargit.toml";
    std::fs::File::create_new(filename)
        .context("Failed creating config file")?
        .write_all(crate::config::Config::sample().as_bytes())
        .context("Failed writing configuration")?;

    log::info!("Successfully wrote {filename}");
    Ok(())
}

fn process_release_command(
    project: &Pargit,
    cmd: ReleaseCommand,
    release_kind: ObjectKind,
) -> Result<()> {
    use commands::ReleaseCommand::*;

    match cmd {
        Start { spec, from_ref } => project
            .release_start(spec, release_kind, from_ref.as_deref())
            .map(drop),
        Publish { name } => project.pargit_publish(release_kind, name),
        ReleaseCommand::Delete { name } => project.pargit_delete(release_kind, name),
        Finish { name, options } => project.release_finish(name, None, release_kind, options),
        ReleaseCommand::Version { kind, options } => {
            project.release_version(kind, release_kind, options)
        }
    }
}

fn process_flow_command(project: &Pargit, kind: ObjectKind, cmd: FlowCommand) -> Result<()> {
    match cmd {
        FlowCommand::Delete { name } => project.pargit_delete(kind, name),
        FlowCommand::Start { name, from_ref } => {
            project.pargit_start(kind, &name, from_ref.as_deref())
        }
        FlowCommand::Publish { name } => project.pargit_publish(kind, name),
        FlowCommand::Finish { name } => {
            project.pargit_finish(kind, name, &project.config().develop_branch_name)
        }
    }
}

fn main() {
    let opts = Opts::parse();

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

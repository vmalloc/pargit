use anyhow::Result;
use structopt::StructOpt;

mod commands;
mod repo;

#[derive(StructOpt)]
struct Opts {
    #[structopt(long)]
    verbose: bool,

    #[structopt(short = "-p", long = "--path", default_value = ".")]
    path: String,

    #[structopt(subcommand)]
    command: commands::Command,
}

fn entry_point(opts: Opts) -> Result<()> {
    use commands::{Command::*, ReleaseCommand::*};

    let repo = repo::Repository::on_path(opts.path)?;

    match opts.command {
        Release(Start { name }) => repo.release_start(&name),
    }
}

fn main() {
    let opts = Opts::from_args();
    env_logger::Builder::new()
        .filter_level(if opts.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Error
        })
        .init();

    if let Err(e) = entry_point(opts) {
        eprintln!(
            "{}",
            console::style(format!("Error encountered: {:?}", e)).red()
        );
        std::process::exit(-1);
    }
}

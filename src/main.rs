use anyhow::Result;
use git2::Repository;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    #[structopt(long)]
    verbose: bool,
}

fn entry_point(opts: Opts) -> Result<()> {
    let repo = Repository::open(".")?;
    for branch in repo.branches(None)? {
        println!("{:?}", branch);
    }
    Ok(())
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
        eprintln!("Error: {:?}", e);
        std::process::exit(-1);
    }
}

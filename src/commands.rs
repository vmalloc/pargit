use structopt::StructOpt;

#[derive(StructOpt)]
pub enum Command {
    Release(ReleaseCommand),
}

#[derive(StructOpt)]
pub enum ReleaseCommand {
    Start { name: String },
}

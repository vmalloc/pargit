use structopt::StructOpt;

#[derive(StructOpt)]
pub enum Command {
    Release(ReleaseCommand),
}

#[derive(StructOpt)]
pub enum ReleaseCommand {
    Start { name: String },
    Publish { name: Option<String> },
    Delete { name: Option<String> },
    Finish { name: Option<String> },
}

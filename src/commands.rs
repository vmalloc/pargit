use structopt::StructOpt;

#[derive(StructOpt)]
pub enum Command {
    Release(ReleaseCommand),
    Version(VersionCommand),
}

#[derive(StructOpt)]
pub enum ReleaseCommand {
    Start { name: String },
    Publish { name: Option<String> },
    Delete { name: Option<String> },
    Finish { name: Option<String> },
    Version(BumpKind),
}

#[derive(StructOpt)]
pub enum VersionCommand {
    Bump(BumpKind),
}

#[derive(StructOpt, Clone, Copy)]
pub enum BumpKind {
    Major,
    Minor,
    Patch,
}

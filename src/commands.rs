use anyhow::Error;
use clap::{Parser, Subcommand};
use semver::Version;
use std::str::FromStr;
use strum_macros::EnumString;

#[derive(Subcommand)]
pub enum Command {
    Configure,
    #[clap(subcommand)]
    Hotfix(ReleaseCommand),
    #[clap(subcommand)]
    Release(ReleaseCommand),
    #[clap(subcommand)]
    Feature(FlowCommand),
    #[clap(subcommand)]
    Bugfix(FlowCommand),
    #[clap(subcommand)]
    Version(VersionCommand),
    Cleanup,
}

#[derive(Parser)]
pub struct ReleaseOptions {
    #[clap(long)]
    pub no_pull: bool,
}

#[derive(Subcommand)]
pub enum ReleaseCommand {
    Start {
        spec: VersionSpec,
        #[clap(long = "from-ref")]
        from_ref: Option<String>,
    },
    Publish {
        name: Option<String>,
    },
    Delete {
        name: Option<String>,
    },
    Finish {
        #[clap(flatten)]
        options: ReleaseOptions,
        name: Option<String>,
    },
    Version {
        #[clap(flatten)]
        options: ReleaseOptions,
        kind: BumpKind,
    },
}

#[derive(Subcommand)]
pub enum FlowCommand {
    Start {
        name: String,
        #[clap(long = "from-ref")]
        from_ref: Option<String>,
    },
    Publish {
        name: Option<String>,
    },
    Delete {
        name: Option<String>,
    },
    Finish {
        name: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum VersionCommand {
    Bump { kind: BumpKind },
}

#[derive(Clone, Copy, EnumString, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum BumpKind {
    Major,
    Minor,
    Patch,
}

#[derive(Clone)]
pub enum VersionSpec {
    Exact(Version),
    Bump(BumpKind),
}

impl FromStr for VersionSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<BumpKind>()
            .map(VersionSpec::Bump)
            .or_else(|_| s.parse::<Version>().map(VersionSpec::Exact))
            .map_err(Error::from)
    }
}

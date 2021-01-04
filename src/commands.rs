use anyhow::Error;
use semver::Version;
use std::str::FromStr;
use structopt::StructOpt;
use strum_macros::EnumString;

#[derive(StructOpt)]
pub enum Command {
    Release(ReleaseCommand),
    Feature(FeatureCommand),
    Version(VersionCommand),
}

#[derive(StructOpt)]
pub enum ReleaseCommand {
    Start {
        #[structopt(parse(try_from_str))]
        spec: VersionSpec,
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
    Version(BumpKind),
}

#[derive(StructOpt)]
pub enum FeatureCommand {
    Start { name: String },
    Publish { name: Option<String> },
    Delete { name: Option<String> },
    Finish { name: Option<String> },
}

#[derive(StructOpt)]
pub enum VersionCommand {
    Bump(BumpKind),
}

#[derive(StructOpt, Clone, Copy, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum BumpKind {
    Major,
    Minor,
    Patch,
}

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

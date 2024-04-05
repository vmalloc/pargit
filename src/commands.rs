use anyhow::Error;
use clap::{Parser, Subcommand};
use semver::Version;
use std::str::FromStr;
use strum_macros::EnumString;

#[derive(Subcommand)]
pub enum Command {
    #[clap(verbatim_doc_comment)]
    /// Pargit has several aspects of its behavior that can be controlled and customized through an optional repository file,
    /// called .pargit.toml
    ///
    /// This command generates this file interactively.
    ///
    /// Below is a sample .pargit.toml file with its optional fields:
    ///
    /// # .pargit.toml
    /// main_branch_name = "main"
    /// develop_branch_name = "develop"
    /// # if present, specifies a prefix to be used for tags created
    /// # by pargit
    /// tag_prefix = "v"
    ///
    /// [project]
    /// # if present - points to the subdirectory in which
    /// # the actual project resides
    /// subdpath = "./subpath"
    /// # if present, specifies the command to be executed
    /// # when the project is to be compiled during version bump
    /// # and release
    /// compilation_command = "cargo check"
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

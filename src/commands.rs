use anyhow::Error;
use clap::{Parser, Subcommand};
use semver::Version;
use std::str::FromStr;
use strum_macros::EnumString;

#[derive(Subcommand)]
pub enum Command {
    #[clap(verbatim_doc_comment)]
    /// Configures the current pargit repository
    ///
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
    /// Creates and manipulates releases
    Release(ReleaseCommand),
    /// Manipulate feature branches
    #[clap(subcommand)]
    Feature(FlowCommand),
    /// Manipulate bugfix branches
    #[clap(subcommand)]
    Bugfix(FlowCommand),
    #[clap(subcommand)]
    /// Manipulates versions of the current repository
    Version(VersionCommand),
    /// Cleans up the current branch if it is already merged to develop or main branches.
    Cleanup,
}

#[derive(Parser)]
pub struct ReleaseOptions {
    #[clap(long)]
    /// avoids pulling upstream when performing the release
    pub no_pull: bool,
}

#[derive(Subcommand)]
pub enum ReleaseCommand {
    /// Creates a new branch for release
    Start {
        /// Names this new release. If "major", "minor" or "patch" are specified, the name pargit will use is a new version bumped from the current
        /// version of the project
        spec: VersionSpec,
        #[clap(long = "from-ref")]
        /// Starts the release branch from the specified ref (commit hash, branch name, etc.)
        from_ref: Option<String>,
    },
    /// Publishes this release to a remote branch upstream
    Publish {
        /// The release to publish (defaults to the current branch)
        name: Option<String>,
    },
    /// Deletes the release branch, and its published upstream branch (if one exists)
    Delete {
        /// The name of the release branch to delete (defaults to current branch)
        name: Option<String>,
    },
    /// Finishes a release. This tags the release branch and merges back to the main and develop branches. Once successful,
    /// the release branch and its upstream will be deleted
    Finish {
        #[clap(flatten)]
        options: ReleaseOptions,
        /// Name of the release branch to finalize. Defaults to current branch
        name: Option<String>,
    },
    /// Releases a version in one shot. This means creating the branch, bumping its version as specified, and finalizing a release from it
    Version {
        #[clap(flatten)]
        options: ReleaseOptions,

        /// Kind of release to perform (major, minor or patch)
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

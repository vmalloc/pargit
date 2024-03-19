use crate::{commands::VersionSpec, utils::next_version};
use anyhow::Result;
use log::debug;
use semver::Version;
use std::path::{Path, PathBuf};

pub enum VersionFile {
    CargoToml {
        path: PathBuf,
        version: Version,
        is_workspace: bool,
    },
}

impl std::fmt::Debug for VersionFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VersionFile")
            .field(
                "path",
                match self {
                    VersionFile::CargoToml { path, .. } => path,
                },
            )
            .finish()
    }
}

impl VersionFile {
    pub fn bump(&self, spec: VersionSpec) -> Result<()> {
        match self {
            VersionFile::CargoToml {
                path,
                version,
                is_workspace,
            } => {
                debug!("Bumping Cargo.toml file {:?}", path);

                let version = match spec {
                    VersionSpec::Exact(version) => version,
                    VersionSpec::Bump(kind) => next_version(version, kind),
                };
                crate::project_types::rust::write_cargo_toml_version(path, &version, *is_workspace)
            }
        }
    }

    pub fn version(&self) -> Version {
        match self {
            VersionFile::CargoToml { version, .. } => version.clone(),
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            VersionFile::CargoToml { path, .. } => path,
        }
    }

    pub fn typename(&self) -> &'static str {
        match self {
            VersionFile::CargoToml { .. } => "Cargo.toml",
        }
    }
}

use crate::{commands::VersionSpec, utils::next_version};
use anyhow::Result;
use log::debug;
use semver::Version;
use std::path::{Path, PathBuf};

pub enum VersionFile {
    CargoToml(PathBuf, Version),
}

impl VersionFile {
    pub fn bump(&self, spec: VersionSpec) -> Result<()> {
        match self {
            VersionFile::CargoToml(path, version) => {
                debug!("Bumping Cargo.toml file {:?}", path);

                let version = match spec {
                    VersionSpec::Exact(version) => version,
                    VersionSpec::Bump(kind) => next_version(version, kind),
                };
                crate::project_types::rust::write_cargo_toml_version(path, &version)
            }
        }
    }

    pub fn version(&self) -> Version {
        match self {
            VersionFile::CargoToml(_, version) => version.clone(),
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            VersionFile::CargoToml(p, _) => p,
        }
    }

    pub fn typename(&self) -> &'static str {
        match self {
            VersionFile::CargoToml(_, _) => "Cargo.toml",
        }
    }
}

use crate::{commands::BumpKind, repo::Repository};
use anyhow::{Context, Result};
use log::debug;
use semver::Version;
use std::{
    fs::read_to_string,
    io::Write,
    path::{Path, PathBuf},
};
use toml_edit::{value, Document};

pub fn bump_version(repo: &Repository, bump_kind: BumpKind) -> Result<()> {
    let cargo_tomls = find_cargo_tomls(repo.path())?;

    for (path, mut toml, mut version) in cargo_tomls {
        match bump_kind {
            BumpKind::Major => version.increment_major(),
            BumpKind::Minor => version.increment_minor(),
            BumpKind::Patch => version.increment_patch(),
        }
        toml["package"]["version"] = value(version.to_string());
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)?
            .write_all(toml.to_string().as_bytes())?
    }

    repo.cargo_check()
}

fn find_cargo_tomls(path: &Path) -> Result<Vec<(PathBuf, Document, Version)>> {
    let mut returned = Vec::new();
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        let path = entry.path();

        if path.file_name().unwrap() == "Cargo.toml" {
            let toml: toml_edit::Document = read_to_string(path)
                .context("Failed reading file")?
                .parse()
                .context("Failed parsing Cargo.toml file")?;

            if let Some(version) = toml["package"]["version"]
                .as_str()
                .map(Version::parse)
                .transpose()
                .with_context(|| format!("Failed parsing version for {:?}", path))?
            {
                debug!("Found Cargo.toml: {:?} (version={})", path, version);
                returned.push((path.into(), toml, version));
            }
        }
    }
    Ok(returned)
}

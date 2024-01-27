use anyhow::{Context, Result};
use log::debug;
use semver::Version;
use std::{fs::read_to_string, io::Write, path::Path};
use toml_edit::value;

use crate::{repo::Repository, version_file::VersionFile};

pub fn find_cargo_tomls(repo: &Repository) -> Result<Vec<VersionFile>> {
    let mut returned = Vec::new();
    for entry in walkdir::WalkDir::new(repo.path())
        .contents_first(false)
        .into_iter()
        .filter_entry(|entry| {
            !(entry.path().is_dir()
                && entry.path().file_name().map(|s| s.to_string_lossy()) == Some("target".into()))
        })
    {
        let entry = entry?;
        let path = entry.path();

        if path.file_name().unwrap() == "Cargo.toml" {
            if repo.is_path_ignored(path)? {
                debug!("{:?} is ignored in .gitignore. Skipping", path);
                continue;
            }

            let toml: toml_edit::Document = read_to_string(path)
                .with_context(|| format!("Failed reading file {path:?} "))?
                .parse()
                .context("Failed parsing Cargo.toml file")?;

            if let Some(version) = toml["package"]["version"]
                .as_str()
                .map(Version::parse)
                .transpose()
                .with_context(|| format!("Failed parsing version for {:?}", path))?
            {
                debug!("Found Cargo.toml: {:?} (version={})", path, version);
                returned.push(VersionFile::CargoToml(path.to_owned(), version))
            }
        }
    }

    if returned.is_empty() {
        anyhow::bail!("Could not find Cargo.toml files in {:?}", repo.path());
    }

    Ok(returned)
}

pub fn write_cargo_toml_version(path: &Path, new_version: &Version) -> Result<()> {
    let mut toml: toml_edit::Document = read_to_string(path)
        .with_context(|| format!("Failed reading file {path:?}"))?
        .parse()
        .context("Failed parsing Cargo.toml file")?;

    toml["package"]["version"] = value(new_version.to_string());
    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)?
        .write_all(toml.to_string().as_bytes())?;

    Ok(())
}

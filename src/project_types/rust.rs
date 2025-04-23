use anyhow::{Context, Result};
use log::debug;
use semver::Version;
use std::{fs::read_to_string, io::Write, path::Path};
use toml_edit::value;

use crate::{repo::Repository, version_file::VersionFile};

pub fn find_cargo_tomls(repo: &Repository) -> Result<Vec<VersionFile>> {
    let mut ignore_dirs: Vec<_> = repo
        .submodule_paths()?
        .into_iter()
        .map(|path_buf| path_buf.to_string_lossy().to_string())
        .collect();
    ignore_dirs.push("target".into());

    let mut returned = Vec::new();
    for entry in walkdir::WalkDir::new(repo.path())
        .contents_first(false)
        .into_iter()
        .filter_entry(|entry| {
            !(entry.path().is_dir()
                && entry
                    .path()
                    .file_name()
                    .map(|s| {
                        let entry_file_name = s.to_string_lossy().to_string();
                        ignore_dirs.contains(&entry_file_name)
                    })
                    .unwrap_or(false))
        })
    {
        let entry = entry?;
        let path = entry.path();

        if path.file_name().unwrap() == "Cargo.toml" {
            if repo.is_path_ignored(path)? {
                debug!("{:?} is ignored in .gitignore. Skipping", path);
                continue;
            }

            let toml: toml_edit::DocumentMut = read_to_string(path)
                .with_context(|| format!("Failed reading file {path:?} "))?
                .parse()
                .with_context(|| format!("Failed parsing {path:?}"))?;

            if let Some(version) = toml
                .get("package")
                .and_then(|t| t.get("version")?.as_str())
                .map(Version::parse)
                .transpose()
                .with_context(|| format!("Failed parsing version for {path:?}"))?
            {
                debug!("Found Cargo.toml: {path:?} (version={version})");
                returned.push(VersionFile::CargoToml {
                    path: path.to_owned(),
                    version,
                    is_workspace: false,
                })
            } else if let Some(version) = toml
                .get("workspace")
                .and_then(|w| w.get("package")?.get("version")?.as_str())
                .map(Version::parse)
                .transpose()
                .with_context(|| format!("Failed parsing workspace package version for {path:?}"))?
            {
                debug!("Found workspace Cargo.toml: {path:?} (version={version})",);
                returned.push(VersionFile::CargoToml {
                    path: path.to_owned(),
                    version,
                    is_workspace: true,
                })
            }
        }
    }

    if returned.is_empty() {
        anyhow::bail!("Could not find Cargo.toml files in {:?}", repo.path());
    }

    Ok(returned)
}

pub fn write_cargo_toml_version(
    path: &Path,
    new_version: &Version,
    is_workspace_file: bool,
) -> Result<()> {
    let mut toml: toml_edit::DocumentMut = read_to_string(path)
        .with_context(|| format!("Failed reading file {path:?}"))?
        .parse()
        .context("Failed parsing Cargo.toml file")?;

    let new_version = value(new_version.to_string());

    if is_workspace_file {
        toml["workspace"]["package"]["version"] = new_version;
    } else {
        toml["package"]["version"] = new_version;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)?
        .write_all(toml.to_string().as_bytes())?;

    Ok(())
}

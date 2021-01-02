use crate::{commands::BumpKind, repo::Repository};
use anyhow::{bail, format_err, Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use log::debug;
use semver::Version;
use std::{
    fs::read_to_string,
    io::Write,
    path::{Path, PathBuf},
};
use toml_edit::{value, Document};

pub fn release_version(repo: &Repository, bump_kind: BumpKind) -> Result<()> {
    let cargo_tomls = find_cargo_tomls(repo.path())?;
    let index = if cargo_tomls.len() > 1 {
        let selections = cargo_tomls
            .iter()
            .map(|(p, _, _)| {
                let root = repo.path();
                let relpath = pathdiff::diff_paths(p, root).unwrap();
                relpath.to_string_lossy().to_string()
            })
            .collect::<Vec<_>>();

        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Multiple Cargo.toml files found. Which one would you like to bump?")
            .default(0)
            .items(&selections[..])
            .interact()
            .context("Could not get bumped Cargo.toml")?
    } else {
        0
    };

    let cargo_tomls: Vec<_> = cargo_tomls.into_iter().skip(index).take(1).collect();
    let new_version = next_version(&cargo_tomls[0].2, bump_kind);
    let release_name = new_version.to_string();

    repo.release_start(&release_name)?;
    bump_cargo_tomls(repo, cargo_tomls, bump_kind)?;
    repo.commit_all("Bump version")?;
    repo.release_finish(None)
}

pub fn bump_version(repo: &Repository, bump_kind: BumpKind) -> Result<()> {
    bump_cargo_tomls(repo, find_cargo_tomls(repo.path())?, bump_kind)
}

fn bump_cargo_tomls(
    repo: &Repository,
    cargo_tomls: Vec<(PathBuf, Document, Version)>,
    bump_kind: BumpKind,
) -> Result<()> {
    if cargo_tomls.is_empty() {
        bail!("No Cargo.toml files found");
    }

    for (path, mut toml, mut version) in cargo_tomls {
        version = next_version(&version, bump_kind);

        toml["package"]["version"] = value(version.to_string());
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)?
            .write_all(toml.to_string().as_bytes())?
    }
    repo.cargo_check()
}

fn next_version(version: &Version, bump_kind: BumpKind) -> Version {
    let mut version = version.clone();
    match bump_kind {
        BumpKind::Major => version.increment_major(),
        BumpKind::Minor => version.increment_minor(),
        BumpKind::Patch => version.increment_patch(),
    }
    version
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

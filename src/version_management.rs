use crate::{
    commands::{BumpKind, VersionSpec},
    repo::Repository,
};
use anyhow::{bail, format_err, Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use log::debug;
use semver::Version;
use std::{fs::read_to_string, io::Write, path::PathBuf};
use toml_edit::{value, Document};

pub fn release_start(repo: &Repository, spec: VersionSpec) -> Result<()> {
    let (new_version, toml_bump) = match spec {
        VersionSpec::Exact(v) => (v, None),
        VersionSpec::Bump(kind) => {
            let toml = deduce_cargo_toml_version(repo)?;
            (next_version(&toml.2, kind), Some((toml, kind)))
        }
    };

    repo.release_start(&new_version.to_string())?;

    if let Some((toml, kind)) = toml_bump {
        bump_cargo_tomls(repo, vec![toml], kind)?
    }
    Ok(())
}

pub fn release_version(repo: &Repository, bump_kind: BumpKind) -> Result<()> {
    let cargo_tomls = vec![deduce_cargo_toml_version(repo)?];
    let new_version = next_version(&cargo_tomls[0].2, bump_kind);
    let release_name = new_version.to_string();

    repo.release_start(&release_name)?;
    bump_cargo_tomls(repo, cargo_tomls, bump_kind)?;
    repo.commit_all("Bump version")?;
    repo.release_finish(None)
}

fn deduce_cargo_toml_version(repo: &Repository) -> Result<(PathBuf, Document, Version)> {
    let cargo_tomls = find_cargo_tomls(repo)?;
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

    Ok(cargo_tomls
        .into_iter()
        .nth(index)
        .ok_or_else(|| format_err!("Could not find Cargo.toml files to deduce version from"))?)
}

pub fn bump_version(repo: &Repository, bump_kind: BumpKind) -> Result<()> {
    bump_cargo_tomls(repo, vec![deduce_cargo_toml_version(repo)?], bump_kind)
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
    repo.compile_project()
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

fn find_cargo_tomls(repo: &Repository) -> Result<Vec<(PathBuf, Document, Version)>> {
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

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

pub fn release_start(repo: &Repository,  -> Result<()> {
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
        
    }
    repo.project.compile()
}




use anyhow::{Context, Result};
use log::debug;
use semver::Version;
use std::{fs::read_to_string, io::Write, path::Path};

use crate::version_file::VersionFile;

pub fn find_package_json(project_path: &Path) -> Result<Vec<VersionFile>> {
    let package_json_path = project_path.join("package.json");

    if !package_json_path.exists() {
        anyhow::bail!("Could not find package.json in {:?}", project_path);
    }

    let contents = read_to_string(&package_json_path)
        .with_context(|| format!("Failed reading {:?}", package_json_path))?;

    let parsed: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("Failed parsing {:?}", package_json_path))?;

    let version_str = parsed
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::format_err!("No \"version\" field found in {:?}", package_json_path))?;

    let version = Version::parse(version_str)
        .with_context(|| format!("Failed parsing version {:?} in {:?}", version_str, package_json_path))?;

    debug!("Found package.json: {:?} (version={version})", package_json_path);

    Ok(vec![VersionFile::PackageJson {
        path: package_json_path,
        version,
    }])
}

pub fn write_package_json_version(path: &Path, new_version: &Version) -> Result<()> {
    let contents = read_to_string(path)
        .with_context(|| format!("Failed reading {:?}", path))?;

    let mut parsed: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("Failed parsing {:?}", path))?;

    parsed["version"] = serde_json::Value::String(new_version.to_string());

    let new_contents = serde_json::to_string_pretty(&parsed)
        .context("Failed serializing package.json")?;

    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)?
        .write_all(format!("{}\n", new_contents).as_bytes())?;

    Ok(())
}

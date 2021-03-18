use crate::commands::BumpKind;
use anyhow::{bail, Result};
use dialoguer::theme::{ColorfulTheme, SimpleTheme, Theme};
use semver::Version;
use std::{
    path::Path,
    process::{Output, Stdio},
};

pub trait PathExt {
    fn shell(&self, cmd: impl AsRef<str>) -> Result<()> {
        self.shell_output(cmd).map(drop)
    }

    fn shell_output(&self, cmd: impl AsRef<str>) -> Result<Output>;
}

impl<T: AsRef<Path>> PathExt for T {
    fn shell_output(&self, cmd: impl AsRef<str>) -> Result<Output> {
        let child = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd.as_ref())
            .current_dir(self)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            bail!("Git failed: {:?}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(output)
    }
}

pub fn next_version(version: &Version, bump_kind: BumpKind) -> Version {
    let mut version = version.clone();
    match bump_kind {
        BumpKind::Major => version.increment_major(),
        BumpKind::Minor => version.increment_minor(),
        BumpKind::Patch => version.increment_patch(),
    }
    version
}

pub fn get_color_theme() -> Box<dyn Theme> {
    if std::env::var("PARGIT_DISABLE_COLORS").is_ok() {
        Box::new(SimpleTheme)
    } else {
        Box::new(ColorfulTheme::default())
    }
}

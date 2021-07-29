use crate::commands::BumpKind;
use anyhow::{bail, Result};
use dialoguer::theme::{ColorfulTheme, SimpleTheme, Theme};
use log::debug;
use semver::Version;
use std::{
    path::Path,
    process::{Output, Stdio},
};

pub trait ResultExt {
    fn ignore_errors(self);
}

impl<E: std::fmt::Debug> ResultExt for std::result::Result<(), E> {
    fn ignore_errors(self) {
        let _ = self.map_err(|e| log::error!("Ignoring error: {:?}", e));
    }
}

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
            bail!(
                "Git failed: command {:?} failed: {:?}",
                cmd.as_ref(),
                String::from_utf8_lossy(&output.stderr)
            );
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

pub fn can_ask_questions() -> bool {
    std::env::var("PARGIT_NON_INTERACTIVE").as_deref() != Ok("1")
}

struct ExitStackItem<'a> {
    msg: &'static str,
    callback: Box<dyn FnOnce() + 'a>,
}

#[derive(Default)]
pub struct ExitStack<'a> {
    history: Vec<ExitStackItem<'a>>,
}

impl<'a> Drop for ExitStack<'a> {
    fn drop(&mut self) {
        debug!("Rolling back undo history...");
        for item in self.history.drain(..).rev() {
            let callback = item.callback;
            debug!("{}...", item.msg);
            callback()
        }
    }
}

impl<'a> ExitStack<'a> {
    pub fn remember(&mut self, msg: &'static str, f: impl FnOnce() + 'a) {
        self.history.push(ExitStackItem {
            msg,
            callback: Box::new(f),
        })
    }
    pub fn forget(&mut self) {
        self.history.clear()
    }
}

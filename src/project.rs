use crate::{
    commands::{BumpKind, VersionSpec},
    config::Config,
    release::Release,
    repo::Repository,
    utils::{get_color_theme, next_version, PathExt},
    version_file::VersionFile,
};
use anyhow::{bail, format_err, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use git2::ErrorCode;
use log::{debug, error, info};
use std::path::{Path, PathBuf};

pub struct Project {
    path: PathBuf,
    pub repo: Repository,
    type_: Option<ProjectType>,
    config: Config,
}

impl Project {
    pub fn new(path: &Path) -> Result<Self> {
        let type_ = if path.join("Cargo.toml").exists() {
            Some(ProjectType::Rust)
        } else {
            None
        };
        let repo = Repository::on_path(path)?;
        let config = Config::load(path)?;
        let returned = Self {
            path: path.canonicalize()?,
            config,
            repo,
            type_,
        };
        returned.check_configuration()?;
        Ok(returned)
    }

    pub fn check_configuration(&self) -> Result<()> {
        self.repo.find_branch(&self.config.develop_branch_name)?;
        self.ensure_master_branch().map(drop)
    }

    fn ensure_master_branch(&self) -> Result<()> {
        match self.repo.find_branch(&self.config.master_branch_name) {
            Ok(_) => Ok(()),
            Err(e) => {
                if let Some(e) = e.downcast_ref::<git2::Error>() {
                    if e.code() == ErrorCode::NotFound
                        && Confirm::with_theme(get_color_theme().as_ref())
                            .with_prompt("Master branch not found. Create it?")
                            .interact()?
                    {
                        self.repo.path().shell("git branch master origin/master")?;
                        return Ok(());
                    }
                }
                Err(e)
            }
        }
    }

    pub fn configure(&mut self) -> Result<()> {
        self.config.reconfigure()?;
        self.config.save(&self.path)?;
        Ok(())
    }

    // High-level API

    pub fn bump_version(&self, bump_kind: BumpKind) -> Result<()> {
        let bumped_file = self
            .get_version_file()?
            .ok_or_else(|| format_err!("Unable to find version file"))?;
        bumped_file.bump(VersionSpec::Bump(bump_kind))?;
        info!("Compiling project to lock version");
        self.compile()
    }

    pub fn pargit_cleanup(&self) -> Result<()> {
        self.repo.cleanup(&self.config.develop_branch_name)
    }

    pub fn pargit_delete(&self, object_type: &str, name: Option<String>) -> Result<()> {
        let release_name = self.resolve_name(object_type, name)?;
        let branch_name = self.prefix(object_type, &release_name);
        info!("Deleting {}...", branch_name);

        let mut branch = self.repo.find_branch(&branch_name)?;
        if let Ok(upstream) = branch.upstream() {
            let mut parts = upstream.name()?.unwrap().splitn(2, '/');
            let origin_name = parts.next().unwrap();
            let remote_branch_name = parts.next().unwrap();
            info!("Deleting remote branch {}", remote_branch_name);
            self.path
                .shell(format!("git push {} :{}", origin_name, remote_branch_name))?;
        }

        if branch.name()?.unwrap() == self.repo.current_branch_name()? {
            self.repo
                .switch_to_branch_name(&self.config.develop_branch_name)?;
        }
        info!("Deleting branch {:?}", branch_name);
        branch.delete()?;

        Ok(())
    }

    pub fn pargit_finish(
        &self,
        object_type: &str,
        name: Option<String>,
        dest_branch: &str,
    ) -> Result<()> {
        let name = self.resolve_name(object_type, name)?;
        debug!("Switching to branch {}", dest_branch);
        self.repo.switch_to_branch_name(dest_branch)?;
        let branch_name = self.prefix(object_type, &name);
        debug!("Merging {}", branch_name);
        self.repo
            .merge_branch_name(&branch_name, &format!("Merge {}", branch_name))?;
        self.pargit_delete(object_type, Some(name))
    }

    pub fn pargit_publish(&self, object_type: &str, name: Option<String>) -> Result<()> {
        let name = self.resolve_name(object_type, name)?;
        let branch_name = self.prefix(object_type, &name);
        info!("Pushing {} to origin...", branch_name);
        let output = self
            .path
            .shell_output(format!("git push -u origin {0}:{0}", branch_name))?;
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            if line.starts_with("remote:") {
                info!("{}", line);
            }
        }
        Ok(())
    }

    pub fn pargit_start(&self, branch_type: &str, name: &str, start_point: &str) -> Result<()> {
        info!("Creating {} branch {}", branch_type, name);
        let branch_name = self.prefix(branch_type, name);
        if self.repo.find_branch(branch_name).is_ok() {
            bail!(
                "{} {} already in progress. Finish it first",
                branch_type,
                name
            );
        }
        let b = self
            .repo
            .create_branch(self.prefix(branch_type, name), Some(start_point))?;

        self.repo.switch_to_branch(&b)
    }

    pub fn release_version(&self, bump_kind: BumpKind) -> Result<()> {
        let release = self.release_start(VersionSpec::Bump(bump_kind))?;
        self.repo.commit_all("Bump version")?;
        self.release_finish(Some(release.name))
    }

    pub fn release_start(&self, spec: VersionSpec) -> Result<Release> {
        let release = self.resolve_release(spec)?;

        if self.repo.has_tag(&release.tag)? {
            bail!("Tag {} already exists", release.tag);
        }
        self.pargit_start("release", &release.name, &self.config.develop_branch_name)?;
        if let Some(version_file) = release.version_file.as_ref() {
            version_file.bump(VersionSpec::Exact(release.version.clone()))?;
            info!("Compiling project to lock new version");
            self.compile()?;
        }
        Ok(release)
    }

    pub fn release_finish(&self, release_name: Option<String>) -> Result<()> {
        let release_name = self.resolve_name("release", release_name)?;
        let release_branch_name = self.prefix_release(&release_name);
        info!("Finishing release {}", release_name);
        self.repo.switch_to_branch_name(&release_branch_name)?;
        self.check_pre_release()?;

        let temp_branch_name = format!("in-progress-release-{}", release_name);

        self.repo
            .create_branch(&temp_branch_name, Some(&self.config.master_branch_name))?;
        info!("Switching to temporary branch");
        self.repo.switch_to_branch_name(&temp_branch_name)?;
        info!("Merging release branch");
        self.repo
            .merge_branch_name(
                &release_branch_name,
                &format!("Merge release branch {}", release_name),
            )
            .context("Failed merge")?;
        info!("Creating tag and pushing to remote master");
        let tag = self.config.get_tag_name(&release_name);
        let res = self
            .repo
            .create_tag(&tag)
            .and_then(|_| {
                self.path
                    .shell(format!("git push origin {}:master", temp_branch_name))
            })
            .context("Failed tag and push");

        // we try to push the merged master first. If it succeeds, it means we won the release
        if let Err(e) = res {
            error!("Failed pushing to master. Rolling back changes...");
            let _ = self
                .repo
                .delete_tag(&tag)
                .map_err(|e| error!("Failed deleting tag: {:?}", e));
            let _ = self.repo.switch_to_branch_name(&release_branch_name);

            let _ = self
                .repo
                .delete_branch_name(&temp_branch_name)
                .map_err(|e| error!("Failed deleting temporary branch: {:?}", e));

            bail!("Failed pushing new release - {:?}", e);
        }

        info!("Push successful. Merging to local master");
        self.repo
            .switch_to_branch_name(&self.config.master_branch_name)?;
        self.repo
            .merge_branch_name(&temp_branch_name, "Merge temporary release branch")?;
        info!("Pushing tags");
        self.path.shell("git push --tags")?;
        self.repo
            .switch_to_branch_name(&self.config.develop_branch_name)?;
        info!("Merging to develop branch");
        self.repo.merge_branch_name(
            &self.config.master_branch_name,
            &format!("Merge {} branch", self.config.master_branch_name),
        )?;

        self.repo
            .find_branch(temp_branch_name)?
            .delete()
            .context("Failed deleting temporary branch")?;
        self.pargit_delete("release", Some(release_name))?;
        info!("Pushing develop branch");
        self.path.shell("git push origin develop:develop")
    }

    fn resolve_name(&self, object_type: &str, name: Option<String>) -> Result<String> {
        Ok(match name {
            Some(name) => name,
            None => self
                .current_name(object_type)
                .context("Cannot get release name from branch name")?,
        })
    }

    fn prefix_release(&self, s: &str) -> String {
        self.prefix("release", s)
    }

    fn prefix(&self, object_type: &str, s: &str) -> String {
        format!("{}/{}", object_type, s)
    }

    fn current_name(&self, object_type: &str) -> Result<String> {
        self.repo
            .current_branch_name()?
            .strip_prefix(&format!("{}/", object_type))
            .ok_or_else(|| format_err!("Could not get current release name"))
            .map(|s| s.to_owned())
    }

    fn compile(&self) -> Result<()> {
        if let Some(type_) = &self.type_ {
            match type_ {
                ProjectType::Rust => {
                    // info!("Compiling project (cargo check)...");
                    self.path
                        .shell("cargo check --workspace --tests")
                        .context("Failed building project")
                }
            }
        } else {
            Ok(())
        }
    }

    fn resolve_release(&self, version: VersionSpec) -> Result<Release> {
        let version_file = self.get_version_file()?;
        Ok(match version {
            VersionSpec::Exact(version) => Release::version(&self.config, version, version_file),
            VersionSpec::Bump(kind) => {
                if let Some(version_file) = version_file {
                    Release::version(
                        &self.config,
                        next_version(&version_file.version(), kind),
                        Some(version_file),
                    )
                } else {
                    bail!("Cannot find version file to bump. Cannot deduce version")
                }
            }
        })
    }

    fn get_version_file(&self) -> Result<Option<VersionFile>> {
        let version_files = self.get_version_files()?;

        let index = if version_files.len() > 1 {
            let selections = version_files
                .iter()
                .map(|version_file| {
                    let relpath = pathdiff::diff_paths(version_file.path(), &self.path).unwrap();
                    relpath.to_string_lossy().to_string()
                })
                .collect::<Vec<_>>();

            Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "Multiple {} files found. Which one would you like to bump?",
                    version_files[0].typename()
                ))
                .default(0)
                .items(&selections[..])
                .interact()
                .context("Could not get bumped Cargo.toml")?
        } else {
            0
        };

        Ok(version_files.into_iter().nth(index))
    }

    fn get_version_files(&self) -> Result<Vec<VersionFile>> {
        self.type_
            .map(|type_| match type_ {
                ProjectType::Rust => crate::project_types::rust::find_cargo_tomls(&self.repo),
            })
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    // Checks
    fn check_pre_release(&self) -> Result<()> {
        info!("Running pre-release checks...");

        self.compile()?;
        if self.repo.is_dirty()? {
            bail!("Repository became dirty after build attempt. Perhaps Cargo.lock was not a part of the last commit?");
        }
        Ok(())
    }

    /// Get a reference to the project's config.
    pub fn config(&self) -> &Config {
        &self.config
    }
}

#[derive(Clone, Copy, Debug)]
enum ProjectType {
    Rust,
}

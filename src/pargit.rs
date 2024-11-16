use crate::{
    commands::{BumpKind, ReleaseOptions, VersionSpec},
    config::Config,
    release::Release,
    repo::Repository,
    utils::{
        can_ask_questions, get_color_theme, next_version, ExitStack, ObjectKind, PathExt, ResultExt,
    },
    version_file::VersionFile,
};
use anyhow::{bail, format_err, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use git2::ErrorCode;
use log::{debug, error, info, warn};
use semver::Version;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub struct Pargit {
    repo_path: PathBuf,
    project_path: PathBuf,
    repo: Repository,
    type_: Option<ProjectType>,
    config: Config,
}

impl Pargit {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let config = Config::load(repo_path)?;

        let project_path = repo_path.join(

            if let Some(p) = &config.project_subpath {
                log::warn!("project_subpath configuration parameter is deprecated. Use project.subpath instead")   ;
                p.clone()
            } else {
                config.project_config.subpath.as_ref().cloned().unwrap_or_else(||PathBuf::from("."))
            });

        let type_ = if project_path.join("Cargo.toml").exists() {
            Some(ProjectType::Rust)
        } else {
            None
        };
        let repo = Repository::on_path(repo_path)?;
        let returned = Self {
            repo_path: repo_path.canonicalize()?,
            project_path: project_path.canonicalize()?,
            config,
            repo,
            type_,
        };
        returned.check_configuration()?;
        Ok(returned)
    }

    pub fn check_configuration(&self) -> Result<()> {
        self.repo.find_branch(&self.config.develop_branch_name)?;
        self.ensure_main_branch().map(drop)
    }

    fn ensure_main_branch(&self) -> Result<()> {
        match self.repo.find_branch(&self.config.main_branch_name) {
            Ok(_) => Ok(()),
            Err(e) => {
                if let Some(e) = e.downcast_ref::<git2::Error>() {
                    if e.code() == ErrorCode::NotFound
                        && can_ask_questions()
                        && Confirm::with_theme(get_color_theme().as_ref())
                            .with_prompt(format!(
                                "{} branch not found. Create it?",
                                self.config().main_branch_name
                            ))
                            .interact()?
                    {
                        self.repo.path().shell(format!(
                            "git branch {0} origin/{0}",
                            self.config().main_branch_name
                        ))?;
                        return Ok(());
                    }
                }
                Err(e)
            }
        }
    }

    // High-level API

    pub fn bump_version(&self, bump_kind: BumpKind) -> Result<()> {
        debug!("Bumping version: {:?}", bump_kind);

        let files_to_bump = self.get_version_files_to_bump()?;

        if files_to_bump.is_empty() {
            bail!("Could not find version files to bump");
        }

        for bumped_file in files_to_bump {
            debug!("Bumping version file {bumped_file:?}...");
            bumped_file.bump(VersionSpec::Bump(bump_kind))?;
        }

        info!("Compiling project to lock version");
        self.compile()
    }

    pub fn pargit_cleanup(&self) -> Result<()> {
        self.repo.cleanup(&self.config.develop_branch_name)
    }

    pub fn pargit_delete(&self, kind: ObjectKind, name: Option<String>) -> Result<()> {
        let release_name = self.resolve_name(kind, name)?;
        let branch_name = self.prefix(kind, &release_name);
        info!("Deleting {}...", branch_name);

        let mut branch = self.repo.find_branch(&branch_name)?;
        if let Ok(upstream) = branch.upstream() {
            let (origin_name, remote_branch_name) =
                upstream.name()?.unwrap().split_once('/').unwrap();
            info!("Deleting remote branch {}", remote_branch_name);
            self.repo_path
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
        kind: ObjectKind,
        name: Option<String>,
        dest_branch: &str,
    ) -> Result<()> {
        let name = self.resolve_name(kind, name)?;
        debug!("Switching to branch {}", dest_branch);
        self.repo.switch_to_branch_name(dest_branch)?;
        let branch_name = self.prefix(kind, &name);
        debug!("Merging {}", branch_name);
        self.repo
            .merge_branch_name(&branch_name, &format!("Merge {}", branch_name))?;
        self.pargit_delete(kind, Some(name))
    }

    pub fn pargit_publish(&self, kind: ObjectKind, name: Option<String>) -> Result<()> {
        let name = self.resolve_name(kind, name)?;
        let branch_name = self.prefix(kind, &name);
        info!("Pushing {} to origin...", branch_name);
        let output = self
            .repo_path
            .shell_output(format!("git push -u origin {0}:{0}", branch_name))?;
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            if line.starts_with("remote:") {
                info!("{}", line);
            }
        }
        Ok(())
    }

    pub fn pargit_start(&self, kind: ObjectKind, name: &str, from_ref: Option<&str>) -> Result<()> {
        info!("Creating {} branch {}", kind, name);
        let branch_name = self.prefix(kind, name);
        if self.repo.find_branch(branch_name).is_ok() {
            bail!("{} {} already in progress. Finish it first", kind, name);
        }
        let b = self.repo.create_branch(
            self.prefix(kind, name),
            Some(kind.get_start_point(self, from_ref)?),
        )?;

        self.repo.switch_to_branch(&b)
    }

    pub fn release_version(
        &self,
        bump_kind: BumpKind,
        release_kind: ObjectKind,
        options: ReleaseOptions,
    ) -> Result<()> {
        let mut history = ExitStack::default();
        let release = self.release_start(VersionSpec::Bump(bump_kind), release_kind, None)?;
        let release_name = release.name.clone();
        let release_name_clone = release.name.clone();
        history.remember(format!("Delete {} branch", release_kind), move || {
            self.pargit_delete(release_kind, Some(release_name_clone))
                .ignore_errors()
        });
        if self.repo.is_dirty()? {
            self.repo.commit_all("pargit: Bump version")?;
        }
        self.release_finish(
            Some(release_name),
            Some(&release.tag),
            release_kind,
            options,
        )?;
        history.forget();
        Ok(())
    }

    pub fn release_start(
        &self,
        spec: VersionSpec,
        kind: ObjectKind,
        from_ref: Option<&str>,
    ) -> Result<Release> {
        let release = self.resolve_release(spec)?;
        let mut undo = ExitStack::default();

        if self.repo.has_tag(&release.tag)? {
            bail!("Tag {} already exists", release.tag);
        }
        self.pargit_start(kind, &release.name, from_ref)?;
        undo.remember("Deleting release branch", || {
            self.pargit_delete(kind, None).ignore_errors()
        });
        if let Some(version_files) = release.version_files.as_ref() {
            for file in version_files {
                file.bump(VersionSpec::Exact(release.version.clone()))?;
            }
            info!("Compiling project to lock new version");
            self.compile()?;
        }
        undo.forget();
        Ok(release)
    }

    pub fn release_finish(
        &self,
        release_name: Option<String>,
        tag: Option<&str>,
        release_kind: ObjectKind,
        options: ReleaseOptions,
    ) -> Result<()> {
        let release_name = self.resolve_name(release_kind, release_name)?;
        let release_branch_name = self.prefix(release_kind, &release_name);
        info!("Finishing {} {}", release_kind, release_name);
        self.repo.switch_to_branch_name(&release_branch_name)?;
        self.check_pre_release(&options)?;

        let temp_branch_name = format!("in-progress-{}-{}", release_kind, release_name);

        self.repo
            .create_branch(&temp_branch_name, Some(&self.config.main_branch_name))?;
        info!("Switching to temporary branch");
        self.repo.switch_to_branch_name(&temp_branch_name)?;
        info!("Merging {} branch", release_kind);
        self.repo
            .merge_branch_name(
                &release_branch_name,
                &format!("Merge {} branch {}", release_kind, release_name),
            )
            .context("Failed merge")?;
        info!("Creating tag and pushing to remote main branch");
        let tag = tag
            .map(String::from)
            .unwrap_or_else(|| self.config.get_tag_name(&release_name, None));
        let res = self
            .repo
            .create_tag(&tag)
            .and_then(|_| {
                self.repo_path.shell(format!(
                    "git push origin {}:{}",
                    temp_branch_name,
                    self.config().main_branch_name
                ))
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

            bail!("Failed pushing new {} - {:?}", release_kind, e);
        }

        info!("Push successful. Merging to local master");
        self.repo
            .switch_to_branch_name(&self.config.main_branch_name)?;
        self.repo
            .merge_branch_name(&temp_branch_name, "Merge temporary release branch")?;
        info!("Pushing tags");
        self.repo_path.shell("git push --tags")?;
        self.repo
            .switch_to_branch_name(&self.config.develop_branch_name)?;
        info!("Merging to develop branch");
        self.repo.merge_branch_name(
            &self.config.main_branch_name,
            &format!("Merge {} branch", self.config.main_branch_name),
        )?;

        self.repo
            .find_branch(temp_branch_name)?
            .delete()
            .context("Failed deleting temporary branch")?;
        self.pargit_delete(release_kind, Some(release_name))?;
        info!("Pushing develop branch");
        self.repo_path.shell(format!(
            "git push origin {0}:{0}",
            self.config.develop_branch_name
        ))
    }

    fn resolve_name(&self, kind: ObjectKind, name: Option<impl Into<String>>) -> Result<String> {
        Ok(match name {
            Some(name) => name.into(),
            None => self
                .current_name(kind)
                .context("Cannot get release name from branch name")?,
        })
    }

    fn prefix(&self, kind: ObjectKind, s: &str) -> String {
        format!("{}/{}", kind, s)
    }

    fn current_name(&self, kind: ObjectKind) -> Result<String> {
        self.repo
            .current_branch_name()?
            .strip_prefix(&format!("{}/", kind))
            .ok_or_else(|| format_err!("Could not get current {} name", kind))
            .map(|s| s.to_owned())
    }

    fn compile(&self) -> Result<()> {
        if let Some(type_) = &self.type_ {
            match type_ {
                ProjectType::Rust => {
                    let compilation_command = self
                        .config
                        .project_config
                        .compilation_command
                        .as_deref()
                        .unwrap_or("cargo check --workspace --tests");
                    // info!("Compiling project (cargo check)...");
                    self.project_path
                        .shell(compilation_command)
                        .context("Failed building project")
                }
            }
        } else {
            Ok(())
        }
    }

    fn resolve_release(&self, version_spec: VersionSpec) -> Result<Release> {
        let version_files = self.get_version_files_to_bump()?;
        let (new_version, prefix) = match version_spec {
            VersionSpec::Exact(version) => (version, None),
            VersionSpec::Bump(bump_kind) => {
                let (current_version, prefix) = if version_files.is_empty() {
                    self.try_get_latest_tagged_version()?.map(|(v, p)| (v, Some(p))).ok_or_else(|| anyhow::format_err!("Could not deduce current version and no existing versioned files found"))?
                } else {
                    (version_files[0].version(), None)
                };

                (next_version(&current_version, bump_kind), prefix)
            }
        };

        Ok(Release::version(
            &self.config,
            new_version,
            Some(version_files),
            prefix,
        ))
    }

    fn try_get_latest_tagged_version(&self) -> Result<Option<(Version, String)>> {
        let tags = self.repo.tags()?;

        let mut versions = Vec::new();

        for tag in tags {
            for prefix in &["v", ""] {
                if let Some(v) = tag.strip_prefix(prefix) {
                    if let Ok(v) = Version::parse(v) {
                        versions.push((v, (*prefix).to_owned()));
                        break;
                    }
                }
            }
        }

        versions.sort_by_key(|(version, _)| version.clone());

        Ok(versions.into_iter().last())
    }

    fn get_version_files_to_bump(&self) -> Result<Vec<VersionFile>> {
        let version_files = self.get_all_version_files()?;

        // if we have a single version - we should bump them all
        if version_files
            .iter()
            .map(|f| f.version())
            .collect::<HashSet<_>>()
            .len()
            == 1
        {
            return Ok(version_files);
        }

        let index = if version_files.len() > 1 {
            let selections = version_files
                .iter()
                .map(|version_file| {
                    let relpath =
                        pathdiff::diff_paths(version_file.path(), &self.project_path).unwrap();
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

        Ok(version_files.into_iter().nth(index).into_iter().collect())
    }

    fn get_all_version_files(&self) -> Result<Vec<VersionFile>> {
        self.type_
            .map(|type_| match type_ {
                ProjectType::Rust => crate::project_types::rust::find_cargo_tomls(&self.repo),
            })
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    // Checks
    fn check_pre_release(&self, options: &ReleaseOptions) -> Result<()> {
        info!("Running pre-release checks...");

        self.compile()?;
        if self.repo.is_dirty()? {
            bail!("Repository became dirty after build attempt. Perhaps Cargo.lock was not a part of the last commit?");
        }

        for branch_name in &[
            &self.config.develop_branch_name,
            &self.config.main_branch_name,
        ] {
            if !self.repo.is_branch_up_to_date(branch_name)? {
                if !options.no_pull {
                    warn!("Local branch {0} is behind remote. Attempting to pull recent changes (ff-only)...", branch_name);
                    self.repo.pull_branch_from_remote(branch_name, true)?;
                    assert!(self.repo.is_branch_up_to_date(branch_name)?);
                } else {
                    bail!("Local {0} branch is behind remote {0} branch. Update your local {0} branch before creating a release.", branch_name);
                }
            }
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

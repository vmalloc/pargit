use crate::utils::ObjectKind;
use crate::utils::PathExt;

use anyhow::{bail, format_err, Context, Result};
use git2::{Branch, BranchType, Oid, StatusOptions};
use log::info;
use std::path::Path;
use std::path::PathBuf;
use strum::IntoEnumIterator;

pub struct Repository {
    repo: git2::Repository,
    path: PathBuf,
}

impl Repository {
    pub fn on_path(p: impl AsRef<Path>) -> Result<Self> {
        let p = p.as_ref();
        let path = p.canonicalize().context("Cannot canonicalize path")?;

        log::debug!("Opening repository on path {path:?}");

        let repo = git2::Repository::open(&path).context("Failed opening repository")?;
        log::debug!("Repository opened. Reported path is {:?}", repo.path());

        let returned = Self { repo, path };

        if returned.is_dirty()? {
            bail!("Repository is dirty!");
        }

        Ok(returned)
    }

    pub fn is_path_ignored(&self, path: &Path) -> Result<bool> {
        Ok(self.repo.is_path_ignored(path)?)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_dirty(&self) -> Result<bool> {
        Ok(!self
            .repo
            .statuses(Some(StatusOptions::new().include_ignored(false)))?
            .is_empty())
    }

    pub fn commit_all(&self, message: &str) -> Result<()> {
        self.path().shell(format!("git commit -a -m {:?}", message))
    }

    pub fn merge_branch_name(&self, branch_name: &str, message: &str) -> Result<()> {
        self.path()
            .shell(format!("git merge {} -m {:?}", branch_name, message))
    }

    pub fn delete_branch_name(&self, branch_name: &str) -> Result<()> {
        Ok(self.find_branch(branch_name)?.delete()?)
    }

    pub fn delete_tag(&self, tag_name: &str) -> Result<()> {
        self.path().shell(format!("git tag -d {}", tag_name))
    }

    pub fn create_tag(&self, tag_name: &str) -> Result<()> {
        self.path()
            .shell(format!("git tag -a -m {0} {0}", tag_name))
    }

    pub fn tags(&self) -> Result<Vec<String>> {
        Ok(self
            .repo
            .tag_names(None)?
            .into_iter()
            .filter_map(|s| s.map(String::from))
            .collect())
    }

    fn is_merged(&self, commit: Oid, branch: Oid) -> Result<bool> {
        Ok(self.repo.merge_base(commit, branch)? == commit)
    }

    pub fn is_branch_up_to_date(&self, branch_name: &str) -> Result<bool> {
        self.git_fetch("origin")?;
        let branch = self.find_branch(branch_name)?;
        let remote = branch.upstream()?.into_reference().peel_to_commit()?.id();
        self.is_merged(remote, branch.get().peel_to_commit()?.id())
    }

    pub fn pull_branch_from_remote(&self, branch_name: &str, ff_only: bool) -> Result<()> {
        let prev_branch = self.current_branch_name()?;
        let prev_branch = self.find_branch(&prev_branch)?;
        let branch = self.find_branch(branch_name)?;
        self.switch_to_branch(&branch)?;
        let flags = if ff_only { "--ff-only" } else { "" };
        self.path().shell(format!("git pull {flags}"))?;
        self.switch_to_branch(&prev_branch)?;
        Ok(())
    }

    pub fn cleanup(&self, develop_branch_name: &str) -> Result<()> {
        self.git_fetch("origin")?;
        let develop_branch = self.find_branch(develop_branch_name)?;
        let remote_develop = develop_branch
            .upstream()?
            .into_reference()
            .peel_to_commit()?
            .id();
        let develop = develop_branch.get().peel_to_commit()?.id();
        let current_branch_name = self.current_branch_name()?;

        // we only cleanup if remote_develop is ahead of develop and  contains it
        if develop != remote_develop && self.is_merged(develop, remote_develop)? {
            info!("Remote develop branch is ahead of local develop branch");
            for branch in self.repo.branches(Some(BranchType::Local))? {
                let (mut branch, _) = branch?;
                let name = branch.name()?.unwrap();
                if ObjectKind::iter().any(|kind| name.starts_with(format!("{}/", kind).as_str())) {
                    let branch_commit = branch.get().peel_to_commit()?.id();
                    if self.is_merged(branch_commit, remote_develop)?
                        && !self.is_merged(branch_commit, develop)?
                    {
                        info!("Branch {} is not merged into local develop, but is merged to remote develop. Deleting...", name);
                        if current_branch_name == name {
                            info!(
                                "Branch {} is the current branch, switching to {}...",
                                current_branch_name,
                                develop_branch.name()?.unwrap()
                            );
                            self.switch_to_branch(&develop_branch)?;
                        }
                        branch.delete()?;
                    }
                }
            }
        } else {
            info!("Remote develop branch is not ahead of current branch. Not doing anything");
        }
        Ok(())
    }

    fn git_fetch(&self, remote_name: &str) -> Result<()> {
        info!("Fetching remote {:?}...", remote_name);
        self.path().shell(format!("git fetch {}", remote_name))
    }

    pub fn current_branch_name(&self) -> Result<String> {
        self.repo
            .head()?
            .name()
            .and_then(|n| n.strip_prefix("refs/heads/"))
            .map(|s| s.to_owned())
            .ok_or_else(|| format_err!("Could not get current branch name"))
    }

    pub fn has_tag(&self, tag_name: &str) -> Result<bool> {
        Ok(self
            .repo
            .tag_names(None)?
            .into_iter()
            .any(|tag| tag == Some(tag_name)))
    }

    pub fn switch_to_branch_name(&self, branch_name: &str) -> Result<()> {
        self.switch_to_branch(&self.find_branch(branch_name)?)
            .with_context(|| format!("Unable to switch to branch {}", branch_name))
    }

    pub fn switch_to_branch(&self, branch: &Branch) -> Result<()> {
        self.repo
            .checkout_tree(&branch.get().peel_to_tree()?.into_object(), None)?;

        self.repo
            .set_head(&format!("refs/heads/{}", branch.name()?.unwrap()))?;

        Ok(())
    }

    pub fn create_branch(
        &self,
        branch_name: impl AsRef<str>,
        start_point: Option<impl AsRef<str>>,
    ) -> Result<Branch> {
        let start_point = match start_point {
            Some(start_point) => self
                .repo
                .revparse_single(start_point.as_ref())?
                .as_commit()
                .unwrap()
                .clone(),
            None => self.repo.head()?.peel_to_commit()?,
        };
        Ok(self
            .repo
            .branch(branch_name.as_ref(), &start_point, false)?)
    }

    pub fn find_branch(&self, name: impl AsRef<str>) -> Result<Branch> {
        Ok(self.repo.find_branch(name.as_ref(), BranchType::Local)?)
    }
}

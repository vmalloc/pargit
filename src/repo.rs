use anyhow::{bail, format_err, Context, Result};
use git2::{Branch, BranchType, Oid, StatusOptions};
use log::info;
use std::path::Path;

use crate::utils::PathExt;
pub struct Repository {
    repo: git2::Repository,
}

impl Repository {
    pub fn on_path(p: impl AsRef<Path>) -> Result<Self> {
        let repo = git2::Repository::open(p.as_ref())?;

        let returned = Self { repo };

        if returned.is_dirty()? {
            bail!("Repository is dirty!");
        }

        returned.check_configuration()?;

        Ok(returned)
    }

    fn check_configuration(&self) -> Result<()> {
        self.find_develop_branch()
            .context("Cannot find develop branch")?;
        self.find_master_branch()
            .context("Cannot find master branch")
            .map(drop)
    }

    pub fn is_path_ignored(&self, path: &Path) -> Result<bool> {
        Ok(self.repo.is_path_ignored(path)?)
    }

    pub fn path(&self) -> &Path {
        let returned = self.repo.path();
        assert_eq!(returned.file_name().unwrap(), ".git");
        returned.parent().unwrap()
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

    fn is_merged(&self, commit: Oid, branch: Oid) -> Result<bool> {
        Ok(self.repo.merge_base(commit, branch)? == commit)
    }

    pub fn cleanup(&self) -> Result<()> {
        self.git_fetch("origin")?;
        let develop_branch = self.find_develop_branch()?;
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
                if name.starts_with("feature/")
                    || name.starts_with("bugfix/")
                    || name.starts_with("release/")
                {
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

    fn find_develop_branch(&self) -> Result<Branch> {
        self.find_branch("develop")
    }

    fn find_master_branch(&self) -> Result<Branch> {
        self.find_branch("master")
            .or_else(|_| self.find_branch("main"))
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
            Some(start_point) => self.find_branch(start_point)?.get().peel_to_commit()?,
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

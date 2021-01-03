use std::{io::Read, path::Path, process::Stdio};

use anyhow::{bail, format_err, Context, Result};
use git2::{Branch, BranchType, StatusOptions};
use log::{error, info};

pub struct Repository {
    repo: git2::Repository,
}

impl Repository {
    pub fn on_path(p: impl AsRef<str>) -> Result<Self> {
        let repo = git2::Repository::open(p.as_ref())?;

        let returned = Self { repo };

        if returned.is_dirty()? {
            bail!("Repository is dirty!");
        }
        Ok(returned)
    }

    pub fn path(&self) -> &Path {
        let returned = self.repo.path();
        assert_eq!(returned.file_name().unwrap(), ".git");
        returned.parent().unwrap()
    }

    fn is_dirty(&self) -> Result<bool> {
        Ok(!self
            .repo
            .statuses(Some(StatusOptions::new().include_ignored(false)))?
            .is_empty())
    }

    pub fn release_start(&self, release_name: &str) -> Result<()> {
        if self.has_tag(release_name)? {
            bail!("Release {} already exists", release_name);
        }
        info!("Creating release branch {}", release_name);
        for branch in self.repo.branches(None)? {
            let branch = branch?.0;
            if let Some(release_name) = branch.name()?.and_then(|s| s.strip_prefix("release/")) {
                bail!(
                    "Release {} already in progress. Finish it first",
                    release_name
                );
            }
        }
        let b = self.create_branch(self.prefix_release(release_name), Some("develop"))?;

        self.switch_to_branch(&b)
    }

    pub fn release_delete(&self, release_name: Option<String>) -> Result<()> {
        let release_name = self.resolve_release_name(release_name)?;
        let branch_name = self.prefix_release(&release_name);

        let mut branch = self.find_branch(&branch_name)?;
        if let Ok(upstream) = branch.upstream() {
            info!("Deleting remote branch");
            let mut parts = upstream.name()?.unwrap().splitn(2, '/');
            let origin_name = parts.next().unwrap();
            let remote_branch_name = parts.next().unwrap();
            self.shell(format!("git push {} :{}", origin_name, remote_branch_name))?;
        }

        if branch.name()?.unwrap() == self.current_branch_name()? {
            self.switch_to_branch_name("develop")
                .context("Cannot switch to develop branch")?;
        }
        info!("Deleting branch {:?}", branch_name);
        branch.delete()?;

        Ok(())
    }

    pub fn release_finish(&self, release_name: Option<String>) -> Result<()> {
        let release_name = self.resolve_release_name(release_name)?;
        let release_branch_name = self.prefix_release(&release_name);
        info!("Finishing release {}", release_name);
        self.switch_to_branch_name(&release_branch_name)?;
        self.check_pre_release()?;

        let temp_branch_name = format!("in-progress-release-{}", release_name);

        let mut temp_branch = self.create_branch(&temp_branch_name, Some("master"))?;
        info!("Switching to temporary branch");
        self.switch_to_branch_name(&temp_branch_name)?;
        info!("Merging release branch");
        self.merge_branch_name(
            &release_branch_name,
            &format!("Merge release branch {}", release_name),
        )
        .context("Failed merge")?;
        info!("Creating tag and pushing to remote master");
        let res = self
            .create_tag(&release_name)
            .and_then(|_| self.shell(format!("git push origin {}:master", temp_branch_name)))
            .context("Failed tag and push");

        // we try to push the merged master first. If it succeeds, it means we won the release
        if let Err(e) = res {
            error!("Failed pushing to master. Rolling back changes...");
            let _ = self
                .delete_tag(&release_name)
                .map_err(|e| error!("Failed deleting tag: {:?}", e));
            let _ = temp_branch
                .delete()
                .map_err(|e| error!("Failed deleting temporary branch: {:?}", e));

            let _ = self.switch_to_branch_name(&release_branch_name);
            bail!("Failed pushing new release - {:?}", e);
        }

        info!("Push successful. Merging to local master");
        self.switch_to_branch_name("master")?;
        self.merge_branch_name(&temp_branch_name, "Merge temporary release branch")?;
        info!("Pushing tags");
        self.shell("git push --tags")?;
        self.switch_to_branch_name("develop")?;
        info!("Merging to develop branch");
        self.merge_branch_name("master", "Merge master branch")?;

        self.find_branch(temp_branch_name)?
            .delete()
            .context("Failed deleting temporary branch")?;
        self.release_delete(Some(release_name))?;
        info!("Pushing develop branch");
        self.shell("git push origin develop:develop")
    }

    pub fn commit_all(&self, message: &str) -> Result<()> {
        self.shell(format!("git commit -a -m {:?}", message))
    }

    fn merge_branch_name(&self, branch_name: &str, message: &str) -> Result<()> {
        self.shell(format!("git merge {} -m {:?}", branch_name, message))
    }

    fn delete_tag(&self, tag_name: &str) -> Result<()> {
        self.shell(format!("git tag -d {}", tag_name))
    }

    fn create_tag(&self, tag_name: &str) -> Result<()> {
        self.shell(format!("git tag -a -m {0} {0}", tag_name))
    }

    fn check_pre_release(&self) -> Result<()> {
        info!("Running pre-release checks...");
        self.cargo_check()?;
        if self.is_dirty()? {
            bail!("Repository became dirty after build attempt. Perhaps Cargo.lock was not a part of the last commit?");
        }

        Ok(())
    }

    pub fn cargo_check(&self) -> Result<()> {
        info!("Compiling project to ensure consistency (cargo check)...");
        self.shell("cargo check --workspace --tests")
            .context("Failed building project")
    }

    pub fn release_publish(&self, release_name: Option<String>) -> Result<()> {
        let release_name = self.resolve_release_name(release_name)?;
        self.shell(format!(
            "git push -u origin {0}:{0}",
            self.prefix_release(&release_name)
        ))
    }

    fn resolve_release_name(&self, release_name: Option<String>) -> Result<String> {
        Ok(match release_name {
            Some(name) => name,
            None => self
                .current_release_name()
                .context("Cannot get release name from branch name")?,
        })
    }

    fn prefix_release(&self, s: &str) -> String {
        format!("release/{}", s)
    }

    fn current_release_name(&self) -> Result<String> {
        self.current_branch_name()?
            .strip_prefix("release/")
            .ok_or_else(|| format_err!("Could not get current release name"))
            .map(|s| s.to_owned())
    }

    fn current_branch_name(&self) -> Result<String> {
        self.repo
            .head()?
            .name()
            .and_then(|n| n.strip_prefix("refs/heads/"))
            .map(|s| s.to_owned())
            .ok_or_else(|| format_err!("Could not get current branch name"))
    }

    fn has_tag(&self, tag_name: &str) -> Result<bool> {
        Ok(self
            .repo
            .tag_names(None)?
            .into_iter()
            .any(|tag| tag == Some(tag_name)))
    }

    pub fn switch_to_branch_name(&self, branch_name: &str) -> Result<()> {
        self.switch_to_branch(&self.find_branch(branch_name)?)
    }

    fn switch_to_branch(&self, branch: &Branch) -> Result<()> {
        self.repo
            .checkout_tree(&branch.get().peel_to_tree()?.into_object(), None)?;

        self.repo
            .set_head(&format!("refs/heads/{}", branch.name()?.unwrap()))?;

        Ok(())
    }

    fn create_branch(
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

    fn find_branch(&self, name: impl AsRef<str>) -> Result<Branch> {
        Ok(self.repo.find_branch(name.as_ref(), BranchType::Local)?)
    }

    fn shell(&self, cmd: impl AsRef<str>) -> Result<()> {
        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd.as_ref())
            .current_dir(self.path())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        if !child.wait()?.success() {
            let mut buf = Vec::new();
            child.stderr.unwrap().read_to_end(&mut buf)?;
            bail!("Git failed: {:?}", String::from_utf8_lossy(&buf));
        }
        Ok(())
    }
}

use anyhow::{bail, format_err, Context, Result};
use git2::{Branch, BranchType, Oid, StatusOptions};
use log::{debug, error, info};
use std::{
    path::Path,
    process::{Output, Stdio},
};
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
        self.pargit_start("release", release_name, "develop")
    }
    pub fn pargit_start(&self, branch_type: &str, name: &str, start_point: &str) -> Result<()> {
        info!("Creating {} branch {}", branch_type, name);
        let branch_name = self.prefix(branch_type, name);
        if self.find_branch(branch_name).is_ok() {
            bail!(
                "{} {} already in progress. Finish it first",
                branch_type,
                name
            );
        }
        let b = self.create_branch(self.prefix(branch_type, name), Some(start_point))?;

        self.switch_to_branch(&b)
    }

    pub fn release_delete(&self, release_name: Option<String>) -> Result<()> {
        self.pargit_delete("release", release_name)
    }

    pub fn pargit_delete(&self, object_type: &str, name: Option<String>) -> Result<()> {
        let release_name = self.resolve_name(object_type, name)?;
        let branch_name = self.prefix(object_type, &release_name);
        info!("Deleting {}...", branch_name);

        let mut branch = self.find_branch(&branch_name)?;
        if let Ok(upstream) = branch.upstream() {
            let mut parts = upstream.name()?.unwrap().splitn(2, '/');
            let origin_name = parts.next().unwrap();
            let remote_branch_name = parts.next().unwrap();
            info!("Deleting remote branch {}", remote_branch_name);
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

        self.create_branch(&temp_branch_name, Some("master"))?;
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
            let _ = self.switch_to_branch_name(&release_branch_name);

            let _ = self
                .delete_branch_name(&temp_branch_name)
                .map_err(|e| error!("Failed deleting temporary branch: {:?}", e));

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

    fn delete_branch_name(&self, branch_name: &str) -> Result<()> {
        Ok(self.find_branch(branch_name)?.delete()?)
    }

    fn delete_tag(&self, tag_name: &str) -> Result<()> {
        self.shell(format!("git tag -d {}", tag_name))
    }

    fn create_tag(&self, tag_name: &str) -> Result<()> {
        self.shell(format!("git tag -a -m {0} {0}", tag_name))
    }

    fn check_pre_release(&self) -> Result<()> {
        info!("Running pre-release checks...");

        self.compile_project()?;
        if self.is_dirty()? {
            bail!("Repository became dirty after build attempt. Perhaps Cargo.lock was not a part of the last commit?");
        }
        Ok(())
    }

    pub fn compile_project(&self) -> Result<()> {
        if let Some(project_type) = self.get_project_type()? {
            project_type.compile(self)?;
        }

        Ok(())
    }

    fn get_project_type(&self) -> Result<Option<ProjectType>> {
        if self.path().join("Cargo.toml").exists() {
            Ok(Some(ProjectType::Rust))
        } else {
            Ok(None)
        }
    }

    pub fn pargit_publish(&self, object_type: &str, name: Option<String>) -> Result<()> {
        let name = self.resolve_name(object_type, name)?;
        let branch_name = self.prefix(object_type, &name);
        info!("Pushing {} to origin...", branch_name);
        let output = self.shell_output(format!("git push -u origin {0}:{0}", branch_name))?;
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            if line.starts_with("remote:") {
                info!("{}", line);
            }
        }
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
        self.switch_to_branch_name(dest_branch)?;
        let branch_name = self.prefix(object_type, &name);
        debug!("Merging {}", branch_name);
        self.merge_branch_name(&branch_name, &format!("Merge {}", branch_name))
    }

    pub fn pargit_cleanup(&self) -> Result<()> {
        self.git_fetch("origin")?;
        let develop = self.find_develop_branch()?;
        let remote_develop = develop.upstream()?.into_reference().peel_to_commit()?.id();
        let develop = develop.get().peel_to_commit()?.id();

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
                        branch.delete()?;
                    }
                }
            }
        } else {
            info!("Remote develop branch is not ahead of current branch. Not doing anything");
        }
        Ok(())
    }

    fn is_merged(&self, commit: Oid, branch: Oid) -> Result<bool> {
        Ok(self.repo.merge_base(commit, branch)? == commit)
    }

    fn find_develop_branch(&self) -> Result<Branch> {
        self.find_branch("develop")
    }

    fn find_master_branch(&self) -> Result<Branch> {
        self.find_branch("master")
    }

    fn git_fetch(&self, remote_name: &str) -> Result<()> {
        self.shell(format!("git fetch {}", remote_name))
    }

    fn resolve_release_name(&self, release_name: Option<String>) -> Result<String> {
        self.resolve_name("release", release_name)
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
        self.current_branch_name()?
            .strip_prefix(&format!("{}/", object_type))
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
        self.shell_output(cmd).map(drop)
    }

    fn shell_output(&self, cmd: impl AsRef<str>) -> Result<Output> {
        let child = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd.as_ref())
            .current_dir(self.path())
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

enum ProjectType {
    Rust,
}

impl ProjectType {
    fn compile(&self, repo: &Repository) -> Result<()> {
        match self {
            ProjectType::Rust => {
                info!("Compiling project (cargo check)...");
                repo.shell("cargo check --workspace --tests")
                    .context("Failed building project")
            }
        }
    }
}

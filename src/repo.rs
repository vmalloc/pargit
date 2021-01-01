use anyhow::{bail, Result};
use git2::{Branch, BranchType};

pub struct Repository {
    repo: git2::Repository,
}

impl Repository {
    pub fn on_path(p: impl AsRef<str>) -> Result<Self> {
        let repo = git2::Repository::open(p.as_ref())?;
        if !repo.statuses(None)?.is_empty() {
            bail!("Repository is dirty!");
        }
        Ok(Self { repo })
    }

    pub fn push_to_remote(&self, branch_name: &str) -> Result<()> {
        let branch = self.repo.find_branch(branch_name, BranchType::Local)?;
        let mut remote = self.repo.find_remote("origin")?;
        remote.push(&[format!("{0}:{0}", branch_name)], None)?;
        Ok(())
    }

    pub fn release_start(&self, release_name: &str) -> Result<()> {
        if self.has_tag(release_name)? {
            bail!("Release {} already exists", release_name);
        }
        for branch in self.repo.branches(None)? {
            let branch = branch?.0;
            if let Some(release_name) = branch.name()?.and_then(|s| s.strip_prefix("release/")) {
                bail!(
                    "Release {} already in progress. Finish it first",
                    release_name
                );
            }
        }
        let b = self.create_branch(format!("release/{}", release_name), Some("develop"))?;

        self.switch_to_branch(&b)
    }

    fn has_tag(&self, tag_name: &str) -> Result<bool> {
        Ok(self
            .repo
            .tag_names(None)?
            .into_iter()
            .any(|tag| tag == Some(tag_name)))
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
        for branch in self.repo.branches(None)? {
            println!("* {:?}", branch?.0.name()?);
        }
        Ok(self.repo.find_branch(name.as_ref(), BranchType::Local)?)
    }
}

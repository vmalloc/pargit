use anyhow::Result;
use std::path::{Path, PathBuf};

const CONFIG_FILENAME: &str = ".pargit.toml";

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(default)]
    pub project_subpath: Option<PathBuf>,

    #[serde(default = "Default::default")]
    pub tag_prefix: String,

    #[serde(default = "default_main_branch", alias = "master_branch_name")]
    pub main_branch_name: String,

    #[serde(default = "default_develop_branch")]
    pub develop_branch_name: String,

    #[serde(default)]
    #[serde(rename = "project")]
    pub project_config: ProjectConfig,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct ProjectConfig {
    pub subpath: Option<PathBuf>,

    pub compilation_command: Option<String>,
}

fn default_main_branch() -> String {
    "master".into()
}
fn default_develop_branch() -> String {
    "develop".into()
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

impl Config {
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = project_root.join(CONFIG_FILENAME);

        if path.exists() {
            Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
        } else {
            Ok(Self::default())
        }
    }

    pub(crate) fn sample() -> &'static str {
        include_str!("../sample-config.toml")
    }

    pub fn get_tag_name(&self, version: &str, prefix: Option<String>) -> String {
        let prefix = prefix.as_deref().unwrap_or(&self.tag_prefix);
        format!("{}{}", prefix, version)
    }
}

#[cfg(test)]
mod tests {

    use super::Config;
    use itertools::Itertools;

    #[test]
    fn test_skeleton_config_valid() {
        let sample_config: &str = Config::sample();

        let lines = sample_config
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("# "))
            .join("\n");

        let _config: Config = toml::from_str(&lines).unwrap();
    }
}

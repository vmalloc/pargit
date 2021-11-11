use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use std::path::{Path, PathBuf};

const CONFIG_FILENAME: &str = ".pargit.toml";

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(default)]
    pub project_subpath: Option<PathBuf>,

    #[serde(default = "Default::default")]
    pub tag_prefix: String,

    #[serde(default = "default_master_branch")]
    pub master_branch_name: String,

    #[serde(default = "default_develop_branch")]
    pub develop_branch_name: String,
}

fn default_master_branch() -> String {
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

macro_rules! assign {
    ($field:expr, $msg:expr) => {
        let msg = $msg;
        $field = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(msg)
            .with_initial_text(&$field)
            .default($field.clone())
            .interact()?;
    };
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

    pub fn save(&self, project_root: &Path) -> Result<()> {
        std::fs::write(project_root.join(CONFIG_FILENAME), toml::to_string(self)?)?;
        Ok(())
    }

    pub fn reconfigure(&mut self) -> Result<()> {
        assign!(self.tag_prefix, "Tag prefix to use");
        assign!(
            self.master_branch_name,
            "Name of the branch to be used as master/main branch"
        );
        assign!(
            self.develop_branch_name,
            "Name of the branch to be used as development branch"
        );

        Ok(())
    }

    pub fn get_tag_name(&self, version: &str, prefix: Option<String>) -> String {
        let prefix = prefix.as_deref().unwrap_or(&self.tag_prefix);
        format!("{}{}", prefix, version)
    }
}

use anyhow::Result;
use std::path::Path;

#[derive(serde::Deserialize)]
pub struct Config {
    #[serde(default = "Default::default")]
    tag_prefix: String,
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

impl Config {
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = project_root.join(".pargit.toml");

        if path.exists() {
            Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn get_tag_name(&self, version: &str) -> String {
        format!("{}{}", self.tag_prefix, version)
    }
}

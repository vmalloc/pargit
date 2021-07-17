use semver::Version;

use crate::{config::Config, version_file::VersionFile};

pub struct Release {
    pub name: String,
    pub tag: String,
    pub version: Version,
    pub version_file: Option<VersionFile>,
}

impl Release {
    pub fn version(
        config: &Config,
        version: Version,
        version_file: Option<VersionFile>,
        prefix: Option<String>,
    ) -> Self {
        Self {
            name: version.to_string(),
            tag: config.get_tag_name(&version.to_string(), prefix),
            version,
            version_file,
        }
    }
}

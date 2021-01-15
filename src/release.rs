use semver::Version;

use crate::version_file::VersionFile;

pub struct Release {
    pub name: String,
    pub tag: String,
    pub version: Version,
    pub version_file: Option<VersionFile>,
}

impl Release {
    pub fn version(version: Version, version_file: Option<VersionFile>) -> Self {
        Self {
            name: version.to_string(),
            tag: version.to_string(),
            version,
            version_file,
        }
    }
}

use serde::Deserialize;

use crate::{v1::ProfileV1, Workspace, ProfileError};

/// Parse yaml profile.
pub fn parse_profile(input: &str) -> Result<Workspace, ProfileError> {
    let version = parse_version(input)?;
    match version {
        1 => {
            let profile: ProfileV1 = serde_yaml::from_str(input)?;
            let workspace = profile.to_workspace()?;
            Ok(workspace)
        }
        _ => Err(ProfileError::UnsupportedVersion(version)),
    }
}

/// A profile with a version.
#[derive(Debug, Clone, Deserialize)]
struct VersionedProfile {
    version: u8,
}

/// Parse the version of yaml profile.
fn parse_version(input: &str) -> Result<u8, ProfileError> {
    let raw: VersionedProfile = serde_yaml::from_str(input)?;
    Ok(raw.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_profile_yaml_error_when_version_missing() {
        let yaml = "controllers: []\n";
        assert!(matches!(
            parse_profile(yaml),
            Err(ProfileError::YamlDeserializeError(_))
        ));
    }
}

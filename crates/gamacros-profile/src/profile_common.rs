use serde::Deserialize;

use crate::{profile_v1::ProfileV1, profile::Profile, ProfileError};

/// Parse yaml profile.
pub fn parse_profile(input: &str) -> Result<Profile, ProfileError> {
    let version = parse_version(input)?;
    match version {
        1 => ProfileV1::to_settings(input),
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
    fn parse_version_valid() {
        let yaml = "version: 1\n";
        let version = parse_version(yaml).expect("version should parse");
        assert_eq!(version, 1);
    }

    #[test]
    fn parse_profile_version1_minimal() {
        let yaml = "version: 1\n";
        let profile = parse_profile(yaml).expect("profile v1 should parse");
        assert!(profile.controllers.is_empty());
        assert!(profile.rules.is_empty());
    }

    #[test]
    fn parse_profile_unsupported_version() {
        let yaml = "version: 2\n";
        match parse_profile(yaml) {
            Err(ProfileError::UnsupportedVersion(v)) => assert_eq!(v, 2),
            other => panic!("expected UnsupportedVersion, got {other:?}"),
        }
    }

    #[test]
    fn parse_profile_yaml_error_when_version_missing() {
        let yaml = "controllers: []\n";
        assert!(matches!(parse_profile(yaml), Err(ProfileError::Yaml(_))));
    }
}

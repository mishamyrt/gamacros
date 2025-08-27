use serde::Deserialize;

use crate::{profile_v1::ProfileV1, profile::Profile, ProfileError};

#[derive(Debug, Clone, Deserialize)]
pub struct VersionedProfile {
    pub version: u8,
}

pub fn parse_version(input: &str) -> Result<u8, ProfileError> {
    let raw: VersionedProfile = serde_yaml::from_str(input)?;
    Ok(raw.version)
}

pub fn parse_profile(input: &str) -> Result<Profile, ProfileError> {
    let version = parse_version(input)?;
    match version {
        1 => ProfileV1::to_settings(input),
        _ => Err(ProfileError::UnsupportedVersion(version)),
    }
}

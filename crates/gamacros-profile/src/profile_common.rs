use gamacros_gamepad::Button;
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

/// Parse a button name into a `Button` enum.
pub(crate) fn parse_button_name(name: &str) -> Result<Button, ProfileError> {
    Ok(match name {
        "a" => Button::A,
        "b" => Button::B,
        "x" => Button::X,
        "y" => Button::Y,

        "back" | "select" => Button::Back,
        "guide" | "home" => Button::Guide,
        "start" => Button::Start,

        "ls" | "left_stick" => Button::LeftStick,
        "rs" | "right_stick" => Button::RightStick,

        "lb" | "left_bumper" | "left_shoulder" | "l1" => Button::LeftShoulder,
        "rb" | "right_bumper" | "right_shoulder" | "r1" => Button::RightShoulder,
        "lt" | "left_trigger" | "l2" => Button::LeftTrigger,
        "rt" | "right_trigger" | "r2" => Button::RightTrigger,

        "dpad_up" => Button::DPadUp,
        "dpad_down" => Button::DPadDown,
        "dpad_left" => Button::DPadLeft,
        "dpad_right" => Button::DPadRight,

        _ => return Err(ProfileError::InvalidButton(name.to_string())),
    })
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
        assert!(matches!(parse_profile(yaml), Err(ProfileError::Yaml(_))));
    }
}

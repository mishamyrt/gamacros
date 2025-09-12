use std::path::PathBuf;

use crate::ProfileError;

const DEFAULT_PROFILE_APP_SUPPORT_PATH: &str =
    "Library/Application Support/gamacros/gc_profile.yaml";
const DEFAULT_HOME_PROFILE_PATH: &str = ".gc_profile.yaml";

/// Resolve a profile path to an absolute path.
///
/// If the path is absolute, it is returned as is.
/// If the path is relative, it is resolved relative to the current directory.
///
/// If the path does not exist, an error is returned.
pub fn resolve_profile(target_path: Option<&str>) -> Result<PathBuf, ProfileError> {
    let Some(target_path) = target_path else {
        let app_support_path =
            get_default_profile_path(DEFAULT_PROFILE_APP_SUPPORT_PATH)?;
        if app_support_path.exists() {
            return Ok(app_support_path);
        }

        let home_path = get_default_profile_path(DEFAULT_HOME_PROFILE_PATH)?;
        if home_path.exists() {
            return Ok(home_path);
        }

        return Err(ProfileError::ProfileNotFound(
            app_support_path.display().to_string(),
        ));
    };

    let path = PathBuf::from(target_path);
    if path.is_absolute() {
        if !path.exists() {
            return Err(ProfileError::ProfileNotFound(target_path.to_string()));
        }

        return Ok(path);
    }

    Ok(path.canonicalize()?)
}

fn get_default_profile_path(reference_path: &str) -> Result<PathBuf, ProfileError> {
    let path = std::env::var("HOME")
        .map(PathBuf::from)
        .map(|p| p.join(reference_path))
        .map_err(|_| ProfileError::EnvVarNotSet("HOME".to_string()))?;
    Ok(path)
}

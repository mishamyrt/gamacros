use std::path::PathBuf;

use crate::ProfileError;

const GAMACROS_PROFILE_NAME: &str = "gc_profile.yaml";

const DEFAULT_PROFILE_PATH: &str =
    "Library/Application Support/gamacros/gc_profile.yaml";

/// Resolve a profile path to an absolute path.
///
/// If the path is absolute, it is returned as is.
/// If the path is relative, it is resolved relative to the current directory.
///
/// If the path does not exist, an error is returned.
pub fn resolve_profile(target_path: Option<&str>) -> Result<PathBuf, ProfileError> {
    let Some(target_path) = target_path else {
        let local_path = get_local_profile_path(GAMACROS_PROFILE_NAME)?;
        if local_path.exists() {
            return Ok(local_path);
        }

        let default_path = get_default_profile_path()?;
        if default_path.exists() {
            return Ok(default_path);
        }

        return Err(ProfileError::ProfileNotFound(
            default_path.display().to_string(),
        ));
    };

    let path = PathBuf::from(target_path);
    if path.is_absolute() {
        if !path.exists() {
            return Err(ProfileError::ProfileNotFound(target_path.to_string()));
        }

        return Ok(path);
    }

    let local_path = get_local_profile_path(target_path)?;
    if !local_path.exists() {
        return Err(ProfileError::ProfileNotFound(
            local_path.display().to_string(),
        ));
    }

    Ok(local_path.canonicalize()?)
}

fn get_default_profile_path() -> Result<PathBuf, ProfileError> {
    let path = std::env::var("HOME")
        .map(PathBuf::from)
        .map(|p| p.join(DEFAULT_PROFILE_PATH))
        .map_err(|_| ProfileError::EnvVarNotSet("HOME".to_string()))?;
    Ok(path)
}

fn get_local_profile_path(profile_name: &str) -> Result<PathBuf, ProfileError> {
    let path = std::env::current_dir()
        .map_err(|_| ProfileError::CurrentDirNotSet)?
        .join(profile_name);
    Ok(path)
}

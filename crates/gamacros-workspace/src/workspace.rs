use std::path::{Path, PathBuf};

use crate::WorkspaceError;
use crate::{profile_watcher::ProfileEventReceiver, ProfileWatcher};

const DEFAULT_WORKSPACE_PATH: &str = "Library/Application Support/gamacros";
const PROFILE_FILE_NAME: &str = "gc_profile.yaml";

pub struct Workspace {
    path: PathBuf,
}

impl Workspace {
    pub fn new(path: Option<&Path>) -> Result<Self, WorkspaceError> {
        let path = {
            if let Some(path) = path {
                path.to_owned()
            } else {
                Self::default_path()?
            }
        };

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        } else if !path.is_dir() {
            return Err(WorkspaceError::PathIsNotDirectory(
                path.display().to_string(),
            ));
        }

        Ok(Self { path })
    }

    #[cfg(target_os = "macos")]
    pub fn start_profile_watcher(
        &self,
    ) -> Result<
        (ProfileWatcher<notify::FsEventWatcher>, ProfileEventReceiver),
        WorkspaceError,
    > {
        let profile_path = self.profile_path();

        ProfileWatcher::<notify::FsEventWatcher>::new_with_starting_event(
            &profile_path,
        )
        .map_err(WorkspaceError::WatcherError)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn start_profile_watcher(
        &self,
    ) -> Result<
        (ProfileWatcher<notify::PollWatcher>, ProfileEventReceiver),
        WorkspaceError,
    > {
        let profile_path = self.profile_path();

        ProfileWatcher::<notify::PollWatcher>::new_with_starting_event(&profile_path)
            .map_err(WorkspaceError::WatcherError)
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn profile_path(&self) -> PathBuf {
        self.path.join(PROFILE_FILE_NAME)
    }

    pub fn default_path() -> Result<PathBuf, WorkspaceError> {
        let path = std::env::var("HOME")
            .map(PathBuf::from)
            .map(|p| p.join(DEFAULT_WORKSPACE_PATH))
            .map_err(|_| WorkspaceError::EnvVarNotSet("HOME".to_string()))?;

        Ok(path)
    }
}

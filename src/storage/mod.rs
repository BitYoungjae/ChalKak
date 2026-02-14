use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::capture::CaptureArtifact;
use thiserror::Error;

const DEFAULT_TEMP_PREFIX: &str = "capture_";
const PREVIEW_SUBDIR: &str = "Pictures";
const DEFAULT_FALLBACK_TEMP_DIR: &str = "/tmp/chalkak";

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("missing HOME environment variable")]
    MissingHomeDirectory,
    #[error("capture id is empty")]
    MissingCaptureId,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Default, Clone)]
pub struct PruneReport {
    pub removed_files: usize,
}

pub trait CaptureStorage {
    fn save_capture(&self, artifact: &CaptureArtifact) -> StorageResult<PathBuf>;
    fn discard_session_artifacts(&self, capture_id: &str) -> StorageResult<()>;
}

#[derive(Debug, Clone)]
pub struct StorageService {
    temp_dir: PathBuf,
    pictures_dir: PathBuf,
}

impl StorageService {
    pub const fn with_paths(temp_dir: PathBuf, pictures_dir: PathBuf) -> Self {
        Self {
            temp_dir,
            pictures_dir,
        }
    }

    pub fn with_default_paths() -> StorageResult<Self> {
        let home = std::env::var("HOME").map_err(|_| StorageError::MissingHomeDirectory)?;
        let temp_dir = default_runtime_temp_dir();

        let mut pictures_dir = PathBuf::from(home);
        pictures_dir.push(PREVIEW_SUBDIR);

        fs::create_dir_all(&temp_dir)?;
        fs::create_dir_all(&pictures_dir)?;

        Ok(Self::with_paths(temp_dir, pictures_dir))
    }

    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    pub fn pictures_dir(&self) -> &Path {
        &self.pictures_dir
    }

    fn validate_capture_id(capture_id: &str) -> StorageResult<()> {
        if capture_id.is_empty() {
            return Err(StorageError::MissingCaptureId);
        }
        Ok(())
    }

    pub fn temp_path_for_capture(&self, capture_id: &str) -> StorageResult<PathBuf> {
        Self::validate_capture_id(capture_id)?;
        let mut path = self.temp_dir.clone();
        path.push(format!("{DEFAULT_TEMP_PREFIX}{capture_id}.png"));
        Ok(path)
    }

    pub fn allocate_target_path(&self, capture_id: &str) -> StorageResult<PathBuf> {
        Self::validate_capture_id(capture_id)?;
        let mut path = self.pictures_dir.clone();
        path.push(format!("{capture_id}.png"));
        Ok(path)
    }

    pub fn save_capture(&self, artifact: &CaptureArtifact) -> StorageResult<PathBuf> {
        let target = self.allocate_target_path(&artifact.capture_id)?;
        save_overwrite(&artifact.temp_path, &target)?;
        Ok(target)
    }

    pub fn discard_session_artifacts(&self, capture_id: &str) -> StorageResult<()> {
        let path = self.temp_path_for_capture(capture_id)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(StorageError::Io(err)),
        }
    }

    pub fn prune_stale_temp_files(&self, max_age_hours: u64) -> StorageResult<PruneReport> {
        let now = SystemTime::now();
        let mut report = PruneReport::default();
        let max_age = Duration::from_secs(max_age_hours.saturating_mul(60 * 60));

        if !self.temp_dir.exists() {
            return Ok(report);
        }

        for entry in fs::read_dir(&self.temp_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| !name.starts_with(DEFAULT_TEMP_PREFIX))
            {
                continue;
            }

            let metadata = fs::metadata(&path)?;
            let modified = metadata.modified()?;
            let age = now.duration_since(modified).unwrap_or(Duration::ZERO);

            if age > max_age {
                match fs::remove_file(&path) {
                    Ok(()) => {
                        report.removed_files += 1;
                    }
                    Err(err) => {
                        tracing::warn!(
                            path = %path.display(),
                            ?err,
                            "failed to remove stale temp capture file"
                        );
                    }
                }
            }
        }

        Ok(report)
    }
}

impl CaptureStorage for StorageService {
    fn save_capture(&self, artifact: &CaptureArtifact) -> StorageResult<PathBuf> {
        self.save_capture(artifact)
    }

    fn discard_session_artifacts(&self, capture_id: &str) -> StorageResult<()> {
        self.discard_session_artifacts(capture_id)
    }
}

pub fn create_temp_capture(capture_id: &str) -> PathBuf {
    let mut path = default_runtime_temp_dir();
    path.push(format!("{DEFAULT_TEMP_PREFIX}{capture_id}.png"));
    path
}

pub fn allocate_target_path(capture_id: &str) -> StorageResult<PathBuf> {
    StorageService::with_default_paths()?.allocate_target_path(capture_id)
}

pub fn save_overwrite<S: AsRef<Path>, D: AsRef<Path>>(
    source: S,
    destination: D,
) -> StorageResult<()> {
    let source = source.as_ref();
    let destination = destination.as_ref();

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let _ = fs::remove_file(destination);
    fs::copy(source, destination)?;
    Ok(())
}

pub fn discard_session_artifacts(capture_id: &str) -> StorageResult<()> {
    StorageService::with_default_paths()?.discard_session_artifacts(capture_id)
}

pub fn prune_stale_temp_files(max_age_hours: u64) -> StorageResult<PruneReport> {
    StorageService::with_default_paths()?.prune_stale_temp_files(max_age_hours)
}

fn default_runtime_temp_dir() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_FALLBACK_TEMP_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::CaptureArtifact;
    use std::path::PathBuf;

    #[test]
    fn create_temp_capture_targets_temp_directory() {
        let path = create_temp_capture("123");
        assert!(path.ends_with("capture_123.png"));
    }

    #[test]
    fn allocate_target_path_uses_capture_id_filename() {
        let service =
            StorageService::with_paths(PathBuf::from("/tmp"), PathBuf::from("/home/test/Pictures"));
        let path = service.allocate_target_path("abc").unwrap();
        assert_eq!(path, PathBuf::from("/home/test/Pictures/abc.png"));
    }

    #[test]
    fn lifecycle_cleanup_save_overwrite_and_discard_keeps_saved_output() {
        let service = StorageService::with_paths(PathBuf::from("/tmp"), std::env::temp_dir());
        let source = service.temp_path_for_capture("artifact-1").unwrap();
        let source_data = b"png";

        std::fs::write(&source, source_data).unwrap();
        let artifact = CaptureArtifact {
            capture_id: "artifact-1".to_string(),
            temp_path: source.clone(),
            width: 1,
            height: 1,
            screen_x: 0,
            screen_y: 0,
            screen_width: 1,
            screen_height: 1,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let copied_path = service.save_capture(&artifact).unwrap();
        let copied = std::fs::read(&copied_path).unwrap();
        assert_eq!(copied, source_data);
        service
            .discard_session_artifacts(&artifact.capture_id)
            .unwrap();
        assert!(!source.exists());
        assert!(copied_path.exists());
        assert_eq!(std::fs::read(copied_path).unwrap(), source_data);
    }
}

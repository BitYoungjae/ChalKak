use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("failed to open file {path}: {source}")]
    OpenFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to run wl-copy command: {command}")]
    CommandIo {
        command: String,
        #[source]
        source: io::Error,
    },
    #[error("wl-copy exited with non-zero status: {status}")]
    CommandFailed { status: String },
}

pub type ClipboardResult<T> = std::result::Result<T, ClipboardError>;

pub trait ClipboardBackend {
    fn copy_png_file(&self, path: &Path) -> ClipboardResult<()>;
}

#[derive(Debug, Default)]
pub struct WlCopyBackend;

impl ClipboardBackend for WlCopyBackend {
    fn copy_png_file(&self, path: &Path) -> ClipboardResult<()> {
        let file = File::open(path).map_err(|err| ClipboardError::OpenFile {
            path: path.to_path_buf(),
            source: err,
        })?;

        let child = Command::new("wl-copy")
            .stdin(Stdio::from(file))
            .status()
            .map_err(|err| ClipboardError::CommandIo {
                command: "wl-copy".to_string(),
                source: err,
            })?;

        if child.success() {
            Ok(())
        } else {
            Err(ClipboardError::CommandFailed {
                status: child.to_string(),
            })
        }
    }
}

pub fn copy_png_file(path: &Path) -> ClipboardResult<()> {
    WlCopyBackend.copy_png_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    struct DummyBackend;
    impl ClipboardBackend for DummyBackend {
        fn copy_png_file(&self, _path: &Path) -> ClipboardResult<()> {
            Ok(())
        }
    }

    #[test]
    fn copy_png_file_success_with_backend() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("chalkak-copy-test.png");
        std::fs::write(&file_path, b"binary").unwrap();
        let result = DummyBackend.copy_png_file(&file_path);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn command_error_contains_command_name() {
        let err = ClipboardError::CommandFailed {
            status: "exit status 1".to_string(),
        };
        assert!(format!("{err}").contains("wl-copy"));
    }
}

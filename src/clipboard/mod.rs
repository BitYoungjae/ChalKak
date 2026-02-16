use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use gtk4::gdk;
use gtk4::gdk::prelude::*;
use gtk4::glib;
use thiserror::Error;

const WL_COPY_COMMAND: &str = "wl-copy";
const MIME_TEXT_URI_LIST: &str = "text/uri-list";
const MIME_GNOME_COPIED_FILES: &str = "x-special/gnome-copied-files";
const MIME_TEXT_PLAIN: &str = "text/plain";
const MIME_TEXT_PLAIN_UTF8: &str = "text/plain;charset=utf-8";
const MIME_IMAGE_PNG: &str = "image/png";

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
    #[error("failed to convert path to file URI {path}: {source}")]
    PathToUri {
        path: PathBuf,
        #[source]
        source: glib::Error,
    },
    #[error("failed to read image file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to access default display for clipboard operations")]
    DisplayUnavailable,
    #[error("failed to set clipboard content: {source}")]
    SetContent {
        #[source]
        source: glib::BoolError,
    },
    #[error("wl-copy exited with non-zero status: {status}")]
    CommandFailed { status: String },
}

pub type ClipboardResult<T> = std::result::Result<T, ClipboardError>;

pub trait ClipboardBackend {
    fn copy_png_file(&self, path: &Path) -> ClipboardResult<()>;
    fn copy(&self, path: &Path) -> ClipboardResult<()>;
}

#[derive(Debug, Default)]
pub struct WlCopyBackend;

fn uri_list_payload(path: &Path) -> ClipboardResult<String> {
    let absolute_path = resolve_absolute_path(path)?;
    let uri =
        glib::filename_to_uri(&absolute_path, None).map_err(|err| ClipboardError::PathToUri {
            path: absolute_path.clone(),
            source: err,
        })?;
    Ok(format!("{uri}\r\n"))
}

fn gnome_copied_files_payload(path: &Path) -> ClipboardResult<String> {
    let absolute_path = resolve_absolute_path(path)?;
    let uri =
        glib::filename_to_uri(&absolute_path, None).map_err(|err| ClipboardError::PathToUri {
            path: absolute_path.clone(),
            source: err,
        })?;
    Ok(format!("copy\n{uri}"))
}

fn plain_text_path_payload(path: &Path) -> ClipboardResult<String> {
    let absolute_path = resolve_absolute_path(path)?;
    Ok(absolute_path.to_string_lossy().into_owned())
}

fn is_png_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
}

fn resolve_absolute_path(path: &Path) -> ClipboardResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|dir| dir.join(path))
        .map_err(|err| ClipboardError::CommandIo {
            command: "current_dir".to_string(),
            source: err,
        })
}

impl ClipboardBackend for WlCopyBackend {
    fn copy_png_file(&self, path: &Path) -> ClipboardResult<()> {
        let file = File::open(path).map_err(|err| ClipboardError::OpenFile {
            path: path.to_path_buf(),
            source: err,
        })?;

        let child = Command::new(WL_COPY_COMMAND)
            .stdin(Stdio::from(file))
            .status()
            .map_err(|err| ClipboardError::CommandIo {
                command: WL_COPY_COMMAND.to_string(),
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

    fn copy(&self, path: &Path) -> ClipboardResult<()> {
        let absolute_path = resolve_absolute_path(path)?;
        let uri_list_payload = uri_list_payload(path)?;
        let gnome_payload = gnome_copied_files_payload(path)?;
        let text_path_payload = plain_text_path_payload(path)?;
        let display = gdk::Display::default().ok_or(ClipboardError::DisplayUnavailable)?;
        let clipboard = display.clipboard();

        let gnome_provider = gdk::ContentProvider::for_bytes(
            MIME_GNOME_COPIED_FILES,
            &glib::Bytes::from_owned(gnome_payload.into_bytes()),
        );
        let uri_provider = gdk::ContentProvider::for_bytes(
            MIME_TEXT_URI_LIST,
            &glib::Bytes::from_owned(uri_list_payload.clone().into_bytes()),
        );
        let text_provider = gdk::ContentProvider::for_bytes(
            MIME_TEXT_PLAIN_UTF8,
            &glib::Bytes::from_owned(text_path_payload.clone().into_bytes()),
        );
        let text_plain_provider = gdk::ContentProvider::for_bytes(
            MIME_TEXT_PLAIN,
            &glib::Bytes::from_owned(text_path_payload.into_bytes()),
        );
        let mut providers = vec![
            gnome_provider,
            uri_provider,
            text_provider,
            text_plain_provider,
        ];
        if is_png_path(&absolute_path) {
            let image_bytes =
                std::fs::read(&absolute_path).map_err(|source| ClipboardError::ReadFile {
                    path: absolute_path,
                    source,
                })?;
            let image_provider = gdk::ContentProvider::for_bytes(
                MIME_IMAGE_PNG,
                &glib::Bytes::from_owned(image_bytes),
            );
            providers.push(image_provider);
        }
        let provider = gdk::ContentProvider::new_union(&providers);
        clipboard
            .set_content(Some(&provider))
            .map_err(|source| ClipboardError::SetContent { source })
    }
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

        fn copy(&self, _path: &Path) -> ClipboardResult<()> {
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
    fn copy_success_with_backend() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("chalkak-copy-ref-test.png");
        std::fs::write(&file_path, b"binary").unwrap();
        let result = DummyBackend.copy(&file_path);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn uri_list_payload_encodes_spaces() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("chalkak uri payload test.png");
        std::fs::write(&file_path, b"binary").unwrap();
        let payload = uri_list_payload(&file_path).expect("uri payload");
        assert!(payload.starts_with("file://"));
        assert!(payload.contains("%20"));
        assert!(payload.ends_with("\r\n"));
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn gnome_copied_files_payload_has_copy_prefix_and_uri() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("chalkak copied files payload test.png");
        std::fs::write(&file_path, b"binary").unwrap();
        let payload = gnome_copied_files_payload(&file_path).expect("payload");
        assert!(payload.starts_with("copy\nfile://"));
        assert!(payload.contains("%20"));
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn plain_text_path_payload_returns_absolute_path() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("chalkak plain text payload test.png");
        std::fs::write(&file_path, b"binary").unwrap();
        let payload = plain_text_path_payload(&file_path).expect("payload");
        assert_eq!(payload, file_path.to_string_lossy());
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn is_png_path_detects_case_insensitive_png_extension() {
        assert!(is_png_path(Path::new("/tmp/capture.PNG")));
        assert!(is_png_path(Path::new("/tmp/capture.png")));
        assert!(!is_png_path(Path::new("/tmp/capture.jpg")));
    }

    #[test]
    fn command_error_contains_command_name() {
        let err = ClipboardError::CommandFailed {
            status: "exit status 1".to_string(),
        };
        assert!(format!("{err}").contains("wl-copy"));
    }
}

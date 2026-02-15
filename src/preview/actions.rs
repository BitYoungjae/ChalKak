use thiserror::Error;

use crate::capture::CaptureArtifact;
use crate::clipboard::{ClipboardBackend, ClipboardError};
use crate::storage::{CaptureStorage, StorageError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewAction {
    Save,
    Copy,
    CopyFileReference,
    Edit,
    Delete,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewEvent {
    Save { capture_id: String },
    Copy { capture_id: String },
    CopyFileReference { capture_id: String },
    Edit { capture_id: String },
    Delete { capture_id: String },
    Close { capture_id: String },
}

#[derive(Debug, Error)]
pub enum PreviewActionError {
    #[error("storage error while {operation} {capture_id}: {source}")]
    StorageError {
        operation: &'static str,
        capture_id: String,
        #[source]
        source: StorageError,
    },

    #[error("clipboard error while {operation} {capture_id}: {source}")]
    ClipboardError {
        operation: &'static str,
        capture_id: String,
        #[source]
        source: ClipboardError,
    },
}

pub fn execute_preview_action<S: CaptureStorage, C: ClipboardBackend>(
    artifact: &CaptureArtifact,
    action: PreviewAction,
    storage: &S,
    clipboard: &C,
) -> Result<PreviewEvent, PreviewActionError> {
    let capture_id = artifact.capture_id.clone();
    match action {
        PreviewAction::Save => {
            storage
                .save_capture(artifact)
                .map_err(|err| PreviewActionError::StorageError {
                    operation: "save",
                    capture_id: capture_id.clone(),
                    source: err,
                })?;
            Ok(PreviewEvent::Save { capture_id })
        }
        PreviewAction::Copy => {
            clipboard
                .copy_png_file(&artifact.temp_path)
                .map_err(|err| PreviewActionError::ClipboardError {
                    operation: "copy",
                    capture_id: capture_id.clone(),
                    source: err,
                })?;
            Ok(PreviewEvent::Copy { capture_id })
        }
        PreviewAction::CopyFileReference => {
            clipboard
                .copy_file_reference(&artifact.temp_path)
                .map_err(|err| PreviewActionError::ClipboardError {
                    operation: "copy file reference",
                    capture_id: capture_id.clone(),
                    source: err,
                })?;
            Ok(PreviewEvent::CopyFileReference { capture_id })
        }
        PreviewAction::Edit => Ok(PreviewEvent::Edit { capture_id }),
        PreviewAction::Delete => {
            storage
                .discard_session_artifacts(&capture_id)
                .map_err(|err| PreviewActionError::StorageError {
                    operation: "delete",
                    capture_id: capture_id.clone(),
                    source: err,
                })?;
            Ok(PreviewEvent::Delete { capture_id })
        }
        PreviewAction::Close => Ok(PreviewEvent::Close { capture_id }),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use crate::clipboard::ClipboardBackend;
    use crate::storage::CaptureStorage;

    use super::*;

    #[derive(Default)]
    struct FakeClipboard {
        copied_paths: RefCell<Vec<PathBuf>>,
        copied_file_references: RefCell<Vec<PathBuf>>,
    }

    impl ClipboardBackend for FakeClipboard {
        fn copy_png_file(&self, path: &Path) -> crate::clipboard::ClipboardResult<()> {
            self.copied_paths.borrow_mut().push(path.to_path_buf());
            Ok(())
        }

        fn copy_file_reference(&self, path: &Path) -> crate::clipboard::ClipboardResult<()> {
            self.copied_file_references
                .borrow_mut()
                .push(path.to_path_buf());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeStorage {
        save_requests: RefCell<Vec<String>>,
        discarded: RefCell<Vec<String>>,
    }

    impl CaptureStorage for FakeStorage {
        fn save_capture(
            &self,
            artifact: &CaptureArtifact,
        ) -> crate::storage::StorageResult<PathBuf> {
            self.save_requests
                .borrow_mut()
                .push(artifact.capture_id.clone());
            Ok(artifact.temp_path.clone())
        }

        fn discard_session_artifacts(&self, capture_id: &str) -> crate::storage::StorageResult<()> {
            self.discarded.borrow_mut().push(capture_id.to_string());
            Ok(())
        }
    }

    fn artifact(id: &str) -> CaptureArtifact {
        CaptureArtifact {
            capture_id: id.to_string(),
            temp_path: PathBuf::from("/tmp/test.png"),
            width: 320,
            height: 200,
            screen_x: 0,
            screen_y: 0,
            screen_width: 320,
            screen_height: 200,
            created_at: 0,
        }
    }

    #[test]
    fn preview_action_save_calls_storage_save() {
        let storage = FakeStorage::default();
        let clipboard = FakeClipboard::default();
        let current = artifact("capture-save");

        let event = execute_preview_action(&current, PreviewAction::Save, &storage, &clipboard)
            .expect("save should succeed");

        assert_eq!(
            event,
            PreviewEvent::Save {
                capture_id: "capture-save".to_string()
            }
        );
        assert_eq!(
            storage.save_requests.borrow().as_slice(),
            &["capture-save".to_string()]
        );
        assert!(clipboard.copied_paths.borrow().is_empty());
    }

    #[test]
    fn preview_action_copy_calls_clipboard() {
        let storage = FakeStorage::default();
        let clipboard = FakeClipboard::default();
        let current = artifact("capture-copy");

        let event = execute_preview_action(&current, PreviewAction::Copy, &storage, &clipboard)
            .expect("copy should succeed");

        assert_eq!(
            event,
            PreviewEvent::Copy {
                capture_id: "capture-copy".to_string()
            }
        );
        assert_eq!(
            clipboard.copied_paths.borrow().as_slice(),
            &[PathBuf::from("/tmp/test.png")]
        );
        assert!(clipboard.copied_file_references.borrow().is_empty());
        assert!(storage.save_requests.borrow().is_empty());
    }

    #[test]
    fn preview_action_copy_file_reference_calls_clipboard() {
        let storage = FakeStorage::default();
        let clipboard = FakeClipboard::default();
        let current = artifact("capture-copy-file-ref");

        let event = execute_preview_action(
            &current,
            PreviewAction::CopyFileReference,
            &storage,
            &clipboard,
        )
        .expect("copy file reference should succeed");

        assert_eq!(
            event,
            PreviewEvent::CopyFileReference {
                capture_id: "capture-copy-file-ref".to_string()
            }
        );
        assert_eq!(
            clipboard.copied_file_references.borrow().as_slice(),
            &[PathBuf::from("/tmp/test.png")]
        );
        assert!(clipboard.copied_paths.borrow().is_empty());
        assert!(storage.save_requests.borrow().is_empty());
    }

    #[test]
    fn preview_action_edit_and_close_no_side_effects() {
        let storage = FakeStorage::default();
        let clipboard = FakeClipboard::default();
        let current = artifact("capture-edit-close");

        let edit_event =
            execute_preview_action(&current, PreviewAction::Edit, &storage, &clipboard)
                .expect("edit should succeed");
        let close_event =
            execute_preview_action(&current, PreviewAction::Close, &storage, &clipboard)
                .expect("close should succeed");

        assert_eq!(
            edit_event,
            PreviewEvent::Edit {
                capture_id: "capture-edit-close".to_string()
            }
        );
        assert_eq!(
            close_event,
            PreviewEvent::Close {
                capture_id: "capture-edit-close".to_string()
            }
        );
        assert!(storage.save_requests.borrow().is_empty());
        assert!(storage.discarded.borrow().is_empty());
        assert!(clipboard.copied_paths.borrow().is_empty());
    }

    #[test]
    fn preview_action_delete_discards_artifact() {
        let storage = FakeStorage::default();
        let clipboard = FakeClipboard::default();
        let current = artifact("capture-delete");

        let event = execute_preview_action(&current, PreviewAction::Delete, &storage, &clipboard)
            .expect("delete should succeed");

        assert_eq!(
            event,
            PreviewEvent::Delete {
                capture_id: "capture-delete".to_string()
            }
        );
        assert_eq!(
            storage.discarded.borrow().as_slice(),
            &["capture-delete".to_string()]
        );
        assert!(storage.save_requests.borrow().is_empty());
        assert!(clipboard.copied_paths.borrow().is_empty());
        assert!(clipboard.copied_file_references.borrow().is_empty());
    }
}

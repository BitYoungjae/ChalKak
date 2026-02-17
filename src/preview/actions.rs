use thiserror::Error;

use crate::clipboard::ClipboardError;
use crate::storage::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewAction {
    Save,
    Copy,
    Edit,
    Delete,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewEvent {
    Save { capture_id: String },
    Copy { capture_id: String },
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

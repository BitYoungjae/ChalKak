use crate::state::{AppEvent, AppState};
use thiserror::Error;

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid state transition: from {from:?} using event {event:?}")]
    InvalidStateTransition { from: AppState, event: AppEvent },

    #[error("internal application error: {message}")]
    Internal { message: String },
}

impl AppError {
    pub fn internal<M: Into<String>>(message: M) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

use crate::state::{AppEvent, AppState};
use thiserror::Error;

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid state transition: from {from:?} using event {event:?}")]
    InvalidStateTransition { from: AppState, event: AppEvent },
}

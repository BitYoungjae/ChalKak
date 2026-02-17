use super::event::AppEvent;
use super::model::AppState;
use thiserror::Error;

pub type StateResult<T> = std::result::Result<T, StateError>;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("invalid state transition: from {from:?} using event {event:?}")]
    InvalidStateTransition { from: AppState, event: AppEvent },
}

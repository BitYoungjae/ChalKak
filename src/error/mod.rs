use crate::state::StateError;
use thiserror::Error;

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    State(#[from] StateError),
}

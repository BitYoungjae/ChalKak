pub mod app;
pub mod capture;
pub mod clipboard;
mod config;
pub mod editor;
pub mod error;
pub mod input;
pub mod logging;
pub mod preview;
pub mod state;
pub mod storage;
pub mod theme;
pub mod ui;
pub use error::{AppError, AppResult};

/// Entrypoint used by higher-level integrations and CLI bindings.
pub fn run() -> AppResult<()> {
    logging::init();
    tracing::info!("starting ChalKak");

    let mut app = app::App::new();
    app.start()?;

    tracing::info!("startup complete with state={:?}", app.state().state());
    Ok(())
}

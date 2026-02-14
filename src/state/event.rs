use super::model::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEvent {
    Start,
    OpenPreview,
    OpenEditor,
    CloseEditor,
    ClosePreview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateTransition {
    pub from: Option<AppState>,
    pub event: AppEvent,
    pub to: AppState,
}

impl StateTransition {
    pub const fn new(from: Option<AppState>, event: AppEvent, to: AppState) -> Self {
        Self { from, event, to }
    }
}

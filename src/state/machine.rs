use super::{event::StateTransition, AppEvent, AppState};
use crate::error::{AppError, AppResult};

#[derive(Debug)]
pub struct StateMachine {
    state: AppState,
    transition_history: Vec<StateTransition>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
            transition_history: Vec::new(),
        }
    }

    pub fn state(&self) -> AppState {
        self.state
    }

    pub fn can_transition(&self, event: AppEvent) -> bool {
        self.next_state(event).is_some()
    }

    pub fn next_state(&self, event: AppEvent) -> Option<AppState> {
        use AppEvent::*;
        match (self.state, event) {
            (AppState::Idle, Start) => Some(AppState::Idle),
            (AppState::Idle, OpenPreview) => Some(AppState::Preview),
            (AppState::Preview, OpenPreview) => Some(AppState::Preview),
            (AppState::Preview, OpenEditor) => Some(AppState::Editor),
            (AppState::Editor, CloseEditor) => Some(AppState::Preview),
            (AppState::Preview, ClosePreview) => Some(AppState::Idle),
            _ => None,
        }
    }

    pub fn transition(&mut self, event: AppEvent) -> AppResult<AppState> {
        tracing::debug!(from = ?self.state, event = ?event, "request state transition");
        let next = self.next_state(event).ok_or_else(|| {
            let from = self.state;
            tracing::warn!(from = ?from, event = ?event, "invalid state transition requested");
            AppError::InvalidStateTransition { from, event }
        })?;

        let record = StateTransition::new(Some(self.state), event, next);
        self.state = next;
        self.transition_history.push(record);

        Ok(self.state)
    }
}

#[cfg(test)]
impl StateMachine {
    fn history(&self) -> &[StateTransition] {
        &self.transition_history
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StateMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppState::{:?}", self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_transition_tracks_valid_and_invalid_events() {
        let mut machine = StateMachine::new();
        assert!(machine.can_transition(AppEvent::Start));
        assert!(machine.can_transition(AppEvent::OpenPreview));
        assert!(!machine.can_transition(AppEvent::CloseEditor));

        let _ = machine
            .transition(AppEvent::OpenPreview)
            .expect("idle -> preview should transition");

        assert!(machine.can_transition(AppEvent::OpenEditor));
        assert!(machine.can_transition(AppEvent::ClosePreview));
        assert!(!machine.can_transition(AppEvent::Start));
    }

    #[test]
    fn transition_records_history_with_ordered_entries() {
        let mut machine = StateMachine::new();
        let _ = machine
            .transition(AppEvent::Start)
            .expect("start should work");
        let _ = machine
            .transition(AppEvent::OpenPreview)
            .expect("open preview should work");
        let _ = machine
            .transition(AppEvent::OpenEditor)
            .expect("open editor should work");
        let _ = machine
            .transition(AppEvent::CloseEditor)
            .expect("close editor should work");

        assert_eq!(machine.state(), AppState::Preview);
        assert_eq!(machine.history().len(), 4);
        assert_eq!(
            machine.history()[0],
            StateTransition::new(Some(AppState::Idle), AppEvent::Start, AppState::Idle)
        );
        assert_eq!(
            machine.history()[1],
            StateTransition::new(
                Some(AppState::Idle),
                AppEvent::OpenPreview,
                AppState::Preview
            )
        );
        assert_eq!(
            machine.history()[2],
            StateTransition::new(
                Some(AppState::Preview),
                AppEvent::OpenEditor,
                AppState::Editor
            )
        );
        assert_eq!(
            machine.history()[3],
            StateTransition::new(
                Some(AppState::Editor),
                AppEvent::CloseEditor,
                AppState::Preview
            )
        );
    }

    #[test]
    fn invalid_transition_returns_error_without_mutating_history() {
        let mut machine = StateMachine::new();

        let err = machine
            .transition(AppEvent::ClosePreview)
            .expect_err("idle -> close preview should fail");
        assert!(matches!(
            err,
            AppError::InvalidStateTransition {
                from: AppState::Idle,
                event: AppEvent::ClosePreview
            }
        ));
        assert_eq!(machine.state(), AppState::Idle);
        assert!(machine.history().is_empty());
    }
}

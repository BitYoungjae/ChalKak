use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use crate::capture;
use crate::clipboard::WlCopyBackend;
use crate::preview::{self, PreviewAction, PreviewEvent};
use crate::state::{AppEvent, AppState, RuntimeWindowState, StateMachine};
use crate::storage::StorageService;

use super::runtime_support::{
    close_preview_window_for_capture, show_toast_for_capture, PreviewWindowRuntime, RuntimeSession,
    ToastRuntime,
};

pub(super) type SharedMachine = Rc<RefCell<StateMachine>>;
pub(super) type SharedRuntimeSession = Rc<RefCell<RuntimeSession>>;
pub(super) type SharedStatusLog = Rc<RefCell<String>>;
pub(super) type SharedCaptureSelection = Rc<RefCell<Option<String>>>;

const ACTION_RESULT_POLL_INTERVAL: Duration = Duration::from_millis(24);

#[derive(Clone)]
pub(super) struct LaunchpadActionExecutor {
    runtime_session: SharedRuntimeSession,
    capture_selection: SharedCaptureSelection,
    machine: SharedMachine,
    storage_service: Rc<Option<StorageService>>,
    status_log: SharedStatusLog,
    preview_windows: Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    runtime_window_state: Rc<RefCell<RuntimeWindowState>>,
    fallback_toast: ToastRuntime,
    toast_duration_ms: u32,
}

#[derive(Debug, Clone)]
struct PreparedPreviewAction {
    action: PreviewAction,
    active_capture: capture::CaptureArtifact,
    storage_service: StorageService,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreviewActionUiOutcome {
    event: Option<PreviewEvent>,
    status_message: String,
    toast_capture_id: String,
    toast_message: String,
}

impl LaunchpadActionExecutor {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        runtime_session: SharedRuntimeSession,
        capture_selection: SharedCaptureSelection,
        machine: SharedMachine,
        storage_service: Rc<Option<StorageService>>,
        status_log: SharedStatusLog,
        preview_windows: Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
        runtime_window_state: Rc<RefCell<RuntimeWindowState>>,
        fallback_toast: ToastRuntime,
        toast_duration_ms: u32,
    ) -> Self {
        Self {
            runtime_session,
            capture_selection,
            machine,
            storage_service,
            status_log,
            preview_windows,
            runtime_window_state,
            fallback_toast,
            toast_duration_ms,
        }
    }

    pub(super) fn capture_and_open_preview(
        &self,
        capture_result: Result<capture::CaptureArtifact, capture::CaptureError>,
        success_toast_message: &str,
        failure_status_prefix: &str,
        failure_toast_prefix: &str,
    ) {
        match capture_result {
            Ok(artifact) => {
                let capture_id = artifact.capture_id.clone();
                self.runtime_session.borrow_mut().push_capture(artifact);
                if !transition_with_status(
                    &self.machine,
                    &self.status_log,
                    AppEvent::OpenPreview,
                    "preview transition blocked for current state",
                    "failed state transition",
                ) {
                    return;
                }
                set_status(&self.status_log, format!("preview opened for {capture_id}"));
                self.fallback_toast
                    .show(success_toast_message, self.toast_duration_ms);
            }
            Err(err) => {
                set_status(&self.status_log, format!("{failure_status_prefix}: {err}"));
                self.fallback_toast.show(
                    format!("{failure_toast_prefix}: {err}"),
                    self.toast_duration_ms,
                );
            }
        }
    }

    pub(super) fn capture_and_open_preview_async<F, R>(
        &self,
        capture_work: F,
        success_toast_message: &'static str,
        failure_status_prefix: &'static str,
        failure_toast_prefix: &'static str,
        on_complete: R,
    ) where
        F: FnOnce() -> Result<capture::CaptureArtifact, capture::CaptureError> + Send + 'static,
        R: Fn() + 'static,
    {
        let executor = self.clone();
        let mut on_complete = Some(on_complete);
        spawn_worker_action(capture_work, move |capture_result| {
            executor.capture_and_open_preview(
                capture_result,
                success_toast_message,
                failure_status_prefix,
                failure_toast_prefix,
            );
            if let Some(on_complete) = on_complete.take() {
                on_complete();
            }
        });
    }

    pub(super) fn open_preview(&self) {
        let Some(active_capture_id) = self
            .runtime_session
            .borrow()
            .active_capture()
            .map(|artifact| artifact.capture_id.clone())
        else {
            set_status(&self.status_log, "capture required before preview");
            return;
        };

        let state = self.machine.borrow().state();
        if matches!(state, AppState::Preview) {
            set_status(
                &self.status_log,
                format!("preview opened for {active_capture_id}"),
            );
            return;
        }
        if !matches!(state, AppState::Idle) {
            set_status(
                &self.status_log,
                format!("cannot open preview from state {state:?}"),
            );
            return;
        }

        if transition_with_status(
            &self.machine,
            &self.status_log,
            AppEvent::OpenPreview,
            "preview transition blocked for current state",
            "cannot open preview",
        ) {
            set_status(
                &self.status_log,
                format!("preview opened for {active_capture_id}"),
            );
        }
    }

    pub(super) fn open_editor(&self) {
        let active_capture_id =
            consume_and_resolve_active_capture_id(&self.runtime_session, &self.capture_selection);
        let Some(active_capture_id) = active_capture_id else {
            set_status(&self.status_log, "capture required before editor");
            return;
        };
        if !matches!(self.machine.borrow().state(), AppState::Preview) {
            set_status(&self.status_log, "editor can only open from preview state");
            return;
        }

        if transition_with_status(
            &self.machine,
            &self.status_log,
            AppEvent::OpenEditor,
            "editor transition blocked for current state",
            "cannot open editor",
        ) {
            set_status(
                &self.status_log,
                format!("editor opened for {active_capture_id}"),
            );
        }
    }

    pub(super) fn close_preview(&self) {
        if !matches!(self.machine.borrow().state(), AppState::Preview) {
            set_status(&self.status_log, "close preview requires preview state");
            return;
        }
        let capture_id_to_close =
            consume_and_resolve_active_capture_id(&self.runtime_session, &self.capture_selection);
        let Some(capture_id_to_close) = capture_id_to_close else {
            set_status(&self.status_log, "preview close requires an active capture");
            return;
        };

        self.runtime_session
            .borrow_mut()
            .remove_capture(&capture_id_to_close);
        if let Some(service) = self.storage_service.as_ref() {
            if let Err(err) = service.discard_session_artifacts(&capture_id_to_close) {
                tracing::warn!(
                    capture_id = %capture_id_to_close,
                    ?err,
                    "failed to discard temporary artifact after preview close"
                );
            }
        }
        close_preview_window_for_capture(
            &self.preview_windows,
            &capture_id_to_close,
            &self.runtime_window_state,
        );

        if self.runtime_session.borrow().active_capture().is_none() {
            if transition_with_status(
                &self.machine,
                &self.status_log,
                AppEvent::ClosePreview,
                "cannot close preview",
                "cannot close preview",
            ) {
                set_status(&self.status_log, "preview closed");
            }
        } else {
            set_status(
                &self.status_log,
                format!("preview closed for {capture_id_to_close}"),
            );
        }
    }

    pub(super) fn close_editor(&self) {
        if transition_with_status(
            &self.machine,
            &self.status_log,
            AppEvent::CloseEditor,
            "cannot close editor",
            "cannot close editor",
        ) {
            set_status(&self.status_log, "editor closed");
        }
    }

    pub(super) fn run_preview_action_async<R>(&self, action: PreviewAction, on_complete: R)
    where
        R: Fn() + 'static,
    {
        let Some(prepared) = prepare_preview_action_request(
            action,
            &self.runtime_session,
            &self.capture_selection,
            &self.machine,
            &self.storage_service,
            &self.status_log,
        ) else {
            on_complete();
            return;
        };

        let worker_capture = prepared.active_capture.clone();
        let worker_action = prepared.action;
        let worker_storage = prepared.storage_service.clone();

        let executor = self.clone();
        let mut on_complete = Some(on_complete);
        spawn_worker_action(
            move || {
                preview::execute_preview_action(
                    &worker_capture,
                    worker_action,
                    &worker_storage,
                    &WlCopyBackend,
                )
            },
            move |result| {
                let _ = apply_preview_action_result(
                    prepared.action,
                    &prepared.active_capture,
                    result,
                    &executor.status_log,
                    &executor.preview_windows,
                    &executor.fallback_toast,
                    executor.toast_duration_ms,
                );
                if let Some(on_complete) = on_complete.take() {
                    on_complete();
                }
            },
        );
    }

    pub(super) fn delete_active_capture_async<R>(&self, on_complete: R)
    where
        R: Fn() + 'static,
    {
        let Some(prepared) = prepare_preview_action_request(
            PreviewAction::Delete,
            &self.runtime_session,
            &self.capture_selection,
            &self.machine,
            &self.storage_service,
            &self.status_log,
        ) else {
            on_complete();
            return;
        };

        let worker_capture = prepared.active_capture.clone();
        let worker_storage = prepared.storage_service.clone();

        let executor = self.clone();
        let mut on_complete = Some(on_complete);
        spawn_worker_action(
            move || {
                preview::execute_preview_action(
                    &worker_capture,
                    PreviewAction::Delete,
                    &worker_storage,
                    &WlCopyBackend,
                )
            },
            move |result| {
                let event = apply_preview_action_result(
                    PreviewAction::Delete,
                    &prepared.active_capture,
                    result,
                    &executor.status_log,
                    &executor.preview_windows,
                    &executor.fallback_toast,
                    executor.toast_duration_ms,
                );
                if let Some(PreviewEvent::Delete { capture_id }) = event {
                    executor.apply_deleted_capture(&capture_id);
                }
                if let Some(on_complete) = on_complete.take() {
                    on_complete();
                }
            },
        );
    }

    fn apply_deleted_capture(&self, capture_id: &str) {
        self.runtime_session.borrow_mut().remove_capture(capture_id);
        close_preview_window_for_capture(
            &self.preview_windows,
            capture_id,
            &self.runtime_window_state,
        );
        if self.runtime_session.borrow().active_capture().is_none() {
            let _ = self.machine.borrow_mut().transition(AppEvent::ClosePreview);
        }
    }
}

#[derive(Clone, Copy)]
struct PreviewActionLabels {
    operation: &'static str,
    title: &'static str,
    past: &'static str,
    success_title: &'static str,
}

impl PreviewActionLabels {
    const fn for_action(action: PreviewAction) -> Self {
        match action {
            PreviewAction::Save => Self {
                operation: "save",
                title: "Save",
                past: "saved",
                success_title: "Saved",
            },
            PreviewAction::Copy => Self {
                operation: "copy",
                title: "Copy",
                past: "copied",
                success_title: "Copied",
            },
            PreviewAction::Edit => Self {
                operation: "edit",
                title: "Edit",
                past: "edited",
                success_title: "Edited",
            },
            PreviewAction::Delete => Self {
                operation: "delete",
                title: "Delete",
                past: "deleted",
                success_title: "Deleted",
            },
            PreviewAction::Close => Self {
                operation: "close",
                title: "Close",
                past: "closed",
                success_title: "Closed",
            },
        }
    }
}

pub(super) fn set_status(status_log: &SharedStatusLog, message: impl Into<String>) {
    *status_log.borrow_mut() = message.into();
}

pub(super) fn consume_selected_capture_id(
    capture_selection: &SharedCaptureSelection,
) -> Option<String> {
    capture_selection.borrow_mut().take()
}

fn resolve_active_capture_with<T>(
    runtime_session: &SharedRuntimeSession,
    selected_capture_id: Option<&str>,
    map: impl FnOnce(&capture::CaptureArtifact) -> T,
) -> Option<T> {
    let mut runtime = runtime_session.borrow_mut();
    if let Some(capture_id) = selected_capture_id {
        let _ = runtime.set_active_capture(capture_id);
    }
    runtime.active_capture().map(map)
}

pub(super) fn resolve_active_capture_id(
    runtime_session: &SharedRuntimeSession,
    selected_capture_id: Option<&str>,
) -> Option<String> {
    resolve_active_capture_with(runtime_session, selected_capture_id, |artifact| {
        artifact.capture_id.clone()
    })
}

pub(super) fn resolve_active_capture(
    runtime_session: &SharedRuntimeSession,
    selected_capture_id: Option<&str>,
) -> Option<capture::CaptureArtifact> {
    resolve_active_capture_with(runtime_session, selected_capture_id, Clone::clone)
}

fn consume_and_resolve<T>(
    runtime_session: &SharedRuntimeSession,
    capture_selection: &SharedCaptureSelection,
    resolve: impl FnOnce(&SharedRuntimeSession, Option<&str>) -> Option<T>,
) -> Option<T> {
    let selected_capture_id = consume_selected_capture_id(capture_selection);
    resolve(runtime_session, selected_capture_id.as_deref())
}

pub(super) fn consume_and_resolve_active_capture_id(
    runtime_session: &SharedRuntimeSession,
    capture_selection: &SharedCaptureSelection,
) -> Option<String> {
    consume_and_resolve(
        runtime_session,
        capture_selection,
        resolve_active_capture_id,
    )
}

pub(super) fn consume_and_resolve_active_capture(
    runtime_session: &SharedRuntimeSession,
    capture_selection: &SharedCaptureSelection,
) -> Option<capture::CaptureArtifact> {
    consume_and_resolve(runtime_session, capture_selection, resolve_active_capture)
}

pub(super) fn transition_with_status(
    machine: &SharedMachine,
    status_log: &SharedStatusLog,
    event: AppEvent,
    blocked_message: &str,
    failure_prefix: &str,
) -> bool {
    if !machine.borrow().can_transition(event) {
        set_status(status_log, blocked_message);
        return false;
    }

    match machine.borrow_mut().transition(event) {
        Ok(_) => true,
        Err(err) => {
            set_status(status_log, format!("{failure_prefix}: {err}"));
            false
        }
    }
}

fn prepare_preview_action_request(
    action: PreviewAction,
    runtime_session: &SharedRuntimeSession,
    capture_selection: &SharedCaptureSelection,
    machine: &SharedMachine,
    storage_service: &Rc<Option<StorageService>>,
    status_log: &SharedStatusLog,
) -> Option<PreparedPreviewAction> {
    let labels = PreviewActionLabels::for_action(action);
    let active_capture =
        match consume_and_resolve_active_capture(runtime_session, capture_selection) {
            Some(artifact) => artifact,
            None => {
                set_status(
                    status_log,
                    format!("{} requires an active capture", labels.operation),
                );
                return None;
            }
        };
    if !matches!(machine.borrow().state(), AppState::Preview) {
        set_status(
            status_log,
            format!("{} requires preview state", labels.operation),
        );
        return None;
    }

    let Some(service) = storage_service.as_ref() else {
        set_status(status_log, "storage service unavailable");
        return None;
    };

    Some(PreparedPreviewAction {
        action,
        active_capture,
        storage_service: service.clone(),
    })
}

fn apply_preview_action_result(
    action: PreviewAction,
    active_capture: &capture::CaptureArtifact,
    result: Result<PreviewEvent, preview::PreviewActionError>,
    status_log: &SharedStatusLog,
    preview_windows: &Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    fallback_toast: &ToastRuntime,
    toast_duration_ms: u32,
) -> Option<PreviewEvent> {
    let outcome = preview_action_ui_outcome(action, &active_capture.capture_id, result);
    set_status(status_log, outcome.status_message);
    show_toast_for_capture(
        preview_windows,
        &outcome.toast_capture_id,
        fallback_toast,
        outcome.toast_message,
        toast_duration_ms,
    );
    outcome.event
}

fn preview_action_ui_outcome(
    action: PreviewAction,
    active_capture_id: &str,
    result: Result<PreviewEvent, preview::PreviewActionError>,
) -> PreviewActionUiOutcome {
    let labels = PreviewActionLabels::for_action(action);
    match result {
        Ok(event) if matches_preview_action(action, &event) => {
            let capture_id = preview_event_capture_id(&event).to_string();
            PreviewActionUiOutcome {
                event: Some(event),
                status_message: format!("{} capture {capture_id}", labels.past),
                toast_capture_id: capture_id.clone(),
                toast_message: format!("{} {capture_id}", labels.success_title),
            }
        }
        Ok(other) => PreviewActionUiOutcome {
            status_message: format!("{} produced {other:?}", labels.operation),
            toast_capture_id: active_capture_id.to_string(),
            toast_message: format!("{} produced {other:?}", labels.title),
            event: Some(other),
        },
        Err(err) => PreviewActionUiOutcome {
            event: None,
            status_message: format!("{} failed: {err}", labels.operation),
            toast_capture_id: active_capture_id.to_string(),
            toast_message: format!("{} failed: {err}", labels.title),
        },
    }
}

fn spawn_worker_action<T, W, H>(work: W, mut on_result: H)
where
    T: Send + 'static,
    W: FnOnce() -> T + Send + 'static,
    H: FnMut(T) + 'static,
{
    let (tx, rx) = mpsc::channel::<T>();
    std::thread::spawn(move || {
        let result = work();
        let _ = tx.send(result);
    });

    gtk4::glib::timeout_add_local(ACTION_RESULT_POLL_INTERVAL, move || match rx.try_recv() {
        Ok(result) => {
            on_result(result);
            gtk4::glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => gtk4::glib::ControlFlow::Continue,
        Err(mpsc::TryRecvError::Disconnected) => gtk4::glib::ControlFlow::Break,
    });
}

fn matches_preview_action(action: PreviewAction, event: &PreviewEvent) -> bool {
    matches!(
        (action, event),
        (PreviewAction::Save, PreviewEvent::Save { .. })
            | (PreviewAction::Copy, PreviewEvent::Copy { .. })
            | (PreviewAction::Edit, PreviewEvent::Edit { .. })
            | (PreviewAction::Delete, PreviewEvent::Delete { .. })
            | (PreviewAction::Close, PreviewEvent::Close { .. })
    )
}

fn preview_event_capture_id(event: &PreviewEvent) -> &str {
    match event {
        PreviewEvent::Save { capture_id }
        | PreviewEvent::Copy { capture_id }
        | PreviewEvent::Edit { capture_id }
        | PreviewEvent::Delete { capture_id }
        | PreviewEvent::Close { capture_id } => capture_id,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn artifact(id: &str) -> capture::CaptureArtifact {
        capture::CaptureArtifact {
            capture_id: id.to_string(),
            temp_path: PathBuf::from(format!("/tmp/{id}.png")),
            width: 320,
            height: 180,
            screen_x: 0,
            screen_y: 0,
            screen_width: 320,
            screen_height: 180,
            created_at: 0,
        }
    }

    #[test]
    fn resolve_active_capture_id_prefers_selected_target_when_available() {
        let runtime = Rc::new(RefCell::new(RuntimeSession::default()));
        runtime.borrow_mut().push_capture(artifact("one"));
        runtime.borrow_mut().push_capture(artifact("two"));

        let resolved = resolve_active_capture_id(&runtime, Some("one"));

        assert_eq!(resolved.as_deref(), Some("one"));
        assert_eq!(
            runtime
                .borrow()
                .active_capture()
                .map(|item| item.capture_id.as_str()),
            Some("one")
        );
    }

    #[test]
    fn consume_and_resolve_active_capture_uses_selected_capture() {
        let runtime = Rc::new(RefCell::new(RuntimeSession::default()));
        runtime.borrow_mut().push_capture(artifact("one"));
        runtime.borrow_mut().push_capture(artifact("two"));
        let capture_selection = Rc::new(RefCell::new(Some("one".to_string())));

        let resolved = consume_and_resolve_active_capture(&runtime, &capture_selection);

        assert_eq!(
            resolved.map(|item| item.capture_id),
            Some("one".to_string())
        );
        assert!(capture_selection.borrow().is_none());
    }

    #[test]
    fn transition_with_status_sets_message_when_transition_is_blocked() {
        let machine = Rc::new(RefCell::new(StateMachine::new()));
        let status_log = Rc::new(RefCell::new(String::new()));

        let changed = transition_with_status(
            &machine,
            &status_log,
            AppEvent::CloseEditor,
            "transition blocked",
            "cannot transition",
        );

        assert!(!changed);
        assert_eq!(status_log.borrow().as_str(), "transition blocked");
        assert_eq!(machine.borrow().state(), AppState::Idle);
    }

    #[test]
    fn transition_with_status_updates_machine_on_success() {
        let machine = Rc::new(RefCell::new(StateMachine::new()));
        let status_log = Rc::new(RefCell::new(String::new()));

        let changed = transition_with_status(
            &machine,
            &status_log,
            AppEvent::OpenPreview,
            "transition blocked",
            "cannot transition",
        );

        assert!(changed);
        assert_eq!(machine.borrow().state(), AppState::Preview);
        assert!(status_log.borrow().is_empty());
    }

    #[test]
    fn preview_action_ui_outcome_reports_success_for_matching_event() {
        let outcome = preview_action_ui_outcome(
            PreviewAction::Save,
            "active-capture",
            Ok(PreviewEvent::Save {
                capture_id: "capture-save".to_string(),
            }),
        );

        assert_eq!(
            outcome.event,
            Some(PreviewEvent::Save {
                capture_id: "capture-save".to_string(),
            })
        );
        assert_eq!(outcome.status_message, "saved capture capture-save");
        assert_eq!(outcome.toast_capture_id, "capture-save");
        assert_eq!(outcome.toast_message, "Saved capture-save");
    }

    #[test]
    fn preview_action_ui_outcome_reports_error_for_failed_copy() {
        let result = Err(preview::PreviewActionError::ClipboardError {
            operation: "copy",
            capture_id: "capture-copy".to_string(),
            source: crate::clipboard::ClipboardError::CommandFailed {
                status: "exit status 1".to_string(),
            },
        });

        let outcome = preview_action_ui_outcome(PreviewAction::Copy, "capture-copy", result);

        assert_eq!(outcome.event, None);
        assert_eq!(outcome.toast_capture_id, "capture-copy");
        assert!(outcome
            .status_message
            .starts_with("copy failed: clipboard error while copy capture-copy"));
        assert!(outcome
            .toast_message
            .starts_with("Copy failed: clipboard error while copy capture-copy"));
    }

    #[test]
    fn prepare_preview_action_request_sets_status_when_state_is_not_preview() {
        let runtime = Rc::new(RefCell::new(RuntimeSession::default()));
        runtime.borrow_mut().push_capture(artifact("one"));
        let capture_selection = Rc::new(RefCell::new(None));
        let machine = Rc::new(RefCell::new(StateMachine::new()));
        let storage_service = Rc::new(Some(StorageService::with_paths(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
        )));
        let status_log = Rc::new(RefCell::new(String::new()));

        let prepared = prepare_preview_action_request(
            PreviewAction::Save,
            &runtime,
            &capture_selection,
            &machine,
            &storage_service,
            &status_log,
        );

        assert!(prepared.is_none());
        assert_eq!(status_log.borrow().as_str(), "save requires preview state");
    }

    #[test]
    fn prepare_preview_action_request_keeps_selected_capture_for_async_execution() {
        let runtime = Rc::new(RefCell::new(RuntimeSession::default()));
        runtime.borrow_mut().push_capture(artifact("one"));
        runtime.borrow_mut().push_capture(artifact("two"));
        let capture_selection = Rc::new(RefCell::new(Some("one".to_string())));
        let machine = Rc::new(RefCell::new(StateMachine::new()));
        let _ = machine.borrow_mut().transition(AppEvent::OpenPreview);
        let storage_service = Rc::new(Some(StorageService::with_paths(
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
        )));
        let status_log = Rc::new(RefCell::new(String::new()));

        let prepared = prepare_preview_action_request(
            PreviewAction::Copy,
            &runtime,
            &capture_selection,
            &machine,
            &storage_service,
            &status_log,
        )
        .expect("preview action should be prepared");

        assert_eq!(prepared.action, PreviewAction::Copy);
        assert_eq!(prepared.active_capture.capture_id, "one");
        assert!(capture_selection.borrow().is_none());
    }
}

use super::*;

#[derive(Clone)]
struct EditorCloseRequestRuntime {
    runtime_session: Rc<RefCell<RuntimeSession>>,
    shared_machine: Rc<RefCell<StateMachine>>,
    storage_service: Rc<Option<StorageService>>,
    status_log_for_render: Rc<RefCell<String>>,
    close_editor_button: Button,
    editor_has_unsaved_changes: Rc<RefCell<bool>>,
    editor_close_dialog_open: Rc<RefCell<bool>>,
    editor_window_for_dialog: ApplicationWindow,
    editor_toast_runtime: ToastRuntime,
    editor_tools: Rc<RefCell<editor::EditorTools>>,
    pending_crop_for_close: Rc<RefCell<Option<CropElement>>>,
    editor_source_pixbuf: Option<gtk4::gdk_pixbuf::Pixbuf>,
    style_tokens: StyleTokens,
}

#[derive(Clone, Copy)]
enum EditorCloseOrigin {
    EditorButton,
    WindowCloseRequest,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorCloseOutcome {
    AllowWindowClose,
    KeepWindowOpen,
}

fn trigger_editor_close_transition(runtime: &EditorCloseRequestRuntime) -> bool {
    runtime.close_editor_button.emit_clicked();
    !matches!(runtime.shared_machine.borrow().state(), AppState::Editor)
}

fn open_editor_unsaved_close_dialog(
    runtime: &EditorCloseRequestRuntime,
    active_capture: capture::CaptureArtifact,
    service: StorageService,
) {
    if *runtime.editor_close_dialog_open.borrow() {
        return;
    }
    *runtime.editor_close_dialog_open.borrow_mut() = true;

    let dialog = Dialog::new();
    dialog.add_css_class("chalkak-root");
    dialog.set_title(Some("Unsaved edits"));
    dialog.set_transient_for(Some(&runtime.editor_window_for_dialog));
    dialog.set_modal(true);
    dialog.set_destroy_with_parent(true);
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Don't Save", ResponseType::Reject);
    dialog.add_button("Save and Close", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    let body = Label::new(Some("You have unused edits.\nSave before closing?"));
    body.set_xalign(0.5);
    body.set_justify(gtk4::Justification::Center);

    let style_tokens = runtime.style_tokens;
    let dialog_content = GtkBox::new(Orientation::Vertical, 0);
    dialog_content.set_margin_top(style_tokens.spacing_12);
    dialog_content.set_margin_bottom(style_tokens.spacing_12);
    dialog_content.set_margin_start(style_tokens.spacing_12);
    dialog_content.set_margin_end(style_tokens.spacing_12);
    dialog_content.append(&body);
    dialog.content_area().append(&dialog_content);

    {
        let dialog_for_key = dialog.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, keycode, modifier| {
            let Some(shortcut_key) = normalize_shortcut_key(key, keycode) else {
                return gtk4::glib::Propagation::Proceed;
            };
            let shortcut = resolve_shortcut(
                shortcut_key,
                shortcut_modifiers(modifier),
                InputContext {
                    mode: InputMode::Dialog,
                },
            );
            match shortcut {
                Some(ShortcutAction::DialogConfirm) => {
                    dialog_for_key.response(ResponseType::Accept);
                    gtk4::glib::Propagation::Stop
                }
                Some(ShortcutAction::DialogCancel) => {
                    dialog_for_key.response(ResponseType::Cancel);
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });
        dialog.add_controller(key_controller);
    }

    let runtime_session = runtime.runtime_session.clone();
    let status_log_for_render = runtime.status_log_for_render.clone();
    let editor_has_unsaved_changes = runtime.editor_has_unsaved_changes.clone();
    let editor_close_dialog_open = runtime.editor_close_dialog_open.clone();
    let editor_toast_runtime = runtime.editor_toast_runtime.clone();
    let editor_tools = runtime.editor_tools.clone();
    let editor_source_pixbuf = runtime.editor_source_pixbuf.clone();
    let pending_crop = runtime.pending_crop_for_close.clone();
    let close_runtime = runtime.clone();
    let capture_id = active_capture.capture_id.clone();
    dialog.connect_response(move |dialog, response| {
        match response {
            ResponseType::Accept => {
                let Some(source_pixbuf) = editor_source_pixbuf.as_ref() else {
                    *status_log_for_render.borrow_mut() =
                        "editor source image unavailable".to_string();
                    editor_toast_runtime
                        .show("Source image unavailable", style_tokens.toast_duration_ms);
                    *editor_close_dialog_open.borrow_mut() = false;
                    dialog.close();
                    return;
                };
                let tools = editor_tools.borrow();
                let saved = execute_editor_output_action(EditorOutputActionContext {
                    action: EditorAction::Save,
                    active_capture: &active_capture,
                    editor_tools: &tools,
                    pending_crop: pending_crop.borrow().as_ref().copied(),
                    source_pixbuf,
                    storage_service: &service,
                    status_log: &status_log_for_render,
                    editor_toast: &editor_toast_runtime,
                    toast_duration_ms: style_tokens.toast_duration_ms,
                    editor_has_unsaved_changes: &editor_has_unsaved_changes,
                });
                if saved {
                    let _ = trigger_editor_close_transition(&close_runtime);
                }
            }
            ResponseType::Reject => match service.discard_session_artifacts(&capture_id) {
                Ok(()) => {
                    runtime_session.borrow_mut().remove_capture(&capture_id);
                    *editor_has_unsaved_changes.borrow_mut() = false;
                    *status_log_for_render.borrow_mut() =
                        format!("discarded unsaved capture {capture_id}");
                    editor_toast_runtime.show(
                        format!("Discarded {capture_id}"),
                        style_tokens.toast_duration_ms,
                    );
                    let _ = trigger_editor_close_transition(&close_runtime);
                }
                Err(err) => {
                    *status_log_for_render.borrow_mut() = format!("discard failed: {err}");
                    editor_toast_runtime.show(
                        format!("Discard failed: {err}"),
                        style_tokens.toast_duration_ms,
                    );
                }
            },
            _ => {
                *status_log_for_render.borrow_mut() = "editor close canceled".to_string();
                editor_toast_runtime.show("Close canceled", style_tokens.toast_duration_ms);
            }
        }

        *editor_close_dialog_open.borrow_mut() = false;
        dialog.close();
    });
    dialog.present();
}

fn handle_editor_close_request(
    runtime: &EditorCloseRequestRuntime,
    origin: EditorCloseOrigin,
) -> EditorCloseOutcome {
    if !matches!(runtime.shared_machine.borrow().state(), AppState::Editor) {
        if matches!(origin, EditorCloseOrigin::EditorButton) {
            *runtime.status_log_for_render.borrow_mut() =
                "editor close requested outside editor state; closing window directly".to_string();
            runtime.editor_window_for_dialog.close();
        }
        return EditorCloseOutcome::AllowWindowClose;
    }

    if !*runtime.editor_has_unsaved_changes.borrow() {
        if trigger_editor_close_transition(runtime) {
            return EditorCloseOutcome::AllowWindowClose;
        }
        *runtime.status_log_for_render.borrow_mut() = "editor close transition blocked".to_string();
        return EditorCloseOutcome::KeepWindowOpen;
    }

    let active_capture = match runtime.runtime_session.borrow().active_capture().cloned() {
        Some(artifact) => artifact,
        None => {
            *runtime.status_log_for_render.borrow_mut() =
                "editor close requires an active capture".to_string();
            return EditorCloseOutcome::KeepWindowOpen;
        }
    };

    let Some(service) = runtime.storage_service.as_ref().clone() else {
        *runtime.status_log_for_render.borrow_mut() = "storage service unavailable".to_string();
        return EditorCloseOutcome::KeepWindowOpen;
    };

    open_editor_unsaved_close_dialog(runtime, active_capture, service);
    EditorCloseOutcome::KeepWindowOpen
}

pub(in crate::app::editor_runtime) struct EditorCloseDialogContext {
    pub(in crate::app::editor_runtime) runtime_session: Rc<RefCell<RuntimeSession>>,
    pub(in crate::app::editor_runtime) shared_machine: Rc<RefCell<StateMachine>>,
    pub(in crate::app::editor_runtime) storage_service: Rc<Option<StorageService>>,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) close_editor_button: Button,
    pub(in crate::app::editor_runtime) editor_close_button: Button,
    pub(in crate::app::editor_runtime) editor_has_unsaved_changes: Rc<RefCell<bool>>,
    pub(in crate::app::editor_runtime) editor_close_dialog_open: Rc<RefCell<bool>>,
    pub(in crate::app::editor_runtime) editor_window_for_dialog: ApplicationWindow,
    pub(in crate::app::editor_runtime) editor_toast_runtime: ToastRuntime,
    pub(in crate::app::editor_runtime) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(in crate::app::editor_runtime) pending_crop_for_close: Rc<RefCell<Option<CropElement>>>,
    pub(in crate::app::editor_runtime) editor_source_pixbuf: Option<gtk4::gdk_pixbuf::Pixbuf>,
    pub(in crate::app::editor_runtime) style_tokens: StyleTokens,
}

pub(in crate::app::editor_runtime) fn connect_editor_close_dialog(
    context: EditorCloseDialogContext,
) {
    let close_runtime = EditorCloseRequestRuntime {
        runtime_session: context.runtime_session.clone(),
        shared_machine: context.shared_machine.clone(),
        storage_service: context.storage_service.clone(),
        status_log_for_render: context.status_log_for_render.clone(),
        close_editor_button: context.close_editor_button.clone(),
        editor_has_unsaved_changes: context.editor_has_unsaved_changes.clone(),
        editor_close_dialog_open: context.editor_close_dialog_open.clone(),
        editor_window_for_dialog: context.editor_window_for_dialog.clone(),
        editor_toast_runtime: context.editor_toast_runtime.clone(),
        editor_tools: context.editor_tools.clone(),
        pending_crop_for_close: context.pending_crop_for_close.clone(),
        editor_source_pixbuf: context.editor_source_pixbuf.clone(),
        style_tokens: context.style_tokens,
    };

    context.editor_close_button.connect_clicked(move |_| {
        let _ = handle_editor_close_request(&close_runtime, EditorCloseOrigin::EditorButton);
    });
}

#[derive(Clone)]
pub(in crate::app::editor_runtime) struct EditorWindowCloseRequestContext {
    pub(in crate::app::editor_runtime) editor_window_instance: ApplicationWindow,
    pub(in crate::app::editor_runtime) runtime_session: Rc<RefCell<RuntimeSession>>,
    pub(in crate::app::editor_runtime) shared_machine: Rc<RefCell<StateMachine>>,
    pub(in crate::app::editor_runtime) storage_service: Rc<Option<StorageService>>,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) close_editor_button: Button,
    pub(in crate::app::editor_runtime) editor_has_unsaved_changes: Rc<RefCell<bool>>,
    pub(in crate::app::editor_runtime) editor_close_dialog_open: Rc<RefCell<bool>>,
    pub(in crate::app::editor_runtime) editor_window_for_dialog: ApplicationWindow,
    pub(in crate::app::editor_runtime) editor_toast_runtime: ToastRuntime,
    pub(in crate::app::editor_runtime) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(in crate::app::editor_runtime) pending_crop_for_close: Rc<RefCell<Option<CropElement>>>,
    pub(in crate::app::editor_runtime) editor_source_pixbuf: Option<gtk4::gdk_pixbuf::Pixbuf>,
    pub(in crate::app::editor_runtime) style_tokens: StyleTokens,
    pub(in crate::app::editor_runtime) editor_close_guard: Rc<Cell<bool>>,
}

pub(in crate::app::editor_runtime) fn connect_editor_window_close_request(
    context: EditorWindowCloseRequestContext,
) {
    let close_runtime = EditorCloseRequestRuntime {
        runtime_session: context.runtime_session.clone(),
        shared_machine: context.shared_machine.clone(),
        storage_service: context.storage_service.clone(),
        status_log_for_render: context.status_log_for_render.clone(),
        close_editor_button: context.close_editor_button.clone(),
        editor_has_unsaved_changes: context.editor_has_unsaved_changes.clone(),
        editor_close_dialog_open: context.editor_close_dialog_open.clone(),
        editor_window_for_dialog: context.editor_window_for_dialog.clone(),
        editor_toast_runtime: context.editor_toast_runtime.clone(),
        editor_tools: context.editor_tools.clone(),
        pending_crop_for_close: context.pending_crop_for_close.clone(),
        editor_source_pixbuf: context.editor_source_pixbuf.clone(),
        style_tokens: context.style_tokens,
    };
    let editor_close_guard = context.editor_close_guard.clone();
    context
        .editor_window_instance
        .connect_close_request(move |_| {
            if editor_close_guard.get() {
                return gtk4::glib::Propagation::Proceed;
            }
            match handle_editor_close_request(&close_runtime, EditorCloseOrigin::WindowCloseRequest)
            {
                EditorCloseOutcome::AllowWindowClose => gtk4::glib::Propagation::Proceed,
                EditorCloseOutcome::KeepWindowOpen => gtk4::glib::Propagation::Stop,
            }
        });
}

use super::*;

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
    let runtime_session = context.runtime_session.clone();
    let shared_machine = context.shared_machine.clone();
    let storage_service = context.storage_service.clone();
    let status_log_for_render = context.status_log_for_render.clone();
    let close_editor_button = context.close_editor_button.clone();
    let editor_has_unsaved_changes = context.editor_has_unsaved_changes.clone();
    let editor_close_dialog_open = context.editor_close_dialog_open.clone();
    let editor_window_for_dialog = context.editor_window_for_dialog.clone();
    let editor_toast_runtime = context.editor_toast_runtime.clone();
    let editor_tools = context.editor_tools.clone();
    let pending_crop_for_close = context.pending_crop_for_close.clone();
    let editor_source_pixbuf = context.editor_source_pixbuf.clone();
    let style_tokens = context.style_tokens;

    context.editor_close_button.connect_clicked(move |_| {
        if !matches!(shared_machine.borrow().state(), AppState::Editor) {
            *status_log_for_render.borrow_mut() =
                "editor close requested outside editor state; closing window directly".to_string();
            editor_window_for_dialog.close();
            return;
        }

        if !*editor_has_unsaved_changes.borrow() {
            close_editor_button.emit_clicked();
            return;
        }

        let active_capture = match runtime_session.borrow().active_capture().cloned() {
            Some(artifact) => artifact,
            None => {
                *status_log_for_render.borrow_mut() =
                    "editor close requires an active capture".to_string();
                return;
            }
        };

        let Some(service) = storage_service.as_ref().clone() else {
            *status_log_for_render.borrow_mut() = "storage service unavailable".to_string();
            return;
        };

        if *editor_close_dialog_open.borrow() {
            return;
        }

        *editor_close_dialog_open.borrow_mut() = true;
        let dialog = Dialog::new();
        dialog.add_css_class("chalkak-root");
        dialog.set_title(Some("Unsaved edits"));
        dialog.set_transient_for(Some(&editor_window_for_dialog));
        dialog.set_modal(true);
        dialog.set_destroy_with_parent(true);
        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Don't Save", ResponseType::Reject);
        dialog.add_button("Save and Close", ResponseType::Accept);
        dialog.set_default_response(ResponseType::Accept);
        let body = Label::new(Some("You have unused edits.\nSave before closing?"));
        body.set_xalign(0.5);
        body.set_justify(gtk4::Justification::Center);
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
                        dialog_open: true,
                        ..Default::default()
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

        let runtime_session = runtime_session.clone();
        let status_log_for_render = status_log_for_render.clone();
        let close_editor_button = close_editor_button.clone();
        let editor_has_unsaved_changes = editor_has_unsaved_changes.clone();
        let editor_close_dialog_open = editor_close_dialog_open.clone();
        let capture_id = active_capture.capture_id.clone();
        let editor_toast_runtime = editor_toast_runtime.clone();
        let editor_tools = editor_tools.clone();
        let editor_source_pixbuf = editor_source_pixbuf.clone();
        let pending_crop = pending_crop_for_close.clone();
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
                        close_editor_button.emit_clicked();
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
                        close_editor_button.emit_clicked();
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
    });
}

#[derive(Clone)]
pub(in crate::app::editor_runtime) struct EditorWindowCloseRequestContext {
    pub(in crate::app::editor_runtime) editor_window_instance: ApplicationWindow,
    pub(in crate::app::editor_runtime) editor_close_button: Button,
    pub(in crate::app::editor_runtime) shared_machine: Rc<RefCell<StateMachine>>,
    pub(in crate::app::editor_runtime) editor_close_guard: Rc<Cell<bool>>,
}

pub(in crate::app::editor_runtime) fn connect_editor_window_close_request(
    context: EditorWindowCloseRequestContext,
) {
    let editor_close_button = context.editor_close_button.clone();
    let shared_machine = context.shared_machine.clone();
    let editor_close_guard = context.editor_close_guard.clone();
    context
        .editor_window_instance
        .connect_close_request(move |_| {
            if editor_close_guard.get() {
                return gtk4::glib::Propagation::Proceed;
            }

            let in_editor_state = matches!(shared_machine.borrow().state(), AppState::Editor);
            if !in_editor_state {
                return gtk4::glib::Propagation::Proceed;
            }

            editor_close_button.emit_clicked();

            // If close handling transitioned out of Editor, allow this original
            // close request to proceed so the first CMD+w actually closes.
            if matches!(shared_machine.borrow().state(), AppState::Editor) {
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
}

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::editor::tools::CropElement;
use crate::editor::{self, ToolKind, ToolObject};
use crate::input::{
    resolve_shortcut, InputContext, InputMode, ShortcutAction, ShortcutKey, TextInputAction,
    TextInputEvent,
};

use gtk4::prelude::*;
use gtk4::{Button, DrawingArea, Label, Overlay, Scale, ScrolledWindow};

use crate::app::editor_history::{record_undo_snapshot, snapshot_editor_objects};
use crate::app::editor_popup::{clear_selection, copy_active_text_to_clipboard, TextPreeditState};
use crate::app::editor_text_runtime::{
    handle_editor_text_commit, handle_editor_text_key_action, handle_editor_text_preedit_changed,
    EditorTextCommitContext, EditorTextKeyContext, EditorTextPreeditContext,
};
use crate::app::input_bridge::{
    key_name, modifier_state, normalize_shortcut_key, resolve_text_input_event, shortcut_modifiers,
};
use crate::app::{shortcut_editor_tool_switch, EditorToolSwitchContext, TextInputActivation};

use super::tools::switch_editor_tool_with_text_policy;
use super::viewport::{
    editor_viewport_runtime, handle_editor_viewport_shortcuts, EditorViewportShortcutContext,
};

struct EditorShortcutActionContext {
    editor_undo_button: Button,
    editor_redo_button: Button,
    editor_save_button: Button,
    editor_copy_button: Button,
    tool_options_toggle_button: Button,
    editor_close_button: Button,
    editor_tool_switch: EditorToolSwitchContext,
    active_editor_tool: Rc<Cell<ToolKind>>,
    pending_crop: Rc<RefCell<Option<CropElement>>>,
    selected_object_ids: Rc<RefCell<Vec<u64>>>,
    editor_tools: Rc<RefCell<editor::EditorTools>>,
    editor_undo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    editor_redo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    editor_has_unsaved_changes: Rc<RefCell<bool>>,
    status_log_for_render: Rc<RefCell<String>>,
    editor_canvas: DrawingArea,
}

fn handle_editor_shortcut_action(
    action: ShortcutAction,
    context: &EditorShortcutActionContext,
) -> gtk4::glib::Propagation {
    match action {
        ShortcutAction::DialogConfirm | ShortcutAction::DialogCancel => {}
        ShortcutAction::EditorSave => {
            context.editor_save_button.emit_clicked();
        }
        ShortcutAction::EditorCopyImage => {
            context.editor_copy_button.emit_clicked();
        }
        ShortcutAction::EditorToggleToolOptions => {
            context.tool_options_toggle_button.emit_clicked();
        }
        ShortcutAction::EditorCloseRequested => {
            context.editor_close_button.emit_clicked();
        }
        ShortcutAction::CropApply => {
            *context.status_log_for_render.borrow_mut() =
                "crop frame is applied on Save/Copy".to_string();
        }
        ShortcutAction::CropCancel => {
            context.editor_tool_switch.switch_to(ToolKind::Select, true);
            *context.status_log_for_render.borrow_mut() =
                "crop canceled via Esc; switched to Select".to_string();
            context.editor_canvas.queue_draw();
        }
        ShortcutAction::EditorUndo => {
            context.editor_undo_button.emit_clicked();
        }
        ShortcutAction::EditorRedo => {
            context.editor_redo_button.emit_clicked();
        }
        ShortcutAction::EditorDeleteSelection => {
            let selected_id = context.selected_object_ids.borrow().first().copied();
            let Some(selected_id) = selected_id else {
                *context.status_log_for_render.borrow_mut() = "no selection to delete".to_string();
                return gtk4::glib::Propagation::Stop;
            };
            let snapshot = snapshot_editor_objects(context.editor_tools.as_ref());
            let removed_object = context.editor_tools.borrow_mut().remove_object(selected_id);
            if let Some(removed_object) = removed_object {
                record_undo_snapshot(
                    context.editor_undo_stack.as_ref(),
                    context.editor_redo_stack.as_ref(),
                    snapshot,
                );
                clear_selection(&context.selected_object_ids);
                if matches!(removed_object, ToolObject::Text(_)) {
                    switch_editor_tool_with_text_policy(
                        &context.editor_tool_switch,
                        context.active_editor_tool.get(),
                        false,
                        TextInputActivation::ForceOff,
                    );
                }
                if matches!(removed_object, ToolObject::Crop(_)) {
                    context.pending_crop.borrow_mut().take();
                }
                *context.editor_has_unsaved_changes.borrow_mut() = true;
                *context.status_log_for_render.borrow_mut() =
                    format!("deleted object #{selected_id}");
                context.editor_canvas.queue_draw();
            } else {
                clear_selection(&context.selected_object_ids);
                *context.status_log_for_render.borrow_mut() =
                    "selected object not found".to_string();
            }
        }
        ShortcutAction::TextInsertLineBreak => {
            let snapshot = snapshot_editor_objects(context.editor_tools.as_ref());
            let input_action = context
                .editor_tools
                .borrow_mut()
                .apply_text_input(TextInputEvent::Enter);
            if matches!(input_action, TextInputAction::InsertLineBreak) {
                record_undo_snapshot(
                    context.editor_undo_stack.as_ref(),
                    context.editor_redo_stack.as_ref(),
                    snapshot,
                );
                *context.editor_has_unsaved_changes.borrow_mut() = true;
                *context.status_log_for_render.borrow_mut() =
                    "text line break inserted".to_string();
                context.editor_canvas.queue_draw();
            }
        }
        ShortcutAction::TextCommit | ShortcutAction::TextExitFocus => {
            let text_event = if matches!(action, ShortcutAction::TextCommit) {
                TextInputEvent::CtrlEnter
            } else {
                TextInputEvent::Escape
            };
            let _ = context
                .editor_tools
                .borrow_mut()
                .apply_text_input(text_event);
            switch_editor_tool_with_text_policy(
                &context.editor_tool_switch,
                ToolKind::Text,
                false,
                TextInputActivation::ForceOff,
            );
            *context.status_log_for_render.borrow_mut() = "text editing completed".to_string();
            context.editor_canvas.queue_draw();
        }
        ShortcutAction::TextCopySelection => {
            let _ = copy_active_text_to_clipboard(
                &context.editor_tools,
                &context.status_log_for_render,
            );
        }
        _ => return gtk4::glib::Propagation::Proceed,
    }

    gtk4::glib::Propagation::Stop
}

pub(in crate::app::editor_runtime) struct EditorKeyHandlingContext {
    pub(in crate::app::editor_runtime) editor_overlay: Overlay,
    pub(in crate::app::editor_runtime) editor_canvas: DrawingArea,
    pub(in crate::app::editor_runtime) editor_scroller: ScrolledWindow,
    pub(in crate::app::editor_runtime) editor_viewport: Rc<RefCell<editor::EditorViewport>>,
    pub(in crate::app::editor_runtime) editor_viewport_status: Label,
    pub(in crate::app::editor_runtime) zoom_slider: Scale,
    pub(in crate::app::editor_runtime) zoom_slider_syncing: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) schedule_fit_settle_pass: Rc<dyn Fn(&'static str)>,
    pub(in crate::app::editor_runtime) editor_navigation_bindings:
        Rc<crate::input::EditorNavigationBindings>,
    pub(in crate::app::editor_runtime) editor_close_dialog_open: Rc<RefCell<bool>>,
    pub(in crate::app::editor_runtime) editor_input_mode: Rc<RefCell<editor::EditorInputMode>>,
    pub(in crate::app::editor_runtime) text_im_context: Rc<gtk4::IMMulticontext>,
    pub(in crate::app::editor_runtime) text_preedit_state: Rc<RefCell<TextPreeditState>>,
    pub(in crate::app::editor_runtime) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(in crate::app::editor_runtime) editor_undo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    pub(in crate::app::editor_runtime) editor_redo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    pub(in crate::app::editor_runtime) editor_has_unsaved_changes: Rc<RefCell<bool>>,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) editor_undo_button: Button,
    pub(in crate::app::editor_runtime) editor_redo_button: Button,
    pub(in crate::app::editor_runtime) editor_save_button: Button,
    pub(in crate::app::editor_runtime) editor_copy_button: Button,
    pub(in crate::app::editor_runtime) tool_options_toggle_button: Button,
    pub(in crate::app::editor_runtime) editor_close_button: Button,
    pub(in crate::app::editor_runtime) editor_tool_switch_context: EditorToolSwitchContext,
    pub(in crate::app::editor_runtime) active_editor_tool: Rc<Cell<ToolKind>>,
    pub(in crate::app::editor_runtime) pending_crop: Rc<RefCell<Option<CropElement>>>,
    pub(in crate::app::editor_runtime) selected_object_ids: Rc<RefCell<Vec<u64>>>,
    pub(in crate::app::editor_runtime) space_pan_pressed: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) drag_pan_active: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) drag_pan_pointer_origin: Rc<Cell<(f64, f64)>>,
    pub(in crate::app::editor_runtime) refresh_editor_cursor: Rc<dyn Fn()>,
    pub(in crate::app::editor_runtime) editor_image_base_width: i32,
    pub(in crate::app::editor_runtime) editor_image_base_height: i32,
}

pub(in crate::app::editor_runtime) fn connect_editor_key_handling(
    context: EditorKeyHandlingContext,
) {
    let editor_undo_button = context.editor_undo_button.clone();
    let editor_redo_button = context.editor_redo_button.clone();
    let editor_save_button = context.editor_save_button.clone();
    let editor_copy_button = context.editor_copy_button.clone();
    let tool_options_toggle_button = context.tool_options_toggle_button.clone();
    let editor_close_button = context.editor_close_button.clone();
    let editor_close_dialog_open = context.editor_close_dialog_open.clone();
    let editor_input_mode = context.editor_input_mode.clone();
    let editor_tools = context.editor_tools.clone();
    let editor_undo_stack = context.editor_undo_stack.clone();
    let editor_redo_stack = context.editor_redo_stack.clone();
    let active_editor_tool = context.active_editor_tool.clone();
    let pending_crop = context.pending_crop.clone();
    let selected_object_ids = context.selected_object_ids.clone();
    let status_log_for_render = context.status_log_for_render.clone();
    let space_pan_pressed = context.space_pan_pressed.clone();
    let drag_pan_active = context.drag_pan_active.clone();
    let drag_pan_pointer_origin = context.drag_pan_pointer_origin.clone();
    let editor_navigation_bindings = context.editor_navigation_bindings.clone();
    let editor_viewport = context.editor_viewport.clone();
    let editor_canvas = context.editor_canvas.clone();
    let editor_scroller = context.editor_scroller.clone();
    let editor_has_unsaved_changes = context.editor_has_unsaved_changes.clone();
    let editor_viewport_status = context.editor_viewport_status.clone();
    let zoom_slider = context.zoom_slider.clone();
    let zoom_slider_syncing = context.zoom_slider_syncing.clone();
    let space_pan_pressed_for_press = space_pan_pressed.clone();
    let editor_navigation_bindings_for_press = editor_navigation_bindings.clone();
    let text_im_context = context.text_im_context.clone();
    let text_preedit_state = context.text_preedit_state.clone();
    let refresh_editor_cursor = context.refresh_editor_cursor.clone();
    let editor_tool_switch_context = context.editor_tool_switch_context.clone();
    let schedule_fit_settle_pass = context.schedule_fit_settle_pass.clone();
    let editor_image_base_width = context.editor_image_base_width;
    let editor_image_base_height = context.editor_image_base_height;

    {
        let editor_tools = editor_tools.clone();
        let editor_undo_stack = editor_undo_stack.clone();
        let editor_redo_stack = editor_redo_stack.clone();
        let editor_has_unsaved_changes = editor_has_unsaved_changes.clone();
        let status_log_for_render = status_log_for_render.clone();
        let editor_canvas = editor_canvas.clone();
        let editor_input_mode = editor_input_mode.clone();
        let text_preedit_state = text_preedit_state.clone();
        text_im_context.connect_commit(move |_, committed| {
            let context = EditorTextCommitContext {
                editor_tools: &editor_tools,
                editor_undo_stack: editor_undo_stack.as_ref(),
                editor_redo_stack: editor_redo_stack.as_ref(),
                editor_has_unsaved_changes: editor_has_unsaved_changes.as_ref(),
                status_log_for_render: &status_log_for_render,
                editor_canvas: &editor_canvas,
                editor_input_mode: editor_input_mode.as_ref(),
                text_preedit_state: text_preedit_state.as_ref(),
            };
            handle_editor_text_commit(committed, &context);
        });
    }
    {
        let editor_canvas = editor_canvas.clone();
        let editor_input_mode = editor_input_mode.clone();
        let text_preedit_state = text_preedit_state.clone();
        text_im_context.connect_preedit_changed(move |context| {
            let (preedit, _, cursor_index) = context.preedit_string();
            let context = EditorTextPreeditContext {
                editor_input_mode: editor_input_mode.as_ref(),
                text_preedit_state: text_preedit_state.as_ref(),
                editor_canvas: &editor_canvas,
            };
            handle_editor_text_preedit_changed(preedit.as_str(), cursor_index, &context);
        });
    }

    let key_controller = gtk4::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let im_key_controller = gtk4::EventControllerKey::new();
    im_key_controller.set_im_context(Some(text_im_context.as_ref()));
    im_key_controller.set_propagation_phase(gtk4::PropagationPhase::Bubble);
    editor_canvas.add_controller(im_key_controller);
    let refresh_editor_cursor_for_press = refresh_editor_cursor.clone();
    let editor_tool_switch_for_press = editor_tool_switch_context.clone();
    let shortcut_action_context = EditorShortcutActionContext {
        editor_undo_button: editor_undo_button.clone(),
        editor_redo_button: editor_redo_button.clone(),
        editor_save_button: editor_save_button.clone(),
        editor_copy_button: editor_copy_button.clone(),
        tool_options_toggle_button: tool_options_toggle_button.clone(),
        editor_close_button: editor_close_button.clone(),
        editor_tool_switch: editor_tool_switch_for_press.clone(),
        active_editor_tool: active_editor_tool.clone(),
        pending_crop: pending_crop.clone(),
        selected_object_ids: selected_object_ids.clone(),
        editor_tools: editor_tools.clone(),
        editor_undo_stack: editor_undo_stack.clone(),
        editor_redo_stack: editor_redo_stack.clone(),
        editor_has_unsaved_changes: editor_has_unsaved_changes.clone(),
        status_log_for_render: status_log_for_render.clone(),
        editor_canvas: editor_canvas.clone(),
    };
    key_controller.connect_key_pressed(move |_, key, keycode, modifier| {
        let mode = *editor_input_mode.borrow();
        let key_name = key_name(key);
        if mode.text_input_active() {
            text_im_context.focus_in();
            if let Some(text_event) = resolve_text_input_event(key, modifier) {
                let context = EditorTextKeyContext {
                    editor_tools: &editor_tools,
                    editor_undo_stack: editor_undo_stack.as_ref(),
                    editor_redo_stack: editor_redo_stack.as_ref(),
                    editor_has_unsaved_changes: editor_has_unsaved_changes.as_ref(),
                    status_log_for_render: &status_log_for_render,
                    editor_canvas: &editor_canvas,
                    editor_input_mode: editor_input_mode.as_ref(),
                    text_im_context: text_im_context.as_ref(),
                    text_preedit_state: text_preedit_state.as_ref(),
                };
                return handle_editor_text_key_action(text_event, &context);
            }
            if let Some(ShortcutKey::Tab) = normalize_shortcut_key(key, keycode) {
                return handle_editor_shortcut_action(
                    ShortcutAction::EditorToggleToolOptions,
                    &shortcut_action_context,
                );
            }
            return gtk4::glib::Propagation::Proceed;
        }
        if !mode.text_input_active()
            && editor_navigation_bindings_for_press.matches_pan_hold_key_name(key_name.as_deref())
        {
            space_pan_pressed_for_press.set(true);
            refresh_editor_cursor_for_press();
            return gtk4::glib::Propagation::Stop;
        }
        let state = modifier_state(modifier);
        let viewport_runtime = editor_viewport_runtime(
            &editor_canvas,
            &editor_scroller,
            &editor_viewport_status,
            &zoom_slider,
            zoom_slider_syncing.as_ref(),
            editor_image_base_width,
            editor_image_base_height,
        );
        if handle_editor_viewport_shortcuts(
            key_name.as_deref(),
            state,
            editor_navigation_bindings_for_press.as_ref(),
            EditorViewportShortcutContext {
                editor_viewport: editor_viewport.as_ref(),
                editor_scroller: &editor_scroller,
                viewport_runtime: &viewport_runtime,
                schedule_fit_settle_pass: schedule_fit_settle_pass.as_ref(),
                status_log_for_render: status_log_for_render.as_ref(),
            },
        ) {
            return gtk4::glib::Propagation::Stop;
        }
        let Some(shortcut_key) = normalize_shortcut_key(key, keycode) else {
            return gtk4::glib::Propagation::Proceed;
        };

        let shortcut = resolve_shortcut(
            shortcut_key,
            shortcut_modifiers(modifier),
            InputContext {
                mode: if *editor_close_dialog_open.borrow() {
                    InputMode::Dialog
                } else if mode.text_input_active() {
                    InputMode::TextInput
                } else if mode.crop_active() {
                    InputMode::Crop
                } else {
                    InputMode::Editor {
                        select_mode: active_editor_tool.get() == ToolKind::Select,
                    }
                },
            },
        );

        let Some(action) = shortcut else {
            return gtk4::glib::Propagation::Proceed;
        };

        if let Some((selected_tool, status_message)) = shortcut_editor_tool_switch(action) {
            editor_tool_switch_for_press.switch_to(selected_tool, true);
            *status_log_for_render.borrow_mut() = status_message.to_string();
            editor_canvas.queue_draw();
            return gtk4::glib::Propagation::Stop;
        }
        handle_editor_shortcut_action(action, &shortcut_action_context)
    });

    key_controller.connect_key_released({
        let refresh_editor_cursor = refresh_editor_cursor.clone();
        move |_, key, _, _| {
            let key_name = key_name(key);
            if editor_navigation_bindings.matches_pan_hold_key_name(key_name.as_deref()) {
                space_pan_pressed.set(false);
                drag_pan_active.set(false);
                drag_pan_pointer_origin.set((0.0, 0.0));
                refresh_editor_cursor();
            }
        }
    });
    context.editor_overlay.add_controller(key_controller);
}

use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::{self, ToolObject};
use crate::input::{TextInputAction, TextInputEvent};
use gtk4::prelude::*;
use gtk4::DrawingArea;

use super::editor_history::{record_undo_snapshot_if_some, snapshot_active_text_objects};
use super::editor_popup::{
    copy_active_text_to_clipboard, preedit_cursor_char_index, TextPreeditState,
};
use super::stop_editor_text_input;

pub(super) struct EditorTextCommitContext<'a> {
    pub(super) editor_tools: &'a Rc<RefCell<editor::EditorTools>>,
    pub(super) editor_undo_stack: &'a RefCell<Vec<Vec<ToolObject>>>,
    pub(super) editor_redo_stack: &'a RefCell<Vec<Vec<ToolObject>>>,
    pub(super) editor_has_unsaved_changes: &'a RefCell<bool>,
    pub(super) status_log_for_render: &'a Rc<RefCell<String>>,
    pub(super) editor_canvas: &'a DrawingArea,
    pub(super) editor_input_mode: &'a RefCell<editor::EditorInputMode>,
    pub(super) text_preedit_state: &'a RefCell<TextPreeditState>,
}

pub(super) fn handle_editor_text_commit(committed: &str, context: &EditorTextCommitContext<'_>) {
    if committed.is_empty() || !context.editor_input_mode.borrow().text_input_active() {
        return;
    }

    let snapshot = snapshot_active_text_objects(context.editor_tools.as_ref());
    if snapshot.is_none() {
        return;
    }

    let mut inserted = String::new();
    {
        let mut tools = context.editor_tools.borrow_mut();
        for character in committed.chars() {
            if matches!(
                tools.apply_text_input(TextInputEvent::Character(character)),
                TextInputAction::InsertCharacter(_)
            ) {
                inserted.push(character);
            }
        }
    }

    if inserted.is_empty() {
        return;
    }

    record_undo_snapshot_if_some(
        context.editor_undo_stack,
        context.editor_redo_stack,
        snapshot,
    );
    *context.text_preedit_state.borrow_mut() = TextPreeditState::default();
    *context.editor_has_unsaved_changes.borrow_mut() = true;
    *context.status_log_for_render.borrow_mut() = format!("text inserted: {inserted}");
    context.editor_canvas.queue_draw();
}

pub(super) struct EditorTextKeyContext<'a> {
    pub(super) editor_tools: &'a Rc<RefCell<editor::EditorTools>>,
    pub(super) editor_undo_stack: &'a RefCell<Vec<Vec<ToolObject>>>,
    pub(super) editor_redo_stack: &'a RefCell<Vec<Vec<ToolObject>>>,
    pub(super) editor_has_unsaved_changes: &'a RefCell<bool>,
    pub(super) status_log_for_render: &'a Rc<RefCell<String>>,
    pub(super) editor_canvas: &'a DrawingArea,
    pub(super) editor_input_mode: &'a RefCell<editor::EditorInputMode>,
    pub(super) text_im_context: &'a gtk4::IMMulticontext,
    pub(super) text_preedit_state: &'a RefCell<TextPreeditState>,
}

pub(super) struct EditorTextPreeditContext<'a> {
    pub(super) editor_input_mode: &'a RefCell<editor::EditorInputMode>,
    pub(super) text_preedit_state: &'a RefCell<TextPreeditState>,
    pub(super) editor_canvas: &'a DrawingArea,
}

pub(super) fn handle_editor_text_preedit_changed(
    preedit: &str,
    cursor_index: i32,
    context: &EditorTextPreeditContext<'_>,
) {
    if !context.editor_input_mode.borrow().text_input_active() {
        *context.text_preedit_state.borrow_mut() = TextPreeditState::default();
        return;
    }

    let cursor_chars = preedit_cursor_char_index(preedit, cursor_index);
    *context.text_preedit_state.borrow_mut() = TextPreeditState {
        content: preedit.to_string(),
        cursor_chars,
    };
    context.editor_canvas.queue_draw();
}

pub(super) fn handle_editor_text_key_action(
    text_event: TextInputEvent,
    context: &EditorTextKeyContext<'_>,
) -> gtk4::glib::Propagation {
    let snapshot = if matches!(
        text_event,
        TextInputEvent::Character(_)
            | TextInputEvent::Backspace
            | TextInputEvent::Enter
            | TextInputEvent::ShiftEnter
    ) {
        snapshot_active_text_objects(context.editor_tools.as_ref())
    } else {
        None
    };

    let action = context
        .editor_tools
        .borrow_mut()
        .apply_text_input(text_event);
    match action {
        TextInputAction::InsertCharacter(c) => {
            record_undo_snapshot_if_some(
                context.editor_undo_stack,
                context.editor_redo_stack,
                snapshot,
            );
            *context.editor_has_unsaved_changes.borrow_mut() = true;
            *context.status_log_for_render.borrow_mut() = format!("text inserted: {c}");
            context.editor_canvas.queue_draw();
        }
        TextInputAction::DeleteBackward => {
            record_undo_snapshot_if_some(
                context.editor_undo_stack,
                context.editor_redo_stack,
                snapshot,
            );
            *context.editor_has_unsaved_changes.borrow_mut() = true;
            *context.status_log_for_render.borrow_mut() = "text delete backward".to_string();
            context.editor_canvas.queue_draw();
        }
        TextInputAction::InsertLineBreak => {
            record_undo_snapshot_if_some(
                context.editor_undo_stack,
                context.editor_redo_stack,
                snapshot,
            );
            *context.editor_has_unsaved_changes.borrow_mut() = true;
            *context.status_log_for_render.borrow_mut() = "text line break inserted".to_string();
            context.editor_canvas.queue_draw();
        }
        TextInputAction::MoveCursor => {
            *context.status_log_for_render.borrow_mut() = "text cursor moved".to_string();
            context.editor_canvas.queue_draw();
        }
        TextInputAction::Commit | TextInputAction::ExitFocus => {
            stop_editor_text_input(
                context.editor_input_mode,
                context.text_im_context,
                context.text_preedit_state,
            );
            *context.status_log_for_render.borrow_mut() = "text editing completed".to_string();
            context.editor_canvas.queue_draw();
        }
        TextInputAction::CopyRequested => {
            let _ =
                copy_active_text_to_clipboard(context.editor_tools, context.status_log_for_render);
        }
        TextInputAction::NoTextTarget | TextInputAction::NoAction => {}
    }

    gtk4::glib::Propagation::Stop
}

use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::{self, ToolKind};
use gtk4::prelude::*;
use gtk4::Button;

pub(in crate::app) fn tool_kind_label(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Select => "Select",
        ToolKind::Pan => "Pan",
        ToolKind::Blur => "Blur",
        ToolKind::Pen => "Pen",
        ToolKind::Arrow => "Arrow",
        ToolKind::Rectangle => "Rect",
        ToolKind::Crop => "Crop",
        ToolKind::Text => "Text",
    }
}

pub(in crate::app) fn sync_active_tool_buttons(
    buttons: &[(ToolKind, Button)],
    active_tool: ToolKind,
) {
    for (tool, button) in buttons {
        if *tool == active_tool {
            button.add_css_class("tool-active");
        } else {
            button.remove_css_class("tool-active");
        }
    }
}

pub(in crate::app) fn ensure_selected_object_exists(
    tools: &editor::EditorTools,
    selected_object_ids: &Rc<RefCell<Vec<u64>>>,
) {
    selected_object_ids
        .borrow_mut()
        .retain(|selected_id| tools.object(*selected_id).is_some());
}

pub(in crate::app) fn set_single_selection(
    selected_object_ids: &Rc<RefCell<Vec<u64>>>,
    object_id: u64,
) {
    let mut selected = selected_object_ids.borrow_mut();
    selected.clear();
    selected.push(object_id);
}

pub(in crate::app) fn set_optional_single_selection(
    selected_object_ids: &Rc<RefCell<Vec<u64>>>,
    object_id: Option<u64>,
) {
    let mut selected = selected_object_ids.borrow_mut();
    selected.clear();
    if let Some(id) = object_id {
        selected.push(id);
    }
}

pub(in crate::app) fn clear_selection(selected_object_ids: &Rc<RefCell<Vec<u64>>>) {
    selected_object_ids.borrow_mut().clear();
}

pub(in crate::app) fn is_object_selected(selected_object_ids: &[u64], object_id: u64) -> bool {
    selected_object_ids.contains(&object_id)
}

pub(in crate::app) fn copy_active_text_to_clipboard(
    editor_tools: &Rc<RefCell<editor::EditorTools>>,
    status_log_for_render: &Rc<RefCell<String>>,
) -> bool {
    let Some(content) = editor_tools
        .borrow()
        .active_text_focus_content()
        .filter(|content| !content.is_empty())
        .map(|content| content.to_string())
    else {
        *status_log_for_render.borrow_mut() = "text copy requested with no active text".to_string();
        return false;
    };

    let Some(display) = gtk4::gdk::Display::default() else {
        *status_log_for_render.borrow_mut() = "text copy failed: no active display".to_string();
        return false;
    };
    display.clipboard().set_text(&content);
    *status_log_for_render.borrow_mut() =
        format!("text copied ({} chars)", content.chars().count());
    true
}

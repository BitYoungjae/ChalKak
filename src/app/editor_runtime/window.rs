use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::{self, ToolKind};
use crate::input::{resolve_shortcut, InputContext, InputMode, ShortcutKey};

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, Overlay, Revealer};

use crate::app::input_bridge::normalize_shortcut_key;
use crate::app::shortcut_editor_tool_switch;
use crate::app::window_state::RuntimeWindowGeometry;
use crate::ui::StyleTokens;

pub(super) fn build_editor_window_shell(
    app: &Application,
    saved_geometry: Option<RuntimeWindowGeometry>,
    style_tokens: StyleTokens,
    capture_id: &str,
) -> (ApplicationWindow, String, RuntimeWindowGeometry) {
    let editor_window_instance = ApplicationWindow::new(app);
    let editor_title = format!("Editor - {capture_id}");
    editor_window_instance.set_title(Some(&editor_title));
    editor_window_instance.set_decorated(false);
    editor_window_instance.add_css_class("chalkak-root");
    editor_window_instance.add_css_class("floating-editor-window");
    let geometry = saved_geometry.unwrap_or(RuntimeWindowGeometry::new(
        style_tokens.editor_initial_width,
        style_tokens.editor_initial_height,
    ));
    let resolved_geometry = RuntimeWindowGeometry::with_position(
        geometry.x,
        geometry.y,
        geometry.width.max(style_tokens.editor_min_width),
        geometry.height.max(style_tokens.editor_min_height),
    );
    editor_window_instance.set_default_size(resolved_geometry.width, resolved_geometry.height);
    editor_window_instance.set_resizable(true);
    editor_window_instance.set_size_request(
        style_tokens.editor_min_width,
        style_tokens.editor_min_height,
    );
    (editor_window_instance, editor_title, resolved_geometry)
}

fn resolve_editor_tool_fallback_shortcut(shortcut_key: ShortcutKey) -> Option<ToolKind> {
    if !matches!(shortcut_key, ShortcutKey::Character(_)) {
        return None;
    }
    let action = resolve_shortcut(
        shortcut_key,
        crate::input::ShortcutModifiers::default(),
        InputContext {
            mode: InputMode::Editor { select_mode: false },
        },
    )?;
    shortcut_editor_tool_switch(action).map(|(tool, _)| tool)
}

pub(super) fn connect_editor_tool_shortcut_fallback(
    editor_window_instance: &ApplicationWindow,
    tool_buttons: &Rc<RefCell<Vec<(ToolKind, Button)>>>,
    editor_input_mode: &Rc<RefCell<editor::EditorInputMode>>,
) {
    let tool_buttons = tool_buttons.clone();
    let editor_input_mode = editor_input_mode.clone();
    let tool_shortcut_controller = gtk4::EventControllerKey::new();
    tool_shortcut_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    tool_shortcut_controller.connect_key_pressed(move |_, key, keycode, modifier| {
        if editor_input_mode.borrow().text_input_active()
            || modifier.intersects(
                gtk4::gdk::ModifierType::CONTROL_MASK
                    | gtk4::gdk::ModifierType::ALT_MASK
                    | gtk4::gdk::ModifierType::SUPER_MASK
                    | gtk4::gdk::ModifierType::META_MASK,
            )
        {
            return gtk4::glib::Propagation::Proceed;
        }
        let Some(shortcut_key) = normalize_shortcut_key(key, keycode) else {
            return gtk4::glib::Propagation::Proceed;
        };
        let Some(tool) = resolve_editor_tool_fallback_shortcut(shortcut_key) else {
            return gtk4::glib::Propagation::Proceed;
        };
        let maybe_button = {
            let buttons = tool_buttons.borrow();
            buttons
                .iter()
                .find(|(kind, _)| *kind == tool)
                .map(|(_, button)| button.clone())
        };
        if let Some(button) = maybe_button {
            tracing::debug!(
                tool = ?tool,
                "editor tool shortcut fallback triggered"
            );
            button.emit_clicked();
            return gtk4::glib::Propagation::Stop;
        }
        gtk4::glib::Propagation::Proceed
    });
    editor_window_instance.add_controller(tool_shortcut_controller);
}

pub(super) fn connect_editor_overlay_hover_controls(
    editor_overlay: &Overlay,
    top_controls_left_revealer: &Revealer,
    top_controls_right_revealer: &Revealer,
    bottom_left_controls_revealer: &Revealer,
    bottom_right_controls_revealer: &Revealer,
) {
    let pointer = gtk4::EventControllerMotion::new();
    {
        let top_controls_left_revealer = top_controls_left_revealer.clone();
        let top_controls_right_revealer = top_controls_right_revealer.clone();
        let bottom_left_controls_revealer = bottom_left_controls_revealer.clone();
        let bottom_right_controls_revealer = bottom_right_controls_revealer.clone();
        pointer.connect_enter(move |_, _, _| {
            top_controls_left_revealer.set_reveal_child(true);
            top_controls_left_revealer.set_can_target(true);
            top_controls_right_revealer.set_reveal_child(true);
            top_controls_right_revealer.set_can_target(true);
            bottom_left_controls_revealer.set_reveal_child(true);
            bottom_left_controls_revealer.set_can_target(true);
            bottom_right_controls_revealer.set_reveal_child(true);
            bottom_right_controls_revealer.set_can_target(true);
        });
    }
    {
        let top_controls_left_revealer = top_controls_left_revealer.clone();
        let top_controls_right_revealer = top_controls_right_revealer.clone();
        let bottom_left_controls_revealer = bottom_left_controls_revealer.clone();
        let bottom_right_controls_revealer = bottom_right_controls_revealer.clone();
        pointer.connect_motion(move |_, _, _| {
            top_controls_left_revealer.set_reveal_child(true);
            top_controls_left_revealer.set_can_target(true);
            top_controls_right_revealer.set_reveal_child(true);
            top_controls_right_revealer.set_can_target(true);
            bottom_left_controls_revealer.set_reveal_child(true);
            bottom_left_controls_revealer.set_can_target(true);
            bottom_right_controls_revealer.set_reveal_child(true);
            bottom_right_controls_revealer.set_can_target(true);
        });
    }
    {
        let top_controls_left_revealer = top_controls_left_revealer.clone();
        let top_controls_right_revealer = top_controls_right_revealer.clone();
        let bottom_left_controls_revealer = bottom_left_controls_revealer.clone();
        let bottom_right_controls_revealer = bottom_right_controls_revealer.clone();
        pointer.connect_leave(move |_| {
            top_controls_left_revealer.set_reveal_child(false);
            top_controls_left_revealer.set_can_target(false);
            top_controls_right_revealer.set_reveal_child(false);
            top_controls_right_revealer.set_can_target(false);
            bottom_left_controls_revealer.set_reveal_child(false);
            bottom_left_controls_revealer.set_can_target(false);
            bottom_right_controls_revealer.set_reveal_child(false);
            bottom_right_controls_revealer.set_can_target(false);
        });
    }
    editor_overlay.add_controller(pointer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_editor_tool_fallback_shortcut_maps_editor_tool_keys() {
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('v')),
            Some(ToolKind::Select)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('h')),
            Some(ToolKind::Pan)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('b')),
            Some(ToolKind::Blur)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('p')),
            Some(ToolKind::Pen)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('a')),
            Some(ToolKind::Arrow)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('r')),
            Some(ToolKind::Rectangle)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('c')),
            Some(ToolKind::Crop)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('t')),
            Some(ToolKind::Text)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('o')),
            Some(ToolKind::Ocr)
        );
    }

    #[test]
    fn resolve_editor_tool_fallback_shortcut_ignores_non_tool_keys() {
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('x')),
            None
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Escape),
            None
        );
    }
}

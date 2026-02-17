use super::*;

pub(in crate::app::editor_runtime) fn connect_editor_output_button(
    button: &Button,
    runtime: &EditorOutputActionRuntime,
    action: EditorAction,
    action_label: &'static str,
) {
    let runtime = runtime.clone();
    button.connect_clicked(move |_| {
        let _ = runtime.run(action, action_label);
    });
}

pub(in crate::app::editor_runtime) const EDITOR_TOOLBAR_ENTRIES: [(ToolKind, &str, &str); 9] = [
    (ToolKind::Select, "mouse-pointer-symbolic", "Select (V)"),
    (ToolKind::Pan, "hand-symbolic", "Pan (H)"),
    (ToolKind::Blur, "eye-off-symbolic", "Blur (B)"),
    (ToolKind::Pen, EDITOR_PEN_ICON_NAME, "Pen (P)"),
    (ToolKind::Arrow, "move-up-right-symbolic", "Arrow (A)"),
    (
        ToolKind::Rectangle,
        "rectangle-horizontal-symbolic",
        "Rectangle (R)",
    ),
    (ToolKind::Crop, "crop-symbolic", "Crop (C)"),
    (ToolKind::Text, "text-cursor-input-symbolic", "Text (T)"),
    (ToolKind::Ocr, "scan-text-symbolic", "OCR (O)"),
];

pub(in crate::app::editor_runtime) fn connect_tool_button_selection(
    button: &Button,
    tool_kind: ToolKind,
    editor_tool_switch_context: &EditorToolSwitchContext,
    status_log_for_render: &Rc<RefCell<String>>,
) {
    let status_log_for_render = status_log_for_render.clone();
    let editor_tool_switch_context = editor_tool_switch_context.clone();
    button.connect_clicked(move |_| {
        editor_tool_switch_context.switch_to(tool_kind, true);
        *status_log_for_render.borrow_mut() =
            format!("editor tool selected: {}", tool_kind_label(tool_kind));
    });
}

pub(super) fn switch_editor_tool_with_text_policy(
    editor_tool_switch_context: &EditorToolSwitchContext,
    selected_tool: ToolKind,
    clear_pending_crop_when_not_crop: bool,
    text_input: TextInputActivation,
) {
    editor_tool_switch_context.switch_to_with_config(
        selected_tool,
        ToolSwitchConfig {
            clear_pending_crop_when_not_crop,
            text_input,
        },
    );
}

pub(super) fn enter_text_box_editing(
    editor_tool_switch_context: &EditorToolSwitchContext,
    editor_tools: &RefCell<editor::EditorTools>,
    selected_object_ids: &Rc<RefCell<Vec<u64>>>,
    text_preedit_state: &RefCell<TextPreeditState>,
    text_id: u64,
) {
    set_single_selection(selected_object_ids, text_id);
    switch_editor_tool_with_text_policy(
        editor_tool_switch_context,
        ToolKind::Text,
        true,
        TextInputActivation::ForceOn,
    );
    let _ = editor_tools.borrow_mut().focus_text_box(text_id);
    *text_preedit_state.borrow_mut() = TextPreeditState::default();
}

pub(super) fn arm_text_tool_for_selection(
    editor_tool_switch_context: &EditorToolSwitchContext,
    editor_tools: &RefCell<editor::EditorTools>,
    selected_object_ids: &Rc<RefCell<Vec<u64>>>,
    text_id: u64,
) {
    set_single_selection(selected_object_ids, text_id);
    editor_tools.borrow_mut().finish_text_box();
    switch_editor_tool_with_text_policy(
        editor_tool_switch_context,
        ToolKind::Text,
        true,
        TextInputActivation::ForceOff,
    );
}

pub(super) fn finish_text_editing_and_arm_text_tool(
    editor_tool_switch_context: &EditorToolSwitchContext,
    editor_tools: &RefCell<editor::EditorTools>,
) {
    editor_tools.borrow_mut().finish_text_box();
    switch_editor_tool_with_text_policy(
        editor_tool_switch_context,
        ToolKind::Text,
        true,
        TextInputActivation::ForceOff,
    );
}

pub(super) fn add_text_box_and_enter_editing(
    editor_tool_switch_context: &EditorToolSwitchContext,
    editor_tools: &RefCell<editor::EditorTools>,
    selected_object_ids: &Rc<RefCell<Vec<u64>>>,
    text_preedit_state: &RefCell<TextPreeditState>,
    anchor: editor::tools::ToolPoint,
) -> u64 {
    let text_id = editor_tools.borrow_mut().add_text_box(anchor);
    set_single_selection(selected_object_ids, text_id);
    switch_editor_tool_with_text_policy(
        editor_tool_switch_context,
        ToolKind::Text,
        true,
        TextInputActivation::ForceOn,
    );
    *text_preedit_state.borrow_mut() = TextPreeditState::default();
    text_id
}

use super::*;

#[derive(Clone, Copy)]
pub(super) enum EditorHistoryAction {
    Undo,
    Redo,
}

impl EditorHistoryAction {
    const fn applied_message(self) -> &'static str {
        match self {
            Self::Undo => "undo applied",
            Self::Redo => "redo applied",
        }
    }

    const fn empty_message(self) -> &'static str {
        match self {
            Self::Undo => "undo stack empty",
            Self::Redo => "redo stack empty",
        }
    }
}

#[derive(Clone)]
pub(super) struct EditorHistoryRuntime {
    pub(super) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(super) editor_undo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    pub(super) editor_redo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    pub(super) selected_object_ids: Rc<RefCell<Vec<u64>>>,
    pub(super) pending_crop: Rc<RefCell<Option<CropElement>>>,
    pub(super) editor_has_unsaved_changes: Rc<RefCell<bool>>,
    pub(super) status_log_for_render: Rc<RefCell<String>>,
    pub(super) editor_canvas: DrawingArea,
}

impl EditorHistoryRuntime {
    pub(super) fn connect_button(&self, button: &Button, action: EditorHistoryAction) {
        let runtime = self.clone();
        button.connect_clicked(move |_| {
            let (source_stack, target_stack) = match action {
                EditorHistoryAction::Undo => {
                    (&runtime.editor_undo_stack, &runtime.editor_redo_stack)
                }
                EditorHistoryAction::Redo => {
                    (&runtime.editor_redo_stack, &runtime.editor_undo_stack)
                }
            };

            if let Some(snapshot) = source_stack.borrow_mut().pop() {
                let current = snapshot_editor_objects(runtime.editor_tools.as_ref());
                target_stack.borrow_mut().push(current);
                {
                    let mut tools = runtime.editor_tools.borrow_mut();
                    tools.replace_objects(snapshot);
                    ensure_selected_object_exists(&tools, &runtime.selected_object_ids);
                }
                runtime.pending_crop.borrow_mut().take();
                *runtime.editor_has_unsaved_changes.borrow_mut() = true;
                *runtime.status_log_for_render.borrow_mut() = action.applied_message().to_string();
                runtime.editor_canvas.queue_draw();
            } else {
                *runtime.status_log_for_render.borrow_mut() = action.empty_message().to_string();
            }
        });
    }
}

pub(super) fn snapshot_editor_objects(
    editor_tools: &RefCell<editor::EditorTools>,
) -> Vec<ToolObject> {
    editor_tools.borrow().objects().to_vec()
}

pub(super) fn snapshot_active_text_objects(
    editor_tools: &RefCell<editor::EditorTools>,
) -> Option<Vec<ToolObject>> {
    let tools = editor_tools.borrow();
    tools.active_text().map(|_| tools.objects().to_vec())
}

pub(super) fn record_undo_snapshot(
    editor_undo_stack: &RefCell<Vec<Vec<ToolObject>>>,
    editor_redo_stack: &RefCell<Vec<Vec<ToolObject>>>,
    snapshot: Vec<ToolObject>,
) {
    editor_undo_stack.borrow_mut().push(snapshot);
    editor_redo_stack.borrow_mut().clear();
}

pub(super) fn record_undo_snapshot_if_some(
    editor_undo_stack: &RefCell<Vec<Vec<ToolObject>>>,
    editor_redo_stack: &RefCell<Vec<Vec<ToolObject>>>,
    snapshot: Option<Vec<ToolObject>>,
) {
    if let Some(snapshot) = snapshot {
        record_undo_snapshot(editor_undo_stack, editor_redo_stack, snapshot);
    }
}

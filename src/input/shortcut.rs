#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKey {
    Character(char),
    Enter,
    Escape,
    Delete,
    Backspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShortcutModifiers {
    pub ctrl: bool,
    pub shift: bool,
}

impl ShortcutModifiers {
    pub const fn new(ctrl: bool, shift: bool) -> Self {
        Self { ctrl, shift }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputContext {
    pub dialog_open: bool,
    pub text_input_active: bool,
    pub crop_active: bool,
    pub editor_select_mode: bool,
    pub in_editor: bool,
    pub in_preview: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    DialogConfirm,
    DialogCancel,
    TextInsertLineBreak,
    TextCommit,
    TextCopySelection,
    TextExitFocus,
    CropApply,
    CropCancel,
    EditorUndo,
    EditorRedo,
    EditorDeleteSelection,
    EditorSave,
    EditorCopyImage,
    EditorEnterSelect,
    EditorEnterPan,
    EditorEnterBlur,
    EditorEnterPen,
    EditorEnterArrow,
    EditorEnterRectangle,
    EditorEnterCrop,
    EditorEnterText,
    EditorToggleToolOptions,
    EditorCloseRequested,
    PreviewSave,
    PreviewCopy,
    PreviewEdit,
    PreviewDelete,
    PreviewClose,
}

fn resolve_dialog_shortcut(key: ShortcutKey) -> Option<ShortcutAction> {
    match key {
        ShortcutKey::Enter => Some(ShortcutAction::DialogConfirm),
        ShortcutKey::Escape => Some(ShortcutAction::DialogCancel),
        _ => None,
    }
}

fn resolve_text_shortcut(key: ShortcutKey, modifiers: ShortcutModifiers) -> Option<ShortcutAction> {
    match (key, modifiers.ctrl) {
        (ShortcutKey::Enter, true) => Some(ShortcutAction::TextCommit),
        (ShortcutKey::Enter, false) => Some(ShortcutAction::TextInsertLineBreak),
        (ShortcutKey::Character('c'), true) => Some(ShortcutAction::TextCopySelection),
        (ShortcutKey::Escape, _) => Some(ShortcutAction::TextExitFocus),
        _ => None,
    }
}

fn resolve_crop_shortcut(key: ShortcutKey) -> Option<ShortcutAction> {
    match key {
        ShortcutKey::Enter => Some(ShortcutAction::CropApply),
        ShortcutKey::Escape => Some(ShortcutAction::CropCancel),
        _ => None,
    }
}

fn resolve_editor_tool_shortcut(key: ShortcutKey) -> Option<ShortcutAction> {
    match key {
        ShortcutKey::Character('v') => Some(ShortcutAction::EditorEnterSelect),
        ShortcutKey::Character('h') => Some(ShortcutAction::EditorEnterPan),
        ShortcutKey::Character('b') => Some(ShortcutAction::EditorEnterBlur),
        ShortcutKey::Character('p') => Some(ShortcutAction::EditorEnterPen),
        ShortcutKey::Character('a') => Some(ShortcutAction::EditorEnterArrow),
        ShortcutKey::Character('r') => Some(ShortcutAction::EditorEnterRectangle),
        ShortcutKey::Character('c') => Some(ShortcutAction::EditorEnterCrop),
        ShortcutKey::Character('t') => Some(ShortcutAction::EditorEnterText),
        _ => None,
    }
}

fn resolve_editor_shortcut(
    key: ShortcutKey,
    modifiers: ShortcutModifiers,
    context: InputContext,
) -> Option<ShortcutAction> {
    match (key, modifiers.ctrl, modifiers.shift) {
        (ShortcutKey::Character('z'), true, false) => Some(ShortcutAction::EditorUndo),
        (ShortcutKey::Character('z'), true, true) => Some(ShortcutAction::EditorRedo),
        (ShortcutKey::Delete, false, false) | (ShortcutKey::Backspace, false, false) => {
            Some(ShortcutAction::EditorDeleteSelection)
        }
        (ShortcutKey::Character('s'), true, _) => Some(ShortcutAction::EditorSave),
        (ShortcutKey::Character('c'), true, _) => Some(ShortcutAction::EditorCopyImage),
        (ShortcutKey::Character('o'), false, false) => {
            Some(ShortcutAction::EditorToggleToolOptions)
        }
        (ShortcutKey::Escape, false, false) => {
            if context.editor_select_mode {
                Some(ShortcutAction::EditorCloseRequested)
            } else {
                Some(ShortcutAction::EditorEnterSelect)
            }
        }
        (ShortcutKey::Escape, _, _) => Some(ShortcutAction::EditorCloseRequested),
        (_, false, false) => resolve_editor_tool_shortcut(key),
        _ => None,
    }
}

fn resolve_preview_shortcut(
    key: ShortcutKey,
    modifiers: ShortcutModifiers,
) -> Option<ShortcutAction> {
    match (key, modifiers.ctrl, modifiers.shift) {
        (ShortcutKey::Character('s'), false, false) => Some(ShortcutAction::PreviewSave),
        (ShortcutKey::Character('c'), false, false) => Some(ShortcutAction::PreviewCopy),
        (ShortcutKey::Character('e'), false, false) => Some(ShortcutAction::PreviewEdit),
        (ShortcutKey::Delete, false, false) => Some(ShortcutAction::PreviewDelete),
        (ShortcutKey::Escape, false, false) => Some(ShortcutAction::PreviewClose),
        _ => None,
    }
}

pub fn resolve_shortcut(
    key: ShortcutKey,
    modifiers: ShortcutModifiers,
    context: InputContext,
) -> Option<ShortcutAction> {
    if context.dialog_open {
        return resolve_dialog_shortcut(key);
    }

    if context.text_input_active {
        return resolve_text_shortcut(key, modifiers);
    }

    if context.crop_active {
        return resolve_crop_shortcut(key);
    }

    if context.in_editor {
        return resolve_editor_shortcut(key, modifiers, context);
    }

    if context.in_preview {
        return resolve_preview_shortcut(key, modifiers);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_shortcut_prioritizes_dialog_context() {
        let context = InputContext {
            dialog_open: true,
            text_input_active: true,
            crop_active: true,
            editor_select_mode: true,
            in_editor: true,
            in_preview: true,
        };
        assert_eq!(
            resolve_shortcut(ShortcutKey::Enter, ShortcutModifiers::default(), context),
            Some(ShortcutAction::DialogConfirm)
        );
        assert_eq!(
            resolve_shortcut(ShortcutKey::Escape, ShortcutModifiers::default(), context),
            Some(ShortcutAction::DialogCancel)
        );
    }

    #[test]
    fn resolve_shortcut_prioritizes_text_over_editor_copy() {
        let context = InputContext {
            text_input_active: true,
            in_editor: true,
            ..Default::default()
        };
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('c'),
                ShortcutModifiers::new(true, false),
                context
            ),
            Some(ShortcutAction::TextCopySelection)
        );
    }

    #[test]
    fn resolve_shortcut_prioritizes_crop_over_editor_escape() {
        let context = InputContext {
            crop_active: true,
            in_editor: true,
            ..Default::default()
        };
        assert_eq!(
            resolve_shortcut(ShortcutKey::Escape, ShortcutModifiers::default(), context),
            Some(ShortcutAction::CropCancel)
        );
    }

    #[test]
    fn resolve_shortcut_maps_editor_shortcuts() {
        let context = InputContext {
            in_editor: true,
            ..Default::default()
        };
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('z'),
                ShortcutModifiers::new(true, false),
                context
            ),
            Some(ShortcutAction::EditorUndo)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('z'),
                ShortcutModifiers::new(true, true),
                context
            ),
            Some(ShortcutAction::EditorRedo)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('s'),
                ShortcutModifiers::new(true, false),
                context
            ),
            Some(ShortcutAction::EditorSave)
        );
        assert_eq!(
            resolve_shortcut(ShortcutKey::Delete, ShortcutModifiers::default(), context),
            Some(ShortcutAction::EditorDeleteSelection)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Backspace,
                ShortcutModifiers::default(),
                context
            ),
            Some(ShortcutAction::EditorDeleteSelection)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('v'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorEnterSelect)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('h'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorEnterPan)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('b'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorEnterBlur)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('p'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorEnterPen)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('a'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorEnterArrow)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('r'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorEnterRectangle)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('c'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorEnterCrop)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('t'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorEnterText)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('o'),
                ShortcutModifiers::new(false, false),
                context
            ),
            Some(ShortcutAction::EditorToggleToolOptions)
        );
        assert_eq!(
            resolve_shortcut(ShortcutKey::Escape, ShortcutModifiers::default(), context),
            Some(ShortcutAction::EditorEnterSelect)
        );
    }

    #[test]
    fn resolve_shortcut_maps_editor_escape_to_close_when_select_mode() {
        let context = InputContext {
            in_editor: true,
            editor_select_mode: true,
            ..Default::default()
        };
        assert_eq!(
            resolve_shortcut(ShortcutKey::Escape, ShortcutModifiers::default(), context),
            Some(ShortcutAction::EditorCloseRequested)
        );
    }

    #[test]
    fn resolve_shortcut_maps_preview_shortcuts() {
        let context = InputContext {
            in_preview: true,
            ..Default::default()
        };
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('s'),
                ShortcutModifiers::default(),
                context
            ),
            Some(ShortcutAction::PreviewSave)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('c'),
                ShortcutModifiers::default(),
                context
            ),
            Some(ShortcutAction::PreviewCopy)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Character('e'),
                ShortcutModifiers::default(),
                context
            ),
            Some(ShortcutAction::PreviewEdit)
        );
        assert_eq!(
            resolve_shortcut(ShortcutKey::Delete, ShortcutModifiers::default(), context),
            Some(ShortcutAction::PreviewDelete)
        );
        assert_eq!(
            resolve_shortcut(
                ShortcutKey::Backspace,
                ShortcutModifiers::default(),
                context
            ),
            None
        );
        assert_eq!(
            resolve_shortcut(ShortcutKey::Escape, ShortcutModifiers::default(), context),
            Some(ShortcutAction::PreviewClose)
        );
    }
}

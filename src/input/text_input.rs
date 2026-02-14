#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputEvent {
    Character(char),
    Backspace,
    Enter,
    ShiftEnter,
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
    CtrlEnter,
    Escape,
    CtrlC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputAction {
    NoTextTarget,
    NoAction,
    InsertCharacter(char),
    DeleteBackward,
    InsertLineBreak,
    MoveCursor,
    Commit,
    ExitFocus,
    CopyRequested,
}

pub fn resolve_text_input(event: TextInputEvent, has_text_focus: bool) -> TextInputAction {
    if !has_text_focus {
        return TextInputAction::NoTextTarget;
    }

    match event {
        TextInputEvent::Character(c) => TextInputAction::InsertCharacter(c),
        TextInputEvent::Backspace => TextInputAction::DeleteBackward,
        TextInputEvent::Enter | TextInputEvent::ShiftEnter => TextInputAction::InsertLineBreak,
        TextInputEvent::CursorLeft
        | TextInputEvent::CursorRight
        | TextInputEvent::CursorUp
        | TextInputEvent::CursorDown => TextInputAction::MoveCursor,
        TextInputEvent::CtrlEnter => TextInputAction::Commit,
        TextInputEvent::Escape => TextInputAction::ExitFocus,
        TextInputEvent::CtrlC => TextInputAction::CopyRequested,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_text_input_without_focus_is_no_target() {
        let event = resolve_text_input(TextInputEvent::Character('a'), false);
        assert_eq!(event, TextInputAction::NoTextTarget);
    }

    #[test]
    fn resolve_text_input_applies_text_priority_rules() {
        assert_eq!(
            resolve_text_input(TextInputEvent::Enter, true),
            TextInputAction::InsertLineBreak
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::ShiftEnter, true),
            TextInputAction::InsertLineBreak
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::CtrlEnter, true),
            TextInputAction::Commit
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::CursorLeft, true),
            TextInputAction::MoveCursor
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::CursorRight, true),
            TextInputAction::MoveCursor
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::CursorUp, true),
            TextInputAction::MoveCursor
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::CursorDown, true),
            TextInputAction::MoveCursor
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::Escape, true),
            TextInputAction::ExitFocus
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::CtrlC, true),
            TextInputAction::CopyRequested
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::Character('x'), true),
            TextInputAction::InsertCharacter('x')
        );
        assert_eq!(
            resolve_text_input(TextInputEvent::Backspace, true),
            TextInputAction::DeleteBackward
        );
    }
}

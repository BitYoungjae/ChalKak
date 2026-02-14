use crate::input::{ModifierState, ShortcutKey, ShortcutModifiers, TextInputEvent};

pub(super) fn resolve_text_input_event(
    key: gtk4::gdk::Key,
    modifier: gtk4::gdk::ModifierType,
) -> Option<TextInputEvent> {
    let state = modifier_state(modifier);
    if state.ctrl {
        if matches!(key, gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter) {
            return Some(TextInputEvent::CtrlEnter);
        }
        if matches!(key.to_unicode(), Some('c' | 'C')) {
            return Some(TextInputEvent::CtrlC);
        }
        return cursor_event_from_key(key);
    }

    if matches!(key, gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter) {
        return if state.shift {
            Some(TextInputEvent::ShiftEnter)
        } else {
            Some(TextInputEvent::Enter)
        };
    }
    if key == gtk4::gdk::Key::BackSpace {
        return Some(TextInputEvent::Backspace);
    }
    if let Some(cursor_event) = cursor_event_from_key(key) {
        return Some(cursor_event);
    }
    if key == gtk4::gdk::Key::Escape {
        return Some(TextInputEvent::Escape);
    }
    None
}

fn cursor_event_from_key(key: gtk4::gdk::Key) -> Option<TextInputEvent> {
    match key {
        gtk4::gdk::Key::Left | gtk4::gdk::Key::KP_Left => Some(TextInputEvent::CursorLeft),
        gtk4::gdk::Key::Right | gtk4::gdk::Key::KP_Right => Some(TextInputEvent::CursorRight),
        gtk4::gdk::Key::Up | gtk4::gdk::Key::KP_Up => Some(TextInputEvent::CursorUp),
        gtk4::gdk::Key::Down | gtk4::gdk::Key::KP_Down => Some(TextInputEvent::CursorDown),
        _ => None,
    }
}

fn shortcut_character_from_keycode(keycode: u32) -> Option<char> {
    // Wayland/XKB keycodes are commonly evdev+8. Handle both to keep shortcuts
    // layout-agnostic under different backends/IME states.
    match keycode {
        47 | 55 => Some('v'),
        35 | 43 => Some('h'),
        48 | 56 => Some('b'),
        25 | 33 => Some('p'),
        30 | 38 => Some('a'),
        19 | 27 => Some('r'),
        46 | 54 => Some('c'),
        20 | 28 => Some('t'),
        24 | 32 => Some('o'),
        31 | 39 => Some('s'),
        44 | 52 => Some('z'),
        18 | 26 => Some('e'),
        _ => None,
    }
}

pub(super) fn normalize_shortcut_key(key: gtk4::gdk::Key, keycode: u32) -> Option<ShortcutKey> {
    if matches!(key, gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter) {
        return Some(ShortcutKey::Enter);
    }
    if key == gtk4::gdk::Key::Escape {
        return Some(ShortcutKey::Escape);
    }
    if key == gtk4::gdk::Key::Delete {
        return Some(ShortcutKey::Delete);
    }
    if key == gtk4::gdk::Key::BackSpace {
        return Some(ShortcutKey::Backspace);
    }

    let keyval_shortcut = key
        .to_unicode()
        .filter(|character| !character.is_control())
        .map(|character| ShortcutKey::Character(character.to_ascii_lowercase()));
    match keyval_shortcut {
        Some(ShortcutKey::Character(character)) if character.is_ascii() => {
            Some(ShortcutKey::Character(character))
        }
        Some(_) | None => shortcut_character_from_keycode(keycode).map(ShortcutKey::Character),
    }
}

pub(super) fn key_name(key: gtk4::gdk::Key) -> Option<String> {
    key.name().map(|name| name.to_string().to_ascii_lowercase())
}

pub(super) fn modifier_state(modifier: gtk4::gdk::ModifierType) -> ModifierState {
    ModifierState {
        ctrl: modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK),
        shift: modifier.contains(gtk4::gdk::ModifierType::SHIFT_MASK),
        alt: modifier.contains(gtk4::gdk::ModifierType::ALT_MASK),
        super_key: modifier
            .intersects(gtk4::gdk::ModifierType::SUPER_MASK | gtk4::gdk::ModifierType::META_MASK),
    }
}

pub(super) fn shortcut_modifiers(modifier: gtk4::gdk::ModifierType) -> ShortcutModifiers {
    let state = modifier_state(modifier);
    ShortcutModifiers::new(state.ctrl, state.shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_shortcut_key_falls_back_to_hardware_keycode_for_letters() {
        assert_eq!(
            normalize_shortcut_key(gtk4::gdk::Key::Hangul, 47),
            Some(ShortcutKey::Character('v'))
        );
        assert_eq!(
            normalize_shortcut_key(gtk4::gdk::Key::Hangul, 55),
            Some(ShortcutKey::Character('v'))
        );
        assert_eq!(
            normalize_shortcut_key(gtk4::gdk::Key::Hangul, 24),
            Some(ShortcutKey::Character('o'))
        );
        assert_eq!(normalize_shortcut_key(gtk4::gdk::Key::Hangul, 999), None);
    }

    #[test]
    fn normalize_shortcut_key_keeps_ascii_from_keyval() {
        assert_eq!(
            normalize_shortcut_key(gtk4::gdk::Key::v, 999),
            Some(ShortcutKey::Character('v'))
        );
        assert_eq!(
            normalize_shortcut_key(gtk4::gdk::Key::Escape, 47),
            Some(ShortcutKey::Escape)
        );
    }

    #[test]
    fn resolve_text_input_event_maps_arrow_keys_for_cursor_navigation() {
        let modifier = gtk4::gdk::ModifierType::empty();
        assert_eq!(
            resolve_text_input_event(gtk4::gdk::Key::Left, modifier),
            Some(TextInputEvent::CursorLeft)
        );
        assert_eq!(
            resolve_text_input_event(gtk4::gdk::Key::Right, modifier),
            Some(TextInputEvent::CursorRight)
        );
        assert_eq!(
            resolve_text_input_event(gtk4::gdk::Key::Up, modifier),
            Some(TextInputEvent::CursorUp)
        );
        assert_eq!(
            resolve_text_input_event(gtk4::gdk::Key::Down, modifier),
            Some(TextInputEvent::CursorDown)
        );
        assert_eq!(
            resolve_text_input_event(gtk4::gdk::Key::Left, gtk4::gdk::ModifierType::CONTROL_MASK,),
            Some(TextInputEvent::CursorLeft)
        );
    }
}

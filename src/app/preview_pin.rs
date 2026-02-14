use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::ToggleButton;

use super::hypr::{current_window_pin_state, request_window_pin};

const PIN_SYNC_POLL_INTERVAL: Duration = Duration::from_millis(24);
const PIN_ICON_UNPINNED: &str = "view-pin-symbolic";
const PIN_ICON_PINNED_CANDIDATES: [&str; 4] = [
    "view-pinned-symbolic",
    "view-pin-filled-symbolic",
    "pin-symbolic",
    PIN_ICON_UNPINNED,
];

fn resolve_pinned_icon_name() -> &'static str {
    if let Some(display) = gtk4::gdk::Display::default() {
        let theme = gtk4::IconTheme::for_display(&display);
        for icon_name in PIN_ICON_PINNED_CANDIDATES {
            if theme.has_icon(icon_name) {
                return icon_name;
            }
        }
    }
    PIN_ICON_UNPINNED
}

fn apply_preview_pin_toggle_state(
    toggle: &ToggleButton,
    pin_toggle_syncing: &Rc<Cell<bool>>,
    pinned_icon_name: &str,
    pinned: bool,
) {
    pin_toggle_syncing.set(true);
    toggle.set_active(pinned);
    toggle.set_icon_name(if pinned {
        pinned_icon_name
    } else {
        PIN_ICON_UNPINNED
    });
    pin_toggle_syncing.set(false);
    toggle.set_tooltip_text(Some(if pinned {
        "Unpin preview window"
    } else {
        "Pin preview window"
    }));
}

fn request_preview_pin_state(
    toggle: ToggleButton,
    pin_toggle_syncing: Rc<Cell<bool>>,
    pinned_icon_name: Rc<String>,
    pin_request_seq: Rc<Cell<u64>>,
    preview_title: String,
    desired: bool,
    fallback_pin_state: bool,
) {
    let request_id = pin_request_seq.get().saturating_add(1);
    pin_request_seq.set(request_id);

    let (tx, rx) = mpsc::channel::<bool>();
    std::thread::spawn(move || {
        let applied = request_window_pin("preview", &preview_title, desired);
        let actual_pin_state = if applied {
            desired
        } else {
            current_window_pin_state(&preview_title).unwrap_or(fallback_pin_state)
        };
        let _ = tx.send(actual_pin_state);
    });

    gtk4::glib::timeout_add_local(PIN_SYNC_POLL_INTERVAL, move || match rx.try_recv() {
        Ok(pin_state) => {
            if pin_request_seq.get() == request_id {
                apply_preview_pin_toggle_state(
                    &toggle,
                    &pin_toggle_syncing,
                    pinned_icon_name.as_str(),
                    pin_state,
                );
            }
            gtk4::glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => gtk4::glib::ControlFlow::Continue,
        Err(mpsc::TryRecvError::Disconnected) => gtk4::glib::ControlFlow::Break,
    });
}

pub(super) fn setup_preview_pin_toggle(toggle: &ToggleButton, preview_title: &str) {
    let pin_toggle_syncing = Rc::new(Cell::new(false));
    let pinned_icon_name = Rc::new(resolve_pinned_icon_name().to_string());
    let pin_request_seq = Rc::new(Cell::new(0_u64));
    toggle.set_icon_name(PIN_ICON_UNPINNED);

    {
        let pin_toggle_syncing = pin_toggle_syncing.clone();
        let pinned_icon_name = pinned_icon_name.clone();
        let pin_request_seq = pin_request_seq.clone();
        let preview_title = preview_title.to_string();
        toggle.connect_toggled(move |toggle| {
            if pin_toggle_syncing.get() {
                return;
            }
            let desired = toggle.is_active();
            request_preview_pin_state(
                toggle.clone(),
                pin_toggle_syncing.clone(),
                pinned_icon_name.clone(),
                pin_request_seq.clone(),
                preview_title.clone(),
                desired,
                !desired,
            );
        });
    }

    request_preview_pin_state(
        toggle.clone(),
        pin_toggle_syncing,
        pinned_icon_name,
        pin_request_seq,
        preview_title.to_string(),
        true,
        false,
    );
}

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Overlay, Revealer};

pub(crate) fn set_revealer_visibility(revealer: &Revealer, visible: bool) {
    revealer.set_reveal_child(visible);
    revealer.set_can_target(visible);
}

pub(crate) fn set_revealers_visibility(revealers: &[Revealer], visible: bool) {
    for revealer in revealers {
        set_revealer_visibility(revealer, visible);
    }
}

pub(crate) fn connect_overlay_hover<FEnter, FLeave>(
    overlay: &Overlay,
    on_enter: Rc<FEnter>,
    on_leave: Rc<FLeave>,
    show_on_motion: bool,
) where
    FEnter: Fn() + 'static,
    FLeave: Fn() + 'static,
{
    let pointer = gtk4::EventControllerMotion::new();

    {
        let on_enter = on_enter.clone();
        pointer.connect_enter(move |_, _, _| on_enter());
    }

    if show_on_motion {
        let on_enter = on_enter.clone();
        pointer.connect_motion(move |_, _, _| on_enter());
    }

    {
        let on_leave = on_leave.clone();
        pointer.connect_leave(move |_| on_leave());
    }

    overlay.add_controller(pointer);
}

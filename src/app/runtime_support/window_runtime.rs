use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use crate::preview;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Frame, Label, Revealer};

use super::super::hypr::current_window_geometry;
use super::super::layout::read_window_geometry;
use super::super::window_state::{RuntimeWindowGeometry, RuntimeWindowKind, RuntimeWindowState};

#[derive(Clone)]
pub(crate) struct ToastRuntime {
    label: Label,
    sequence: Rc<Cell<u64>>,
}

impl ToastRuntime {
    pub(crate) fn new(label: &Label) -> Self {
        Self {
            label: label.clone(),
            sequence: Rc::new(Cell::new(0)),
        }
    }

    pub(crate) fn show(&self, message: impl Into<String>, duration_ms: u32) {
        let message = message.into();
        self.label.set_text(&message);
        self.label.set_visible(true);

        let sequence = self.sequence.get().saturating_add(1);
        self.sequence.set(sequence);

        let label = self.label.clone();
        let latest_sequence = self.sequence.clone();
        gtk4::glib::timeout_add_local_once(
            Duration::from_millis(u64::from(duration_ms)),
            move || {
                if latest_sequence.get() == sequence {
                    label.set_visible(false);
                }
            },
        );
    }
}

#[derive(Clone)]
pub(crate) struct PreviewWindowRuntime {
    pub(crate) window: ApplicationWindow,
    pub(crate) shell: Rc<RefCell<preview::PreviewWindowShell>>,
    pub(crate) preview_surface: Frame,
    pub(crate) controls: Revealer,
    pub(crate) toast: ToastRuntime,
    pub(crate) close_guard: Rc<Cell<bool>>,
}

pub(crate) fn show_toast_for_capture(
    preview_windows: &Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    capture_id: &str,
    fallback: &ToastRuntime,
    message: impl Into<String>,
    duration_ms: u32,
) {
    let message = message.into();
    if let Some(runtime) = preview_windows.borrow().get(capture_id) {
        runtime.toast.show(message, duration_ms);
    } else {
        fallback.show(message, duration_ms);
    }
}

pub(crate) fn close_preview_window_for_capture(
    preview_windows: &Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    capture_id: &str,
    runtime_window_state: &Rc<RefCell<RuntimeWindowState>>,
) {
    let Some(runtime) = preview_windows.borrow_mut().remove(capture_id) else {
        return;
    };
    let fallback = {
        let geometry = runtime.shell.borrow().geometry();
        RuntimeWindowGeometry::with_position(
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height,
        )
    };
    let resolved = resolve_window_geometry_with_hypr(
        &runtime.window,
        fallback,
        RuntimeWindowGeometry::new(1, 1),
    );
    runtime
        .shell
        .borrow_mut()
        .set_geometry(preview::PreviewWindowGeometry {
            x: resolved.x,
            y: resolved.y,
            width: resolved.width,
            height: resolved.height,
        });
    {
        let mut window_state = runtime_window_state.borrow_mut();
        window_state.set_geometry(RuntimeWindowKind::Preview, resolved);
        window_state.set_preview_geometry_for_capture(capture_id, resolved);
    }
    guarded_close(&runtime.window, &runtime.close_guard);
}

pub(crate) fn close_all_preview_windows(
    preview_windows: &Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    runtime_window_state: &Rc<RefCell<RuntimeWindowState>>,
) {
    let capture_ids = preview_windows.borrow().keys().cloned().collect::<Vec<_>>();
    for capture_id in capture_ids {
        close_preview_window_for_capture(preview_windows, &capture_id, runtime_window_state);
    }
}

pub(crate) fn close_editor_window_if_open(
    editor_window: &Rc<RefCell<Option<ApplicationWindow>>>,
    runtime_window_state: &Rc<RefCell<RuntimeWindowState>>,
    close_guard: &Rc<Cell<bool>>,
    fallback_geometry: RuntimeWindowGeometry,
    minimum_geometry: RuntimeWindowGeometry,
) -> bool {
    let Some(window) = editor_window.borrow_mut().take() else {
        return false;
    };
    let fallback = runtime_window_state
        .borrow()
        .geometry_for(RuntimeWindowKind::Editor)
        .unwrap_or(fallback_geometry);
    persist_window_geometry(
        &window,
        runtime_window_state,
        RuntimeWindowKind::Editor,
        fallback,
        minimum_geometry,
    );
    guarded_close(&window, close_guard);
    true
}

fn persist_window_geometry(
    window: &ApplicationWindow,
    runtime_window_state: &Rc<RefCell<RuntimeWindowState>>,
    kind: RuntimeWindowKind,
    fallback: RuntimeWindowGeometry,
    minimum: RuntimeWindowGeometry,
) {
    let geometry = resolve_window_geometry_with_hypr(window, fallback, minimum);
    runtime_window_state
        .borrow_mut()
        .set_geometry(kind, geometry);
}

fn resolve_window_geometry_with_hypr(
    window: &ApplicationWindow,
    fallback: RuntimeWindowGeometry,
    minimum: RuntimeWindowGeometry,
) -> RuntimeWindowGeometry {
    let measured = read_window_geometry(window, fallback, minimum);
    let minimum_width = minimum.width.max(1);
    let minimum_height = minimum.height.max(1);
    let title = window.title().map(|title| title.to_string());
    if let Some((x, y, width, height)) = title.as_deref().and_then(current_window_geometry) {
        return RuntimeWindowGeometry::with_position(
            x,
            y,
            width.max(minimum_width),
            height.max(minimum_height),
        );
    }

    measured
}

fn guarded_close(window: &ApplicationWindow, close_guard: &Rc<Cell<bool>>) {
    close_guard.set(true);
    window.close();
    close_guard.set(false);
}

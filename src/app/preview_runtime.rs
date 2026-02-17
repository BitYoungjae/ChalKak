use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::capture;
use crate::input::{resolve_shortcut, InputContext, InputMode, ShortcutAction};
use crate::preview;
use crate::ui::StyleTokens;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, Frame, Orientation, Overflow,
    Overlay, Revealer, RevealerTransitionType, Scale,
};

use super::hypr::request_window_floating_with_geometry;
use super::input_bridge::{normalize_shortcut_key, shortcut_modifiers};
use super::layout::{clamp_window_geometry_to_current_monitors, compute_initial_preview_placement};
use super::preview_pin::setup_preview_pin_toggle;
use super::runtime_support::{
    close_all_preview_windows, close_preview_window_for_capture, PreviewWindowRuntime, ToastRuntime,
};
use super::window_state::{RuntimeWindowGeometry, RuntimeWindowState};
use super::{
    close_editor_if_open_and_clear, icon_button, icon_toggle_button, EditorRuntimeState,
    EDITOR_PEN_ICON_NAME,
};

#[derive(Clone)]
pub(super) struct PreviewRenderContext {
    app: Application,
    style_tokens: StyleTokens,
    motion_hover_ms: u32,
    status_log: Rc<RefCell<String>>,
    save_button: Button,
    copy_button: Button,
    ocr_button: Button,
    open_editor_button: Button,
    close_preview_button: Button,
    delete_button: Button,
    preview_windows: Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    preview_action_target_capture_id: Rc<RefCell<Option<String>>>,
    runtime_window_state: Rc<RefCell<RuntimeWindowState>>,
    editor_window: Rc<RefCell<Option<ApplicationWindow>>>,
    editor_close_guard: Rc<Cell<bool>>,
    editor_runtime: Rc<EditorRuntimeState>,
}

impl PreviewRenderContext {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        app: Application,
        style_tokens: StyleTokens,
        motion_hover_ms: u32,
        status_log: Rc<RefCell<String>>,
        save_button: Button,
        copy_button: Button,
        ocr_button: Button,
        open_editor_button: Button,
        close_preview_button: Button,
        delete_button: Button,
        preview_windows: Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
        preview_action_target_capture_id: Rc<RefCell<Option<String>>>,
        runtime_window_state: Rc<RefCell<RuntimeWindowState>>,
        editor_window: Rc<RefCell<Option<ApplicationWindow>>>,
        editor_close_guard: Rc<Cell<bool>>,
        editor_runtime: Rc<EditorRuntimeState>,
    ) -> Self {
        Self {
            app,
            style_tokens,
            motion_hover_ms,
            status_log,
            save_button,
            copy_button,
            ocr_button,
            open_editor_button,
            close_preview_button,
            delete_button,
            preview_windows,
            preview_action_target_capture_id,
            runtime_window_state,
            editor_window,
            editor_close_guard,
            editor_runtime,
        }
    }
}

pub(super) fn render_preview_state(
    context: &PreviewRenderContext,
    captures: &[capture::CaptureArtifact],
) {
    close_editor_if_open_and_clear(
        &context.editor_window,
        &context.runtime_window_state,
        &context.editor_close_guard,
        context.editor_runtime.as_ref(),
        context.style_tokens,
    );
    close_stale_preview_windows(
        captures,
        &context.preview_windows,
        &context.runtime_window_state,
    );
    prune_stale_preview_window_geometries(captures, &context.runtime_window_state);

    for artifact in captures {
        if let Some(runtime) = context
            .preview_windows
            .borrow()
            .get(&artifact.capture_id)
            .cloned()
        {
            sync_existing_preview_runtime(&runtime);
            continue;
        }
        create_preview_window_for_capture(context, artifact);
    }

    if captures.is_empty() {
        close_all_preview_windows(&context.preview_windows, &context.runtime_window_state);
    }
}

pub(super) fn install_preview_hover_tick(
    preview_windows: Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    tick_interval: Duration,
) {
    gtk4::glib::timeout_add_local(tick_interval, move || {
        let preview_runtimes = preview_windows
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let now = Instant::now();
        for runtime in preview_runtimes {
            let mut shell = runtime.shell.borrow_mut();
            let was_visible = shell.controls_visible();
            shell.update_hover_controls_visibility(now);
            if was_visible != shell.controls_visible() {
                runtime.controls.set_reveal_child(shell.controls_visible());
            }
        }
        gtk4::glib::ControlFlow::Continue
    });
}

fn sync_existing_preview_runtime(runtime: &PreviewWindowRuntime) {
    let shell = runtime.shell.borrow();
    runtime.controls.set_reveal_child(shell.controls_visible());
    runtime.controls.set_can_target(shell.controls_visible());
    runtime
        .preview_surface
        .set_opacity(shell.transparency() as f64);
}

fn close_stale_preview_windows(
    captures: &[capture::CaptureArtifact],
    preview_windows: &Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    runtime_window_state: &Rc<RefCell<RuntimeWindowState>>,
) {
    let capture_ids = captures
        .iter()
        .map(|artifact| artifact.capture_id.clone())
        .collect::<Vec<_>>();
    let stale_preview_ids = preview_windows
        .borrow()
        .keys()
        .filter(|capture_id| !capture_ids.iter().any(|id| id == *capture_id))
        .cloned()
        .collect::<Vec<_>>();
    for capture_id in stale_preview_ids {
        close_preview_window_for_capture(preview_windows, &capture_id, runtime_window_state);
    }
}

fn prune_stale_preview_window_geometries(
    captures: &[capture::CaptureArtifact],
    runtime_window_state: &Rc<RefCell<RuntimeWindowState>>,
) {
    let capture_ids = captures
        .iter()
        .map(|artifact| artifact.capture_id.clone())
        .collect::<Vec<_>>();
    let stale_geometry_ids = runtime_window_state
        .borrow()
        .preview_geometry_capture_ids()
        .into_iter()
        .filter(|capture_id| !capture_ids.iter().any(|id| id == capture_id))
        .collect::<Vec<_>>();
    if stale_geometry_ids.is_empty() {
        return;
    }
    let mut state = runtime_window_state.borrow_mut();
    for capture_id in stale_geometry_ids {
        state.remove_preview_geometry_for_capture(&capture_id);
    }
}

fn connect_preview_action_bridge(
    trigger_button: &Button,
    launchpad_button: &Button,
    preview_action_target_capture_id: &Rc<RefCell<Option<String>>>,
    capture_id: &str,
) {
    let launchpad_button = launchpad_button.clone();
    let preview_action_target_capture_id = preview_action_target_capture_id.clone();
    let capture_id = capture_id.to_string();
    trigger_button.connect_clicked(move |_| {
        *preview_action_target_capture_id.borrow_mut() = Some(capture_id.clone());
        launchpad_button.emit_clicked();
    });
}

fn connect_preview_action_bridges(
    bridges: &[(&Button, &Button)],
    preview_action_target_capture_id: &Rc<RefCell<Option<String>>>,
    capture_id: &str,
) {
    for (trigger_button, launchpad_button) in bridges {
        connect_preview_action_bridge(
            trigger_button,
            launchpad_button,
            preview_action_target_capture_id,
            capture_id,
        );
    }
}

#[derive(Clone)]
struct PreviewLaunchpadButtons {
    save_button: Button,
    copy_button: Button,
    ocr_button: Button,
    open_editor_button: Button,
    close_preview_button: Button,
    delete_button: Button,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewShortcutTarget {
    Save,
    Copy,
    Ocr,
    Edit,
    Delete,
    Close,
}

fn preview_shortcut_target(action: ShortcutAction) -> Option<PreviewShortcutTarget> {
    match action {
        ShortcutAction::PreviewSave => Some(PreviewShortcutTarget::Save),
        ShortcutAction::PreviewCopy => Some(PreviewShortcutTarget::Copy),
        ShortcutAction::PreviewOcr => Some(PreviewShortcutTarget::Ocr),
        ShortcutAction::PreviewEdit => Some(PreviewShortcutTarget::Edit),
        ShortcutAction::PreviewDelete => Some(PreviewShortcutTarget::Delete),
        ShortcutAction::PreviewClose => Some(PreviewShortcutTarget::Close),
        _ => None,
    }
}

impl PreviewLaunchpadButtons {
    fn from_context(context: &PreviewRenderContext) -> Self {
        Self {
            save_button: context.save_button.clone(),
            copy_button: context.copy_button.clone(),
            ocr_button: context.ocr_button.clone(),
            open_editor_button: context.open_editor_button.clone(),
            close_preview_button: context.close_preview_button.clone(),
            delete_button: context.delete_button.clone(),
        }
    }

    fn emit_shortcut_action(&self, action: ShortcutAction) -> bool {
        match preview_shortcut_target(action) {
            Some(PreviewShortcutTarget::Save) => self.save_button.emit_clicked(),
            Some(PreviewShortcutTarget::Copy) => self.copy_button.emit_clicked(),
            Some(PreviewShortcutTarget::Ocr) => self.ocr_button.emit_clicked(),
            Some(PreviewShortcutTarget::Edit) => self.open_editor_button.emit_clicked(),
            Some(PreviewShortcutTarget::Delete) => self.delete_button.emit_clicked(),
            Some(PreviewShortcutTarget::Close) => self.close_preview_button.emit_clicked(),
            _ => return false,
        }
        true
    }
}

struct PreviewWindowBuild {
    window: ApplicationWindow,
    title: String,
    floating_geometry: (i32, i32, i32, i32),
    shell: Rc<RefCell<preview::PreviewWindowShell>>,
    overlay: Overlay,
    controls_revealer: Revealer,
    preview_surface: Frame,
    toast_label: gtk4::Label,
    opacity_slider: Scale,
    copy_button: Button,
    save_button: Button,
    edit_button: Button,
    ocr_button: Button,
    close_button: Button,
}

struct PreviewControlsBuild {
    pin_toggle: gtk4::ToggleButton,
    controls_revealer: Revealer,
    opacity_slider: Scale,
    copy_button: Button,
    save_button: Button,
    edit_button: Button,
    ocr_button: Button,
    close_button: Button,
}

fn build_preview_controls(
    context: &PreviewRenderContext,
    preview_shell: &Rc<RefCell<preview::PreviewWindowShell>>,
) -> PreviewControlsBuild {
    let controls_layout = GtkBox::new(Orientation::Vertical, 0);
    controls_layout.set_hexpand(true);
    controls_layout.set_vexpand(true);
    controls_layout.set_margin_top(context.style_tokens.spacing_16);
    controls_layout.set_margin_bottom(context.style_tokens.spacing_16);
    controls_layout.set_margin_start(context.style_tokens.spacing_16);
    controls_layout.set_margin_end(context.style_tokens.spacing_16);

    let top_controls_wrap = GtkBox::new(Orientation::Horizontal, context.style_tokens.spacing_8);
    top_controls_wrap.add_css_class("preview-top-controls");
    top_controls_wrap.set_halign(Align::Fill);
    top_controls_wrap.set_valign(Align::Start);
    top_controls_wrap.set_hexpand(true);

    let preview_pin_toggle = icon_toggle_button(
        "pin-off-symbolic",
        "Pin preview window",
        context.style_tokens.control_size as i32,
        &["preview-pin-toggle", "preview-round-button"],
    );
    preview_pin_toggle.set_valign(Align::Center);

    let top_center_actions = GtkBox::new(Orientation::Horizontal, context.style_tokens.spacing_8);
    top_center_actions.add_css_class("preview-action-group");

    let preview_copy_button = icon_button(
        "copy-symbolic",
        "Copy",
        context.style_tokens.control_size as i32,
        &["preview-icon-button"],
    );

    let preview_save_button = icon_button(
        "save-symbolic",
        "Save image",
        context.style_tokens.control_size as i32,
        &["preview-icon-button"],
    );

    let preview_edit_button = icon_button(
        EDITOR_PEN_ICON_NAME,
        "Open editor",
        context.style_tokens.control_size as i32,
        &["preview-icon-button"],
    );

    let preview_ocr_button = icon_button(
        "scan-text-symbolic",
        "Extract text (OCR)",
        context.style_tokens.control_size as i32,
        &["preview-icon-button"],
    );

    top_center_actions.append(&preview_copy_button);
    top_center_actions.append(&preview_save_button);
    top_center_actions.append(&preview_edit_button);
    top_center_actions.append(&preview_ocr_button);

    let preview_close_button = icon_button(
        "x-symbolic",
        "Close preview",
        context.style_tokens.control_size as i32,
        &[
            "preview-icon-button",
            "preview-round-button",
            "preview-close-button",
        ],
    );
    preview_close_button.set_valign(Align::Center);

    let top_spacer_left = GtkBox::new(Orientation::Horizontal, 0);
    top_spacer_left.set_hexpand(true);
    let top_spacer_right = GtkBox::new(Orientation::Horizontal, 0);
    top_spacer_right.set_hexpand(true);

    top_controls_wrap.append(&preview_pin_toggle);
    top_controls_wrap.append(&top_spacer_left);
    top_controls_wrap.append(&top_center_actions);
    top_controls_wrap.append(&top_spacer_right);
    top_controls_wrap.append(&preview_close_button);

    let controls_spacer = GtkBox::new(Orientation::Vertical, 0);
    controls_spacer.set_vexpand(true);

    let bottom_controls_wrap = GtkBox::new(Orientation::Horizontal, context.style_tokens.spacing_8);
    bottom_controls_wrap.set_halign(Align::Center);
    bottom_controls_wrap.set_valign(Align::End);
    let opacity_row = GtkBox::new(Orientation::Horizontal, context.style_tokens.spacing_8);
    opacity_row.add_css_class("preview-bottom-controls");
    opacity_row.set_halign(Align::Center);
    let opacity_slider = Scale::with_range(Orientation::Horizontal, 0.2, 1.0, 0.01);
    opacity_slider.add_css_class("accent-slider");
    opacity_slider.add_css_class("preview-opacity-slider");
    opacity_slider.set_draw_value(false);
    opacity_slider.set_width_request(160);
    opacity_slider.set_tooltip_text(Some("Preview opacity"));
    opacity_slider.set_value(preview_shell.borrow().transparency() as f64);
    opacity_row.append(&opacity_slider);
    bottom_controls_wrap.append(&opacity_row);

    controls_layout.append(&top_controls_wrap);
    controls_layout.append(&controls_spacer);
    controls_layout.append(&bottom_controls_wrap);

    let controls_revealer = Revealer::new();
    controls_revealer.add_css_class("preview-controls-revealer");
    controls_revealer.set_transition_duration(context.motion_hover_ms);
    controls_revealer.set_transition_type(RevealerTransitionType::Crossfade);
    controls_revealer.set_halign(Align::Fill);
    controls_revealer.set_valign(Align::Fill);
    controls_revealer.set_child(Some(&controls_layout));

    PreviewControlsBuild {
        pin_toggle: preview_pin_toggle,
        controls_revealer,
        opacity_slider,
        copy_button: preview_copy_button,
        save_button: preview_save_button,
        edit_button: preview_edit_button,
        ocr_button: preview_ocr_button,
        close_button: preview_close_button,
    }
}

fn build_preview_window(
    context: &PreviewRenderContext,
    artifact: &capture::CaptureArtifact,
) -> PreviewWindowBuild {
    let preview_window_instance = ApplicationWindow::new(&context.app);
    let preview_title = format!("Preview - {}", artifact.capture_id);
    preview_window_instance.set_title(Some(&preview_title));
    preview_window_instance.set_decorated(false);
    preview_window_instance.add_css_class("chalkak-root");
    preview_window_instance.add_css_class("floating-preview-window");

    let placement = compute_initial_preview_placement(artifact, context.style_tokens);
    let restored_geometry = context
        .runtime_window_state
        .borrow()
        .preview_geometry_for_capture(&artifact.capture_id)
        .and_then(|geometry| {
            clamp_window_geometry_to_current_monitors(RuntimeWindowGeometry::with_position(
                geometry.x,
                geometry.y,
                geometry.width.max(placement.min_width),
                geometry.height.max(placement.min_height),
            ))
        })
        .map(|geometry| preview::PreviewWindowGeometry {
            x: geometry.x,
            y: geometry.y,
            width: geometry.width,
            height: geometry.height,
        });
    let mut preview_shell_model = preview::PreviewWindowShell::with_capture_size(
        artifact.screen_width,
        artifact.screen_height,
    );
    preview_shell_model.set_geometry(restored_geometry.unwrap_or(placement.geometry));
    let preview_shell = Rc::new(RefCell::new(preview_shell_model));
    let geometry = preview_shell.borrow().geometry();

    preview_window_instance.set_default_size(geometry.width, geometry.height);
    preview_window_instance.set_size_request(placement.min_width, placement.min_height);
    preview_window_instance.set_resizable(true);

    let preview_overlay = Overlay::new();
    preview_overlay.add_css_class("transparent-bg");
    if !artifact.temp_path.exists() {
        *context.status_log.borrow_mut() = format!(
            "preview image path missing: {}",
            artifact.temp_path.display()
        );
    }

    let preview_controls = build_preview_controls(context, &preview_shell);

    let preview_image = gtk4::Picture::for_file(&gtk4::gio::File::for_path(&artifact.temp_path));
    preview_image.set_hexpand(true);
    preview_image.set_vexpand(true);
    preview_image.set_can_shrink(true);
    preview_image.set_keep_aspect_ratio(true);
    let preview_surface = Frame::new(None);
    preview_surface.add_css_class("preview-surface");
    preview_surface.set_hexpand(true);
    preview_surface.set_vexpand(true);
    preview_surface.set_overflow(Overflow::Hidden);
    preview_surface.set_opacity(preview_shell.borrow().transparency() as f64);
    preview_surface.set_child(Some(&preview_image));
    preview_overlay.set_child(Some(&preview_surface));
    preview_overlay.add_overlay(&preview_controls.controls_revealer);

    let preview_toast_anchor = GtkBox::new(Orientation::Vertical, 0);
    preview_toast_anchor.set_halign(Align::End);
    preview_toast_anchor.set_valign(Align::End);
    preview_toast_anchor.set_margin_top(context.style_tokens.spacing_12);
    preview_toast_anchor.set_margin_bottom(context.style_tokens.spacing_12);
    preview_toast_anchor.set_margin_start(context.style_tokens.spacing_12);
    preview_toast_anchor.set_margin_end(context.style_tokens.spacing_12);
    let preview_toast_label = gtk4::Label::new(Some(""));
    preview_toast_label.add_css_class("toast-badge");
    preview_toast_label.set_visible(false);
    preview_toast_anchor.append(&preview_toast_label);
    preview_overlay.add_overlay(&preview_toast_anchor);

    preview_window_instance.set_child(Some(&preview_overlay));
    setup_preview_pin_toggle(&preview_controls.pin_toggle, &preview_title);

    PreviewWindowBuild {
        window: preview_window_instance,
        title: preview_title,
        floating_geometry: (geometry.x, geometry.y, geometry.width, geometry.height),
        shell: preview_shell,
        overlay: preview_overlay,
        controls_revealer: preview_controls.controls_revealer,
        preview_surface,
        toast_label: preview_toast_label,
        opacity_slider: preview_controls.opacity_slider,
        copy_button: preview_controls.copy_button,
        save_button: preview_controls.save_button,
        edit_button: preview_controls.edit_button,
        ocr_button: preview_controls.ocr_button,
        close_button: preview_controls.close_button,
    }
}

fn connect_preview_window_action_wiring(
    context: &PreviewRenderContext,
    build: &PreviewWindowBuild,
    capture_id: &str,
) -> Rc<Cell<bool>> {
    connect_preview_action_bridges(
        &[
            (&build.save_button, &context.save_button),
            (&build.copy_button, &context.copy_button),
            (&build.ocr_button, &context.ocr_button),
            (&build.edit_button, &context.open_editor_button),
            (&build.close_button, &context.close_preview_button),
        ],
        &context.preview_action_target_capture_id,
        capture_id,
    );

    let launchpad_buttons = PreviewLaunchpadButtons::from_context(context);
    {
        let preview_action_target_capture_id = context.preview_action_target_capture_id.clone();
        let capture_id = capture_id.to_string();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, keycode, modifier| {
            let Some(shortcut_key) = normalize_shortcut_key(key, keycode) else {
                return gtk4::glib::Propagation::Proceed;
            };
            let shortcut = resolve_shortcut(
                shortcut_key,
                shortcut_modifiers(modifier),
                InputContext {
                    mode: InputMode::Preview,
                },
            );
            let Some(action) = shortcut else {
                return gtk4::glib::Propagation::Proceed;
            };

            *preview_action_target_capture_id.borrow_mut() = Some(capture_id.clone());
            if launchpad_buttons.emit_shortcut_action(action) {
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        build.window.add_controller(key_controller);
    }

    let close_guard = Rc::new(Cell::new(false));
    {
        let close_preview_button = context.close_preview_button.clone();
        let preview_action_target_capture_id = context.preview_action_target_capture_id.clone();
        let capture_id = capture_id.to_string();
        let close_guard = close_guard.clone();
        build.window.connect_close_request(move |_| {
            if close_guard.get() {
                return gtk4::glib::Propagation::Proceed;
            }
            *preview_action_target_capture_id.borrow_mut() = Some(capture_id.clone());
            close_preview_button.emit_clicked();
            gtk4::glib::Propagation::Stop
        });
    }

    close_guard
}

fn connect_preview_window_interactions(build: &PreviewWindowBuild) {
    {
        let preview_shell = build.shell.clone();
        let preview_surface = build.preview_surface.clone();
        build.opacity_slider.connect_value_changed(move |slider| {
            let mut shell = preview_shell.borrow_mut();
            shell.set_transparency(slider.value() as f32);
            preview_surface.set_opacity(shell.transparency() as f64);
        });
    }
    {
        let pointer = gtk4::EventControllerMotion::new();
        {
            let preview_shell = build.shell.clone();
            let controls_revealer = build.controls_revealer.clone();
            pointer.connect_enter(move |_, _, _| {
                let mut shell = preview_shell.borrow_mut();
                shell.hover_enter(Instant::now());
                controls_revealer.set_reveal_child(shell.controls_visible());
                controls_revealer.set_can_target(shell.controls_visible());
            });
        }
        {
            let preview_shell = build.shell.clone();
            pointer.connect_leave(move |_| {
                preview_shell.borrow_mut().hover_exit(Instant::now());
            });
        }
        build.overlay.add_controller(pointer);
    }
}

fn create_preview_window_for_capture(
    context: &PreviewRenderContext,
    artifact: &capture::CaptureArtifact,
) {
    let build = build_preview_window(context, artifact);
    let close_guard = connect_preview_window_action_wiring(context, &build, &artifact.capture_id);
    connect_preview_window_interactions(&build);

    build
        .controls_revealer
        .set_reveal_child(build.shell.borrow().controls_visible());
    build
        .controls_revealer
        .set_can_target(build.shell.borrow().controls_visible());
    build.window.present();
    request_window_floating_with_geometry(
        "preview",
        &build.title,
        true,
        Some(build.floating_geometry),
    );

    context.preview_windows.borrow_mut().insert(
        artifact.capture_id.clone(),
        PreviewWindowRuntime {
            window: build.window,
            shell: build.shell,
            preview_surface: build.preview_surface,
            controls: build.controls_revealer,
            toast: ToastRuntime::new(&build.toast_label),
            close_guard,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_shortcut_target_maps_preview_actions() {
        assert_eq!(
            preview_shortcut_target(ShortcutAction::PreviewSave),
            Some(PreviewShortcutTarget::Save)
        );
        assert_eq!(
            preview_shortcut_target(ShortcutAction::PreviewCopy),
            Some(PreviewShortcutTarget::Copy)
        );
        assert_eq!(
            preview_shortcut_target(ShortcutAction::PreviewEdit),
            Some(PreviewShortcutTarget::Edit)
        );
        assert_eq!(
            preview_shortcut_target(ShortcutAction::PreviewDelete),
            Some(PreviewShortcutTarget::Delete)
        );
        assert_eq!(
            preview_shortcut_target(ShortcutAction::PreviewClose),
            Some(PreviewShortcutTarget::Close)
        );
    }

    #[test]
    fn preview_shortcut_target_ignores_non_preview_actions() {
        assert_eq!(preview_shortcut_target(ShortcutAction::EditorSave), None);
        assert_eq!(preview_shortcut_target(ShortcutAction::DialogConfirm), None);
    }
}

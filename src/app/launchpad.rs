use std::rc::Rc;

use crate::capture;
use crate::preview::PreviewAction;
use crate::state::AppState;
use crate::ui::StyleTokens;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Frame, Label, Orientation, ScrolledWindow};

use super::launchpad_actions::LaunchpadActionExecutor;

#[derive(Clone)]
pub(super) struct LaunchpadUi {
    pub(super) root: GtkBox,
    pub(super) toast_label: Label,
    pub(super) state_label: Label,
    pub(super) status_label: Label,
    pub(super) active_capture_label: Label,
    pub(super) capture_count_label: Label,
    pub(super) latest_label: Label,
    pub(super) capture_ids_label: Label,
    pub(super) full_capture_button: Button,
    pub(super) region_capture_button: Button,
    pub(super) window_capture_button: Button,
    pub(super) open_preview_button: Button,
    pub(super) open_editor_button: Button,
    pub(super) close_preview_button: Button,
    pub(super) close_editor_button: Button,
    pub(super) save_button: Button,
    pub(super) copy_button: Button,
    pub(super) ocr_button: Button,
    pub(super) delete_button: Button,
}

impl LaunchpadUi {
    pub(super) fn update_overview(
        &self,
        state: AppState,
        active_capture_id: &str,
        latest_capture_label: &str,
        ids: &[String],
    ) {
        self.state_label.set_text(&format!("{:?}", state));
        self.active_capture_label.set_text(active_capture_id);
        self.capture_count_label.set_text(&format!("{}", ids.len()));
        self.latest_label.set_text(latest_capture_label);
        self.capture_ids_label
            .set_text(&format_capture_ids_for_display(ids));
    }

    pub(super) fn set_action_availability(
        &self,
        state: AppState,
        has_capture: bool,
        ocr_available: bool,
    ) {
        self.open_preview_button
            .set_sensitive(matches!(state, AppState::Idle) && has_capture);
        self.open_editor_button
            .set_sensitive(matches!(state, AppState::Preview) && has_capture);
        self.close_preview_button
            .set_sensitive(matches!(state, AppState::Preview));
        self.close_editor_button
            .set_sensitive(matches!(state, AppState::Editor));
        self.save_button
            .set_sensitive(matches!(state, AppState::Preview) && has_capture);
        self.copy_button
            .set_sensitive(matches!(state, AppState::Preview) && has_capture);
        self.ocr_button
            .set_sensitive(ocr_available && matches!(state, AppState::Preview) && has_capture);
        if !ocr_available {
            self.ocr_button
                .set_tooltip_text(Some("OCR models not installed"));
        }
        self.delete_button
            .set_sensitive(matches!(state, AppState::Preview) && has_capture);
    }

    pub(super) fn set_status_text(&self, message: &str) {
        self.status_label.set_text(message);
    }
}

pub(super) fn launchpad_kv_row(key: &str, value_label: &Label) -> GtkBox {
    let key_label = Label::new(Some(key));
    key_label.add_css_class("launchpad-kv-key");
    key_label.set_halign(Align::Start);
    key_label.set_xalign(0.0);

    value_label.add_css_class("launchpad-kv-value");
    value_label.set_halign(Align::Start);
    value_label.set_xalign(0.0);
    value_label.set_hexpand(true);
    value_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&key_label);
    row.append(value_label);
    row
}

pub(super) fn launchpad_kv_static(key: &str, value: &str) -> GtkBox {
    let value_label = Label::new(Some(value));
    launchpad_kv_row(key, &value_label)
}

pub(super) fn launchpad_section_title(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.add_css_class("launchpad-section-title");
    label.set_halign(Align::Start);
    label.set_xalign(0.0);
    label
}

pub(super) fn launchpad_panel(style_tokens: StyleTokens, title: &str, child: &GtkBox) -> Frame {
    let panel = Frame::new(None);
    panel.add_css_class("launchpad-panel");
    let panel_box = GtkBox::new(Orientation::Vertical, style_tokens.spacing_8);
    panel_box.append(&launchpad_section_title(title));
    panel_box.append(child);
    panel.set_child(Some(&panel_box));
    panel
}

pub(super) fn format_capture_ids_for_display(ids: &[String]) -> String {
    if ids.is_empty() {
        return "IDs: none".to_string();
    }

    let id_lines = ids
        .iter()
        .enumerate()
        .map(|(index, capture_id)| format!("{:>2}. {capture_id}", index + 1))
        .collect::<Vec<_>>()
        .join("\n");
    format!("IDs:\n{id_lines}")
}

pub(super) struct LaunchpadSettingsInfo {
    pub(super) theme_label: String,
    pub(super) ocr_language_label: String,
    pub(super) ocr_model_dir_label: String,
    pub(super) config_path: String,
    pub(super) theme_config_path: String,
    pub(super) keybinding_config_path: String,
}

pub(super) fn build_launchpad_ui(
    style_tokens: StyleTokens,
    show_launchpad: bool,
    settings_info: &LaunchpadSettingsInfo,
) -> LaunchpadUi {
    let root = GtkBox::new(Orientation::Vertical, style_tokens.spacing_12);
    root.set_margin_top(style_tokens.spacing_12);
    root.set_margin_bottom(style_tokens.spacing_12);
    root.set_margin_start(style_tokens.spacing_12);
    root.set_margin_end(style_tokens.spacing_12);
    root.add_css_class("launchpad-root");

    let toast_label = Label::new(Some(""));
    toast_label.add_css_class("toast-badge");
    toast_label.set_halign(Align::Start);
    toast_label.set_visible(false);

    // ── Header row: title + version badge ──
    let title_label = Label::new(Some("ChalKak Launchpad"));
    title_label.add_css_class("launchpad-title");
    title_label.set_halign(Align::Start);
    title_label.set_xalign(0.0);

    let version_label = Label::new(Some(env!("CARGO_PKG_VERSION")));
    version_label.add_css_class("launchpad-version");
    version_label.set_halign(Align::Start);
    version_label.set_valign(Align::Center);

    let header_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
    header_row.append(&title_label);
    header_row.append(&version_label);

    let subtitle_label = Label::new(Some(
        "Quick control panel for validating capture, preview, and editor flow.",
    ));
    subtitle_label.add_css_class("launchpad-subtitle");
    subtitle_label.set_halign(Align::Start);
    subtitle_label.set_xalign(0.0);
    subtitle_label.set_wrap(true);

    // ── Capture panel (3 buttons, horizontal) ──
    let full_capture_button = Button::with_label("Full Capture");
    full_capture_button.add_css_class("launchpad-primary-button");
    full_capture_button.set_hexpand(true);
    let region_capture_button = Button::with_label("Region Capture");
    region_capture_button.set_hexpand(true);
    let window_capture_button = Button::with_label("Window Capture");
    window_capture_button.set_hexpand(true);
    let capture_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
    capture_row.append(&full_capture_button);
    capture_row.append(&region_capture_button);
    capture_row.append(&window_capture_button);
    let capture_panel = launchpad_panel(style_tokens, "Capture", &capture_row);

    // ── Session panel (key-value grid) ──
    let state_label = Label::new(Some("initializing"));
    let status_label = Label::new(Some("Ready"));
    let active_capture_label = Label::new(Some("none"));
    let capture_count_label = Label::new(Some("0"));
    let latest_label = Label::new(Some("No capture yet"));

    let capture_ids_label = Label::new(Some("IDs: none"));
    capture_ids_label.add_css_class("launchpad-capture-ids");
    capture_ids_label.set_halign(Align::Start);
    capture_ids_label.set_xalign(0.0);
    capture_ids_label.set_wrap(true);
    capture_ids_label.set_selectable(true);

    let session_content = GtkBox::new(Orientation::Vertical, style_tokens.spacing_4);
    session_content.append(&launchpad_kv_row("State", &state_label));
    session_content.append(&launchpad_kv_row("Status", &status_label));
    session_content.append(&launchpad_kv_row("Active", &active_capture_label));
    session_content.append(&launchpad_kv_row("Count", &capture_count_label));
    session_content.append(&launchpad_kv_row("Latest", &latest_label));
    session_content.append(&capture_ids_label);
    let session_panel = launchpad_panel(style_tokens, "Session", &session_content);
    session_panel.set_hexpand(true);

    // ── Configuration panel (key-value grid) ──
    let config_content = GtkBox::new(Orientation::Vertical, style_tokens.spacing_4);
    config_content.append(&launchpad_kv_static("Theme", &settings_info.theme_label));
    config_content.append(&launchpad_kv_static(
        "OCR Lang",
        &settings_info.ocr_language_label,
    ));
    config_content.append(&launchpad_kv_static(
        "OCR Models",
        &settings_info.ocr_model_dir_label,
    ));
    config_content.append(&launchpad_kv_static(
        "config.json",
        &settings_info.config_path,
    ));
    config_content.append(&launchpad_kv_static(
        "theme.json",
        &settings_info.theme_config_path,
    ));
    config_content.append(&launchpad_kv_static(
        "keybindings",
        &settings_info.keybinding_config_path,
    ));
    let config_panel = launchpad_panel(style_tokens, "Configuration", &config_content);
    config_panel.set_hexpand(true);

    // ── 2-column info row ──
    let info_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_12);
    info_row.add_css_class("launchpad-info-row");
    info_row.append(&session_panel);
    info_row.append(&config_panel);

    // ── Actions panel (unified, 2 rows) ──
    let open_preview_button = Button::with_label("Open Preview");
    open_preview_button.set_hexpand(true);
    let open_editor_button = Button::with_label("Open Editor");
    open_editor_button.set_hexpand(true);
    let save_button = Button::with_label("Save");
    save_button.set_hexpand(true);
    let copy_button = Button::with_label("Copy");
    copy_button.set_hexpand(true);

    let actions_row1 = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
    actions_row1.append(&open_preview_button);
    actions_row1.append(&open_editor_button);
    actions_row1.append(&save_button);
    actions_row1.append(&copy_button);

    let close_preview_button = Button::with_label("Close Preview");
    close_preview_button.set_hexpand(true);
    let close_editor_button = Button::with_label("Close Editor");
    close_editor_button.set_hexpand(true);
    let ocr_button = Button::with_label("OCR");
    ocr_button.set_hexpand(true);
    let delete_button = Button::with_label("Delete");
    delete_button.set_hexpand(true);
    delete_button.add_css_class("launchpad-danger-button");

    let actions_row2 = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
    actions_row2.append(&close_preview_button);
    actions_row2.append(&close_editor_button);
    actions_row2.append(&ocr_button);
    actions_row2.append(&delete_button);

    let actions_content = GtkBox::new(Orientation::Vertical, style_tokens.spacing_8);
    actions_content.append(&actions_row1);
    actions_content.append(&actions_row2);
    let actions_panel = launchpad_panel(style_tokens, "Actions", &actions_content);

    // ── Scrollable content area ──
    let launchpad_content = GtkBox::new(Orientation::Vertical, style_tokens.spacing_12);
    launchpad_content.append(&capture_panel);
    launchpad_content.append(&info_row);
    launchpad_content.append(&actions_panel);

    let scrolled_window = ScrolledWindow::new();
    scrolled_window.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled_window.set_vexpand(true);
    scrolled_window.set_child(Some(&launchpad_content));

    let hint_label = Label::new(Some(
        "Buttons are enabled only when valid for the current state. (Idle \u{2192} Preview \u{2192} Editor)",
    ));
    hint_label.add_css_class("launchpad-hint");
    hint_label.set_halign(Align::Start);
    hint_label.set_xalign(0.0);
    hint_label.set_wrap(true);

    // ── Assemble root ──
    root.append(&header_row);
    root.append(&subtitle_label);
    root.append(&scrolled_window);
    root.append(&hint_label);
    root.append(&toast_label);

    if !show_launchpad {
        header_row.set_visible(false);
        subtitle_label.set_visible(false);
        scrolled_window.set_visible(false);
        hint_label.set_visible(false);
    }

    LaunchpadUi {
        root,
        toast_label,
        state_label,
        status_label,
        active_capture_label,
        capture_count_label,
        latest_label,
        capture_ids_label,
        full_capture_button,
        region_capture_button,
        window_capture_button,
        open_preview_button,
        open_editor_button,
        close_preview_button,
        close_editor_button,
        save_button,
        copy_button,
        ocr_button,
        delete_button,
    }
}

pub(super) fn connect_launchpad_button<F, R>(
    button: &Button,
    launchpad_actions: &LaunchpadActionExecutor,
    render: &Rc<R>,
    action: F,
) where
    F: Fn(&LaunchpadActionExecutor) + 'static,
    R: Fn() + 'static,
{
    let launchpad_actions = launchpad_actions.clone();
    let render = render.clone();
    button.connect_clicked(move |_| {
        action(&launchpad_actions);
        (render.as_ref())();
    });
}

pub(super) fn connect_launchpad_default_buttons<R: Fn() + 'static>(
    launchpad: &LaunchpadUi,
    launchpad_actions: &LaunchpadActionExecutor,
    render: &Rc<R>,
) {
    {
        let launchpad_actions = launchpad_actions.clone();
        let render = render.clone();
        launchpad.full_capture_button.connect_clicked(move |_| {
            let render = render.clone();
            launchpad_actions.capture_and_open_preview_async(
                capture::capture_full,
                "Captured full screen",
                "full capture failed",
                "Full capture failed",
                move || {
                    (render.as_ref())();
                },
            );
        });
    }
    {
        let launchpad_actions = launchpad_actions.clone();
        let render = render.clone();
        launchpad.region_capture_button.connect_clicked(move |_| {
            let render = render.clone();
            launchpad_actions.capture_and_open_preview_async(
                capture::capture_region,
                "Captured selected region",
                "region capture failed",
                "Region capture failed",
                move || {
                    (render.as_ref())();
                },
            );
        });
    }
    {
        let launchpad_actions = launchpad_actions.clone();
        let render = render.clone();
        launchpad.window_capture_button.connect_clicked(move |_| {
            let render = render.clone();
            launchpad_actions.capture_and_open_preview_async(
                capture::capture_window,
                "Captured selected window",
                "window capture failed",
                "Window capture failed",
                move || {
                    (render.as_ref())();
                },
            );
        });
    }
    connect_launchpad_button(
        &launchpad.open_preview_button,
        launchpad_actions,
        render,
        |actions| {
            actions.open_preview();
        },
    );
    connect_launchpad_button(
        &launchpad.open_editor_button,
        launchpad_actions,
        render,
        |actions| {
            actions.open_editor();
        },
    );
    connect_launchpad_button(
        &launchpad.close_preview_button,
        launchpad_actions,
        render,
        |actions| {
            actions.close_preview();
        },
    );
    connect_launchpad_button(
        &launchpad.close_editor_button,
        launchpad_actions,
        render,
        |actions| {
            actions.close_editor();
        },
    );
    {
        let launchpad_actions = launchpad_actions.clone();
        let render = render.clone();
        launchpad.save_button.connect_clicked(move |_| {
            let render = render.clone();
            launchpad_actions.run_preview_action_async(PreviewAction::Save, move || {
                (render.as_ref())();
            });
        });
    }
    {
        let launchpad_actions = launchpad_actions.clone();
        let render = render.clone();
        launchpad.copy_button.connect_clicked(move |_| {
            let render = render.clone();
            launchpad_actions.run_preview_action_async(PreviewAction::Copy, move || {
                (render.as_ref())();
            });
        });
    }
    connect_launchpad_button(
        &launchpad.ocr_button,
        launchpad_actions,
        render,
        |actions| {
            actions.run_preview_ocr_action();
        },
    );
    {
        let launchpad_actions = launchpad_actions.clone();
        let render = render.clone();
        launchpad.delete_button.connect_clicked(move |_| {
            let render = render.clone();
            launchpad_actions.delete_active_capture_async(move || {
                (render.as_ref())();
            });
        });
    }
}

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Once;
use std::time::Duration;

use crate::capture;
use crate::editor::tools::{CropElement, ImageBounds, RectangleElement};
use crate::editor::{self, EditorAction, ToolKind, ToolObject};
use crate::error::AppResult;
use crate::input::{
    resolve_shortcut, InputContext, ShortcutAction, ShortcutKey, TextInputAction, TextInputEvent,
};
use crate::preview::PreviewAction;
use crate::state::{
    AppEvent, AppState, RuntimeWindowGeometry, RuntimeWindowKind, RuntimeWindowState, StateMachine,
};
use crate::storage::StorageService;
use crate::ui::StyleTokens;
use gtk4::gdk::prelude::GdkCairoContextExt;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, Dialog, DrawingArea, Frame,
    Label, Orientation, Overflow, Overlay, ResponseType, Revealer, RevealerTransitionType, Scale,
    ScrolledWindow, ToggleButton,
};

mod adaptive;
mod bootstrap;
mod editor_history;
mod editor_popup;
mod editor_runtime;
mod editor_text_runtime;
mod editor_viewport;
mod hypr;
mod input_bridge;
mod launchpad_actions;
mod layout;
mod lifecycle;
mod preview_pin;
mod preview_runtime;
mod runtime_css;
mod runtime_support;

use self::adaptive::*;
use self::bootstrap::*;
use self::editor_history::*;
use self::editor_popup::*;
use self::editor_runtime::*;
use self::editor_text_runtime::*;
use self::editor_viewport::*;
use self::hypr::*;
use self::input_bridge::*;
use self::launchpad_actions::*;
use self::layout::*;
use self::lifecycle::*;
use self::preview_runtime::*;
use self::runtime_support::*;

const UI_TICK_INTERVAL: Duration = Duration::from_millis(100);
const EDITOR_PEN_ICON_NAME: &str = "pencil-symbolic";
const LUCIDE_ICON_RESOURCE_PATH: &str = "/com/github/bityoungjae/chalkak/icons/hicolor";
type ToolOptionsRefresh = Rc<dyn Fn(ToolKind)>;
type ToolOptionsRefreshSlot = RefCell<Option<ToolOptionsRefresh>>;
type SharedToolOptionsRefresh = Rc<ToolOptionsRefreshSlot>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextInputActivation {
    Auto,
    ForceOn,
    ForceOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ToolSwitchConfig {
    clear_pending_crop_when_not_crop: bool,
    text_input: TextInputActivation,
}

impl ToolSwitchConfig {
    const fn auto(clear_pending_crop_when_not_crop: bool) -> Self {
        Self {
            clear_pending_crop_when_not_crop,
            text_input: TextInputActivation::Auto,
        }
    }
}

fn text_input_should_be_active(selected_tool: ToolKind, activation: TextInputActivation) -> bool {
    match activation {
        TextInputActivation::Auto => selected_tool == ToolKind::Text,
        TextInputActivation::ForceOn => true,
        TextInputActivation::ForceOff => false,
    }
}

fn set_editor_pan_cursor<W: IsA<gtk4::Widget>>(
    widget: &W,
    active_editor_tool: &Cell<ToolKind>,
    space_pan_pressed: &Cell<bool>,
    drag_pan_active: &Cell<bool>,
) {
    let cursor = if drag_pan_active.get() {
        Some("grabbing")
    } else if active_editor_tool.get() == ToolKind::Pan || space_pan_pressed.get() {
        Some("grab")
    } else {
        None
    };

    widget.set_cursor_from_name(cursor);
}

fn set_active_editor_tool(
    active_editor_tool: &Cell<ToolKind>,
    tool: ToolKind,
    refresh_editor_cursor: &dyn Fn(),
) {
    active_editor_tool.set(tool);
    refresh_editor_cursor();
}

fn start_editor_text_input(
    editor_input_mode: &RefCell<editor::EditorInputMode>,
    text_im_context: &gtk4::IMMulticontext,
) {
    editor_input_mode.borrow_mut().start_text_input();
    text_im_context.focus_in();
}

fn stop_editor_text_input(
    editor_input_mode: &RefCell<editor::EditorInputMode>,
    text_im_context: &gtk4::IMMulticontext,
    text_preedit_state: &RefCell<TextPreeditState>,
) {
    editor_input_mode.borrow_mut().end_text_input();
    text_im_context.reset();
    text_im_context.focus_out();
    *text_preedit_state.borrow_mut() = TextPreeditState::default();
}

fn sync_editor_tool_controls(
    tool_buttons: &RefCell<Vec<(ToolKind, Button)>>,
    refresh_tool_options: &ToolOptionsRefreshSlot,
    selected_tool: ToolKind,
) {
    sync_active_tool_buttons(tool_buttons.borrow().as_slice(), selected_tool);
    if let Some(refresh) = refresh_tool_options.borrow().as_ref() {
        refresh(selected_tool);
    }
}

#[derive(Clone)]
struct EditorToolSwitchContext {
    active_editor_tool: Rc<Cell<ToolKind>>,
    editor_tools: Rc<RefCell<editor::EditorTools>>,
    editor_input_mode: Rc<RefCell<editor::EditorInputMode>>,
    tool_drag_preview: Rc<RefCell<Option<ToolDragPreview>>>,
    pending_crop: Rc<RefCell<Option<CropElement>>>,
    text_im_context: Rc<gtk4::IMMulticontext>,
    text_preedit_state: Rc<RefCell<TextPreeditState>>,
    tool_buttons: Rc<RefCell<Vec<(ToolKind, Button)>>>,
    refresh_tool_options: SharedToolOptionsRefresh,
    refresh_editor_cursor: Rc<dyn Fn()>,
}

impl EditorToolSwitchContext {
    fn switch_to(&self, selected_tool: ToolKind, clear_pending_crop_when_not_crop: bool) {
        self.switch_to_with_config(
            selected_tool,
            ToolSwitchConfig::auto(clear_pending_crop_when_not_crop),
        );
    }

    fn switch_to_with_config(&self, selected_tool: ToolKind, config: ToolSwitchConfig) {
        set_active_editor_tool(
            self.active_editor_tool.as_ref(),
            selected_tool,
            self.refresh_editor_cursor.as_ref(),
        );
        self.editor_tools.borrow_mut().select_tool(selected_tool);
        self.tool_drag_preview.borrow_mut().take();

        if config.clear_pending_crop_when_not_crop && selected_tool != ToolKind::Crop {
            self.pending_crop.borrow_mut().take();
        }

        if selected_tool == ToolKind::Crop {
            self.editor_input_mode.borrow_mut().activate_crop();
        } else {
            self.editor_input_mode.borrow_mut().deactivate_crop();
        }

        if text_input_should_be_active(selected_tool, config.text_input) {
            start_editor_text_input(
                self.editor_input_mode.as_ref(),
                self.text_im_context.as_ref(),
            );
        } else {
            stop_editor_text_input(
                self.editor_input_mode.as_ref(),
                self.text_im_context.as_ref(),
                self.text_preedit_state.as_ref(),
            );
        }

        sync_editor_tool_controls(
            self.tool_buttons.as_ref(),
            self.refresh_tool_options.as_ref(),
            selected_tool,
        );
    }
}

fn shortcut_editor_tool_switch(action: ShortcutAction) -> Option<(ToolKind, &'static str)> {
    match action {
        ShortcutAction::EditorEnterSelect => Some((ToolKind::Select, "editor select tool armed")),
        ShortcutAction::EditorEnterPan => Some((ToolKind::Pan, "editor pan tool armed")),
        ShortcutAction::EditorEnterBlur => Some((ToolKind::Blur, "editor blur tool armed")),
        ShortcutAction::EditorEnterPen => Some((ToolKind::Pen, "editor pen tool armed")),
        ShortcutAction::EditorEnterArrow => Some((ToolKind::Arrow, "editor arrow tool armed")),
        ShortcutAction::EditorEnterRectangle => {
            Some((ToolKind::Rectangle, "editor rectangle tool armed"))
        }
        ShortcutAction::EditorEnterCrop => Some((ToolKind::Crop, "editor crop interaction armed")),
        ShortcutAction::EditorEnterText => Some((ToolKind::Text, "editor text tool armed")),
        _ => None,
    }
}

fn icon_button(
    icon_name: &str,
    tooltip: &str,
    control_size: i32,
    extra_classes: &[&str],
) -> Button {
    let button = Button::from_icon_name(icon_name);
    button.set_focus_on_click(false);
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("flat");
    button.add_css_class("icon-button");
    for css_class in extra_classes {
        button.add_css_class(css_class);
    }
    button.set_size_request(control_size, control_size);
    button
}

fn install_lucide_icon_theme() {
    static ICON_THEME_SETUP: Once = Once::new();

    ICON_THEME_SETUP.call_once(|| {
        if let Err(err) = gtk4::gio::resources_register_include!("chalkak.gresource") {
            tracing::error!(?err, "failed to register bundled Lucide icon resources");
            return;
        }

        let Some(display) = gtk4::gdk::Display::default() else {
            tracing::warn!("failed to initialize Lucide icon theme; no display available");
            return;
        };

        let icon_theme = gtk4::IconTheme::for_display(&display);
        icon_theme.add_resource_path(LUCIDE_ICON_RESOURCE_PATH);
        tracing::debug!(
            pin = icon_theme.has_icon("pin-symbolic"),
            copy = icon_theme.has_icon("copy-symbolic"),
            save = icon_theme.has_icon("save-symbolic"),
            "registered bundled Lucide icon resource path"
        );
    });
}

fn icon_toggle_button(
    icon_name: &str,
    tooltip: &str,
    control_size: i32,
    extra_classes: &[&str],
) -> ToggleButton {
    let button = ToggleButton::new();
    button.set_icon_name(icon_name);
    button.set_focus_on_click(false);
    button.set_active(false);
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("flat");
    button.add_css_class("icon-button");
    for css_class in extra_classes {
        button.add_css_class(css_class);
    }
    button.set_size_request(control_size, control_size);
    button
}

fn editor_window_default_geometry(style_tokens: StyleTokens) -> RuntimeWindowGeometry {
    RuntimeWindowGeometry::new(
        style_tokens.editor_initial_width,
        style_tokens.editor_initial_height,
    )
}

fn editor_window_min_geometry(style_tokens: StyleTokens) -> RuntimeWindowGeometry {
    RuntimeWindowGeometry::new(
        style_tokens.editor_min_width,
        style_tokens.editor_min_height,
    )
}

#[derive(Clone)]
struct EditorRuntimeState {
    capture_id: Rc<RefCell<Option<String>>>,
    has_unsaved_changes: Rc<RefCell<bool>>,
    close_dialog_open: Rc<RefCell<bool>>,
    toast: Rc<RefCell<Option<ToastRuntime>>>,
    input_mode: Rc<RefCell<editor::EditorInputMode>>,
}

impl EditorRuntimeState {
    fn new() -> Self {
        Self {
            capture_id: Rc::new(RefCell::new(None)),
            has_unsaved_changes: Rc::new(RefCell::new(false)),
            close_dialog_open: Rc::new(RefCell::new(false)),
            toast: Rc::new(RefCell::new(None)),
            input_mode: Rc::new(RefCell::new(editor::EditorInputMode::new())),
        }
    }

    fn reset_session_state(&self) {
        *self.has_unsaved_changes.borrow_mut() = false;
        *self.close_dialog_open.borrow_mut() = false;
        self.input_mode.borrow_mut().reset();
    }

    fn clear_runtime_state(&self) {
        *self.capture_id.borrow_mut() = None;
        *self.toast.borrow_mut() = None;
        self.reset_session_state();
    }
}

fn reset_editor_session_state(editor_runtime: &EditorRuntimeState) {
    editor_runtime.reset_session_state();
}

fn clear_editor_runtime_state(editor_runtime: &EditorRuntimeState) {
    editor_runtime.clear_runtime_state();
}

#[derive(Clone)]
struct LaunchpadUi {
    root: GtkBox,
    toast_label: Label,
    state_label: Label,
    status_label: Label,
    active_capture_label: Label,
    capture_count_label: Label,
    latest_label: Label,
    capture_ids_label: Label,
    full_capture_button: Button,
    region_capture_button: Button,
    window_capture_button: Button,
    open_preview_button: Button,
    open_editor_button: Button,
    close_preview_button: Button,
    close_editor_button: Button,
    save_button: Button,
    copy_button: Button,
    delete_button: Button,
}

impl LaunchpadUi {
    fn update_overview(
        &self,
        state: AppState,
        active_capture_id: &str,
        latest_capture_label: &str,
        ids: &[String],
    ) {
        self.state_label.set_text(&format!("State: {:?}", state));
        self.active_capture_label
            .set_text(&format!("Active capture: {active_capture_id}"));
        self.capture_count_label
            .set_text(&format!("Capture count: {}", ids.len()));
        self.latest_label
            .set_text(&format!("Latest capture: {latest_capture_label}"));
        self.capture_ids_label
            .set_text(&format_capture_ids_for_display(ids));
    }

    fn set_action_availability(&self, state: AppState, has_capture: bool) {
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
        self.delete_button
            .set_sensitive(matches!(state, AppState::Preview) && has_capture);
    }

    fn set_status_text(&self, message: &str) {
        self.status_label.set_text(&format!("Status: {message}"));
    }
}

fn launchpad_value_label(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.add_css_class("launchpad-key-value");
    label.set_halign(Align::Start);
    label.set_xalign(0.0);
    label
}

fn launchpad_section_title(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.add_css_class("launchpad-section-title");
    label.set_halign(Align::Start);
    label.set_xalign(0.0);
    label
}

fn launchpad_panel(style_tokens: StyleTokens, title: &str, child: &GtkBox) -> Frame {
    let panel = Frame::new(None);
    panel.add_css_class("launchpad-panel");
    let panel_box = GtkBox::new(Orientation::Vertical, style_tokens.spacing_8);
    panel_box.append(&launchpad_section_title(title));
    panel_box.append(child);
    panel.set_child(Some(&panel_box));
    panel
}

fn format_capture_ids_for_display(ids: &[String]) -> String {
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

fn build_launchpad_ui(style_tokens: StyleTokens, show_launchpad: bool) -> LaunchpadUi {
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

    let title_label = Label::new(Some("ChalKak Launchpad"));
    title_label.add_css_class("launchpad-title");
    title_label.set_halign(Align::Start);
    title_label.set_xalign(0.0);

    let subtitle_label = Label::new(Some(
        "Quick control panel for validating capture, preview, and editor flow.",
    ));
    subtitle_label.add_css_class("launchpad-subtitle");
    subtitle_label.set_halign(Align::Start);
    subtitle_label.set_xalign(0.0);
    subtitle_label.set_wrap(true);

    let state_label = launchpad_value_label("State: initializing");
    let status_label = launchpad_value_label("Status: Ready");
    let active_capture_label = launchpad_value_label("Active capture: none");
    let capture_count_label = launchpad_value_label("Capture count: 0");
    let latest_label = launchpad_value_label("No capture yet");
    let capture_ids_label = Label::new(Some("IDs: none"));
    capture_ids_label.add_css_class("launchpad-capture-ids");
    capture_ids_label.set_halign(Align::Start);
    capture_ids_label.set_xalign(0.0);
    capture_ids_label.set_wrap(true);
    capture_ids_label.set_selectable(true);

    let info_content = GtkBox::new(Orientation::Vertical, style_tokens.spacing_8);
    info_content.append(&state_label);
    info_content.append(&status_label);
    info_content.append(&active_capture_label);
    info_content.append(&capture_count_label);
    info_content.append(&latest_label);
    info_content.append(&capture_ids_label);
    let info_panel = launchpad_panel(style_tokens, "Session Overview", &info_content);

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

    let open_preview_button = Button::with_label("Open Preview");
    open_preview_button.set_hexpand(true);
    let open_editor_button = Button::with_label("Open Editor");
    open_editor_button.set_hexpand(true);
    let close_preview_button = Button::with_label("Close Preview");
    close_preview_button.set_hexpand(true);
    let close_editor_button = Button::with_label("Close Editor");
    close_editor_button.set_hexpand(true);
    let preview_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
    preview_row.append(&open_preview_button);
    preview_row.append(&open_editor_button);
    preview_row.append(&close_preview_button);
    preview_row.append(&close_editor_button);
    let preview_panel = launchpad_panel(style_tokens, "Preview / Editor", &preview_row);

    let save_button = Button::with_label("Save");
    save_button.set_hexpand(true);
    let copy_button = Button::with_label("Copy");
    copy_button.set_hexpand(true);
    let delete_button = Button::with_label("Delete");
    delete_button.set_hexpand(true);
    delete_button.add_css_class("launchpad-danger-button");
    let action_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
    action_row.append(&save_button);
    action_row.append(&copy_button);
    action_row.append(&delete_button);
    let action_panel = launchpad_panel(style_tokens, "Capture Actions", &action_row);

    let hint_label = Label::new(Some(
        "Buttons are enabled only when valid for the current state. (Idle -> Preview -> Editor)",
    ));
    hint_label.add_css_class("launchpad-hint");
    hint_label.set_halign(Align::Start);
    hint_label.set_xalign(0.0);
    hint_label.set_wrap(true);

    let launchpad_content = GtkBox::new(Orientation::Vertical, style_tokens.spacing_12);
    launchpad_content.append(&info_panel);
    launchpad_content.append(&capture_panel);
    launchpad_content.append(&preview_panel);
    launchpad_content.append(&action_panel);

    root.append(&title_label);
    root.append(&subtitle_label);
    root.append(&launchpad_content);
    root.append(&hint_label);
    root.append(&toast_label);

    if !show_launchpad {
        title_label.set_visible(false);
        subtitle_label.set_visible(false);
        launchpad_content.set_visible(false);
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
        delete_button,
    }
}

fn close_editor_if_open_and_clear(
    editor_window: &Rc<RefCell<Option<ApplicationWindow>>>,
    runtime_window_state: &Rc<RefCell<RuntimeWindowState>>,
    editor_close_guard: &Rc<Cell<bool>>,
    editor_runtime: &EditorRuntimeState,
    style_tokens: StyleTokens,
) -> bool {
    if close_editor_window_if_open(
        editor_window,
        runtime_window_state,
        editor_close_guard,
        editor_window_default_geometry(style_tokens),
        editor_window_min_geometry(style_tokens),
    ) {
        clear_editor_runtime_state(editor_runtime);
        true
    } else {
        false
    }
}

#[derive(Clone)]
struct EditorOutputActionRuntime {
    runtime_session: Rc<RefCell<RuntimeSession>>,
    shared_machine: Rc<RefCell<StateMachine>>,
    storage_service: Rc<Option<StorageService>>,
    status_log: Rc<RefCell<String>>,
    editor_toast: ToastRuntime,
    editor_tools: Rc<RefCell<editor::EditorTools>>,
    pending_crop: Rc<RefCell<Option<CropElement>>>,
    editor_source_pixbuf: Option<gtk4::gdk_pixbuf::Pixbuf>,
    editor_has_unsaved_changes: Rc<RefCell<bool>>,
    toast_duration_ms: u32,
}

impl EditorOutputActionRuntime {
    fn run(&self, action: EditorAction, action_label: &'static str) -> bool {
        let active_capture = match self.runtime_session.borrow().active_capture().cloned() {
            Some(artifact) => artifact,
            None => {
                *self.status_log.borrow_mut() =
                    format!("{action_label} requires an active capture");
                return false;
            }
        };

        if !matches!(self.shared_machine.borrow().state(), AppState::Editor) {
            *self.status_log.borrow_mut() = format!("editor {action_label} requires editor state");
            return false;
        }

        let Some(service) = self.storage_service.as_ref().as_ref() else {
            *self.status_log.borrow_mut() = "storage service unavailable".to_string();
            return false;
        };

        let Some(source_pixbuf) = self.editor_source_pixbuf.as_ref() else {
            *self.status_log.borrow_mut() = "editor source image unavailable".to_string();
            self.editor_toast
                .show("Source image unavailable", self.toast_duration_ms);
            return false;
        };

        let tools = self.editor_tools.borrow();
        execute_editor_output_action(EditorOutputActionContext {
            action,
            active_capture: &active_capture,
            editor_tools: &tools,
            pending_crop: self.pending_crop.borrow().as_ref().copied(),
            source_pixbuf,
            storage_service: service,
            status_log: &self.status_log,
            editor_toast: &self.editor_toast,
            toast_duration_ms: self.toast_duration_ms,
            editor_has_unsaved_changes: &self.editor_has_unsaved_changes,
        })
    }
}

fn connect_launchpad_button<F, R>(
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

fn connect_launchpad_default_buttons<R: Fn() + 'static>(
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
            launchpad_actions.run_preview_action_async(
                PreviewAction::CopyFileReference,
                move || {
                    (render.as_ref())();
                },
            );
        });
    }
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

fn run_startup_capture<R: Fn() + 'static>(
    launchpad_actions: &LaunchpadActionExecutor,
    startup_capture: StartupCaptureMode,
    on_complete: R,
) {
    match startup_capture {
        StartupCaptureMode::Full => launchpad_actions.capture_and_open_preview_async(
            capture::capture_full,
            "Captured full screen",
            "full capture failed",
            "Full capture failed",
            on_complete,
        ),
        StartupCaptureMode::Region => launchpad_actions.capture_and_open_preview_async(
            capture::capture_region,
            "Captured selected region",
            "region capture failed",
            "Region capture failed",
            on_complete,
        ),
        StartupCaptureMode::Window => launchpad_actions.capture_and_open_preview_async(
            capture::capture_window,
            "Captured selected window",
            "window capture failed",
            "Window capture failed",
            on_complete,
        ),
        StartupCaptureMode::None => {}
    }
}

fn should_release_headless_startup_hold(
    hold_active: bool,
    startup_capture_completed: bool,
    state: AppState,
    has_active_capture: bool,
    preview_window_count: usize,
    editor_window_open: bool,
) -> bool {
    hold_active
        && startup_capture_completed
        && matches!(state, AppState::Idle)
        && !has_active_capture
        && preview_window_count == 0
        && !editor_window_open
}

pub struct App {
    machine: StateMachine,
}

impl App {
    pub fn new() -> Self {
        Self {
            machine: StateMachine::new(),
        }
    }

    pub fn start(&mut self) -> AppResult<()> {
        let bootstrap = bootstrap_app_runtime();
        let startup_config = bootstrap.startup_config;
        let theme_config = bootstrap.theme_config;
        let editor_navigation_bindings = bootstrap.editor_navigation_bindings;

        tracing::info!(event = "start", from = ?self.machine.state());
        let _ = self.machine.transition(AppEvent::Start)?;

        let runtime_session = Rc::new(RefCell::new(RuntimeSession::default()));
        let shared_machine = Rc::new(RefCell::new(std::mem::take(&mut self.machine)));
        let storage_service = initialize_storage_service();

        tracing::info!("starting gtk runtime");
        let application = Application::new(
            Some("com.github.bityoungjae.chalkak"),
            gtk4::gio::ApplicationFlags::NON_UNIQUE,
        );

        let status_log = Rc::new(RefCell::new(String::from(
            "Ready. Capture to open preview/editor flow.",
        )));
        let status_log_for_activate = status_log.clone();
        let runtime_session_for_activate = runtime_session.clone();
        let machine_for_activate = shared_machine.clone();
        let runtime_window_state = Rc::new(RefCell::new(RuntimeWindowState::default()));
        let storage_service = Rc::new(storage_service);
        let storage_service_for_activate = storage_service.clone();
        let preview_windows = Rc::new(RefCell::new(HashMap::<String, PreviewWindowRuntime>::new()));
        let preview_action_target_capture_id = Rc::new(RefCell::new(None::<String>));
        let editor_runtime = Rc::new(EditorRuntimeState::new());
        let editor_window = Rc::new(RefCell::new(None::<ApplicationWindow>));
        let editor_capture_id = editor_runtime.capture_id.clone();
        let editor_has_unsaved_changes = editor_runtime.has_unsaved_changes.clone();
        let editor_close_dialog_open = editor_runtime.close_dialog_open.clone();
        let editor_input_mode = editor_runtime.input_mode.clone();
        let editor_toast = editor_runtime.toast.clone();
        let editor_close_guard = Rc::new(Cell::new(false));
        let editor_navigation_bindings = Rc::new(editor_navigation_bindings);
        let startup_capture = startup_config.capture;
        let show_launchpad = startup_config.show_launchpad;
        let headless_startup_capture =
            !show_launchpad && !matches!(startup_capture, StartupCaptureMode::None);
        let activate_once = Rc::new(Cell::new(false));

        application.connect_activate(move |app| {
            if activate_once.replace(true) {
                tracing::debug!("ignoring duplicate gtk activate signal");
                return;
            }
            install_lucide_icon_theme();
            let headless_hold_guard =
                Rc::new(RefCell::new(None::<gtk4::gio::ApplicationHoldGuard>));
            let startup_capture_completed = Rc::new(Cell::new(!headless_startup_capture));
            if headless_startup_capture {
                tracing::info!("holding app lifecycle for headless startup capture");
                let hold_guard =
                    <gtk4::Application as gtk4::gio::prelude::ApplicationExtManual>::hold(app);
                headless_hold_guard.borrow_mut().replace(hold_guard);
            }
            let gtk_settings = gtk4::Settings::default();
            let theme_mode = resolve_runtime_theme_mode(theme_config.mode, gtk_settings.as_ref());
            let resolved_theme_runtime = resolve_theme_runtime(&theme_config, theme_mode);
            let style_tokens = resolved_theme_runtime.style_tokens;
            let color_tokens = resolved_theme_runtime.color_tokens;
            let rectangle_border_radius_override = resolved_theme_runtime
                .editor_theme_overrides
                .rectangle_border_radius;
            let editor_selection_palette = resolved_theme_runtime
                .editor_theme_overrides
                .selection_palette;
            let default_tool_color_override = resolved_theme_runtime
                .editor_theme_overrides
                .default_tool_color;
            let default_text_size_override = resolved_theme_runtime
                .editor_theme_overrides
                .default_text_size;
            let default_stroke_width_override = resolved_theme_runtime
                .editor_theme_overrides
                .default_stroke_width;
            let editor_tool_option_presets = resolved_theme_runtime.editor_tool_option_presets;
            tracing::info!(
                requested_mode = ?theme_config.mode,
                resolved_mode = ?theme_mode,
                "resolved runtime theme mode"
            );
            let motion_enabled = gtk_settings
                .as_ref()
                .map(|settings| settings.is_gtk_enable_animations())
                .unwrap_or(true);
            let motion_hover_ms = if motion_enabled {
                style_tokens.motion_hover_ms
            } else {
                0
            };
            install_runtime_css(style_tokens, &color_tokens, motion_enabled);
            let window = ApplicationWindow::new(app);
            window.add_css_class("chalkak-root");
            window.set_title(Some("ChalKak"));
            window.set_default_size(760, 520);

            let launchpad = build_launchpad_ui(style_tokens, show_launchpad);
            let launchpad_toast_runtime = ToastRuntime::new(&launchpad.toast_label);
            let open_editor_button = launchpad.open_editor_button.clone();
            let close_preview_button = launchpad.close_preview_button.clone();
            let close_editor_button = launchpad.close_editor_button.clone();
            let save_button = launchpad.save_button.clone();
            let copy_button = launchpad.copy_button.clone();
            let delete_button = launchpad.delete_button.clone();

            window.set_child(Some(&launchpad.root));
            let app_for_preview = app.clone();
            let app_for_lifecycle = app.clone();
            let preview_render_context = PreviewRenderContext::new(
                app_for_preview.clone(),
                style_tokens,
                motion_hover_ms,
                status_log_for_activate.clone(),
                save_button.clone(),
                copy_button.clone(),
                open_editor_button.clone(),
                close_preview_button.clone(),
                delete_button.clone(),
                preview_windows.clone(),
                preview_action_target_capture_id.clone(),
                runtime_window_state.clone(),
                editor_window.clone(),
                editor_close_guard.clone(),
                editor_runtime.clone(),
            );
            let editor_render_context = EditorRenderContext {
                preview_windows: preview_windows.clone(),
                runtime_window_state: runtime_window_state.clone(),
                editor_window: editor_window.clone(),
                editor_capture_id: editor_capture_id.clone(),
                editor_close_guard: editor_close_guard.clone(),
                editor_runtime: editor_runtime.clone(),
                app_for_preview: app_for_preview.clone(),
                motion_hover_ms,
                runtime_session: runtime_session_for_activate.clone(),
                style_tokens,
                theme_mode,
                editor_selection_palette,
                rectangle_border_radius_override,
                default_tool_color_override,
                default_text_size_override,
                default_stroke_width_override,
                editor_tool_option_presets: editor_tool_option_presets.clone(),
                editor_navigation_bindings: editor_navigation_bindings.clone(),
                status_log_for_render: status_log_for_activate.clone(),
                editor_input_mode: editor_input_mode.clone(),
                editor_has_unsaved_changes: editor_has_unsaved_changes.clone(),
                editor_close_dialog_open: editor_close_dialog_open.clone(),
                editor_toast: editor_toast.clone(),
                close_editor_button: close_editor_button.clone(),
                storage_service: storage_service_for_activate.clone(),
                shared_machine: machine_for_activate.clone(),
            };

            let render = {
                let runtime_session = runtime_session_for_activate.clone();
                let shared_machine = machine_for_activate.clone();
                let launchpad = launchpad.clone();
                let runtime_window_state = runtime_window_state.clone();
                let preview_windows = preview_windows.clone();
                let preview_render_context = preview_render_context.clone();
                let editor_render_context = editor_render_context.clone();
                let editor_runtime = editor_runtime.clone();
                let editor_window = editor_window.clone();
                let editor_close_guard = editor_close_guard.clone();
                let status_log_for_render = status_log_for_activate.clone();
                let app_for_lifecycle = app_for_lifecycle.clone();
                let headless_hold_guard = headless_hold_guard.clone();
                let startup_capture_completed = startup_capture_completed.clone();

                Rc::new(move || {
                    let runtime = runtime_session.borrow();
                    let state = shared_machine.borrow().state();
                    let has_capture = runtime.active_capture().is_some();
                    let active_capture = runtime.active_capture().cloned();
                    let captures = runtime.captures_for_display();
                    let ids = runtime.ids_for_display();
                    let active_capture_id = active_capture
                        .as_ref()
                        .map(|artifact| artifact.capture_id.clone())
                        .unwrap_or_else(|| "none".to_string());

                    launchpad.update_overview(
                        state,
                        &active_capture_id,
                        &runtime.latest_label_text(),
                        &ids,
                    );
                    launchpad.set_action_availability(state, has_capture);

                    match state {
                        AppState::Preview => {
                            render_preview_state(&preview_render_context, &captures);
                        }
                        AppState::Editor => {
                            render_editor_state(&editor_render_context, active_capture.clone());
                        }
                        _ => {
                            close_all_preview_windows(&preview_windows, &runtime_window_state);
                            close_editor_if_open_and_clear(
                                &editor_window,
                                &runtime_window_state,
                                &editor_close_guard,
                                &editor_runtime,
                                style_tokens,
                            );
                        }
                    }

                    launchpad.set_status_text(status_log_for_render.borrow().as_str());

                    let preview_window_count = preview_windows.borrow().len();
                    let editor_window_open = editor_window.borrow().is_some();
                    if should_release_headless_startup_hold(
                        headless_hold_guard.borrow().is_some(),
                        startup_capture_completed.get(),
                        state,
                        has_capture,
                        preview_window_count,
                        editor_window_open,
                    ) {
                        tracing::info!("releasing headless startup capture hold");
                        let _ = headless_hold_guard.borrow_mut().take();
                        app_for_lifecycle.quit();
                    }
                })
            };

            {
                install_preview_hover_tick(preview_windows.clone(), UI_TICK_INTERVAL);
            }

            let launchpad_actions = LaunchpadActionExecutor::new(
                runtime_session_for_activate.clone(),
                preview_action_target_capture_id.clone(),
                machine_for_activate.clone(),
                storage_service_for_activate.clone(),
                status_log_for_activate.clone(),
                preview_windows.clone(),
                runtime_window_state.clone(),
                launchpad_toast_runtime.clone(),
                style_tokens.toast_duration_ms,
            );
            connect_launchpad_default_buttons(&launchpad, &launchpad_actions, &render);

            {
                let render = render.clone();
                render();
            }

            run_startup_capture(&launchpad_actions, startup_capture, {
                let render = render.clone();
                let startup_capture_completed = startup_capture_completed.clone();
                move || {
                    startup_capture_completed.set(true);
                    (render.as_ref())();
                }
            });

            tracing::info!("presenting startup launcher window");
            if show_launchpad {
                window.present();
            } else {
                window.close();
            }
        });

        // Pass only argv[0] to GTK so app-specific flags (e.g. --launchpad) do not fail GTK parsing.
        let gtk_args = gtk_launch_args();
        application.run_with_args(&gtk_args);

        let remaining_capture_ids = runtime_session.borrow().ids_for_display();
        cleanup_remaining_session_artifacts(
            storage_service.as_ref().as_ref(),
            &remaining_capture_ids,
        );

        self.machine = std::mem::take(&mut *shared_machine.borrow_mut());
        Ok(())
    }

    pub fn state(&self) -> &StateMachine {
        &self.machine
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypr_client_address_from_json_matches_exact_title() {
        let payload = br#"
[
  {"address":"0x100","title":"Preview - first"},
  {"address":"0x200","title":"Preview - second"}
]
"#;
        let address = hypr_client_address_from_json(payload, "Preview - second");
        assert_eq!(address.as_deref(), Some("0x200"));
    }

    #[test]
    fn hypr_client_address_from_json_ignores_non_object_entries() {
        let payload = br#"
[
  "ok",
  {"address":"0x300","title":"Preview - stable"}
]
"#;
        let address = hypr_client_address_from_json(payload, "Preview - stable");
        assert_eq!(address.as_deref(), Some("0x300"));
    }

    #[test]
    fn hypr_client_match_from_json_reads_pin_state_when_available() {
        let payload = br#"
[
  {"address":"0x111","title":"chalkak-preview","size":[384,244],"pinned":true}
]
"#;
        let matched =
            hypr_client_match_from_json(payload, "chalkak-preview").expect("expected client");
        assert_eq!(matched.address, "0x111");
        assert!(matched.pinned);
    }

    #[test]
    fn zoom_slider_mapping_preserves_min_max_bounds() {
        assert_eq!(
            zoom_percent_from_slider_value(0.0),
            editor::EditorViewport::min_zoom_percent()
        );
        assert_eq!(
            zoom_percent_from_slider_value(ZOOM_SLIDER_STEPS),
            editor::EditorViewport::max_zoom_percent()
        );
        assert_eq!(
            zoom_slider_value_for_percent(editor::EditorViewport::min_zoom_percent()),
            0.0
        );
        assert_eq!(
            zoom_slider_value_for_percent(editor::EditorViewport::max_zoom_percent()),
            ZOOM_SLIDER_STEPS
        );
    }

    #[test]
    fn zoom_slider_mapping_round_trip_stays_near_input_levels() {
        for zoom_percent in [5_u16, 25, 50, 100, 200, 400, 800, 1600] {
            let slider_value = zoom_slider_value_for_percent(zoom_percent);
            let mapped = zoom_percent_from_slider_value(slider_value);
            assert!(
                (i32::from(mapped) - i32::from(zoom_percent)).abs() <= 1,
                "zoom_percent={zoom_percent}, mapped={mapped}, slider_value={slider_value}"
            );
        }
    }

    #[test]
    fn editor_window_geometry_helpers_match_style_tokens() {
        let tokens = crate::ui::LAYOUT_TOKENS;
        assert_eq!(
            editor_window_default_geometry(tokens),
            RuntimeWindowGeometry::new(tokens.editor_initial_width, tokens.editor_initial_height)
        );
        assert_eq!(
            editor_window_min_geometry(tokens),
            RuntimeWindowGeometry::new(tokens.editor_min_width, tokens.editor_min_height)
        );
    }

    #[test]
    fn reset_editor_session_state_clears_dirty_flags_and_modes() {
        let editor_runtime = EditorRuntimeState::new();
        *editor_runtime.has_unsaved_changes.borrow_mut() = true;
        *editor_runtime.close_dialog_open.borrow_mut() = true;
        editor_runtime.input_mode.borrow_mut().activate_crop();

        reset_editor_session_state(&editor_runtime);

        assert!(!*editor_runtime.has_unsaved_changes.borrow());
        assert!(!*editor_runtime.close_dialog_open.borrow());
        assert!(!editor_runtime.input_mode.borrow().crop_active());
        assert!(!editor_runtime.input_mode.borrow().text_input_active());
    }

    #[test]
    fn format_capture_ids_for_display_returns_none_for_empty_ids() {
        assert_eq!(format_capture_ids_for_display(&[]), "IDs: none");
    }

    #[test]
    fn format_capture_ids_for_display_numbers_each_capture_id() {
        let ids = vec!["capture-a".to_string(), "capture-b".to_string()];
        assert_eq!(
            format_capture_ids_for_display(&ids),
            "IDs:\n 1. capture-a\n 2. capture-b"
        );
    }

    #[test]
    fn shortcut_editor_tool_switch_maps_tool_shortcuts() {
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorEnterSelect),
            Some((ToolKind::Select, "editor select tool armed"))
        );
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorEnterPan),
            Some((ToolKind::Pan, "editor pan tool armed"))
        );
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorEnterBlur),
            Some((ToolKind::Blur, "editor blur tool armed"))
        );
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorEnterPen),
            Some((ToolKind::Pen, "editor pen tool armed"))
        );
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorEnterArrow),
            Some((ToolKind::Arrow, "editor arrow tool armed"))
        );
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorEnterRectangle),
            Some((ToolKind::Rectangle, "editor rectangle tool armed"))
        );
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorEnterCrop),
            Some((ToolKind::Crop, "editor crop interaction armed"))
        );
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorEnterText),
            Some((ToolKind::Text, "editor text tool armed"))
        );
    }

    #[test]
    fn shortcut_editor_tool_switch_ignores_non_tool_actions() {
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::EditorSave),
            None
        );
        assert_eq!(
            shortcut_editor_tool_switch(ShortcutAction::CropCancel),
            None
        );
    }

    #[test]
    fn text_input_activation_auto_follows_text_tool() {
        assert!(text_input_should_be_active(
            ToolKind::Text,
            TextInputActivation::Auto
        ));
        assert!(!text_input_should_be_active(
            ToolKind::Select,
            TextInputActivation::Auto
        ));
    }

    #[test]
    fn text_input_activation_force_modes_override_tool_kind() {
        assert!(text_input_should_be_active(
            ToolKind::Select,
            TextInputActivation::ForceOn
        ));
        assert!(!text_input_should_be_active(
            ToolKind::Text,
            TextInputActivation::ForceOff
        ));
    }

    #[test]
    fn should_release_headless_startup_hold_only_when_idle_without_runtime_windows() {
        assert!(should_release_headless_startup_hold(
            true,
            true,
            AppState::Idle,
            false,
            0,
            false
        ));

        assert!(!should_release_headless_startup_hold(
            false,
            true,
            AppState::Idle,
            false,
            0,
            false
        ));
        assert!(!should_release_headless_startup_hold(
            true,
            false,
            AppState::Idle,
            false,
            0,
            false
        ));
        assert!(!should_release_headless_startup_hold(
            true,
            true,
            AppState::Preview,
            false,
            0,
            false
        ));
        assert!(!should_release_headless_startup_hold(
            true,
            true,
            AppState::Idle,
            true,
            0,
            false
        ));
        assert!(!should_release_headless_startup_hold(
            true,
            true,
            AppState::Idle,
            false,
            1,
            false
        ));
        assert!(!should_release_headless_startup_hold(
            true,
            true,
            AppState::Idle,
            false,
            0,
            true
        ));
    }
}

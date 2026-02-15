use super::*;

#[derive(Clone)]
pub(super) struct EditorRenderContext {
    pub(super) preview_windows: Rc<RefCell<HashMap<String, PreviewWindowRuntime>>>,
    pub(super) runtime_window_state: Rc<RefCell<RuntimeWindowState>>,
    pub(super) editor_window: Rc<RefCell<Option<ApplicationWindow>>>,
    pub(super) editor_capture_id: Rc<RefCell<Option<String>>>,
    pub(super) editor_close_guard: Rc<Cell<bool>>,
    pub(super) editor_runtime: Rc<EditorRuntimeState>,
    pub(super) app_for_preview: Application,
    pub(super) motion_hover_ms: u32,
    pub(super) runtime_session: Rc<RefCell<RuntimeSession>>,
    pub(super) style_tokens: StyleTokens,
    pub(super) theme_mode: crate::theme::ThemeMode,
    pub(super) rectangle_border_radius_override: Option<u16>,
    pub(super) default_tool_color_override: Option<(u8, u8, u8)>,
    pub(super) default_text_size_override: Option<u8>,
    pub(super) default_stroke_width_override: Option<u8>,
    pub(super) editor_tool_option_presets: EditorToolOptionPresets,
    pub(super) editor_navigation_bindings: Rc<crate::input::EditorNavigationBindings>,
    pub(super) status_log_for_render: Rc<RefCell<String>>,
    pub(super) editor_input_mode: Rc<RefCell<editor::EditorInputMode>>,
    pub(super) editor_has_unsaved_changes: Rc<RefCell<bool>>,
    pub(super) editor_close_dialog_open: Rc<RefCell<bool>>,
    pub(super) editor_toast: Rc<RefCell<Option<ToastRuntime>>>,
    pub(super) close_editor_button: Button,
    pub(super) storage_service: Rc<Option<StorageService>>,
    pub(super) shared_machine: Rc<RefCell<StateMachine>>,
}

mod wiring;

use self::wiring::*;

type EditorCursorRefresh = Rc<dyn Fn()>;
type SharedEditorCursorRefreshImpl = Rc<RefCell<Option<EditorCursorRefresh>>>;

struct EditorCanvasDrawDeps {
    editor_tools: Rc<RefCell<editor::EditorTools>>,
    tool_drag_preview: Rc<RefCell<Option<ToolDragPreview>>>,
    selected_object_ids: Rc<RefCell<Vec<u64>>>,
    pending_crop: Rc<RefCell<Option<CropElement>>>,
    editor_input_mode: Rc<RefCell<editor::EditorInputMode>>,
    text_preedit_state: Rc<RefCell<TextPreeditState>>,
    text_im_context: Rc<gtk4::IMMulticontext>,
}

struct InitializedEditorRuntime {
    editor_navigation_bindings: Rc<crate::input::EditorNavigationBindings>,
    editor_tools: Rc<RefCell<editor::EditorTools>>,
    tool_option_presets: EditorToolOptionPresets,
    editor_undo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    editor_redo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    active_editor_tool: Rc<Cell<ToolKind>>,
    tool_drag_preview: Rc<RefCell<Option<ToolDragPreview>>>,
    tool_drag_start_canvas: Rc<Cell<(f64, f64)>>,
    active_pen_stroke_id: Rc<Cell<Option<u64>>>,
    selected_object_ids: Rc<RefCell<Vec<u64>>>,
    object_drag_state: Rc<RefCell<Option<ObjectDragState>>>,
    pending_crop: Rc<RefCell<Option<CropElement>>>,
    tool_buttons: Rc<RefCell<Vec<(ToolKind, Button)>>>,
    refresh_tool_options: SharedToolOptionsRefresh,
    text_im_context: Rc<gtk4::IMMulticontext>,
    text_preedit_state: Rc<RefCell<TextPreeditState>>,
    refresh_editor_cursor_impl: SharedEditorCursorRefreshImpl,
    refresh_editor_cursor: EditorCursorRefresh,
    editor_tool_switch_context: EditorToolSwitchContext,
}

fn build_editor_window_shell(
    app: &Application,
    saved_geometry: Option<RuntimeWindowGeometry>,
    style_tokens: StyleTokens,
    capture_id: &str,
) -> (ApplicationWindow, String, RuntimeWindowGeometry) {
    let editor_window_instance = ApplicationWindow::new(app);
    let editor_title = format!("Editor - {capture_id}");
    editor_window_instance.set_title(Some(&editor_title));
    editor_window_instance.set_decorated(false);
    editor_window_instance.add_css_class("chalkak-root");
    editor_window_instance.add_css_class("floating-editor-window");
    let geometry = saved_geometry.unwrap_or(RuntimeWindowGeometry::new(
        style_tokens.editor_initial_width,
        style_tokens.editor_initial_height,
    ));
    let resolved_geometry = RuntimeWindowGeometry::with_position(
        geometry.x,
        geometry.y,
        geometry.width.max(style_tokens.editor_min_width),
        geometry.height.max(style_tokens.editor_min_height),
    );
    editor_window_instance.set_default_size(resolved_geometry.width, resolved_geometry.height);
    editor_window_instance.set_resizable(true);
    editor_window_instance.set_size_request(
        style_tokens.editor_min_width,
        style_tokens.editor_min_height,
    );
    (editor_window_instance, editor_title, resolved_geometry)
}

fn initialize_editor_runtime_state(
    editor_window_instance: &ApplicationWindow,
    editor_navigation_bindings: &Rc<crate::input::EditorNavigationBindings>,
    editor_input_mode: &Rc<RefCell<editor::EditorInputMode>>,
    editor_tool_option_presets: &EditorToolOptionPresets,
) -> InitializedEditorRuntime {
    let editor_navigation_bindings = editor_navigation_bindings.clone();
    let editor_tools = Rc::new(RefCell::new(editor::EditorTools::new()));
    let tool_option_presets = editor_tool_option_presets.clone();
    let editor_undo_stack = Rc::new(RefCell::new(Vec::<Vec<ToolObject>>::new()));
    let editor_redo_stack = Rc::new(RefCell::new(Vec::<Vec<ToolObject>>::new()));
    let active_editor_tool = Rc::new(Cell::new(ToolKind::Select));
    let tool_drag_preview = Rc::new(RefCell::new(None::<ToolDragPreview>));
    let tool_drag_start_canvas = Rc::new(Cell::new((0.0_f64, 0.0_f64)));
    let active_pen_stroke_id = Rc::new(Cell::new(None::<u64>));
    let selected_object_ids = Rc::new(RefCell::new(Vec::<u64>::new()));
    let object_drag_state = Rc::new(RefCell::new(None::<ObjectDragState>));
    let pending_crop = Rc::new(RefCell::new(None::<CropElement>));
    let tool_buttons = Rc::new(RefCell::new(Vec::<(ToolKind, Button)>::new()));
    let refresh_tool_options: SharedToolOptionsRefresh = Rc::new(RefCell::new(None));
    let text_im_context = Rc::new(gtk4::IMMulticontext::new());
    text_im_context.set_client_widget(Some(editor_window_instance));
    text_im_context.set_use_preedit(true);
    let text_preedit_state = Rc::new(RefCell::new(TextPreeditState::default()));
    let refresh_editor_cursor_impl: SharedEditorCursorRefreshImpl = Rc::new(RefCell::new(None));
    let refresh_editor_cursor: EditorCursorRefresh = {
        let refresh_editor_cursor_impl = refresh_editor_cursor_impl.clone();
        Rc::new(move || {
            if let Some(refresh) = refresh_editor_cursor_impl.borrow().as_ref() {
                refresh();
            }
        })
    };
    let editor_tool_switch_context = EditorToolSwitchContext {
        active_editor_tool: active_editor_tool.clone(),
        editor_tools: editor_tools.clone(),
        editor_input_mode: editor_input_mode.clone(),
        tool_drag_preview: tool_drag_preview.clone(),
        pending_crop: pending_crop.clone(),
        text_im_context: text_im_context.clone(),
        text_preedit_state: text_preedit_state.clone(),
        tool_buttons: tool_buttons.clone(),
        refresh_tool_options: refresh_tool_options.clone(),
        refresh_editor_cursor: refresh_editor_cursor.clone(),
    };

    InitializedEditorRuntime {
        editor_navigation_bindings,
        editor_tools,
        tool_option_presets,
        editor_undo_stack,
        editor_redo_stack,
        active_editor_tool,
        tool_drag_preview,
        tool_drag_start_canvas,
        active_pen_stroke_id,
        selected_object_ids,
        object_drag_state,
        pending_crop,
        tool_buttons,
        refresh_tool_options,
        text_im_context,
        text_preedit_state,
        refresh_editor_cursor_impl,
        refresh_editor_cursor,
        editor_tool_switch_context,
    }
}

fn resolve_editor_tool_fallback_shortcut(shortcut_key: ShortcutKey) -> Option<ToolKind> {
    if !matches!(shortcut_key, ShortcutKey::Character(_)) {
        return None;
    }
    let action = resolve_shortcut(
        shortcut_key,
        crate::input::ShortcutModifiers::default(),
        InputContext {
            in_editor: true,
            ..Default::default()
        },
    )?;
    shortcut_editor_tool_switch(action).map(|(tool, _)| tool)
}

fn connect_editor_tool_shortcut_fallback(
    editor_window_instance: &ApplicationWindow,
    tool_buttons: &Rc<RefCell<Vec<(ToolKind, Button)>>>,
    editor_input_mode: &Rc<RefCell<editor::EditorInputMode>>,
) {
    let tool_buttons = tool_buttons.clone();
    let editor_input_mode = editor_input_mode.clone();
    let tool_shortcut_controller = gtk4::EventControllerKey::new();
    tool_shortcut_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    tool_shortcut_controller.connect_key_pressed(move |_, key, keycode, modifier| {
        if editor_input_mode.borrow().text_input_active()
            || modifier.intersects(
                gtk4::gdk::ModifierType::CONTROL_MASK
                    | gtk4::gdk::ModifierType::ALT_MASK
                    | gtk4::gdk::ModifierType::SUPER_MASK
                    | gtk4::gdk::ModifierType::META_MASK,
            )
        {
            return gtk4::glib::Propagation::Proceed;
        }
        let Some(shortcut_key) = normalize_shortcut_key(key, keycode) else {
            return gtk4::glib::Propagation::Proceed;
        };
        let Some(tool) = resolve_editor_tool_fallback_shortcut(shortcut_key) else {
            return gtk4::glib::Propagation::Proceed;
        };
        let maybe_button = {
            let buttons = tool_buttons.borrow();
            buttons
                .iter()
                .find(|(kind, _)| *kind == tool)
                .map(|(_, button)| button.clone())
        };
        if let Some(button) = maybe_button {
            tracing::debug!(
                tool = ?tool,
                "editor tool shortcut fallback triggered"
            );
            button.emit_clicked();
            return gtk4::glib::Propagation::Stop;
        }
        gtk4::glib::Propagation::Proceed
    });
    editor_window_instance.add_controller(tool_shortcut_controller);
}

fn connect_editor_overlay_hover_controls(
    editor_overlay: &Overlay,
    top_controls_left_revealer: &Revealer,
    top_controls_right_revealer: &Revealer,
    bottom_left_controls_revealer: &Revealer,
    bottom_right_controls_revealer: &Revealer,
) {
    let pointer = gtk4::EventControllerMotion::new();
    {
        let top_controls_left_revealer = top_controls_left_revealer.clone();
        let top_controls_right_revealer = top_controls_right_revealer.clone();
        let bottom_left_controls_revealer = bottom_left_controls_revealer.clone();
        let bottom_right_controls_revealer = bottom_right_controls_revealer.clone();
        pointer.connect_enter(move |_, _, _| {
            top_controls_left_revealer.set_reveal_child(true);
            top_controls_left_revealer.set_can_target(true);
            top_controls_right_revealer.set_reveal_child(true);
            top_controls_right_revealer.set_can_target(true);
            bottom_left_controls_revealer.set_reveal_child(true);
            bottom_left_controls_revealer.set_can_target(true);
            bottom_right_controls_revealer.set_reveal_child(true);
            bottom_right_controls_revealer.set_can_target(true);
        });
    }
    {
        let top_controls_left_revealer = top_controls_left_revealer.clone();
        let top_controls_right_revealer = top_controls_right_revealer.clone();
        let bottom_left_controls_revealer = bottom_left_controls_revealer.clone();
        let bottom_right_controls_revealer = bottom_right_controls_revealer.clone();
        pointer.connect_motion(move |_, _, _| {
            top_controls_left_revealer.set_reveal_child(true);
            top_controls_left_revealer.set_can_target(true);
            top_controls_right_revealer.set_reveal_child(true);
            top_controls_right_revealer.set_can_target(true);
            bottom_left_controls_revealer.set_reveal_child(true);
            bottom_left_controls_revealer.set_can_target(true);
            bottom_right_controls_revealer.set_reveal_child(true);
            bottom_right_controls_revealer.set_can_target(true);
        });
    }
    {
        let top_controls_left_revealer = top_controls_left_revealer.clone();
        let top_controls_right_revealer = top_controls_right_revealer.clone();
        let bottom_left_controls_revealer = bottom_left_controls_revealer.clone();
        let bottom_right_controls_revealer = bottom_right_controls_revealer.clone();
        pointer.connect_leave(move |_| {
            top_controls_left_revealer.set_reveal_child(false);
            top_controls_left_revealer.set_can_target(false);
            top_controls_right_revealer.set_reveal_child(false);
            top_controls_right_revealer.set_can_target(false);
            bottom_left_controls_revealer.set_reveal_child(false);
            bottom_left_controls_revealer.set_can_target(false);
            bottom_right_controls_revealer.set_reveal_child(false);
            bottom_right_controls_revealer.set_can_target(false);
        });
    }
    editor_overlay.add_controller(pointer);
}

const STROKE_PREVIEW_ENDPOINT_MARGIN: f64 = 4.5;
const STROKE_PREVIEW_STROKE_PADDING: i32 = 2;

fn stroke_preview_line_width(thickness: u8, preview_width: i32, preview_height: i32) -> f64 {
    let requested = f64::from(thickness.max(1));
    let vertical_limit = f64::from(
        preview_height
            .max(1)
            .saturating_sub(STROKE_PREVIEW_STROKE_PADDING),
    );
    let cap_limit = (STROKE_PREVIEW_ENDPOINT_MARGIN * 2.0).max(1.0);
    let width_limit = f64::from(preview_width.max(1));
    requested
        .min(vertical_limit.max(1.0))
        .min(cap_limit)
        .min(width_limit)
        .max(1.0)
}

#[allow(clippy::too_many_arguments)]
fn apply_editor_default_tool_settings(
    editor_tools: &Rc<RefCell<editor::EditorTools>>,
    tool_option_presets: &EditorToolOptionPresets,
    default_tool_color_override: Option<(u8, u8, u8)>,
    default_text_size_override: Option<u8>,
    default_stroke_width_override: Option<u8>,
    rectangle_border_radius_override: Option<u16>,
    editor_image_base_width: i32,
    editor_image_base_height: i32,
) {
    let default_text_size = default_text_size_override.unwrap_or_else(|| {
        adaptive_text_size_for_image_with_presets(
            editor_image_base_width,
            editor_image_base_height,
            tool_option_presets.text_size_presets(),
        )
    });
    let default_stroke_size = default_stroke_width_override.unwrap_or_else(|| {
        adaptive_stroke_size_for_image_with_presets(
            editor_image_base_width,
            editor_image_base_height,
            tool_option_presets.adaptive_stroke_width_presets(),
        )
    });
    let (default_stroke_r, default_stroke_g, default_stroke_b) = default_tool_color_override
        .unwrap_or_else(|| tool_option_presets.stroke_color_palette().default_color());
    let mut tools = editor_tools.borrow_mut();
    tools.set_text_size(default_text_size);
    tools.set_shared_stroke_color(default_stroke_r, default_stroke_g, default_stroke_b);
    tools.set_shared_stroke_thickness(default_stroke_size);
    if let Some(radius) = rectangle_border_radius_override {
        tools.set_rectangle_border_radius(radius);
    }
    tools.set_arrow_head_size(default_stroke_size.saturating_mul(3).max(8));
}

fn configure_editor_canvas_draw(
    editor_canvas: &DrawingArea,
    editor_source_pixbuf: Option<gtk4::gdk_pixbuf::Pixbuf>,
    deps: EditorCanvasDrawDeps,
) {
    let Some(pixbuf) = editor_source_pixbuf else {
        return;
    };
    let EditorCanvasDrawDeps {
        editor_tools,
        tool_drag_preview,
        selected_object_ids,
        pending_crop,
        editor_input_mode,
        text_preedit_state,
        text_im_context,
    } = deps;
    let blur_render_cache = Rc::new(RefCell::new(BlurRenderCache::default()));
    let blur_render_cache_for_draw = blur_render_cache.clone();
    editor_canvas.set_draw_func(move |_, context, width, height| {
        if width <= 0 || height <= 0 {
            return;
        }
        let source_width = f64::from(pixbuf.width().max(1));
        let source_height = f64::from(pixbuf.height().max(1));
        let scale_x = width as f64 / source_width;
        let scale_y = height as f64 / source_height;
        context.save().ok();
        context.scale(scale_x, scale_y);
        context.set_source_pixbuf(&pixbuf, 0.0, 0.0);
        context.paint().ok();
        let tools = editor_tools.borrow();
        let preedit_state = text_preedit_state.borrow().clone();
        draw_editor_tool_objects(
            context,
            &tools,
            ToolRenderContext {
                image_bounds: ImageBounds::new(pixbuf.width(), pixbuf.height()),
                show_crop_mask: true,
                selected_object_ids: selected_object_ids.borrow().as_slice(),
                source_pixbuf: Some(&pixbuf),
                active_text_id: tools.active_text_id(),
                active_text_preedit: Some(&preedit_state),
                blur_cache: Some(&blur_render_cache_for_draw),
            },
        );
        if editor_input_mode.borrow().text_input_active() {
            if let Some(active_text) = tools.active_text() {
                let preedit = if preedit_state.content.is_empty() {
                    None
                } else {
                    Some(&preedit_state)
                };
                let caret = text_caret_layout(context, active_text, preedit);
                let cursor_rect = caret_layout_to_canvas_cursor_rect(
                    caret,
                    width,
                    height,
                    pixbuf.width(),
                    pixbuf.height(),
                );
                text_im_context.set_cursor_location(&cursor_rect);
            }
        }
        if let Some(crop) = pending_crop.borrow().as_ref().copied() {
            draw_crop_mask(
                context,
                crop.x,
                crop.y,
                crop.width,
                crop.height,
                pixbuf.width(),
                pixbuf.height(),
            );
            context.save().ok();
            context.set_source_rgba(1.0, 1.0, 1.0, 0.95);
            context.set_line_width(2.0);
            context.rectangle(
                f64::from(crop.x),
                f64::from(crop.y),
                f64::from(crop.width),
                f64::from(crop.height),
            );
            let _ = context.stroke();
            context.restore().ok();
        }
        if let Some(preview) = tool_drag_preview.borrow().as_ref().copied() {
            draw_drag_preview_overlay(context, &preview, &tools, pixbuf.width(), pixbuf.height());
        }
        context.restore().ok();
    });
}

pub(super) fn render_editor_state(
    context: &EditorRenderContext,
    active_capture: Option<capture::CaptureArtifact>,
) {
    let preview_windows = &context.preview_windows;
    let runtime_window_state = &context.runtime_window_state;
    let editor_window = &context.editor_window;
    let editor_capture_id = &context.editor_capture_id;
    let editor_close_guard = &context.editor_close_guard;
    let editor_runtime = &context.editor_runtime;
    let app_for_preview = context.app_for_preview.clone();
    let motion_hover_ms = context.motion_hover_ms;
    let runtime_session = context.runtime_session.clone();
    let style_tokens = context.style_tokens;
    let theme_mode = context.theme_mode;
    let rectangle_border_radius_override = context.rectangle_border_radius_override;
    let default_tool_color_override = context.default_tool_color_override;
    let default_text_size_override = context.default_text_size_override;
    let default_stroke_width_override = context.default_stroke_width_override;
    let editor_tool_option_presets = context.editor_tool_option_presets.clone();
    let editor_navigation_bindings = &context.editor_navigation_bindings;
    let status_log_for_render = &context.status_log_for_render;
    let editor_input_mode = &context.editor_input_mode;
    let editor_has_unsaved_changes = &context.editor_has_unsaved_changes;
    let editor_close_dialog_open = &context.editor_close_dialog_open;
    let editor_toast = &context.editor_toast;
    let close_editor_button = &context.close_editor_button;
    let storage_service = &context.storage_service;
    let shared_machine = &context.shared_machine;
    let preview_anchor = active_capture.as_ref().and_then(|artifact| {
        let preview_title = format!("Preview - {}", artifact.capture_id);
        current_window_center(&preview_title)
    });

    close_all_preview_windows(preview_windows, runtime_window_state);

    if let Some(artifact) = active_capture {
        let current_editor_id = editor_capture_id.borrow().clone();
        let needs_new_window = editor_window.borrow().is_none()
            || current_editor_id != Some(artifact.capture_id.clone());
        if needs_new_window {
            let saved_editor_geometry = runtime_window_state
                .borrow()
                .geometry_for(RuntimeWindowKind::Editor);
            let _ = close_editor_window_if_open(
                editor_window,
                runtime_window_state,
                editor_close_guard,
                editor_window_default_geometry(style_tokens),
                editor_window_min_geometry(style_tokens),
            );

            let (editor_window_instance, editor_title, editor_window_geometry) =
                build_editor_window_shell(
                    &app_for_preview,
                    saved_editor_geometry,
                    style_tokens,
                    &artifact.capture_id,
                );
            let InitializedEditorRuntime {
                editor_navigation_bindings,
                editor_tools,
                tool_option_presets,
                editor_undo_stack,
                editor_redo_stack,
                active_editor_tool,
                tool_drag_preview,
                tool_drag_start_canvas,
                active_pen_stroke_id,
                selected_object_ids,
                object_drag_state,
                pending_crop,
                tool_buttons,
                refresh_tool_options,
                text_im_context,
                text_preedit_state,
                refresh_editor_cursor_impl,
                refresh_editor_cursor,
                editor_tool_switch_context,
            } = initialize_editor_runtime_state(
                &editor_window_instance,
                editor_navigation_bindings,
                editor_input_mode,
                &editor_tool_option_presets,
            );

            let editor_overlay = Overlay::new();
            editor_overlay.add_css_class("transparent-bg");
            let editor_root = GtkBox::new(Orientation::Vertical, 0);
            editor_root.set_overflow(Overflow::Hidden);

            let top_toolbar_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
            top_toolbar_row.add_css_class("editor-toolbar");
            for (tool_kind, icon_name, tooltip) in EDITOR_TOOLBAR_ENTRIES {
                let button = icon_button(
                    icon_name,
                    tooltip,
                    style_tokens.control_size as i32,
                    &["editor-tool-button"],
                );
                connect_tool_button_selection(
                    &button,
                    tool_kind,
                    &editor_tool_switch_context,
                    status_log_for_render,
                );
                if tool_kind == ToolKind::Select {
                    button.add_css_class("tool-active");
                }
                tool_buttons.borrow_mut().push((tool_kind, button.clone()));
                top_toolbar_row.append(&button);
            }

            let canvas_panel = GtkBox::new(Orientation::Vertical, 0);
            canvas_panel.add_css_class("editor-canvas");
            canvas_panel.set_hexpand(true);
            canvas_panel.set_vexpand(true);
            let editor_source_pixbuf =
                gtk4::gdk_pixbuf::Pixbuf::from_file(&artifact.temp_path).ok();
            if editor_source_pixbuf.is_none() {
                tracing::error!(
                    capture_id = artifact.capture_id,
                    path = %artifact.temp_path.display(),
                    "failed to load editor pixbuf from capture artifact"
                );
            }
            let editor_image_base_width = editor_source_pixbuf.as_ref().map_or_else(
                || i32::try_from(artifact.width).unwrap_or(i32::MAX),
                gtk4::gdk_pixbuf::Pixbuf::width,
            );
            let editor_image_base_height = editor_source_pixbuf.as_ref().map_or_else(
                || i32::try_from(artifact.height).unwrap_or(i32::MAX),
                gtk4::gdk_pixbuf::Pixbuf::height,
            );
            apply_editor_default_tool_settings(
                &editor_tools,
                &tool_option_presets,
                default_tool_color_override,
                default_text_size_override,
                default_stroke_width_override,
                rectangle_border_radius_override,
                editor_image_base_width,
                editor_image_base_height,
            );
            let editor_canvas = DrawingArea::new();
            editor_canvas.set_hexpand(false);
            editor_canvas.set_vexpand(false);
            editor_canvas.set_halign(Align::Start);
            editor_canvas.set_valign(Align::Start);
            editor_canvas.set_focusable(true);
            text_im_context.set_client_widget(Some(&editor_canvas));
            configure_editor_canvas_draw(
                &editor_canvas,
                editor_source_pixbuf.clone(),
                EditorCanvasDrawDeps {
                    editor_tools: editor_tools.clone(),
                    tool_drag_preview: tool_drag_preview.clone(),
                    selected_object_ids: selected_object_ids.clone(),
                    pending_crop: pending_crop.clone(),
                    editor_input_mode: editor_input_mode.clone(),
                    text_preedit_state: text_preedit_state.clone(),
                    text_im_context: text_im_context.clone(),
                },
            );
            let editor_scroller = ScrolledWindow::new();
            editor_scroller.set_hexpand(true);
            editor_scroller.set_vexpand(true);
            editor_scroller.set_focusable(true);
            editor_scroller.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
            editor_scroller.set_child(Some(&editor_canvas));
            canvas_panel.append(&editor_scroller);

            let editor_viewport = Rc::new(RefCell::new(editor::EditorViewport::new()));
            let editor_viewport_status = Label::new(None);
            editor_viewport_status.set_xalign(0.0);
            let zoom_slider_syncing = Rc::new(Cell::new(false));
            let zoom_slider =
                Scale::with_range(Orientation::Horizontal, 0.0, ZOOM_SLIDER_STEPS, 1.0);
            zoom_slider.add_css_class("accent-slider");
            zoom_slider.add_css_class("editor-zoom-slider");
            zoom_slider.set_draw_value(false);
            zoom_slider.set_focusable(false);
            zoom_slider.set_width_request(180);
            let space_pan_pressed = Rc::new(Cell::new(false));
            let drag_pan_active = Rc::new(Cell::new(false));
            let drag_pan_pointer_origin = Rc::new(Cell::new((0.0_f64, 0.0_f64)));
            let scroll_pointer_anchor = Rc::new(Cell::new(None::<(f64, f64)>));
            {
                let editor_scroller = editor_scroller.clone();
                let editor_canvas = editor_canvas.clone();
                let active_editor_tool = active_editor_tool.clone();
                let space_pan_pressed = space_pan_pressed.clone();
                let drag_pan_active = drag_pan_active.clone();
                *refresh_editor_cursor_impl.borrow_mut() = Some(Rc::new(move || {
                    set_editor_pan_cursor(
                        &editor_scroller,
                        &active_editor_tool,
                        &space_pan_pressed,
                        &drag_pan_active,
                    );
                    set_editor_pan_cursor(
                        &editor_canvas,
                        &active_editor_tool,
                        &space_pan_pressed,
                        &drag_pan_active,
                    );
                }));
            }
            refresh_editor_cursor();
            {
                let mut viewport = editor_viewport.borrow_mut();
                apply_editor_viewport_and_refresh(
                    &mut viewport,
                    &editor_viewport_runtime(
                        &editor_canvas,
                        &editor_scroller,
                        &editor_viewport_status,
                        &zoom_slider,
                        zoom_slider_syncing.as_ref(),
                        editor_image_base_width.max(1),
                        editor_image_base_height.max(1),
                    ),
                );
            }
            {
                let scroll_pointer_anchor_enter = scroll_pointer_anchor.clone();
                let pointer_motion = gtk4::EventControllerMotion::new();
                pointer_motion.connect_enter(move |_, x, y| {
                    scroll_pointer_anchor_enter.set(Some((x, y)));
                });
                let scroll_pointer_anchor_motion = scroll_pointer_anchor.clone();
                pointer_motion.connect_motion(move |_, x, y| {
                    scroll_pointer_anchor_motion.set(Some((x, y)));
                });
                let scroll_pointer_anchor_leave = scroll_pointer_anchor.clone();
                pointer_motion.connect_leave(move |_| {
                    scroll_pointer_anchor_leave.set(None);
                });
                editor_scroller.add_controller(pointer_motion);
            }
            {
                let editor_canvas = editor_canvas.clone();
                let focus_click = gtk4::GestureClick::new();
                focus_click.set_button(gtk4::gdk::BUTTON_PRIMARY);
                focus_click.connect_pressed(move |_, _, _, _| {
                    editor_canvas.grab_focus();
                });
                editor_scroller.add_controller(focus_click);
            }
            {
                connect_editor_text_click_gesture(EditorTextClickContext {
                    editor_canvas: editor_canvas.clone(),
                    editor_tools: editor_tools.clone(),
                    active_editor_tool: active_editor_tool.clone(),
                    editor_undo_stack: editor_undo_stack.clone(),
                    editor_redo_stack: editor_redo_stack.clone(),
                    status_log_for_render: status_log_for_render.clone(),
                    editor_has_unsaved_changes: editor_has_unsaved_changes.clone(),
                    space_pan_pressed: space_pan_pressed.clone(),
                    selected_object_ids: selected_object_ids.clone(),
                    text_preedit_state: text_preedit_state.clone(),
                    editor_tool_switch_context: editor_tool_switch_context.clone(),
                    editor_image_base_width,
                    editor_image_base_height,
                });
            }
            {
                connect_editor_selection_click_gesture(EditorSelectionClickContext {
                    editor_canvas: editor_canvas.clone(),
                    editor_tools: editor_tools.clone(),
                    active_editor_tool: active_editor_tool.clone(),
                    selected_object_ids: selected_object_ids.clone(),
                    status_log_for_render: status_log_for_render.clone(),
                    space_pan_pressed: space_pan_pressed.clone(),
                    text_preedit_state: text_preedit_state.clone(),
                    editor_tool_switch_context: editor_tool_switch_context.clone(),
                    editor_image_base_width,
                    editor_image_base_height,
                });
            }
            {
                connect_editor_draw_gesture(EditorDrawGestureContext {
                    editor_canvas: editor_canvas.clone(),
                    editor_tools: editor_tools.clone(),
                    active_editor_tool: active_editor_tool.clone(),
                    tool_drag_preview: tool_drag_preview.clone(),
                    tool_drag_start_canvas: tool_drag_start_canvas.clone(),
                    active_pen_stroke_id: active_pen_stroke_id.clone(),
                    editor_undo_stack: editor_undo_stack.clone(),
                    editor_redo_stack: editor_redo_stack.clone(),
                    status_log_for_render: status_log_for_render.clone(),
                    editor_has_unsaved_changes: editor_has_unsaved_changes.clone(),
                    space_pan_pressed: space_pan_pressed.clone(),
                    selected_object_ids: selected_object_ids.clone(),
                    object_drag_state: object_drag_state.clone(),
                    pending_crop: pending_crop.clone(),
                    editor_image_base_width,
                    editor_image_base_height,
                });
            }
            {
                let editor_viewport = editor_viewport.clone();
                let editor_canvas = editor_canvas.clone();
                let editor_scroller_for_zoom = editor_scroller.clone();
                let editor_viewport_status = editor_viewport_status.clone();
                let status_log_for_render = status_log_for_render.clone();
                let editor_navigation_bindings = editor_navigation_bindings.clone();
                let scroll_pointer_anchor = scroll_pointer_anchor.clone();
                let zoom_slider = zoom_slider.clone();
                let zoom_slider_syncing = zoom_slider_syncing.clone();
                let zoom_scroll = gtk4::EventControllerScroll::new(
                    gtk4::EventControllerScrollFlags::BOTH_AXES
                        | gtk4::EventControllerScrollFlags::DISCRETE,
                );
                zoom_scroll.set_propagation_phase(gtk4::PropagationPhase::Capture);
                zoom_scroll.connect_scroll(move |controller, _, dy| {
                    let state = modifier_state(controller.current_event_state());
                    if !editor_navigation_bindings.matches_zoom_scroll_modifier(state) {
                        return gtk4::glib::Propagation::Proceed;
                    }
                    if dy == 0.0 {
                        return gtk4::glib::Propagation::Stop;
                    }

                    let mut viewport = editor_viewport.borrow_mut();
                    let (anchor_x, anchor_y) = scroll_pointer_anchor
                        .get()
                        .unwrap_or_else(|| scroller_center_anchor(&editor_scroller_for_zoom));
                    let viewport_runtime = editor_viewport_runtime(
                        &editor_canvas,
                        &editor_scroller_for_zoom,
                        &editor_viewport_status,
                        &zoom_slider,
                        zoom_slider_syncing.as_ref(),
                        editor_image_base_width,
                        editor_image_base_height,
                    );
                    let zoom_in = dy < 0.0;
                    zoom_editor_viewport_and_refresh(
                        &mut viewport,
                        zoom_in,
                        &viewport_runtime,
                        anchor_x,
                        anchor_y,
                    );
                    *status_log_for_render.borrow_mut() = format!(
                        "editor viewport zoom {}% via wheel",
                        viewport.zoom_percent()
                    );
                    gtk4::glib::Propagation::Stop
                });
                editor_scroller.add_controller(zoom_scroll);
            }
            let schedule_fit_settle_pass =
                build_fit_settle_pass_scheduler(FitSettleSchedulerContext {
                    editor_viewport: editor_viewport.clone(),
                    editor_canvas: editor_canvas.clone(),
                    editor_scroller: editor_scroller.clone(),
                    editor_viewport_status: editor_viewport_status.clone(),
                    zoom_slider: zoom_slider.clone(),
                    zoom_slider_syncing: zoom_slider_syncing.clone(),
                    editor_image_base_width,
                    editor_image_base_height,
                });
            {
                // Apply initial fit exactly once after layout is ready.
                let initial_fit_pending = Rc::new(Cell::new(true));
                let initial_fit_retry_armed = Rc::new(Cell::new(false));
                let try_initial_fit_once = Rc::new({
                    let initial_fit_pending = initial_fit_pending.clone();
                    let editor_viewport = editor_viewport.clone();
                    let editor_canvas = editor_canvas.clone();
                    let editor_scroller_for_fit = editor_scroller.clone();
                    let editor_viewport_status = editor_viewport_status.clone();
                    let zoom_slider = zoom_slider.clone();
                    let zoom_slider_syncing = zoom_slider_syncing.clone();
                    let schedule_fit_settle_pass = schedule_fit_settle_pass.clone();
                    move |trigger: &'static str| -> bool {
                        const INITIAL_FIT_MIN_READY_EXTENT: i32 = 64;
                        if !initial_fit_pending.get() {
                            return true;
                        }
                        if !editor_scroller_for_fit.is_mapped() {
                            tracing::debug!(
                                trigger,
                                "skipping initial editor fit; scroller is not mapped"
                            );
                            return false;
                        }
                        let allocated_width = editor_scroller_for_fit.allocated_width();
                        let allocated_height = editor_scroller_for_fit.allocated_height();
                        if allocated_width <= INITIAL_FIT_MIN_READY_EXTENT
                            || allocated_height <= INITIAL_FIT_MIN_READY_EXTENT
                        {
                            tracing::debug!(
                                trigger,
                                allocated_width,
                                allocated_height,
                                "skipping initial editor fit; scroller allocation not ready"
                            );
                            return false;
                        }
                        tracing::debug!(
                            trigger,
                            allocated_width,
                            allocated_height,
                            "running initial editor fit"
                        );
                        initial_fit_pending.set(false);
                        let mut viewport = editor_viewport.borrow_mut();
                        apply_fit_zoom_once(
                            &mut viewport,
                            &editor_viewport_runtime(
                                &editor_canvas,
                                &editor_scroller_for_fit,
                                &editor_viewport_status,
                                &zoom_slider,
                                zoom_slider_syncing.as_ref(),
                                editor_image_base_width,
                                editor_image_base_height,
                            ),
                            "initial",
                        );
                        (schedule_fit_settle_pass.as_ref())("initial-settle");
                        true
                    }
                });
                {
                    let try_initial_fit_once = try_initial_fit_once.clone();
                    let initial_fit_pending = initial_fit_pending.clone();
                    let initial_fit_retry_armed = initial_fit_retry_armed.clone();
                    let editor_scroller_for_retry = editor_scroller.clone();
                    editor_scroller.connect_map(move |_| {
                    if (try_initial_fit_once.as_ref())("map") {
                        return;
                    }
                    if initial_fit_retry_armed.replace(true) {
                        return;
                    }
                    tracing::debug!(
                        "arming initial editor fit retry timer"
                    );
                    let try_initial_fit_once =
                        try_initial_fit_once.clone();
                    let initial_fit_pending =
                        initial_fit_pending.clone();
                    let initial_fit_retry_armed =
                        initial_fit_retry_armed.clone();
                    let editor_scroller_for_retry =
                        editor_scroller_for_retry.clone();
                    let remaining_attempts = Rc::new(Cell::new(120u16));
                    gtk4::glib::timeout_add_local(
                        Duration::from_millis(16),
                        move || {
                            if !editor_scroller_for_retry.is_mapped() {
                                initial_fit_retry_armed.set(false);
                                tracing::debug!(
                                    "stopping initial editor fit retry timer; scroller is not mapped"
                                );
                                return gtk4::glib::ControlFlow::Break;
                            }
                            if !initial_fit_pending.get() {
                                initial_fit_retry_armed.set(false);
                                return gtk4::glib::ControlFlow::Break;
                            }
                            let attempts_left =
                                remaining_attempts.get();
                            if attempts_left == 0 {
                                initial_fit_retry_armed.set(false);
                                tracing::warn!(
                                    "initial editor fit retry timer exhausted before layout became ready"
                                );
                                return gtk4::glib::ControlFlow::Break;
                            }
                            remaining_attempts
                                .set(attempts_left.saturating_sub(1));
                            if (try_initial_fit_once.as_ref())(
                                "retry-timer",
                            ) {
                                initial_fit_retry_armed.set(false);
                                tracing::debug!(
                                    "initial editor fit applied from retry timer"
                                );
                                gtk4::glib::ControlFlow::Break
                            } else {
                                gtk4::glib::ControlFlow::Continue
                            }
                        },
                    );
                });
                }
                {
                    let try_initial_fit_once = try_initial_fit_once.clone();
                    editor_canvas.connect_resize(move |_, width, height| {
                        tracing::debug!(
                            width,
                            height,
                            "editor canvas resized; checking initial fit"
                        );
                        (try_initial_fit_once.as_ref())("canvas-resize");
                    });
                }
            }

            {
                let viewport = editor_viewport.borrow();
                sync_editor_zoom_slider(
                    &zoom_slider,
                    zoom_slider_syncing.as_ref(),
                    &viewport,
                    &editor_canvas,
                    editor_image_base_width,
                    editor_image_base_height,
                );
            }
            let viewport_fit_button = icon_button(
                "scan-symbolic",
                "Fit to window once (Shift+1)",
                style_tokens.control_size as i32,
                &["editor-action-button"],
            );

            let editor_undo_button = icon_button(
                "undo-2-symbolic",
                "Undo (Ctrl+Z)",
                style_tokens.control_size as i32,
                &["editor-action-button"],
            );
            let editor_redo_button = icon_button(
                "redo-2-symbolic",
                "Redo (Ctrl+Shift+Z)",
                style_tokens.control_size as i32,
                &["editor-action-button"],
            );
            let editor_save_button = icon_button(
                "save-symbolic",
                "Save (Ctrl+S)",
                style_tokens.control_size as i32,
                &["editor-action-button"],
            );
            let editor_copy_button = icon_button(
                "copy-symbolic",
                "Copy (Ctrl+C)",
                style_tokens.control_size as i32,
                &["editor-action-button"],
            );
            let editor_copy_file_reference_button = icon_button(
                AI_REFERENCE_ICON_NAME,
                "Copy file URI reference",
                style_tokens.control_size as i32,
                &["editor-action-button"],
            );
            let editor_close_button = icon_button(
                "x-symbolic",
                "Close editor",
                style_tokens.control_size as i32,
                &["editor-action-button", "editor-close-button"],
            );
            editor_close_button.set_valign(Align::Center);
            editor_root.append(&canvas_panel);
            let editor_surface = Frame::new(None);
            editor_surface.add_css_class("editor-surface");
            editor_surface.set_hexpand(true);
            editor_surface.set_vexpand(true);
            editor_surface.set_child(Some(&editor_root));
            editor_overlay.set_child(Some(&editor_surface));

            // ── Top controls: split left/right to avoid a full-width hit target ──
            let top_controls_left = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
            top_controls_left.set_halign(Align::Start);
            top_controls_left.set_valign(Align::Start);
            top_controls_left.set_margin_top(style_tokens.spacing_16);
            top_controls_left.set_margin_bottom(style_tokens.spacing_16);
            top_controls_left.set_margin_start(style_tokens.spacing_16);
            top_controls_left.set_margin_end(style_tokens.spacing_16);
            top_controls_left.add_css_class("transparent-bg");

            // History group (undo/redo)
            let history_group = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
            history_group.add_css_class("editor-action-group");
            history_group.append(&editor_undo_button);
            history_group.append(&editor_redo_button);
            top_controls_left.append(&history_group);

            // Tool selector group
            top_controls_left.append(&top_toolbar_row);

            // File actions group (save/copy/copy file URI)
            let file_actions_group = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
            file_actions_group.add_css_class("editor-action-group");
            file_actions_group.append(&editor_save_button);
            file_actions_group.append(&editor_copy_button);
            file_actions_group.append(&editor_copy_file_reference_button);
            top_controls_left.append(&file_actions_group);
            let top_controls_left_revealer = Revealer::new();
            top_controls_left_revealer.set_transition_duration(motion_hover_ms);
            top_controls_left_revealer.set_transition_type(RevealerTransitionType::Crossfade);
            top_controls_left_revealer.set_halign(Align::Start);
            top_controls_left_revealer.set_valign(Align::Start);
            top_controls_left_revealer.set_vexpand(false);
            top_controls_left_revealer.set_reveal_child(false);
            top_controls_left_revealer.set_can_target(false);
            top_controls_left_revealer.set_child(Some(&top_controls_left));
            editor_overlay.add_overlay(&top_controls_left_revealer);

            // Close button (standalone, top-right)
            let top_controls_right = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
            top_controls_right.set_halign(Align::End);
            top_controls_right.set_valign(Align::Start);
            top_controls_right.set_margin_top(style_tokens.spacing_16);
            top_controls_right.set_margin_bottom(style_tokens.spacing_16);
            top_controls_right.set_margin_start(style_tokens.spacing_16);
            top_controls_right.set_margin_end(style_tokens.spacing_16);
            top_controls_right.add_css_class("transparent-bg");
            top_controls_right.append(&editor_close_button);
            let top_controls_right_revealer = Revealer::new();
            top_controls_right_revealer.set_transition_duration(motion_hover_ms);
            top_controls_right_revealer.set_transition_type(RevealerTransitionType::Crossfade);
            top_controls_right_revealer.set_halign(Align::End);
            top_controls_right_revealer.set_valign(Align::Start);
            top_controls_right_revealer.set_vexpand(false);
            top_controls_right_revealer.set_reveal_child(false);
            top_controls_right_revealer.set_can_target(false);
            top_controls_right_revealer.set_child(Some(&top_controls_right));
            editor_overlay.add_overlay(&top_controls_right_revealer);
            connect_editor_tool_shortcut_fallback(
                &editor_window_instance,
                &tool_buttons,
                editor_input_mode,
            );

            let tool_options_bar = GtkBox::new(Orientation::Vertical, style_tokens.spacing_8);
            tool_options_bar.add_css_class("editor-options-bar");
            tool_options_bar.add_css_class("stroke-options-panel");
            tool_options_bar.set_hexpand(false);
            tool_options_bar.set_halign(Align::Start);
            let tool_options_collapsed = Rc::new(Cell::new(false));
            let thickness_preview_rgb = match theme_mode {
                crate::theme::ThemeMode::Light => (24_u8, 26_u8, 32_u8),
                crate::theme::ThemeMode::Dark | crate::theme::ThemeMode::System => {
                    (236_u8, 238_u8, 244_u8)
                }
            };
            let stroke_color_palette = tool_option_presets.stroke_color_palette().clone();
            let stroke_width_presets = tool_option_presets.stroke_width_presets().to_vec();
            let text_size_presets = tool_option_presets.text_size_presets().to_vec();

            let tool_options_header = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
            tool_options_header.add_css_class("editor-options-header");
            tool_options_header.set_valign(Align::Center);
            let tool_options_collapsed_row =
                GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
            tool_options_collapsed_row.add_css_class("stroke-chip-row");
            tool_options_collapsed_row.add_css_class("editor-options-collapsed-row");
            tool_options_collapsed_row.set_hexpand(true);
            tool_options_collapsed_row.set_valign(Align::Center);
            tool_options_collapsed_row.set_vexpand(false);
            tool_options_collapsed_row.set_visible(false);

            let collapsed_color_rgb = Rc::new(Cell::new((0_u8, 0_u8, 0_u8)));
            let collapsed_thickness = Rc::new(Cell::new(1_u8));
            {
                let tools = editor_tools.borrow();
                let arrow_options = tools.arrow_options();
                collapsed_color_rgb.set((
                    arrow_options.color_r,
                    arrow_options.color_g,
                    arrow_options.color_b,
                ));
                collapsed_thickness.set(nearest_preset_u8(
                    f64::from(arrow_options.thickness),
                    &stroke_width_presets,
                ));
            }

            let collapsed_color_chip = Button::new();
            collapsed_color_chip.set_focus_on_click(false);
            collapsed_color_chip.set_can_target(false);
            collapsed_color_chip.set_valign(Align::Center);
            collapsed_color_chip.add_css_class("flat");
            collapsed_color_chip.add_css_class("stroke-chip-button");
            collapsed_color_chip.add_css_class("stroke-chip-active");
            collapsed_color_chip.set_size_request(30, 30);
            let collapsed_color_swatch = DrawingArea::new();
            collapsed_color_swatch.set_content_width(18);
            collapsed_color_swatch.set_content_height(18);
            collapsed_color_swatch.set_can_target(false);
            {
                let collapsed_color_rgb = collapsed_color_rgb.clone();
                collapsed_color_swatch.set_draw_func(move |_, context, width, height| {
                    let (r, g, b) = collapsed_color_rgb.get();
                    let radius = (f64::from(width.min(height)) / 2.0) - 1.2;
                    let center_x = f64::from(width) / 2.0;
                    let center_y = f64::from(height) / 2.0;
                    context.save().ok();
                    context.arc(
                        center_x,
                        center_y,
                        radius.max(1.0),
                        0.0,
                        std::f64::consts::TAU,
                    );
                    context.set_source_rgb(
                        f64::from(r) / 255.0,
                        f64::from(g) / 255.0,
                        f64::from(b) / 255.0,
                    );
                    let _ = context.fill_preserve();
                    context.set_source_rgba(1.0, 1.0, 1.0, 0.35);
                    context.set_line_width(1.0);
                    let _ = context.stroke();
                    context.restore().ok();
                });
            }
            collapsed_color_chip.set_child(Some(&collapsed_color_swatch));
            tool_options_collapsed_row.append(&collapsed_color_chip);

            let collapsed_thickness_chip = Button::new();
            collapsed_thickness_chip.set_focus_on_click(false);
            collapsed_thickness_chip.set_can_target(false);
            collapsed_thickness_chip.set_valign(Align::Center);
            collapsed_thickness_chip.add_css_class("flat");
            collapsed_thickness_chip.add_css_class("stroke-chip-button");
            collapsed_thickness_chip.add_css_class("stroke-chip-active");
            collapsed_thickness_chip.set_size_request(30, 30);
            let collapsed_thickness_preview = DrawingArea::new();
            collapsed_thickness_preview.set_content_width(18);
            collapsed_thickness_preview.set_content_height(18);
            collapsed_thickness_preview.set_can_target(false);
            {
                let collapsed_thickness = collapsed_thickness.clone();
                let (line_r, line_g, line_b) = thickness_preview_rgb;
                collapsed_thickness_preview.set_draw_func(move |_, context, width, height| {
                    let y = f64::from(height) / 2.0;
                    context.save().ok();
                    context.set_source_rgb(
                        f64::from(line_r) / 255.0,
                        f64::from(line_g) / 255.0,
                        f64::from(line_b) / 255.0,
                    );
                    context.set_line_width(stroke_preview_line_width(
                        collapsed_thickness.get(),
                        width,
                        height,
                    ));
                    context.set_line_cap(gtk4::cairo::LineCap::Round);
                    context.move_to(STROKE_PREVIEW_ENDPOINT_MARGIN, y);
                    context.line_to(f64::from(width) - STROKE_PREVIEW_ENDPOINT_MARGIN, y);
                    let _ = context.stroke();
                    context.restore().ok();
                });
            }
            collapsed_thickness_chip.set_child(Some(&collapsed_thickness_preview));
            tool_options_collapsed_row.append(&collapsed_thickness_chip);

            let collapsed_text_size = {
                let text_size = editor_tools.borrow().text_options().size;
                nearest_preset_u8(f64::from(text_size), &text_size_presets)
            };
            let collapsed_text_size_chip =
                Button::with_label(collapsed_text_size.to_string().as_str());
            collapsed_text_size_chip.set_focus_on_click(false);
            collapsed_text_size_chip.set_can_target(false);
            collapsed_text_size_chip.set_valign(Align::Center);
            collapsed_text_size_chip.add_css_class("flat");
            collapsed_text_size_chip.add_css_class("stroke-chip-button");
            collapsed_text_size_chip.add_css_class("stroke-chip-active");
            collapsed_text_size_chip.set_size_request(34, 30);
            tool_options_collapsed_row.append(&collapsed_text_size_chip);

            let tool_options_toggle = Button::from_icon_name("chevron-down-symbolic");
            tool_options_toggle.set_focus_on_click(false);
            tool_options_toggle.set_size_request(30, 30);
            tool_options_toggle.set_hexpand(false);
            tool_options_toggle.set_vexpand(false);
            tool_options_toggle.set_halign(Align::Center);
            tool_options_toggle.set_valign(Align::Center);
            tool_options_toggle.set_tooltip_text(Some("Collapse tool options (O)"));
            tool_options_toggle.add_css_class("flat");
            tool_options_toggle.add_css_class("editor-options-toggle");
            tool_options_header.append(&tool_options_collapsed_row);
            tool_options_header.append(&tool_options_toggle);
            tool_options_bar.append(&tool_options_header);

            let tool_options_content = GtkBox::new(Orientation::Vertical, style_tokens.spacing_8);
            let tool_options_content_revealer = Revealer::new();
            tool_options_content_revealer.set_transition_duration(motion_hover_ms);
            tool_options_content_revealer.set_transition_type(RevealerTransitionType::SlideDown);
            tool_options_content_revealer.set_reveal_child(true);
            tool_options_content_revealer.set_visible(true);
            tool_options_content_revealer.set_child(Some(&tool_options_content));
            tool_options_bar.append(&tool_options_content_revealer);

            let refresh_collapsed_option_chips = Rc::new({
                let editor_tools = editor_tools.clone();
                let collapsed_color_rgb = collapsed_color_rgb.clone();
                let collapsed_color_swatch = collapsed_color_swatch.clone();
                let collapsed_thickness = collapsed_thickness.clone();
                let collapsed_thickness_preview = collapsed_thickness_preview.clone();
                let collapsed_text_size_chip = collapsed_text_size_chip.clone();
                let stroke_width_presets = stroke_width_presets.clone();
                let text_size_presets = text_size_presets.clone();
                move || {
                    let Ok(tools) = editor_tools.try_borrow() else {
                        return;
                    };
                    let arrow_options = tools.arrow_options();
                    collapsed_color_rgb.set((
                        arrow_options.color_r,
                        arrow_options.color_g,
                        arrow_options.color_b,
                    ));
                    collapsed_color_swatch.queue_draw();
                    collapsed_thickness.set(nearest_preset_u8(
                        f64::from(arrow_options.thickness),
                        &stroke_width_presets,
                    ));
                    collapsed_thickness_preview.queue_draw();
                    let text_size =
                        nearest_preset_u8(f64::from(tools.text_options().size), &text_size_presets);
                    collapsed_text_size_chip.set_label(text_size.to_string().as_str());
                }
            });
            (refresh_collapsed_option_chips.as_ref())();

            let color_group = GtkBox::new(Orientation::Vertical, 2);
            color_group.add_css_class("stroke-options-section");
            let color_title = Label::new(Some("Color Palette"));
            color_title.add_css_class("stroke-options-title");
            color_title.set_xalign(0.0);
            let color_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
            color_row.add_css_class("stroke-chip-row");
            let color_chip_buttons = Rc::new(RefCell::new(Vec::<(usize, Button)>::new()));
            let initial_color_index = {
                let options = editor_tools.borrow().arrow_options();
                stroke_color_palette.presets().iter().position(|preset| {
                    preset.rgb() == (options.color_r, options.color_g, options.color_b)
                })
            };
            for (index, preset) in stroke_color_palette.presets().iter().cloned().enumerate() {
                let chip = Button::new();
                chip.set_focus_on_click(false);
                chip.set_tooltip_text(Some(preset.label.as_str()));
                chip.add_css_class("flat");
                chip.add_css_class("stroke-chip-button");
                chip.set_size_request(30, 30);
                let swatch = DrawingArea::new();
                swatch.set_content_width(18);
                swatch.set_content_height(18);
                swatch.set_can_target(false);
                let (preset_r, preset_g, preset_b) = preset.rgb();
                swatch.set_draw_func(move |_, context, width, height| {
                    let radius = (f64::from(width.min(height)) / 2.0) - 1.2;
                    let center_x = f64::from(width) / 2.0;
                    let center_y = f64::from(height) / 2.0;
                    context.save().ok();
                    context.arc(
                        center_x,
                        center_y,
                        radius.max(1.0),
                        0.0,
                        std::f64::consts::TAU,
                    );
                    context.set_source_rgb(
                        f64::from(preset_r) / 255.0,
                        f64::from(preset_g) / 255.0,
                        f64::from(preset_b) / 255.0,
                    );
                    let _ = context.fill_preserve();
                    context.set_source_rgba(1.0, 1.0, 1.0, 0.35);
                    context.set_line_width(1.0);
                    let _ = context.stroke();
                    context.restore().ok();
                });
                chip.set_child(Some(&swatch));
                color_row.append(&chip);
                color_chip_buttons.borrow_mut().push((index, chip.clone()));
                let color_chip_buttons = color_chip_buttons.clone();
                let editor_tools = editor_tools.clone();
                let editor_canvas = editor_canvas.clone();
                let status_log_for_render = status_log_for_render.clone();
                let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
                chip.connect_clicked(move |_| {
                    for (candidate_index, candidate_button) in color_chip_buttons.borrow().iter() {
                        if *candidate_index == index {
                            candidate_button.add_css_class("stroke-chip-active");
                        } else {
                            candidate_button.remove_css_class("stroke-chip-active");
                        }
                    }
                    {
                        let mut tools = editor_tools.borrow_mut();
                        tools.set_shared_stroke_color(preset_r, preset_g, preset_b);
                    }
                    *status_log_for_render.borrow_mut() =
                        format!("stroke color preset: {preset_r},{preset_g},{preset_b}");
                    editor_canvas.queue_draw();
                    (refresh_collapsed_option_chips.as_ref())();
                });
            }
            for (index, chip) in color_chip_buttons.borrow().iter() {
                if Some(*index) == initial_color_index {
                    chip.add_css_class("stroke-chip-active");
                }
            }
            color_group.append(&color_title);
            color_group.append(&color_row);
            tool_options_content.append(&color_group);

            let thickness_group = GtkBox::new(Orientation::Vertical, 2);
            thickness_group.add_css_class("stroke-options-section");
            thickness_group.set_margin_top(style_tokens.spacing_8);
            let thickness_title = Label::new(Some("Stroke Width"));
            thickness_title.add_css_class("stroke-options-title");
            thickness_title.set_xalign(0.0);
            let thickness_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
            thickness_row.add_css_class("stroke-chip-row");
            let thickness_chip_buttons = Rc::new(RefCell::new(Vec::<(u8, Button)>::new()));
            let initial_thickness = {
                let options = editor_tools.borrow().arrow_options();
                nearest_preset_u8(f64::from(options.thickness), &stroke_width_presets)
            };
            for thickness in stroke_width_presets.iter().copied() {
                let chip = Button::new();
                chip.set_focus_on_click(false);
                chip.set_tooltip_text(Some(format!("Thickness {thickness}").as_str()));
                chip.add_css_class("flat");
                chip.add_css_class("stroke-chip-button");
                chip.set_size_request(30, 30);
                let preview = DrawingArea::new();
                preview.set_content_width(18);
                preview.set_content_height(18);
                preview.set_can_target(false);
                let (line_r, line_g, line_b) = thickness_preview_rgb;
                preview.set_draw_func(move |_, context, width, height| {
                    let y = f64::from(height) / 2.0;
                    context.save().ok();
                    context.set_source_rgb(
                        f64::from(line_r) / 255.0,
                        f64::from(line_g) / 255.0,
                        f64::from(line_b) / 255.0,
                    );
                    context.set_line_width(stroke_preview_line_width(thickness, width, height));
                    context.set_line_cap(gtk4::cairo::LineCap::Round);
                    context.move_to(STROKE_PREVIEW_ENDPOINT_MARGIN, y);
                    context.line_to(f64::from(width) - STROKE_PREVIEW_ENDPOINT_MARGIN, y);
                    let _ = context.stroke();
                    context.restore().ok();
                });
                chip.set_child(Some(&preview));
                thickness_row.append(&chip);
                thickness_chip_buttons
                    .borrow_mut()
                    .push((thickness, chip.clone()));
                let thickness_chip_buttons = thickness_chip_buttons.clone();
                let editor_tools = editor_tools.clone();
                let editor_canvas = editor_canvas.clone();
                let status_log_for_render = status_log_for_render.clone();
                let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
                chip.connect_clicked(move |_| {
                    for (candidate_thickness, candidate_button) in
                        thickness_chip_buttons.borrow().iter()
                    {
                        if *candidate_thickness == thickness {
                            candidate_button.add_css_class("stroke-chip-active");
                        } else {
                            candidate_button.remove_css_class("stroke-chip-active");
                        }
                    }
                    {
                        let mut tools = editor_tools.borrow_mut();
                        tools.set_shared_stroke_thickness(thickness);
                    }
                    *status_log_for_render.borrow_mut() =
                        format!("stroke thickness preset: {thickness}");
                    editor_canvas.queue_draw();
                    (refresh_collapsed_option_chips.as_ref())();
                });
            }
            for (thickness, chip) in thickness_chip_buttons.borrow().iter() {
                if *thickness == initial_thickness {
                    chip.add_css_class("stroke-chip-active");
                }
            }
            thickness_group.append(&thickness_title);
            thickness_group.append(&thickness_row);
            tool_options_content.append(&thickness_group);

            let text_size_group = GtkBox::new(Orientation::Vertical, 2);
            text_size_group.add_css_class("stroke-options-section");
            text_size_group.set_margin_top(style_tokens.spacing_8);
            let text_size_title = Label::new(Some("Text Size"));
            text_size_title.add_css_class("stroke-options-title");
            text_size_title.set_xalign(0.0);
            let text_size_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
            text_size_row.add_css_class("stroke-chip-row");
            let text_size_chip_buttons = Rc::new(RefCell::new(Vec::<(u8, Button)>::new()));
            let initial_text_size = {
                let options = editor_tools.borrow().text_options();
                nearest_preset_u8(f64::from(options.size), &text_size_presets)
            };
            for text_size in text_size_presets.iter().copied() {
                let chip = Button::with_label(text_size.to_string().as_str());
                chip.set_focus_on_click(false);
                chip.set_tooltip_text(Some(format!("Text size {text_size}").as_str()));
                chip.add_css_class("flat");
                chip.add_css_class("stroke-chip-button");
                chip.set_size_request(34, 30);
                text_size_row.append(&chip);
                text_size_chip_buttons
                    .borrow_mut()
                    .push((text_size, chip.clone()));
                let text_size_chip_buttons = text_size_chip_buttons.clone();
                let editor_tools = editor_tools.clone();
                let editor_canvas = editor_canvas.clone();
                let status_log_for_render = status_log_for_render.clone();
                let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
                chip.connect_clicked(move |_| {
                    for (candidate_size, candidate_button) in text_size_chip_buttons.borrow().iter()
                    {
                        if *candidate_size == text_size {
                            candidate_button.add_css_class("stroke-chip-active");
                        } else {
                            candidate_button.remove_css_class("stroke-chip-active");
                        }
                    }
                    editor_tools.borrow_mut().set_text_size(text_size);
                    *status_log_for_render.borrow_mut() = format!("text size preset: {text_size}");
                    editor_canvas.queue_draw();
                    (refresh_collapsed_option_chips.as_ref())();
                });
            }
            for (text_size, chip) in text_size_chip_buttons.borrow().iter() {
                if *text_size == initial_text_size {
                    chip.add_css_class("stroke-chip-active");
                }
            }
            text_size_group.append(&text_size_title);
            text_size_group.append(&text_size_row);
            tool_options_content.append(&text_size_group);

            {
                let tool_options_collapsed = tool_options_collapsed.clone();
                let tool_options_bar = tool_options_bar.clone();
                let tool_options_collapsed_row = tool_options_collapsed_row.clone();
                let tool_options_toggle = tool_options_toggle.clone();
                let tool_options_toggle_for_click = tool_options_toggle.clone();
                let tool_options_content_revealer = tool_options_content_revealer.clone();
                let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
                let status_log_for_render = status_log_for_render.clone();
                tool_options_toggle.connect_clicked(move |_| {
                    let collapsed = !tool_options_collapsed.get();
                    tool_options_collapsed.set(collapsed);
                    tool_options_content_revealer.set_visible(!collapsed);
                    tool_options_content_revealer.set_reveal_child(!collapsed);
                    tool_options_collapsed_row.set_visible(collapsed);
                    if collapsed {
                        tool_options_bar.add_css_class("editor-options-collapsed");
                        tool_options_toggle_for_click.set_icon_name("chevron-up-symbolic");
                        tool_options_toggle_for_click
                            .set_tooltip_text(Some("Expand tool options (O)"));
                        *status_log_for_render.borrow_mut() =
                            "editor tool options collapsed".to_string();
                    } else {
                        tool_options_bar.remove_css_class("editor-options-collapsed");
                        tool_options_toggle_for_click.set_icon_name("chevron-down-symbolic");
                        tool_options_toggle_for_click
                            .set_tooltip_text(Some("Collapse tool options (O)"));
                        *status_log_for_render.borrow_mut() =
                            "editor tool options expanded".to_string();
                    }
                    (refresh_collapsed_option_chips.as_ref())();
                });
            }

            let refresh_tool_options_with_bottom = Rc::new({
                let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
                move |_tool: ToolKind| {
                    (refresh_collapsed_option_chips.as_ref())();
                }
            });
            refresh_tool_options_with_bottom(active_editor_tool.get());
            *refresh_tool_options.borrow_mut() = Some(refresh_tool_options_with_bottom);

            let bottom_left_controls = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
            bottom_left_controls.add_css_class("editor-bottom-controls");
            bottom_left_controls.set_halign(Align::Start);
            bottom_left_controls.set_valign(Align::End);
            bottom_left_controls.set_margin_top(style_tokens.spacing_16);
            bottom_left_controls.set_margin_bottom(style_tokens.spacing_16);
            bottom_left_controls.set_margin_start(style_tokens.spacing_16);
            bottom_left_controls.set_margin_end(style_tokens.spacing_16);
            bottom_left_controls.add_css_class("transparent-bg");
            bottom_left_controls.append(&tool_options_bar);
            let bottom_left_controls_revealer = Revealer::new();
            bottom_left_controls_revealer.set_transition_duration(motion_hover_ms);
            bottom_left_controls_revealer.set_transition_type(RevealerTransitionType::Crossfade);
            bottom_left_controls_revealer.set_halign(Align::Start);
            bottom_left_controls_revealer.set_valign(Align::End);
            bottom_left_controls_revealer.set_vexpand(false);
            bottom_left_controls_revealer.set_reveal_child(false);
            bottom_left_controls_revealer.set_can_target(false);
            bottom_left_controls_revealer.set_child(Some(&bottom_left_controls));
            editor_overlay.add_overlay(&bottom_left_controls_revealer);

            let zoom_group = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
            zoom_group.add_css_class("editor-action-group");
            zoom_group.set_valign(Align::End);
            zoom_group.set_vexpand(false);
            zoom_slider.set_valign(Align::Center);
            viewport_fit_button.set_valign(Align::Center);
            zoom_group.append(&zoom_slider);
            zoom_group.append(&viewport_fit_button);
            let bottom_right_controls =
                GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
            bottom_right_controls.add_css_class("editor-bottom-controls");
            bottom_right_controls.set_halign(Align::End);
            bottom_right_controls.set_valign(Align::End);
            bottom_right_controls.set_margin_top(style_tokens.spacing_16);
            bottom_right_controls.set_margin_bottom(style_tokens.spacing_16);
            bottom_right_controls.set_margin_start(style_tokens.spacing_16);
            bottom_right_controls.set_margin_end(style_tokens.spacing_16);
            bottom_right_controls.add_css_class("transparent-bg");
            bottom_right_controls.append(&zoom_group);
            let bottom_right_controls_revealer = Revealer::new();
            bottom_right_controls_revealer.set_transition_duration(motion_hover_ms);
            bottom_right_controls_revealer.set_transition_type(RevealerTransitionType::Crossfade);
            bottom_right_controls_revealer.set_halign(Align::End);
            bottom_right_controls_revealer.set_valign(Align::End);
            bottom_right_controls_revealer.set_vexpand(false);
            bottom_right_controls_revealer.set_reveal_child(false);
            bottom_right_controls_revealer.set_can_target(false);
            bottom_right_controls_revealer.set_child(Some(&bottom_right_controls));
            editor_overlay.add_overlay(&bottom_right_controls_revealer);

            connect_editor_overlay_hover_controls(
                &editor_overlay,
                &top_controls_left_revealer,
                &top_controls_right_revealer,
                &bottom_left_controls_revealer,
                &bottom_right_controls_revealer,
            );
            let editor_toast_anchor = GtkBox::new(Orientation::Vertical, 0);
            editor_toast_anchor.set_halign(Align::Center);
            editor_toast_anchor.set_valign(Align::Start);
            editor_toast_anchor.set_margin_top(style_tokens.spacing_12);
            editor_toast_anchor.set_margin_bottom(style_tokens.spacing_12);
            editor_toast_anchor.set_margin_start(style_tokens.spacing_12);
            editor_toast_anchor.set_margin_end(style_tokens.spacing_12);
            let editor_toast_label = Label::new(Some(""));
            editor_toast_label.add_css_class("toast-badge");
            editor_toast_label.set_visible(false);
            editor_toast_anchor.append(&editor_toast_label);
            editor_overlay.add_overlay(&editor_toast_anchor);
            editor_window_instance.set_child(Some(&editor_overlay));
            let editor_toast_runtime = ToastRuntime::new(&editor_toast_label);
            *editor_toast.borrow_mut() = Some(editor_toast_runtime.clone());
            // Opening editor with a fresh capture should start as clean.
            reset_editor_session_state(editor_runtime);

            {
                connect_editor_zoom_slider(
                    &zoom_slider,
                    EditorZoomSliderContext {
                        editor_viewport: editor_viewport.clone(),
                        editor_canvas: editor_canvas.clone(),
                        editor_scroller: editor_scroller.clone(),
                        editor_viewport_status: editor_viewport_status.clone(),
                        status_log_for_render: status_log_for_render.clone(),
                        zoom_slider_syncing: zoom_slider_syncing.clone(),
                        editor_image_base_width,
                        editor_image_base_height,
                    },
                );
            }
            {
                connect_editor_fit_button(
                    &viewport_fit_button,
                    EditorFitButtonContext {
                        editor_viewport: editor_viewport.clone(),
                        editor_canvas: editor_canvas.clone(),
                        editor_scroller: editor_scroller.clone(),
                        editor_viewport_status: editor_viewport_status.clone(),
                        status_log_for_render: status_log_for_render.clone(),
                        zoom_slider: zoom_slider.clone(),
                        zoom_slider_syncing: zoom_slider_syncing.clone(),
                        schedule_fit_settle_pass: schedule_fit_settle_pass.clone(),
                        editor_image_base_width,
                        editor_image_base_height,
                    },
                );
            }
            {
                connect_editor_pan_drag_gesture(EditorPanGestureContext {
                    editor_scroller: editor_scroller.clone(),
                    editor_viewport: editor_viewport.clone(),
                    editor_canvas: editor_canvas.clone(),
                    editor_viewport_status: editor_viewport_status.clone(),
                    status_log_for_render: status_log_for_render.clone(),
                    space_pan_pressed: space_pan_pressed.clone(),
                    active_editor_tool: active_editor_tool.clone(),
                    drag_pan_active: drag_pan_active.clone(),
                    drag_pan_pointer_origin: drag_pan_pointer_origin.clone(),
                    refresh_editor_cursor: refresh_editor_cursor.clone(),
                    editor_image_base_width,
                    editor_image_base_height,
                });
            }

            {
                let history_runtime = EditorHistoryRuntime {
                    editor_tools: editor_tools.clone(),
                    editor_undo_stack: editor_undo_stack.clone(),
                    editor_redo_stack: editor_redo_stack.clone(),
                    selected_object_ids: selected_object_ids.clone(),
                    pending_crop: pending_crop.clone(),
                    editor_has_unsaved_changes: editor_has_unsaved_changes.clone(),
                    status_log_for_render: status_log_for_render.clone(),
                    editor_canvas: editor_canvas.clone(),
                };
                history_runtime.connect_button(&editor_undo_button, EditorHistoryAction::Undo);
                history_runtime.connect_button(&editor_redo_button, EditorHistoryAction::Redo);

                let output_action_runtime = EditorOutputActionRuntime {
                    runtime_session: runtime_session.clone(),
                    shared_machine: shared_machine.clone(),
                    storage_service: storage_service.clone(),
                    status_log: status_log_for_render.clone(),
                    editor_toast: editor_toast_runtime.clone(),
                    editor_tools: editor_tools.clone(),
                    pending_crop: pending_crop.clone(),
                    editor_source_pixbuf: editor_source_pixbuf.clone(),
                    editor_has_unsaved_changes: editor_has_unsaved_changes.clone(),
                    toast_duration_ms: style_tokens.toast_duration_ms,
                };
                connect_editor_output_button(
                    &editor_save_button,
                    &output_action_runtime,
                    EditorAction::Save,
                    "save",
                );
                connect_editor_output_button(
                    &editor_copy_button,
                    &output_action_runtime,
                    EditorAction::Copy,
                    "copy",
                );
                connect_editor_output_button(
                    &editor_copy_file_reference_button,
                    &output_action_runtime,
                    EditorAction::CopyFileReference,
                    "copy file reference",
                );
            }
            {
                connect_editor_close_dialog(EditorCloseDialogContext {
                    runtime_session: runtime_session.clone(),
                    shared_machine: shared_machine.clone(),
                    storage_service: storage_service.clone(),
                    status_log_for_render: status_log_for_render.clone(),
                    close_editor_button: close_editor_button.clone(),
                    editor_close_button: editor_close_button.clone(),
                    editor_has_unsaved_changes: editor_has_unsaved_changes.clone(),
                    editor_close_dialog_open: editor_close_dialog_open.clone(),
                    editor_window_for_dialog: editor_window_instance.clone(),
                    editor_toast_runtime: editor_toast_runtime.clone(),
                    editor_tools: editor_tools.clone(),
                    pending_crop_for_close: pending_crop.clone(),
                    editor_source_pixbuf: editor_source_pixbuf.clone(),
                    style_tokens,
                });
            }
            {
                connect_editor_key_handling(EditorKeyHandlingContext {
                    editor_overlay: editor_overlay.clone(),
                    editor_canvas: editor_canvas.clone(),
                    editor_scroller: editor_scroller.clone(),
                    editor_viewport: editor_viewport.clone(),
                    editor_viewport_status: editor_viewport_status.clone(),
                    zoom_slider: zoom_slider.clone(),
                    zoom_slider_syncing: zoom_slider_syncing.clone(),
                    schedule_fit_settle_pass: schedule_fit_settle_pass.clone(),
                    editor_navigation_bindings: editor_navigation_bindings.clone(),
                    editor_close_dialog_open: editor_close_dialog_open.clone(),
                    editor_input_mode: editor_input_mode.clone(),
                    text_im_context: text_im_context.clone(),
                    text_preedit_state: text_preedit_state.clone(),
                    editor_tools: editor_tools.clone(),
                    editor_undo_stack: editor_undo_stack.clone(),
                    editor_redo_stack: editor_redo_stack.clone(),
                    editor_has_unsaved_changes: editor_has_unsaved_changes.clone(),
                    status_log_for_render: status_log_for_render.clone(),
                    editor_undo_button: editor_undo_button.clone(),
                    editor_redo_button: editor_redo_button.clone(),
                    editor_save_button: editor_save_button.clone(),
                    editor_copy_button: editor_copy_button.clone(),
                    tool_options_toggle_button: tool_options_toggle.clone(),
                    editor_close_button: editor_close_button.clone(),
                    editor_tool_switch_context: editor_tool_switch_context.clone(),
                    active_editor_tool: active_editor_tool.clone(),
                    pending_crop: pending_crop.clone(),
                    selected_object_ids: selected_object_ids.clone(),
                    space_pan_pressed: space_pan_pressed.clone(),
                    drag_pan_active: drag_pan_active.clone(),
                    drag_pan_pointer_origin: drag_pan_pointer_origin.clone(),
                    refresh_editor_cursor: refresh_editor_cursor.clone(),
                    editor_image_base_width,
                    editor_image_base_height,
                });
            }
            {
                connect_editor_window_close_request(EditorWindowCloseRequestContext {
                    editor_window_instance: editor_window_instance.clone(),
                    editor_close_button: editor_close_button.clone(),
                    shared_machine: shared_machine.clone(),
                    editor_close_guard: editor_close_guard.clone(),
                });
            }

            editor_window_instance.present();
            {
                let editor_canvas = editor_canvas.clone();
                editor_window_instance.connect_is_active_notify(move |window| {
                    if window.is_active() {
                        editor_canvas.grab_focus();
                    }
                });
            }
            {
                let editor_canvas = editor_canvas.clone();
                gtk4::glib::timeout_add_local_once(Duration::from_millis(1), move || {
                    editor_canvas.grab_focus();
                });
            }
            let restored_editor_geometry = saved_editor_geometry
                .map(|saved| {
                    RuntimeWindowGeometry::with_position(
                        saved.x,
                        saved.y,
                        editor_window_geometry.width,
                        editor_window_geometry.height,
                    )
                })
                .and_then(clamp_window_geometry_to_current_monitors)
                .map(|geometry| (geometry.x, geometry.y, geometry.width, geometry.height));
            request_window_floating_with_geometry(
                "editor",
                &editor_title,
                false,
                Some(
                    restored_editor_geometry
                        .or_else(|| {
                            preview_anchor.map(|(center_x, center_y)| {
                                centered_window_geometry_for_point(
                                    center_x,
                                    center_y,
                                    editor_window_geometry,
                                )
                            })
                        })
                        .unwrap_or_else(|| {
                            centered_window_geometry_for_capture(&artifact, editor_window_geometry)
                        }),
                ),
            );
            *editor_capture_id.borrow_mut() = Some(artifact.capture_id.clone());
            *editor_window.borrow_mut() = Some(editor_window_instance);
        }
    } else {
        close_editor_if_open_and_clear(
            editor_window,
            runtime_window_state,
            editor_close_guard,
            editor_runtime,
            style_tokens,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_editor_tool_fallback_shortcut_maps_editor_tool_keys() {
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('v')),
            Some(ToolKind::Select)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('h')),
            Some(ToolKind::Pan)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('b')),
            Some(ToolKind::Blur)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('p')),
            Some(ToolKind::Pen)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('a')),
            Some(ToolKind::Arrow)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('r')),
            Some(ToolKind::Rectangle)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('c')),
            Some(ToolKind::Crop)
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('t')),
            Some(ToolKind::Text)
        );
    }

    #[test]
    fn resolve_editor_tool_fallback_shortcut_ignores_non_tool_keys() {
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('x')),
            None
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Character('o')),
            None
        );
        assert_eq!(
            resolve_editor_tool_fallback_shortcut(ShortcutKey::Escape),
            None
        );
    }

    #[test]
    fn stroke_preview_line_width_clamps_large_thickness_to_chip_bounds() {
        let clamped = stroke_preview_line_width(64, 18, 18);
        assert!((clamped - 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stroke_preview_line_width_keeps_minimum_visible_width() {
        let clamped = stroke_preview_line_width(0, 18, 18);
        assert!((clamped - 1.0).abs() < f64::EPSILON);
    }
}

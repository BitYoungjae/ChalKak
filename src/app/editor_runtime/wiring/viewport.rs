use super::*;
#[derive(Clone)]
pub(in crate::app::editor_runtime) struct EditorZoomSliderContext {
    pub(in crate::app::editor_runtime) editor_viewport: Rc<RefCell<editor::EditorViewport>>,
    pub(in crate::app::editor_runtime) editor_canvas: DrawingArea,
    pub(in crate::app::editor_runtime) editor_scroller: ScrolledWindow,
    pub(in crate::app::editor_runtime) editor_viewport_status: Label,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) zoom_slider_syncing: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) editor_image_base_width: i32,
    pub(in crate::app::editor_runtime) editor_image_base_height: i32,
}

pub(in crate::app::editor_runtime) fn connect_editor_zoom_slider(
    zoom_slider: &Scale,
    context: EditorZoomSliderContext,
) {
    let context_for_change = context.clone();
    zoom_slider.connect_value_changed(move |slider| {
        if context_for_change.zoom_slider_syncing.get() {
            return;
        }
        let zoom_percent = zoom_percent_from_slider_value(slider.value());
        let mut viewport = context_for_change.editor_viewport.borrow_mut();
        let (anchor_x, anchor_y) = scroller_center_anchor(&context_for_change.editor_scroller);
        let viewport_runtime = editor_viewport_runtime(
            &context_for_change.editor_canvas,
            &context_for_change.editor_scroller,
            &context_for_change.editor_viewport_status,
            slider,
            context_for_change.zoom_slider_syncing.as_ref(),
            context_for_change.editor_image_base_width,
            context_for_change.editor_image_base_height,
        );
        set_editor_zoom_percent_and_refresh(
            &mut viewport,
            zoom_percent,
            &viewport_runtime,
            anchor_x,
            anchor_y,
        );
        *context_for_change.status_log_for_render.borrow_mut() = format!(
            "editor viewport zoom {}% via slider",
            viewport.zoom_percent()
        );
    });
}

#[derive(Clone)]
pub(in crate::app::editor_runtime) struct EditorFitButtonContext {
    pub(in crate::app::editor_runtime) editor_viewport: Rc<RefCell<editor::EditorViewport>>,
    pub(in crate::app::editor_runtime) editor_canvas: DrawingArea,
    pub(in crate::app::editor_runtime) editor_scroller: ScrolledWindow,
    pub(in crate::app::editor_runtime) editor_viewport_status: Label,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) zoom_slider: Scale,
    pub(in crate::app::editor_runtime) zoom_slider_syncing: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) schedule_fit_settle_pass: Rc<dyn Fn(&'static str)>,
    pub(in crate::app::editor_runtime) editor_image_base_width: i32,
    pub(in crate::app::editor_runtime) editor_image_base_height: i32,
}

pub(in crate::app::editor_runtime) fn connect_editor_fit_button(
    viewport_fit_button: &Button,
    context: EditorFitButtonContext,
) {
    let context_for_click = context.clone();
    viewport_fit_button.connect_clicked(move |_| {
        {
            let mut viewport = context_for_click.editor_viewport.borrow_mut();
            apply_fit_zoom_once(
                &mut viewport,
                &editor_viewport_runtime(
                    &context_for_click.editor_canvas,
                    &context_for_click.editor_scroller,
                    &context_for_click.editor_viewport_status,
                    &context_for_click.zoom_slider,
                    context_for_click.zoom_slider_syncing.as_ref(),
                    context_for_click.editor_image_base_width,
                    context_for_click.editor_image_base_height,
                ),
                "button",
            );
        }
        (context_for_click.schedule_fit_settle_pass.as_ref())("button-settle");
        *context_for_click.status_log_for_render.borrow_mut() =
            "editor viewport fit to window".to_string();
    });
}

pub(in crate::app::editor_runtime) fn editor_viewport_runtime<'a>(
    canvas: &'a DrawingArea,
    scroller: &'a ScrolledWindow,
    status_label: &'a Label,
    zoom_slider: &'a Scale,
    zoom_slider_syncing: &'a Cell<bool>,
    base_width: i32,
    base_height: i32,
) -> EditorViewportRuntime<'a> {
    EditorViewportRuntime::new(
        canvas,
        scroller,
        status_label,
        zoom_slider,
        zoom_slider_syncing,
        base_width,
        base_height,
    )
}

pub(super) struct EditorViewportShortcutContext<'a, 'b> {
    pub(super) editor_viewport: &'a RefCell<editor::EditorViewport>,
    pub(super) editor_scroller: &'a ScrolledWindow,
    pub(super) viewport_runtime: &'b EditorViewportRuntime<'a>,
    pub(super) schedule_fit_settle_pass: &'a dyn Fn(&'static str),
    pub(super) status_log_for_render: &'a RefCell<String>,
}

fn apply_zoom_shortcut(
    context: &EditorViewportShortcutContext<'_, '_>,
    zoom_in: bool,
    status_prefix: &str,
) {
    let mut viewport = context.editor_viewport.borrow_mut();
    let (anchor_x, anchor_y) = scroller_center_anchor(context.editor_scroller);
    zoom_editor_viewport_and_refresh(
        &mut viewport,
        zoom_in,
        context.viewport_runtime,
        anchor_x,
        anchor_y,
    );
    *context.status_log_for_render.borrow_mut() =
        format!("{status_prefix} {}% via shortcut", viewport.zoom_percent());
}

pub(in crate::app::editor_runtime) fn handle_editor_viewport_shortcuts(
    key_name: Option<&str>,
    state: crate::input::ModifierState,
    editor_navigation_bindings: &crate::input::EditorNavigationBindings,
    context: EditorViewportShortcutContext<'_, '_>,
) -> bool {
    if editor_navigation_bindings.matches_zoom_in_shortcut(key_name, state) {
        apply_zoom_shortcut(&context, true, "editor viewport zoom");
        return true;
    }
    if editor_navigation_bindings.matches_zoom_out_shortcut(key_name, state) {
        apply_zoom_shortcut(&context, false, "editor viewport zoom");
        return true;
    }
    if editor_navigation_bindings.matches_actual_size_shortcut(key_name, state) {
        let mut viewport = context.editor_viewport.borrow_mut();
        set_editor_actual_size_and_refresh(&mut viewport, context.viewport_runtime);
        *context.status_log_for_render.borrow_mut() =
            "editor viewport reset to 100% via shortcut".to_string();
        return true;
    }
    if editor_navigation_bindings.matches_fit_shortcut(key_name, state) {
        {
            let mut viewport = context.editor_viewport.borrow_mut();
            apply_fit_zoom_once(&mut viewport, context.viewport_runtime, "shortcut");
        }
        (context.schedule_fit_settle_pass)("shortcut-settle");
        *context.status_log_for_render.borrow_mut() =
            "editor viewport fit to window via shortcut".to_string();
        return true;
    }

    false
}

pub(in crate::app::editor_runtime) struct FitSettleSchedulerContext {
    pub(in crate::app::editor_runtime) editor_viewport: Rc<RefCell<editor::EditorViewport>>,
    pub(in crate::app::editor_runtime) editor_canvas: DrawingArea,
    pub(in crate::app::editor_runtime) editor_scroller: ScrolledWindow,
    pub(in crate::app::editor_runtime) editor_viewport_status: Label,
    pub(in crate::app::editor_runtime) zoom_slider: Scale,
    pub(in crate::app::editor_runtime) zoom_slider_syncing: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) editor_image_base_width: i32,
    pub(in crate::app::editor_runtime) editor_image_base_height: i32,
}

pub(in crate::app::editor_runtime) fn build_fit_settle_pass_scheduler(
    context: FitSettleSchedulerContext,
) -> Rc<dyn Fn(&'static str)> {
    let editor_viewport = context.editor_viewport.clone();
    let editor_canvas = context.editor_canvas.clone();
    let editor_scroller = context.editor_scroller.clone();
    let editor_viewport_status = context.editor_viewport_status.clone();
    let zoom_slider = context.zoom_slider.clone();
    let zoom_slider_syncing = context.zoom_slider_syncing.clone();
    let editor_image_base_width = context.editor_image_base_width;
    let editor_image_base_height = context.editor_image_base_height;
    Rc::new(move |reason: &'static str| {
        for (attempt, delay_ms) in [16_u64, 33, 66, 120, 200].into_iter().enumerate() {
            let editor_viewport = editor_viewport.clone();
            let editor_canvas = editor_canvas.clone();
            let editor_scroller = editor_scroller.clone();
            let editor_viewport_status = editor_viewport_status.clone();
            let zoom_slider = zoom_slider.clone();
            let zoom_slider_syncing = zoom_slider_syncing.clone();
            gtk4::glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
                tracing::debug!(reason, attempt, delay_ms, "running fit settle pass");
                let mut viewport = editor_viewport.borrow_mut();
                apply_fit_zoom_once(
                    &mut viewport,
                    &editor_viewport_runtime(
                        &editor_canvas,
                        &editor_scroller,
                        &editor_viewport_status,
                        &zoom_slider,
                        zoom_slider_syncing.as_ref(),
                        editor_image_base_width,
                        editor_image_base_height,
                    ),
                    reason,
                );
            });
        }
    })
}

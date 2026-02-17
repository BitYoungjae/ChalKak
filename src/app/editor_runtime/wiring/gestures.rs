use super::tools::{
    add_text_box_and_enter_editing, arm_text_tool_for_selection, enter_text_box_editing,
    finish_text_editing_and_arm_text_tool,
};
use super::*;

#[derive(Clone)]
pub(in crate::app::editor_runtime) struct EditorTextClickContext {
    pub(in crate::app::editor_runtime) editor_canvas: DrawingArea,
    pub(in crate::app::editor_runtime) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(in crate::app::editor_runtime) active_editor_tool: Rc<Cell<ToolKind>>,
    pub(in crate::app::editor_runtime) editor_undo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    pub(in crate::app::editor_runtime) editor_redo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) editor_has_unsaved_changes: Rc<RefCell<bool>>,
    pub(in crate::app::editor_runtime) space_pan_pressed: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) selected_object_ids: Rc<RefCell<Vec<u64>>>,
    pub(in crate::app::editor_runtime) text_preedit_state: Rc<RefCell<TextPreeditState>>,
    pub(in crate::app::editor_runtime) editor_tool_switch_context: EditorToolSwitchContext,
    pub(in crate::app::editor_runtime) editor_image_base_width: i32,
    pub(in crate::app::editor_runtime) editor_image_base_height: i32,
}

pub(in crate::app::editor_runtime) fn connect_editor_text_click_gesture(
    context: EditorTextClickContext,
) {
    let text_click = gtk4::GestureClick::new();
    text_click.set_button(gtk4::gdk::BUTTON_PRIMARY);
    let editor_canvas_for_text_click = context.editor_canvas.clone();
    let press_context = context.clone();
    text_click.connect_pressed(move |gesture, n_press, x, y| {
        let active_tool = press_context.active_editor_tool.get();
        if press_context.space_pan_pressed.get() || active_tool == ToolKind::Pan {
            return;
        }
        let anchor = canvas_point_to_image_point(
            &editor_canvas_for_text_click,
            x,
            y,
            press_context.editor_image_base_width,
            press_context.editor_image_base_height,
        );
        if active_tool == ToolKind::Text {
            let hit_text_id = {
                let tools = press_context.editor_tools.borrow();
                top_text_id_at_point(&tools, anchor)
            };
            if let Some(text_id) = hit_text_id {
                if n_press >= 2 {
                    enter_text_box_editing(
                        &press_context.editor_tool_switch_context,
                        press_context.editor_tools.as_ref(),
                        &press_context.selected_object_ids,
                        press_context.text_preedit_state.as_ref(),
                        text_id,
                    );
                    *press_context.status_log_for_render.borrow_mut() =
                        format!("text box #{text_id} editing");
                } else {
                    arm_text_tool_for_selection(
                        &press_context.editor_tool_switch_context,
                        press_context.editor_tools.as_ref(),
                        &press_context.selected_object_ids,
                        text_id,
                    );
                    *press_context.status_log_for_render.borrow_mut() =
                        format!("text box #{text_id} selected; text tool armed");
                }
                editor_canvas_for_text_click.queue_draw();
                gesture.set_state(gtk4::EventSequenceState::Claimed);
                return;
            }
        }

        if active_tool != ToolKind::Text || n_press > 1 {
            return;
        }

        let has_active_text = press_context
            .editor_tools
            .borrow()
            .active_text_id()
            .is_some();
        if has_active_text {
            finish_text_editing_and_arm_text_tool(
                &press_context.editor_tool_switch_context,
                press_context.editor_tools.as_ref(),
            );
            *press_context.status_log_for_render.borrow_mut() =
                "text editing completed; text tool armed".to_string();
            editor_canvas_for_text_click.queue_draw();
            return;
        }

        let snapshot = snapshot_editor_objects(press_context.editor_tools.as_ref());
        record_undo_snapshot(
            press_context.editor_undo_stack.as_ref(),
            press_context.editor_redo_stack.as_ref(),
            snapshot,
        );
        let text_id = add_text_box_and_enter_editing(
            &press_context.editor_tool_switch_context,
            press_context.editor_tools.as_ref(),
            &press_context.selected_object_ids,
            press_context.text_preedit_state.as_ref(),
            anchor,
        );
        *press_context.editor_has_unsaved_changes.borrow_mut() = true;
        *press_context.status_log_for_render.borrow_mut() =
            format!("text box added at ({}, {})", anchor.x, anchor.y);
        editor_canvas_for_text_click.queue_draw();
        gesture.set_state(gtk4::EventSequenceState::Claimed);
        tracing::debug!(text_id, "editor text tool add");
    });
    context.editor_canvas.add_controller(text_click);
}

#[derive(Clone)]
pub(in crate::app::editor_runtime) struct EditorSelectionClickContext {
    pub(in crate::app::editor_runtime) editor_canvas: DrawingArea,
    pub(in crate::app::editor_runtime) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(in crate::app::editor_runtime) active_editor_tool: Rc<Cell<ToolKind>>,
    pub(in crate::app::editor_runtime) selected_object_ids: Rc<RefCell<Vec<u64>>>,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) space_pan_pressed: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) text_preedit_state: Rc<RefCell<TextPreeditState>>,
    pub(in crate::app::editor_runtime) editor_tool_switch_context: EditorToolSwitchContext,
    pub(in crate::app::editor_runtime) editor_image_base_width: i32,
    pub(in crate::app::editor_runtime) editor_image_base_height: i32,
}

pub(in crate::app::editor_runtime) fn connect_editor_selection_click_gesture(
    context: EditorSelectionClickContext,
) {
    let selection_click = gtk4::GestureClick::new();
    selection_click.set_button(gtk4::gdk::BUTTON_PRIMARY);
    let editor_canvas_for_selection = context.editor_canvas.clone();
    let press_context = context.clone();
    selection_click.connect_pressed(move |_gesture, n_press, x, y| {
        if press_context.space_pan_pressed.get() {
            return;
        }
        let active_tool = press_context.active_editor_tool.get();
        if matches!(
            active_tool,
            ToolKind::Text
                | ToolKind::Pan
                | ToolKind::Blur
                | ToolKind::Pen
                | ToolKind::Arrow
                | ToolKind::Rectangle
                | ToolKind::Ocr
        ) {
            return;
        }
        let point = canvas_point_to_image_point(
            &editor_canvas_for_selection,
            x,
            y,
            press_context.editor_image_base_width,
            press_context.editor_image_base_height,
        );
        if active_tool == ToolKind::Crop {
            return;
        }
        let selected = {
            let tools = press_context.editor_tools.borrow();
            top_object_id_at_point(&tools, point)
        };
        if let Some(id) = selected {
            if n_press >= 2
                && matches!(
                    press_context.editor_tools.borrow().object(id),
                    Some(ToolObject::Text(_))
                )
            {
                enter_text_box_editing(
                    &press_context.editor_tool_switch_context,
                    press_context.editor_tools.as_ref(),
                    &press_context.selected_object_ids,
                    press_context.text_preedit_state.as_ref(),
                    id,
                );
                *press_context.status_log_for_render.borrow_mut() =
                    format!("text box #{id} editing");
            } else {
                set_single_selection(&press_context.selected_object_ids, id);
                *press_context.status_log_for_render.borrow_mut() =
                    format!("selected object #{id}");
            }
        } else {
            clear_selection(&press_context.selected_object_ids);
            *press_context.status_log_for_render.borrow_mut() =
                "object selection cleared".to_string();
        }
        editor_canvas_for_selection.queue_draw();
    });
    context.editor_canvas.add_controller(selection_click);
}

#[derive(Clone)]
pub(in crate::app::editor_runtime) struct EditorDrawGestureContext {
    pub(in crate::app::editor_runtime) editor_canvas: DrawingArea,
    pub(in crate::app::editor_runtime) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(in crate::app::editor_runtime) active_editor_tool: Rc<Cell<ToolKind>>,
    pub(in crate::app::editor_runtime) tool_drag_preview: Rc<RefCell<Option<ToolDragPreview>>>,
    pub(in crate::app::editor_runtime) tool_drag_start_canvas: Rc<Cell<(f64, f64)>>,
    pub(in crate::app::editor_runtime) active_pen_stroke_id: Rc<Cell<Option<u64>>>,
    pub(in crate::app::editor_runtime) editor_undo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    pub(in crate::app::editor_runtime) editor_redo_stack: Rc<RefCell<Vec<Vec<ToolObject>>>>,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) editor_has_unsaved_changes: Rc<RefCell<bool>>,
    pub(in crate::app::editor_runtime) space_pan_pressed: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) selected_object_ids: Rc<RefCell<Vec<u64>>>,
    pub(in crate::app::editor_runtime) object_drag_state: Rc<RefCell<Option<ObjectDragState>>>,
    pub(in crate::app::editor_runtime) pending_crop: Rc<RefCell<Option<CropElement>>>,
    pub(in crate::app::editor_runtime) editor_image_base_width: i32,
    pub(in crate::app::editor_runtime) editor_image_base_height: i32,
    pub(in crate::app::editor_runtime) editor_source_pixbuf: Option<gtk4::gdk_pixbuf::Pixbuf>,
    pub(in crate::app::editor_runtime) ocr_engine: Rc<RefCell<Option<ocr_rs::OcrEngine>>>,
    pub(in crate::app::editor_runtime) ocr_language: crate::ocr::OcrLanguage,
    pub(in crate::app::editor_runtime) ocr_in_progress: Rc<Cell<bool>>,
}

fn handle_draw_gesture_begin(
    context: &EditorDrawGestureContext,
    gesture: &gtk4::GestureDrag,
    start_x: f64,
    start_y: f64,
) {
    let tool = context.active_editor_tool.get();
    if context.space_pan_pressed.get() || tool == ToolKind::Pan {
        gesture.set_state(gtk4::EventSequenceState::Denied);
        return;
    }
    if tool == ToolKind::Text {
        gesture.set_state(gtk4::EventSequenceState::Denied);
        return;
    }
    context.tool_drag_start_canvas.set((start_x, start_y));
    let start = canvas_point_to_image_point(
        &context.editor_canvas,
        start_x,
        start_y,
        context.editor_image_base_width,
        context.editor_image_base_height,
    );

    if context.active_editor_tool.get() == ToolKind::Crop {
        if let Some(crop) = context.pending_crop.borrow().as_ref().copied() {
            let crop_rect = RectangleElement::new(
                crop.id,
                crop.x,
                crop.y,
                crop.width,
                crop.height,
                Default::default(),
            );
            if let Some(handle) = rectangle_handle_at_point(&crop_rect, start) {
                *context.object_drag_state.borrow_mut() =
                    Some(ObjectDragState::ResizePendingCrop {
                        handle,
                        origin: crop,
                    });
                *context.status_log_for_render.borrow_mut() =
                    "crop frame resize started".to_string();
                gesture.set_state(gtk4::EventSequenceState::Claimed);
                return;
            }
            if point_in_bounds(start, crop.x, crop.y, crop.width, crop.height, 0) {
                *context.object_drag_state.borrow_mut() = Some(ObjectDragState::MovePendingCrop {
                    start,
                    origin: crop,
                });
                *context.status_log_for_render.borrow_mut() = "crop frame move started".to_string();
                gesture.set_state(gtk4::EventSequenceState::Claimed);
                return;
            }
        }
    }

    if tool == ToolKind::Select {
        let hit_id = {
            let tools = context.editor_tools.borrow();
            top_object_id_at_point(&tools, start)
        };
        if let Some(hit_id) = hit_id {
            let selected_id = context.selected_object_ids.borrow().first().copied();
            if selected_id != Some(hit_id) {
                set_single_selection(&context.selected_object_ids, hit_id);
            }

            let selected_drag_mode = {
                let tools = context.editor_tools.borrow();
                if let Some(object) = tools.object(hit_id) {
                    if let Some((kind, handle)) = resizable_object_handle_at_point(object, start) {
                        Some(ObjectDragState::ResizeObject {
                            object_id: hit_id,
                            kind,
                            handle,
                        })
                    } else {
                        Some(ObjectDragState::Move {
                            object_ids: vec![hit_id],
                            last: start,
                        })
                    }
                } else {
                    Some(ObjectDragState::Move {
                        object_ids: vec![hit_id],
                        last: start,
                    })
                }
            };
            if let Some(drag_mode) = selected_drag_mode {
                let snapshot = snapshot_editor_objects(context.editor_tools.as_ref());
                record_undo_snapshot(
                    context.editor_undo_stack.as_ref(),
                    context.editor_redo_stack.as_ref(),
                    snapshot,
                );
                *context.object_drag_state.borrow_mut() = Some(drag_mode);
                *context.status_log_for_render.borrow_mut() =
                    "selected object drag started".to_string();
                gesture.set_state(gtk4::EventSequenceState::Claimed);
                return;
            }
        }
    }

    if matches!(
        tool,
        ToolKind::Pen | ToolKind::Blur | ToolKind::Arrow | ToolKind::Rectangle
    ) {
        let snapshot = snapshot_editor_objects(context.editor_tools.as_ref());
        record_undo_snapshot(
            context.editor_undo_stack.as_ref(),
            context.editor_redo_stack.as_ref(),
            snapshot,
        );
    }

    let mut tools = context.editor_tools.borrow_mut();
    tools.select_tool(tool);
    match tool {
        ToolKind::Select => {
            context.active_pen_stroke_id.set(None);
            *context.tool_drag_preview.borrow_mut() = Some(ToolDragPreview {
                tool,
                start,
                current: start,
            });
        }
        ToolKind::Pan => {
            context.active_pen_stroke_id.set(None);
            context.tool_drag_preview.borrow_mut().take();
        }
        ToolKind::Pen => {
            let stroke_id = tools.begin_pen_stroke(start);
            context.active_pen_stroke_id.set(Some(stroke_id));
            context.tool_drag_preview.borrow_mut().take();
        }
        ToolKind::Blur | ToolKind::Arrow | ToolKind::Rectangle | ToolKind::Ocr => {
            context.active_pen_stroke_id.set(None);
            *context.tool_drag_preview.borrow_mut() = Some(ToolDragPreview {
                tool,
                start,
                current: start,
            });
        }
        ToolKind::Crop => {
            context.active_pen_stroke_id.set(None);
            context.pending_crop.borrow_mut().take();
            *context.tool_drag_preview.borrow_mut() = Some(ToolDragPreview {
                tool,
                start,
                current: start,
            });
        }
        ToolKind::Text => {}
    }

    *context.status_log_for_render.borrow_mut() = format!("{} drag started", tool_kind_label(tool));
    context.editor_canvas.queue_draw();
    gesture.set_state(gtk4::EventSequenceState::Claimed);
}

fn handle_draw_gesture_update(context: &EditorDrawGestureContext, offset_x: f64, offset_y: f64) {
    if context.space_pan_pressed.get() {
        return;
    }
    let (start_x, start_y) = context.tool_drag_start_canvas.get();
    let canvas_x = start_x + offset_x;
    let canvas_y = start_y + offset_y;
    let current = canvas_point_to_image_point(
        &context.editor_canvas,
        canvas_x,
        canvas_y,
        context.editor_image_base_width,
        context.editor_image_base_height,
    );

    if let Some(state) = context.object_drag_state.borrow_mut().as_mut() {
        match state {
            ObjectDragState::Move {
                object_ids, last, ..
            } => {
                let delta_x = current.x - last.x;
                let delta_y = current.y - last.y;
                let mut moved = false;
                {
                    let mut tools = context.editor_tools.borrow_mut();
                    for object_id in object_ids {
                        moved |= tools
                            .move_object_by(
                                *object_id,
                                delta_x,
                                delta_y,
                                context.editor_image_base_width,
                                context.editor_image_base_height,
                            )
                            .is_ok();
                    }
                }
                if moved {
                    *last = current;
                }
            }
            ObjectDragState::ResizeObject {
                object_id,
                kind,
                handle,
            } => {
                let _ = resize_object_from_handle(
                    &mut context.editor_tools.borrow_mut(),
                    *object_id,
                    *kind,
                    *handle,
                    current,
                    context.editor_image_base_width,
                    context.editor_image_base_height,
                );
            }
            ObjectDragState::MovePendingCrop { start, origin } => {
                let delta_x = current.x.saturating_sub(start.x);
                let delta_y = current.y.saturating_sub(start.y);
                let crop_width = i32::try_from(origin.width).unwrap_or(i32::MAX);
                let crop_height = i32::try_from(origin.height).unwrap_or(i32::MAX);
                let limit_x = context
                    .editor_image_base_width
                    .saturating_sub(crop_width)
                    .max(0);
                let limit_y = context
                    .editor_image_base_height
                    .saturating_sub(crop_height)
                    .max(0);
                let next_x = origin.x.saturating_add(delta_x).clamp(0, limit_x);
                let next_y = origin.y.saturating_add(delta_y).clamp(0, limit_y);
                *context.pending_crop.borrow_mut() = Some(CropElement::new(
                    origin.id,
                    next_x,
                    next_y,
                    origin.width,
                    origin.height,
                    origin.options,
                ));
            }
            ObjectDragState::ResizePendingCrop { handle, origin } => {
                if let Some(next_crop) = resized_crop_from_handle(
                    origin,
                    *handle,
                    current,
                    context.editor_image_base_width,
                    context.editor_image_base_height,
                ) {
                    *context.pending_crop.borrow_mut() = Some(next_crop);
                }
            }
        }
        context.editor_canvas.queue_draw();
        return;
    }

    if let Some(stroke_id) = context.active_pen_stroke_id.get() {
        let appended = context
            .editor_tools
            .borrow_mut()
            .append_pen_point(stroke_id, current)
            .is_ok();
        if appended {
            *context.status_log_for_render.borrow_mut() =
                format!("pen stroke point ({}, {})", current.x, current.y);
        }
    } else if let Some(preview) = context.tool_drag_preview.borrow_mut().as_mut() {
        preview.current = current;
    }
    context.editor_canvas.queue_draw();
}

fn handle_draw_gesture_end(context: &EditorDrawGestureContext, offset_x: f64, offset_y: f64) {
    let (start_x, start_y) = context.tool_drag_start_canvas.get();
    let end = canvas_point_to_image_point(
        &context.editor_canvas,
        start_x + offset_x,
        start_y + offset_y,
        context.editor_image_base_width,
        context.editor_image_base_height,
    );
    if let Some(state) = context.object_drag_state.borrow_mut().take() {
        match state {
            ObjectDragState::Move { object_ids, .. } => {
                *context.editor_has_unsaved_changes.borrow_mut() = true;
                *context.status_log_for_render.borrow_mut() =
                    format!("moved {} object(s)", object_ids.len());
            }
            ObjectDragState::ResizeObject {
                object_id, kind, ..
            } => {
                *context.editor_has_unsaved_changes.borrow_mut() = true;
                set_single_selection(&context.selected_object_ids, object_id);
                *context.status_log_for_render.borrow_mut() = resize_status_label(kind).to_string();
            }
            ObjectDragState::MovePendingCrop { .. } | ObjectDragState::ResizePendingCrop { .. } => {
                *context.status_log_for_render.borrow_mut() = "crop frame updated".to_string();
            }
        }
        context.editor_canvas.queue_draw();
        return;
    }

    if let Some(stroke_id) = context.active_pen_stroke_id.get() {
        let mut tools = context.editor_tools.borrow_mut();
        let _ = tools.append_pen_point(stroke_id, end);
        if tools.finish_pen_stroke(stroke_id).is_ok() {
            context.editor_redo_stack.borrow_mut().clear();
            *context.editor_has_unsaved_changes.borrow_mut() = true;
            set_single_selection(&context.selected_object_ids, stroke_id);
            *context.status_log_for_render.borrow_mut() =
                format!("pen stroke finalized at ({}, {})", end.x, end.y);
        }
        context.active_pen_stroke_id.set(None);
        context.editor_canvas.queue_draw();
        return;
    }

    let preview = context.tool_drag_preview.borrow_mut().take();
    let Some(preview) = preview else {
        return;
    };
    let mut tools = context.editor_tools.borrow_mut();
    if preview.tool == ToolKind::Select {
        let selected = top_object_id_in_drag_box(&tools, preview.start, end);
        set_optional_single_selection(&context.selected_object_ids, selected);
        *context.status_log_for_render.borrow_mut() = selected
            .map(|id| format!("selected object #{id}"))
            .unwrap_or_else(|| "object selection cleared".to_string());
        context.editor_canvas.queue_draw();
        return;
    }
    if preview.tool == ToolKind::Crop {
        let crop_options = tools.crop_options();
        let mut probe = editor::EditorTools::new();
        probe.set_crop_preset(crop_options.preset);
        let result = probe.add_crop_in_bounds(
            preview.start,
            end,
            u32::try_from(context.editor_image_base_width.max(1)).unwrap_or(u32::MAX),
            u32::try_from(context.editor_image_base_height.max(1)).unwrap_or(u32::MAX),
        );
        match result {
            Ok(crop_id) => {
                if let Some(crop) = probe.get_crop(crop_id).copied() {
                    *context.pending_crop.borrow_mut() = Some(crop);
                    *context.editor_has_unsaved_changes.borrow_mut() = true;
                    *context.status_log_for_render.borrow_mut() = format!(
                        "crop frame ready {}x{} at ({}, {})",
                        crop.width, crop.height, crop.x, crop.y
                    );
                }
            }
            Err(err) => {
                *context.status_log_for_render.borrow_mut() = format!("crop drag ignored: {err:?}");
            }
        }
        context.editor_canvas.queue_draw();
        return;
    }

    if preview.tool == ToolKind::Ocr {
        drop(tools);
        perform_ocr_recognition(context, preview.start, end);
        context.editor_canvas.queue_draw();
        return;
    }

    let outcome = match preview.tool {
        ToolKind::Select | ToolKind::Pan => Err(editor::ToolError::ToolNotSelected),
        ToolKind::Blur => normalize_tool_box(preview.start, end)
            .ok_or(editor::ToolError::InvalidBlurRegion)
            .and_then(|(x, y, width, height)| {
                tools.add_blur(editor::tools::BlurRegion::new(x, y, width, height))
            }),
        ToolKind::Arrow => tools.add_arrow(preview.start, end),
        ToolKind::Rectangle => tools.add_rectangle(preview.start, end),
        ToolKind::Crop => Err(editor::ToolError::ToolNotSelected),
        ToolKind::Pen | ToolKind::Text | ToolKind::Ocr => Err(editor::ToolError::ToolNotSelected),
    };

    match outcome {
        Ok(object_id) => {
            let created_tool = preview.tool;
            context.editor_redo_stack.borrow_mut().clear();
            *context.editor_has_unsaved_changes.borrow_mut() = true;
            set_single_selection(&context.selected_object_ids, object_id);
            *context.status_log_for_render.borrow_mut() = format!(
                "{} object #{object_id} created",
                tool_kind_label(created_tool)
            );
        }
        Err(err) => {
            *context.status_log_for_render.borrow_mut() =
                format!("{} drag ignored: {err:?}", tool_kind_label(preview.tool));
        }
    }
    context.editor_canvas.queue_draw();
}

fn perform_ocr_recognition(
    context: &EditorDrawGestureContext,
    start: editor::tools::ToolPoint,
    end: editor::tools::ToolPoint,
) {
    use crate::app::ocr_support::{handle_ocr_text_result, ocr_processing_status};
    use crate::app::worker::spawn_worker_action;

    if context.ocr_in_progress.get() {
        *context.status_log_for_render.borrow_mut() = "OCR already in progress".to_string();
        return;
    }

    let Some((x, y, width, height)) = normalize_tool_box(start, end) else {
        *context.status_log_for_render.borrow_mut() = "OCR drag too small".to_string();
        return;
    };

    let Some(ref pixbuf) = context.editor_source_pixbuf else {
        *context.status_log_for_render.borrow_mut() = "OCR: no source image available".to_string();
        return;
    };

    // Convert Pixbuf region to DynamicImage on the main thread (Pixbuf is not Send).
    let image = match crate::ocr::pixbuf_region_to_dynamic_image(pixbuf, x, y, width, height) {
        Ok(img) => img,
        Err(err) => {
            *context.status_log_for_render.borrow_mut() =
                format!("OCR image conversion failed: {err}");
            crate::notification::send(format!("OCR failed: {err}"));
            return;
        }
    };

    // Take the engine for the worker thread.
    let engine = context.ocr_engine.borrow_mut().take();
    let ocr_language = context.ocr_language;

    // Set progress state and feedback.
    context.ocr_in_progress.set(true);
    *context.status_log_for_render.borrow_mut() =
        ocr_processing_status(engine.is_some()).to_string();
    context.editor_canvas.set_cursor_from_name(Some("progress"));

    let ocr_engine = context.ocr_engine.clone();
    let ocr_in_progress = context.ocr_in_progress.clone();
    let status_log = context.status_log_for_render.clone();
    let editor_canvas = context.editor_canvas.clone();

    spawn_worker_action(
        move || {
            let engine = match crate::app::ocr_support::resolve_or_init_engine(engine, ocr_language)
            {
                Ok(e) => e,
                Err(err) => return (None, Err(err)),
            };
            let result = crate::ocr::recognize_text(&engine, &image);
            (Some(engine), result)
        },
        move |(engine, result): (
            Option<ocr_rs::OcrEngine>,
            Result<String, crate::ocr::OcrError>,
        )| {
            // Restore engine.
            if let Some(engine) = engine {
                *ocr_engine.borrow_mut() = Some(engine);
            }
            ocr_in_progress.set(false);

            // Restore cursor.
            editor_canvas.set_cursor_from_name(None::<&str>);

            match result {
                Ok(text) => handle_ocr_text_result(&status_log, text),
                Err(err) => {
                    *status_log.borrow_mut() = format!("OCR failed: {err}");
                    crate::notification::send(format!("OCR failed: {err}"));
                }
            }
            editor_canvas.queue_draw();
        },
    );
}

pub(in crate::app::editor_runtime) fn connect_editor_draw_gesture(
    context: EditorDrawGestureContext,
) {
    let draw_gesture = gtk4::GestureDrag::new();
    draw_gesture.set_button(gtk4::gdk::BUTTON_PRIMARY);

    let begin_context = context.clone();
    draw_gesture.connect_drag_begin(move |gesture, start_x, start_y| {
        handle_draw_gesture_begin(&begin_context, gesture, start_x, start_y);
    });

    let update_context = context.clone();
    draw_gesture.connect_drag_update(move |_, offset_x, offset_y| {
        handle_draw_gesture_update(&update_context, offset_x, offset_y);
    });

    let end_context = context.clone();
    draw_gesture.connect_drag_end(move |_, offset_x, offset_y| {
        handle_draw_gesture_end(&end_context, offset_x, offset_y);
    });

    context.editor_canvas.add_controller(draw_gesture);
}

#[derive(Clone)]
pub(in crate::app::editor_runtime) struct EditorPanGestureContext {
    pub(in crate::app::editor_runtime) editor_scroller: ScrolledWindow,
    pub(in crate::app::editor_runtime) editor_viewport: Rc<RefCell<editor::EditorViewport>>,
    pub(in crate::app::editor_runtime) editor_canvas: DrawingArea,
    pub(in crate::app::editor_runtime) editor_viewport_status: Label,
    pub(in crate::app::editor_runtime) status_log_for_render: Rc<RefCell<String>>,
    pub(in crate::app::editor_runtime) space_pan_pressed: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) active_editor_tool: Rc<Cell<ToolKind>>,
    pub(in crate::app::editor_runtime) drag_pan_active: Rc<Cell<bool>>,
    pub(in crate::app::editor_runtime) drag_pan_pointer_origin: Rc<Cell<(f64, f64)>>,
    pub(in crate::app::editor_runtime) refresh_editor_cursor: Rc<dyn Fn()>,
    pub(in crate::app::editor_runtime) editor_image_base_width: i32,
    pub(in crate::app::editor_runtime) editor_image_base_height: i32,
}

fn handle_pan_drag_begin(context: &EditorPanGestureContext, gesture: &gtk4::GestureDrag) {
    if !context.space_pan_pressed.get() && context.active_editor_tool.get() != ToolKind::Pan {
        context.drag_pan_active.set(false);
        gesture.set_state(gtk4::EventSequenceState::Denied);
        return;
    }
    context.drag_pan_pointer_origin.set((0.0, 0.0));
    gesture.set_state(gtk4::EventSequenceState::Claimed);
    context.drag_pan_active.set(true);
    (context.refresh_editor_cursor.as_ref())();
    *context.status_log_for_render.borrow_mut() = "editor pan drag started".to_string();
}

fn handle_pan_drag_update(context: &EditorPanGestureContext, offset_x: f64, offset_y: f64) {
    if !context.space_pan_pressed.get() && context.active_editor_tool.get() != ToolKind::Pan {
        context.drag_pan_active.set(false);
        return;
    }

    if !context.drag_pan_active.get() {
        context.drag_pan_pointer_origin.set((offset_x, offset_y));
        context.drag_pan_active.set(true);
        (context.refresh_editor_cursor.as_ref())();
    }

    let (origin_offset_x, origin_offset_y) = context.drag_pan_pointer_origin.get();
    let pointer_delta_x = offset_x - origin_offset_x;
    let pointer_delta_y = offset_y - origin_offset_y;
    let pan_delta_x = (-pointer_delta_x).round() as i32;
    let pan_delta_y = (-pointer_delta_y).round() as i32;
    if pan_delta_x == 0 && pan_delta_y == 0 {
        return;
    }
    context.drag_pan_pointer_origin.set((offset_x, offset_y));
    let mut viewport = context.editor_viewport.borrow_mut();
    viewport.pan_by(pan_delta_x, pan_delta_y);
    apply_editor_viewport_to_canvas(
        &context.editor_canvas,
        &context.editor_scroller,
        &mut viewport,
        context.editor_image_base_width,
        context.editor_image_base_height,
    );
    set_editor_viewport_status(
        &context.editor_viewport_status,
        &viewport,
        &context.editor_canvas,
        context.editor_image_base_width,
        context.editor_image_base_height,
    );
    *context.status_log_for_render.borrow_mut() = format!(
        "editor viewport pan ({}, {})",
        viewport.pan_x(),
        viewport.pan_y()
    );
}

fn handle_pan_drag_end(context: &EditorPanGestureContext) {
    context.drag_pan_active.set(false);
    context.drag_pan_pointer_origin.set((0.0, 0.0));
    (context.refresh_editor_cursor.as_ref())();
}

pub(in crate::app::editor_runtime) fn connect_editor_pan_drag_gesture(
    context: EditorPanGestureContext,
) {
    let pan_drag_gesture = gtk4::GestureDrag::new();
    pan_drag_gesture.set_button(gtk4::gdk::BUTTON_PRIMARY);
    pan_drag_gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);

    let begin_context = context.clone();
    pan_drag_gesture.connect_drag_begin(move |gesture, _, _| {
        handle_pan_drag_begin(&begin_context, gesture);
    });

    let update_context = context.clone();
    pan_drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
        handle_pan_drag_update(&update_context, offset_x, offset_y);
    });

    let end_context = context.clone();
    pan_drag_gesture.connect_drag_end(move |_, _, _| {
        handle_pan_drag_end(&end_context);
    });

    context.editor_scroller.add_controller(pan_drag_gesture);
}

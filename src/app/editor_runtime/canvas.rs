use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::tools::{CropElement, ImageBounds};
use crate::editor::{self};

use gtk4::gdk::prelude::GdkCairoContextExt;
use gtk4::prelude::*;
use gtk4::DrawingArea;

use crate::app::adaptive::{
    adaptive_stroke_size_for_image_with_presets, adaptive_text_size_for_image_with_presets,
    EditorToolOptionPresets,
};
use crate::app::editor_popup::{
    caret_layout_to_canvas_cursor_rect, draw_crop_mask, draw_drag_preview_overlay,
    draw_editor_tool_objects, text_caret_layout, BlurRenderCache, EditorSelectionPalette,
    EditorTextInputPalette, TextPreeditState, ToolDragPreview, ToolRenderContext,
};

pub(super) struct EditorCanvasDrawDeps {
    pub(super) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(super) tool_drag_preview: Rc<RefCell<Option<ToolDragPreview>>>,
    pub(super) selected_object_ids: Rc<RefCell<Vec<u64>>>,
    pub(super) pending_crop: Rc<RefCell<Option<CropElement>>>,
    pub(super) editor_selection_palette: EditorSelectionPalette,
    pub(super) text_input_palette: EditorTextInputPalette,
    pub(super) editor_input_mode: Rc<RefCell<editor::EditorInputMode>>,
    pub(super) text_preedit_state: Rc<RefCell<TextPreeditState>>,
    pub(super) text_im_context: Rc<gtk4::IMMulticontext>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_editor_default_tool_settings(
    editor_tools: &Rc<RefCell<editor::EditorTools>>,
    tool_option_presets: &EditorToolOptionPresets,
    default_tool_color_override: Option<editor::tools::Color>,
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
    let default_stroke = default_tool_color_override.unwrap_or_else(|| {
        let (r, g, b) = tool_option_presets.stroke_color_palette().default_color();
        editor::tools::Color::new(r, g, b)
    });
    let mut tools = editor_tools.borrow_mut();
    tools.set_text_size(default_text_size);
    tools.set_shared_stroke_color(default_stroke);
    tools.set_shared_stroke_thickness(default_stroke_size);
    if let Some(radius) = rectangle_border_radius_override {
        tools.set_rectangle_border_radius(radius);
    }
    tools.set_arrow_head_size(default_stroke_size.saturating_mul(3).max(8));
}

pub(super) fn configure_editor_canvas_draw(
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
        editor_selection_palette,
        text_input_palette,
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
                selection_palette: editor_selection_palette,
                text_input_palette,
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
            draw_drag_preview_overlay(
                context,
                &preview,
                &tools,
                pixbuf.width(),
                pixbuf.height(),
                editor_selection_palette,
            );
        }
        context.restore().ok();
    });
}

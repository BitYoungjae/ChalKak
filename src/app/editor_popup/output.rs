use gtk4::gdk::prelude::GdkCairoContextExt;
use std::path::Path;

use crate::clipboard::WlCopyBackend;
use crate::editor::{self, EditorAction, EditorEvent};

use super::{
    draw_editor_tool_objects, EditorOutputActionContext, EditorSelectionPalette,
    EditorTextInputPalette, ToolRenderContext,
};

pub(in crate::app) fn render_editor_output_png(
    source_pixbuf: &gtk4::gdk_pixbuf::Pixbuf,
    tools: &editor::EditorTools,
    pending_crop: Option<editor::tools::CropElement>,
    output_path: &Path,
) -> Result<(), String> {
    let image_width = source_pixbuf.width().max(1);
    let image_height = source_pixbuf.height().max(1);

    let composed_surface =
        gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, image_width, image_height)
            .map_err(|err| format!("create composed surface failed: {err}"))?;
    let composed_context = gtk4::cairo::Context::new(&composed_surface)
        .map_err(|err| format!("create composed context failed: {err}"))?;
    composed_context.set_source_pixbuf(source_pixbuf, 0.0, 0.0);
    composed_context
        .paint()
        .map_err(|err| format!("paint source image failed: {err}"))?;

    draw_editor_tool_objects(
        &composed_context,
        tools,
        ToolRenderContext {
            image_bounds: editor::tools::ImageBounds::new(image_width, image_height),
            show_crop_mask: false,
            selected_object_ids: &[],
            selection_palette: EditorSelectionPalette::default(),
            text_input_palette: EditorTextInputPalette::default(),
            source_pixbuf: Some(source_pixbuf),
            active_text_id: None,
            active_text_preedit: None,
            blur_cache: None,
        },
    );

    let crop_region = pending_crop.or_else(|| tools.crops().last().copied());
    if let Some(crop) = crop_region {
        let max_x = image_width.saturating_sub(1).max(0);
        let max_y = image_height.saturating_sub(1).max(0);
        let crop_x = crop.x.clamp(0, max_x);
        let crop_y = crop.y.clamp(0, max_y);
        let crop_width = i32::try_from(crop.width)
            .unwrap_or(i32::MAX)
            .clamp(1, image_width - crop_x);
        let crop_height = i32::try_from(crop.height)
            .unwrap_or(i32::MAX)
            .clamp(1, image_height - crop_y);

        let cropped_surface =
            gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, crop_width, crop_height)
                .map_err(|err| format!("create cropped surface failed: {err}"))?;
        let cropped_context = gtk4::cairo::Context::new(&cropped_surface)
            .map_err(|err| format!("create cropped context failed: {err}"))?;
        cropped_context
            .set_source_surface(&composed_surface, -f64::from(crop_x), -f64::from(crop_y))
            .map_err(|err| format!("set cropped source failed: {err}"))?;
        cropped_context
            .paint()
            .map_err(|err| format!("paint cropped image failed: {err}"))?;

        let Some(cropped_pixbuf) =
            gtk4::gdk::pixbuf_get_from_surface(&cropped_surface, 0, 0, crop_width, crop_height)
        else {
            return Err("convert cropped surface to pixbuf failed".to_string());
        };
        cropped_pixbuf
            .savev(output_path, "png", &[])
            .map_err(|err| format!("write cropped png failed: {err}"))?;
    } else {
        let Some(composed_pixbuf) =
            gtk4::gdk::pixbuf_get_from_surface(&composed_surface, 0, 0, image_width, image_height)
        else {
            return Err("convert composed surface to pixbuf failed".to_string());
        };
        composed_pixbuf
            .savev(output_path, "png", &[])
            .map_err(|err| format!("write composed png failed: {err}"))?;
    }

    Ok(())
}

fn action_metadata(action: EditorAction) -> Option<(&'static str, &'static str)> {
    match action {
        EditorAction::Save => Some(("save", "Save")),
        EditorAction::Copy => Some(("copy", "Copy")),
        EditorAction::CloseRequested => None,
    }
}

fn ensure_rendered_output(ctx: &EditorOutputActionContext<'_>, action_label: &str) -> bool {
    let should_render_output = ctx.pending_crop.is_some()
        || *ctx.editor_has_unsaved_changes.borrow()
        || !ctx.active_capture.temp_path.exists();
    if !should_render_output {
        return true;
    }

    if let Err(err) = render_editor_output_png(
        ctx.source_pixbuf,
        ctx.editor_tools,
        ctx.pending_crop,
        &ctx.active_capture.temp_path,
    ) {
        *ctx.status_log.borrow_mut() = format!("editor render failed before {action_label}: {err}");
        ctx.editor_toast
            .show(format!("Render failed: {err}"), ctx.toast_duration_ms);
        return false;
    }

    true
}

pub(in crate::app) fn execute_editor_output_action(ctx: EditorOutputActionContext<'_>) -> bool {
    let Some((action_label, action_title)) = action_metadata(ctx.action) else {
        *ctx.status_log.borrow_mut() = "unsupported editor output action: close".to_string();
        return false;
    };

    if !ensure_rendered_output(&ctx, action_label) {
        return false;
    }

    match editor::execute_editor_action(
        ctx.active_capture,
        ctx.action,
        ctx.storage_service,
        &WlCopyBackend,
    ) {
        Ok(EditorEvent::Save { capture_id }) if ctx.action == EditorAction::Save => {
            *ctx.editor_has_unsaved_changes.borrow_mut() = false;
            *ctx.status_log.borrow_mut() = format!("editor saved capture {capture_id}");
            crate::notification::send(format!("Saved {capture_id}"));
            true
        }
        Ok(EditorEvent::Copy { capture_id }) if ctx.action == EditorAction::Copy => {
            *ctx.status_log.borrow_mut() = format!("editor copied capture {capture_id}");
            crate::notification::send(format!("Copied {capture_id}"));
            true
        }
        Ok(other) => {
            *ctx.status_log.borrow_mut() = format!("editor {action_label} produced {other:?}");
            ctx.editor_toast.show(
                format!("{action_title} produced {other:?}"),
                ctx.toast_duration_ms,
            );
            false
        }
        Err(err) => {
            *ctx.status_log.borrow_mut() = format!("editor {action_label} failed: {err}");
            ctx.editor_toast.show(
                format!("{action_title} failed: {err}"),
                ctx.toast_duration_ms,
            );
            false
        }
    }
}

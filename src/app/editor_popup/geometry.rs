use crate::editor::tools::{
    CropElement, ImageBounds, RectangleElement, TextElement, ToolBounds, ToolPoint,
};
use crate::editor::{self, ToolObject};
use gtk4::prelude::*;
use gtk4::DrawingArea;

use super::{RectangleHandle, ResizableObjectKind, RgbaColor, TextCaretLayout};

pub(in crate::app) fn clamp_tool_point(
    mut point: ToolPoint,
    image_width: i32,
    image_height: i32,
) -> ToolPoint {
    let max_x = image_width.saturating_sub(1).max(0);
    let max_y = image_height.saturating_sub(1).max(0);
    point.x = point.x.clamp(0, max_x);
    point.y = point.y.clamp(0, max_y);
    point
}

pub(in crate::app) fn canvas_point_to_image_point(
    canvas: &DrawingArea,
    canvas_x: f64,
    canvas_y: f64,
    image_width: i32,
    image_height: i32,
) -> ToolPoint {
    let canvas_width = f64::from(
        if canvas.allocated_width() > 1 {
            canvas.allocated_width()
        } else {
            canvas.content_width()
        }
        .max(1),
    );
    let canvas_height = f64::from(
        if canvas.allocated_height() > 1 {
            canvas.allocated_height()
        } else {
            canvas.content_height()
        }
        .max(1),
    );
    let source_width = f64::from(image_width.max(1));
    let source_height = f64::from(image_height.max(1));
    let x = (canvas_x.clamp(0.0, canvas_width) * source_width / canvas_width).round() as i32;
    let y = (canvas_y.clamp(0.0, canvas_height) * source_height / canvas_height).round() as i32;
    clamp_tool_point(ToolPoint::new(x, y), image_width, image_height)
}

pub(in crate::app) fn normalize_tool_box(
    start: ToolPoint,
    end: ToolPoint,
) -> Option<(i32, i32, u32, u32)> {
    let left = start.x.min(end.x);
    let right = start.x.max(end.x);
    let top = start.y.min(end.y);
    let bottom = start.y.max(end.y);
    let width = u32::try_from(right - left).ok()?;
    let height = u32::try_from(bottom - top).ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    Some((left, top, width, height))
}

pub(in crate::app) fn text_line_height(text: &TextElement) -> f64 {
    (f64::from(text.options.size.max(1)) * 1.3).max(2.0)
}

pub(in crate::app) fn text_lines_for_render(text: &TextElement) -> Vec<&str> {
    if text.content.is_empty() {
        vec![""]
    } else {
        text.content.split('\n').collect::<Vec<_>>()
    }
}

pub(in crate::app) fn text_baseline_y(text: &TextElement) -> f64 {
    f64::from(text.y) + f64::from(text.options.size.max(1))
}

fn text_measurement_context(text: &TextElement) -> Option<gtk4::cairo::Context> {
    let surface = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 1, 1).ok()?;
    let context = gtk4::cairo::Context::new(&surface).ok()?;
    let weight = if text.options.weight >= 600 {
        gtk4::cairo::FontWeight::Bold
    } else {
        gtk4::cairo::FontWeight::Normal
    };
    context.select_font_face(
        text.options.family.cairo_font_name(),
        gtk4::cairo::FontSlant::Normal,
        weight,
    );
    context.set_font_size(f64::from(text.options.size.max(1)));
    Some(context)
}

fn text_dimensions_from_lines(
    lines: &[&str],
    font_size: f64,
    line_height: f64,
    mut measure_width: impl FnMut(&str) -> f64,
) -> (u32, u32) {
    let max_width = lines
        .iter()
        .map(|line| measure_width(line).max(0.0))
        .fold(0.0, f64::max);
    let width = max_width.ceil().max(8.0) as i32;
    let height = ((lines.len() as f64 * line_height).round() as i32).max(font_size as i32);
    (
        u32::try_from(width).unwrap_or(8),
        u32::try_from(height).unwrap_or(8),
    )
}

pub(in crate::app) fn caret_layout_to_canvas_cursor_rect(
    caret: TextCaretLayout,
    canvas_width: i32,
    canvas_height: i32,
    image_width: i32,
    image_height: i32,
) -> gtk4::gdk::Rectangle {
    let scale_x = f64::from(canvas_width.max(1)) / f64::from(image_width.max(1));
    let scale_y = f64::from(canvas_height.max(1)) / f64::from(image_height.max(1));
    let x = (caret.caret_x * scale_x).round() as i32;
    let y = (caret.caret_top * scale_y).round() as i32;
    let width = scale_x.ceil().max(1.0).round() as i32;
    let height = (caret.caret_height() * scale_y).ceil().max(1.0).round() as i32;
    gtk4::gdk::Rectangle::new(x.max(0), y.max(0), width.max(1), height.max(1))
}

pub(in crate::app) fn text_dimensions(text: &TextElement) -> (u32, u32) {
    let font_size = f64::from(text.options.size.max(1));
    let line_height = text_line_height(text).round();
    let lines = text_lines_for_render(text);
    let fallback_char_width = (font_size * 0.62).max(1.0);
    if let Some(context) = text_measurement_context(text) {
        return text_dimensions_from_lines(&lines, font_size, line_height, |line| {
            if line.is_empty() {
                0.0
            } else {
                context
                    .text_extents(line)
                    .map(|extents| extents.x_advance())
                    .unwrap_or_else(|_| line.chars().count() as f64 * fallback_char_width)
            }
        });
    }

    text_dimensions_from_lines(&lines, font_size, line_height, |line| {
        line.chars().count() as f64 * fallback_char_width
    })
}

pub(in crate::app) fn object_bounds(object: &ToolObject) -> Option<(i32, i32, u32, u32)> {
    match object {
        ToolObject::Blur(blur) => Some((
            blur.region.x,
            blur.region.y,
            blur.region.width,
            blur.region.height,
        )),
        ToolObject::Pen(stroke) => {
            let first = stroke.points.first()?;
            let mut min_x = first.x;
            let mut min_y = first.y;
            let mut max_x = first.x;
            let mut max_y = first.y;
            for point in &stroke.points[1..] {
                min_x = min_x.min(point.x);
                min_y = min_y.min(point.y);
                max_x = max_x.max(point.x);
                max_y = max_y.max(point.y);
            }
            let width = u32::try_from(max_x.saturating_sub(min_x).saturating_add(1)).ok()?;
            let height = u32::try_from(max_y.saturating_sub(min_y).saturating_add(1)).ok()?;
            Some((min_x, min_y, width, height))
        }
        ToolObject::Arrow(arrow) => {
            let min_x = arrow.start.x.min(arrow.end.x);
            let min_y = arrow.start.y.min(arrow.end.y);
            let max_x = arrow.start.x.max(arrow.end.x);
            let max_y = arrow.start.y.max(arrow.end.y);
            let width = u32::try_from(max_x.saturating_sub(min_x).saturating_add(1)).ok()?;
            let height = u32::try_from(max_y.saturating_sub(min_y).saturating_add(1)).ok()?;
            Some((min_x, min_y, width, height))
        }
        ToolObject::Rectangle(rectangle) => {
            Some((rectangle.x, rectangle.y, rectangle.width, rectangle.height))
        }
        ToolObject::Crop(crop) => Some((crop.x, crop.y, crop.width, crop.height)),
        ToolObject::Text(text) => {
            let (width, height) = text_dimensions(text);
            Some((text.x, text.y, width, height))
        }
    }
}

pub(in crate::app) const fn object_is_blur(object: &ToolObject) -> bool {
    matches!(object, ToolObject::Blur(_))
}

pub(in crate::app) fn objects_in_draw_order<'a>(
    tools: &'a editor::EditorTools,
) -> impl Iterator<Item = &'a ToolObject> + 'a {
    tools
        .objects()
        .iter()
        .filter(|object| object_is_blur(object))
        .chain(
            tools
                .objects()
                .iter()
                .filter(|object| !object_is_blur(object)),
        )
}

pub(in crate::app) fn objects_in_hit_test_order<'a>(
    tools: &'a editor::EditorTools,
) -> impl Iterator<Item = &'a ToolObject> + 'a {
    tools
        .objects()
        .iter()
        .rev()
        .filter(|object| !object_is_blur(object))
        .chain(
            tools
                .objects()
                .iter()
                .rev()
                .filter(|object| object_is_blur(object)),
        )
}

pub(in crate::app) fn point_in_bounds(
    point: ToolPoint,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    padding: i32,
) -> bool {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    let height = i32::try_from(height).unwrap_or(i32::MAX);
    let left = x.saturating_sub(padding);
    let top = y.saturating_sub(padding);
    let right = x.saturating_add(width).saturating_add(padding);
    let bottom = y.saturating_add(height).saturating_add(padding);
    point.x >= left && point.x <= right && point.y >= top && point.y <= bottom
}

pub(in crate::app) fn top_object_id_at_point(
    tools: &editor::EditorTools,
    point: ToolPoint,
) -> Option<u64> {
    for object in objects_in_hit_test_order(tools) {
        if let Some((x, y, width, height)) = object_bounds(object) {
            if point_in_bounds(point, x, y, width, height, 4) {
                return Some(object.id());
            }
        }
    }
    None
}

pub(in crate::app) fn bounds_intersect(a: ToolBounds, b: ToolBounds) -> bool {
    let aright =
        a.x.saturating_add(i32::try_from(a.width).unwrap_or(i32::MAX));
    let abottom =
        a.y.saturating_add(i32::try_from(a.height).unwrap_or(i32::MAX));
    let bright =
        b.x.saturating_add(i32::try_from(b.width).unwrap_or(i32::MAX));
    let bbottom =
        b.y.saturating_add(i32::try_from(b.height).unwrap_or(i32::MAX));

    a.x <= bright && b.x <= aright && a.y <= bbottom && b.y <= abottom
}

pub(in crate::app) fn object_ids_in_drag_box(
    tools: &editor::EditorTools,
    start: ToolPoint,
    end: ToolPoint,
) -> Vec<u64> {
    let Some((x, y, width, height)) = normalize_tool_box(start, end) else {
        return Vec::new();
    };
    let selection_bounds = ToolBounds::new(x, y, width, height);
    let mut ids = Vec::new();
    for object in objects_in_hit_test_order(tools) {
        if let Some((object_x, object_y, object_width, object_height)) = object_bounds(object) {
            if bounds_intersect(
                selection_bounds,
                ToolBounds::new(object_x, object_y, object_width, object_height),
            ) {
                ids.push(object.id());
            }
        }
    }
    ids
}

pub(in crate::app) fn top_object_id_in_drag_box(
    tools: &editor::EditorTools,
    start: ToolPoint,
    end: ToolPoint,
) -> Option<u64> {
    object_ids_in_drag_box(tools, start, end).into_iter().next()
}

pub(in crate::app) fn top_text_id_at_point(
    tools: &editor::EditorTools,
    point: ToolPoint,
) -> Option<u64> {
    for object in objects_in_hit_test_order(tools) {
        if let ToolObject::Text(text) = object {
            let (width, height) = text_dimensions(text);
            if point_in_bounds(point, text.x, text.y, width, height, 4) {
                return Some(text.id);
            }
        }
    }
    None
}

pub(in crate::app) fn corner_points_for_bounds(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> [(RectangleHandle, ToolPoint); 4] {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    let height = i32::try_from(height).unwrap_or(i32::MAX);
    [
        (RectangleHandle::TopLeft, ToolPoint::new(x, y)),
        (
            RectangleHandle::TopRight,
            ToolPoint::new(x.saturating_add(width), y),
        ),
        (
            RectangleHandle::BottomLeft,
            ToolPoint::new(x, y.saturating_add(height)),
        ),
        (
            RectangleHandle::BottomRight,
            ToolPoint::new(x.saturating_add(width), y.saturating_add(height)),
        ),
    ]
}

pub(in crate::app) fn handle_at_point_for_bounds(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    point: ToolPoint,
) -> Option<RectangleHandle> {
    for (handle, corner) in corner_points_for_bounds(x, y, width, height) {
        if (point.x - corner.x).abs() <= 8 && (point.y - corner.y).abs() <= 8 {
            return Some(handle);
        }
    }
    None
}

pub(in crate::app) fn rectangle_handle_at_point(
    rectangle: &RectangleElement,
    point: ToolPoint,
) -> Option<RectangleHandle> {
    handle_at_point_for_bounds(
        rectangle.x,
        rectangle.y,
        rectangle.width,
        rectangle.height,
        point,
    )
}

pub(in crate::app) fn resizable_object_handle_at_point(
    object: &ToolObject,
    point: ToolPoint,
) -> Option<(ResizableObjectKind, RectangleHandle)> {
    match object {
        ToolObject::Rectangle(rectangle) => rectangle_handle_at_point(rectangle, point)
            .map(|handle| (ResizableObjectKind::Rectangle, handle)),
        ToolObject::Blur(blur) => handle_at_point_for_bounds(
            blur.region.x,
            blur.region.y,
            blur.region.width,
            blur.region.height,
            point,
        )
        .map(|handle| (ResizableObjectKind::Blur, handle)),
        ToolObject::Crop(crop) => {
            handle_at_point_for_bounds(crop.x, crop.y, crop.width, crop.height, point)
                .map(|handle| (ResizableObjectKind::Crop, handle))
        }
        _ => None,
    }
}

pub(in crate::app) fn adjust_ratio_to_fit(
    width: u32,
    height: u32,
    ratio_x: u32,
    ratio_y: u32,
) -> (u32, u32) {
    editor::tools::adjust_ratio_to_fit(width, height, ratio_x, ratio_y)
}

fn opposite_corner(bounds: ToolBounds, handle: RectangleHandle) -> Option<(i32, i32)> {
    let width = i32::try_from(bounds.width).ok()?;
    let height = i32::try_from(bounds.height).ok()?;
    let point = match handle {
        RectangleHandle::TopLeft => (
            bounds.x.saturating_add(width),
            bounds.y.saturating_add(height),
        ),
        RectangleHandle::TopRight => (bounds.x, bounds.y.saturating_add(height)),
        RectangleHandle::BottomLeft => (bounds.x.saturating_add(width), bounds.y),
        RectangleHandle::BottomRight => (bounds.x, bounds.y),
    };
    Some(point)
}

pub(in crate::app) fn resized_bounds_from_handle(
    bounds: ToolBounds,
    handle: RectangleHandle,
    point: ToolPoint,
    image_bounds: ImageBounds,
) -> Option<ToolBounds> {
    let (anchor_x, anchor_y) = opposite_corner(bounds, handle)?;
    let left = point
        .x
        .min(anchor_x)
        .clamp(0, image_bounds.width.saturating_sub(1).max(0));
    let top = point
        .y
        .min(anchor_y)
        .clamp(0, image_bounds.height.saturating_sub(1).max(0));
    let right = point.x.max(anchor_x).clamp(1, image_bounds.width.max(1));
    let bottom = point.y.max(anchor_y).clamp(1, image_bounds.height.max(1));
    if right <= left || bottom <= top {
        return None;
    }
    let next_width = u32::try_from(right - left).ok()?;
    let next_height = u32::try_from(bottom - top).ok()?;
    if next_width < 4 || next_height < 4 {
        return None;
    }
    Some(ToolBounds::new(left, top, next_width, next_height))
}

pub(in crate::app) fn resized_rectangle_from_handle(
    rectangle: &RectangleElement,
    handle: RectangleHandle,
    point: ToolPoint,
    image_width: i32,
    image_height: i32,
) -> Option<ToolBounds> {
    resized_bounds_from_handle(
        ToolBounds::new(rectangle.x, rectangle.y, rectangle.width, rectangle.height),
        handle,
        point,
        ImageBounds::new(image_width, image_height),
    )
}

fn crop_ratio(crop: &CropElement, image_width: i32, image_height: i32) -> Option<(u32, u32)> {
    let w = u32::try_from(image_width.max(1)).unwrap_or(1);
    let h = u32::try_from(image_height.max(1)).unwrap_or(1);
    crop.options.preset.resolve_ratio(w, h)
}

pub(in crate::app) fn resized_crop_from_handle(
    crop: &CropElement,
    handle: RectangleHandle,
    point: ToolPoint,
    image_width: i32,
    image_height: i32,
) -> Option<CropElement> {
    let crop_bounds = ToolBounds::new(crop.x, crop.y, crop.width, crop.height);
    let (anchor_x, anchor_y) = opposite_corner(crop_bounds, handle)?;

    let bounded_x = point.x.clamp(0, image_width.max(1));
    let bounded_y = point.y.clamp(0, image_height.max(1));
    let mut next_width = u32::try_from((bounded_x - anchor_x).abs()).ok()?;
    let mut next_height = u32::try_from((bounded_y - anchor_y).abs()).ok()?;

    if let Some((ratio_x, ratio_y)) = crop_ratio(crop, image_width, image_height) {
        let adjusted = adjust_ratio_to_fit(next_width, next_height, ratio_x, ratio_y);
        next_width = adjusted.0;
        next_height = adjusted.1;
    }

    let next_width_i32 = i32::try_from(next_width).ok()?;
    let next_height_i32 = i32::try_from(next_height).ok()?;
    let (x, y) = match handle {
        RectangleHandle::TopLeft => (
            anchor_x.saturating_sub(next_width_i32),
            anchor_y.saturating_sub(next_height_i32),
        ),
        RectangleHandle::TopRight => (anchor_x, anchor_y.saturating_sub(next_height_i32)),
        RectangleHandle::BottomLeft => (anchor_x.saturating_sub(next_width_i32), anchor_y),
        RectangleHandle::BottomRight => (anchor_x, anchor_y),
    };

    let width = u32::try_from(next_width_i32).ok()?;
    let height = u32::try_from(next_height_i32).ok()?;
    if width < editor::tools::CROP_MIN_SIZE || height < editor::tools::CROP_MIN_SIZE {
        return None;
    }

    Some(CropElement::new(crop.id, x, y, width, height, crop.options))
}

pub(in crate::app) fn resize_object_from_handle(
    tools: &mut editor::EditorTools,
    object_id: u64,
    kind: ResizableObjectKind,
    handle: RectangleHandle,
    point: ToolPoint,
    image_width: i32,
    image_height: i32,
) -> bool {
    let image_bounds = ImageBounds::new(image_width, image_height);

    match kind {
        ResizableObjectKind::Rectangle => {
            let rectangle = match tools.object(object_id) {
                Some(ToolObject::Rectangle(rectangle)) => *rectangle,
                _ => return false,
            };
            let Some(bounds) =
                resized_rectangle_from_handle(&rectangle, handle, point, image_width, image_height)
            else {
                return false;
            };
            tools
                .resize_rectangle(object_id, bounds, image_bounds)
                .is_ok()
        }
        ResizableObjectKind::Blur => {
            let blur = match tools.object(object_id) {
                Some(ToolObject::Blur(blur)) => blur.clone(),
                _ => return false,
            };
            let Some(bounds) = resized_bounds_from_handle(
                ToolBounds::new(
                    blur.region.x,
                    blur.region.y,
                    blur.region.width,
                    blur.region.height,
                ),
                handle,
                point,
                image_bounds,
            ) else {
                return false;
            };
            tools.resize_blur(object_id, bounds, image_bounds).is_ok()
        }
        ResizableObjectKind::Crop => {
            let crop = match tools.object(object_id) {
                Some(ToolObject::Crop(crop)) => *crop,
                _ => return false,
            };
            let Some(next_crop) =
                resized_crop_from_handle(&crop, handle, point, image_width, image_height)
            else {
                return false;
            };
            tools
                .resize_crop(
                    object_id,
                    ToolBounds::new(next_crop.x, next_crop.y, next_crop.width, next_crop.height),
                    image_bounds,
                )
                .is_ok()
        }
    }
}

pub(in crate::app) fn draw_resize_handles_for_bounds(
    context: &gtk4::cairo::Context,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    color: RgbaColor,
) {
    for (_, corner) in corner_points_for_bounds(x, y, width, height) {
        let (red, green, blue, alpha) = color.to_cairo_rgba();
        context.save().ok();
        context.set_source_rgba(red, green, blue, alpha);
        context.rectangle(
            f64::from(corner.x.saturating_sub(4)),
            f64::from(corner.y.saturating_sub(4)),
            8.0,
            8.0,
        );
        let _ = context.fill();
        context.restore().ok();
    }
}

pub(in crate::app) const fn resize_status_label(kind: ResizableObjectKind) -> &'static str {
    match kind {
        ResizableObjectKind::Rectangle => "rectangle resized",
        ResizableObjectKind::Blur => "blur region resized",
        ResizableObjectKind::Crop => "crop frame resized",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_tool_box_rejects_zero_area() {
        let p = ToolPoint::new(10, 12);
        assert_eq!(normalize_tool_box(p, p), None);
    }

    #[test]
    fn adjust_ratio_to_fit_maintains_aspect_boundary() {
        assert_eq!(adjust_ratio_to_fit(400, 120, 16, 9), (213, 120));
        assert_eq!(adjust_ratio_to_fit(120, 400, 16, 9), (120, 67));
    }

    #[test]
    fn resized_bounds_from_handle_clamps_to_image_bounds() {
        let bounds = ToolBounds::new(10, 10, 100, 60);
        let resized = resized_bounds_from_handle(
            bounds,
            RectangleHandle::TopLeft,
            ToolPoint::new(-30, -20),
            ImageBounds::new(120, 80),
        )
        .expect("resized bounds expected");
        assert_eq!(resized, ToolBounds::new(0, 0, 110, 70));
    }

    #[test]
    fn resized_bounds_from_handle_rejects_tiny_results() {
        let bounds = ToolBounds::new(10, 10, 20, 20);
        let resized = resized_bounds_from_handle(
            bounds,
            RectangleHandle::TopLeft,
            ToolPoint::new(28, 28),
            ImageBounds::new(100, 100),
        );
        assert_eq!(resized, None);
    }

    #[test]
    fn caret_layout_to_canvas_cursor_rect_scales_to_canvas_space() {
        let caret = TextCaretLayout {
            caret_x: 100.0,
            caret_top: 40.0,
            caret_bottom: 60.0,
            baseline_y: 58.0,
            preedit_start_x: None,
            preedit_end_x: None,
        };
        let rect = caret_layout_to_canvas_cursor_rect(caret, 2000, 1000, 1000, 500);
        assert_eq!(rect, gtk4::gdk::Rectangle::new(200, 80, 2, 40));
    }

    #[test]
    fn text_dimensions_from_lines_uses_measured_widths() {
        let lines = vec!["abcd", "가가가가"];
        let (width, height) = text_dimensions_from_lines(&lines, 16.0, 20.0, |line| {
            if line.starts_with('가') {
                64.0
            } else {
                24.0
            }
        });
        assert_eq!(width, 64);
        assert_eq!(height, 40);
    }

    #[test]
    fn text_dimensions_with_multibyte_content_stays_wide_enough() {
        let mut options = crate::editor::tools::TextOptions::default();
        options.set_size(18);
        let text = TextElement::with_text(1, ToolPoint::new(4, 8), "가나다라마바사", options);
        let (width, height) = text_dimensions(&text);
        assert!(width > 8);
        assert!(height >= 18);
    }

    #[test]
    fn top_object_hit_test_prioritizes_non_blur_over_later_blur() {
        let mut tools = editor::EditorTools::new();
        let text_id = tools.add_text_box(ToolPoint::new(24, 24));
        let blur_id = tools
            .add_blur(editor::tools::BlurRegion::new(20, 20, 80, 60))
            .expect("blur should be inserted");

        assert_eq!(
            top_object_id_at_point(&tools, ToolPoint::new(28, 28)),
            Some(text_id)
        );
        assert_eq!(
            objects_in_draw_order(&tools)
                .map(ToolObject::id)
                .collect::<Vec<_>>(),
            vec![blur_id, text_id]
        );
    }
}

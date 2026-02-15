use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::editor;
use crate::editor::{ToolKind, ToolObject};
use image::{imageops, RgbaImage};

use super::{
    draw_resize_handles_for_bounds, is_object_selected, normalize_tool_box, object_bounds,
    objects_in_draw_order, text_baseline_y, text_line_height, text_lines_for_render,
    ArrowDrawStyle, BlurRenderCache, BlurRenderEntry, BlurRenderKey, EditorSelectionPalette,
    RgbaColor, TextCaretLayout, ToolDragPreview, ToolRenderContext,
};

impl BlurRenderCache {
    fn surface_for_blur(
        &mut self,
        source_pixbuf: &gtk4::gdk_pixbuf::Pixbuf,
        blur: &editor::tools::BlurElement,
    ) -> Option<(gtk4::cairo::ImageSurface, i32, i32, u32, u32)> {
        let (x, y, width, height) = bounded_region_for_blur(
            blur.region.x,
            blur.region.y,
            blur.region.width,
            blur.region.height,
            source_pixbuf.width(),
            source_pixbuf.height(),
        )?;
        let key = BlurRenderKey {
            source_width: source_pixbuf.width(),
            source_height: source_pixbuf.height(),
            x,
            y,
            width,
            height,
            intensity: blur.options.intensity,
        };
        if let Some(entry) = self.entries.get(&blur.id) {
            if entry.key == key {
                return Some((entry.surface.clone(), x, y, width, height));
            }
        }

        let region = pixbuf_region_to_rgba_image(source_pixbuf, x, y, width, height)?;
        let sigma = blur_sigma_from_intensity(blur.options.intensity);
        let blurred = blur_region_for_preview(&region, sigma);
        let surface = rgba_image_to_cairo_surface(&blurred)?;
        self.entries.insert(
            blur.id,
            BlurRenderEntry {
                key,
                surface: surface.clone(),
            },
        );
        Some((surface, x, y, width, height))
    }

    fn retain_visible_blurs(&mut self, visible_blur_ids: &[u64]) {
        let visible = visible_blur_ids.iter().copied().collect::<HashSet<_>>();
        self.entries
            .retain(|object_id, _| visible.contains(object_id));
    }
}

pub(in crate::app) fn preedit_cursor_char_index(preedit: &str, cursor_byte_index: i32) -> usize {
    let cursor_byte_index = usize::try_from(cursor_byte_index.max(0)).unwrap_or(usize::MAX);
    preedit
        .char_indices()
        .take_while(|(byte_index, _)| *byte_index < cursor_byte_index)
        .count()
}

fn measure_text_advance(context: &gtk4::cairo::Context, text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    context
        .text_extents(text)
        .map(|extents| extents.x_advance())
        .unwrap_or_else(|_| {
            let fallback = context
                .font_extents()
                .map(|extents| extents.max_x_advance().max(1.0) * 0.62)
                .unwrap_or(6.0);
            text.chars().count() as f64 * fallback
        })
}

fn text_content_with_preedit(
    text: &editor::tools::TextElement,
    preedit: &super::TextPreeditState,
) -> String {
    if preedit.content.is_empty() {
        return text.content.clone();
    }
    let cursor_chars = text.cursor_chars();
    let cursor_byte = text
        .content
        .char_indices()
        .nth(cursor_chars)
        .map(|(index, _)| index)
        .unwrap_or(text.content.len());
    let mut merged =
        String::with_capacity(text.content.len().saturating_add(preedit.content.len()));
    merged.push_str(&text.content[..cursor_byte]);
    merged.push_str(preedit.content.as_str());
    merged.push_str(&text.content[cursor_byte..]);
    merged
}

pub(in crate::app) fn text_caret_layout(
    context: &gtk4::cairo::Context,
    text: &editor::tools::TextElement,
    preedit: Option<&super::TextPreeditState>,
) -> TextCaretLayout {
    let line_height = text_line_height(text);
    let cursor_chars = text.cursor_chars();
    let mut line_index = 0_usize;
    let mut line_start_char_index = 0_usize;
    for (char_index, ch) in text.content.chars().enumerate() {
        if char_index >= cursor_chars {
            break;
        }
        if ch == '\n' {
            line_index = line_index.saturating_add(1);
            line_start_char_index = char_index.saturating_add(1);
        }
    }
    let cursor_column = cursor_chars.saturating_sub(line_start_char_index);
    let lines = text_lines_for_render(text);
    let line_content = lines.get(line_index).copied().unwrap_or("");
    let clamped_column = cursor_column.min(line_content.chars().count());
    let line_prefix = line_content
        .chars()
        .take(clamped_column)
        .collect::<String>();
    let baseline_y = text_baseline_y(text) + (line_index as f64 * line_height);
    let committed_width = measure_text_advance(context, line_prefix.as_str());
    let committed_caret_x = f64::from(text.x) + committed_width;
    let font_size = f64::from(text.options.size.max(1));
    let caret_top = baseline_y - font_size;
    let mut caret_x = committed_caret_x;
    let mut preedit_start_x = None;
    let mut preedit_end_x = None;

    if let Some(preedit) = preedit.filter(|preedit| !preedit.content.is_empty()) {
        let preedit_width = measure_text_advance(context, preedit.content.as_str());
        let cursor_chars = preedit.cursor_chars.min(preedit.content.chars().count());
        let cursor_prefix = preedit
            .content
            .chars()
            .take(cursor_chars)
            .collect::<String>();
        let cursor_width = measure_text_advance(context, cursor_prefix.as_str());
        preedit_start_x = Some(committed_caret_x);
        preedit_end_x = Some(committed_caret_x + preedit_width);
        caret_x = committed_caret_x + cursor_width;
    }

    TextCaretLayout {
        caret_x,
        caret_top,
        caret_bottom: caret_top + line_height,
        baseline_y,
        preedit_start_x,
        preedit_end_x,
    }
}

pub(in crate::app) fn set_source_rgb_u8(
    context: &gtk4::cairo::Context,
    r: u8,
    g: u8,
    b: u8,
    opacity_percent: u8,
) {
    let alpha = f64::from(opacity_percent) / 100.0;
    context.set_source_rgba(
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
        alpha.clamp(0.0, 1.0),
    );
}

fn set_source_rgba_color(context: &gtk4::cairo::Context, color: RgbaColor) {
    let (red, green, blue, alpha) = color.to_cairo_rgba();
    context.set_source_rgba(red, green, blue, alpha);
}

pub(in crate::app) fn draw_arrow_segment(
    context: &gtk4::cairo::Context,
    start: editor::tools::ToolPoint,
    end: editor::tools::ToolPoint,
    style: ArrowDrawStyle,
) {
    if start == end {
        return;
    }
    let dx = f64::from(end.x - start.x);
    let dy = f64::from(end.y - start.y);
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f64::EPSILON {
        return;
    }

    let ux = dx / length;
    let uy = dy / length;
    let backward_x = -ux;
    let backward_y = -uy;
    let stroke_width = f64::from(style.thickness.max(1));
    let max_head = (length * 0.7).max(stroke_width * 2.0);
    let head_length = f64::from(style.head_size.max(style.thickness.saturating_mul(2)))
        .max(stroke_width * 2.0)
        .min(max_head);

    let spread = 26.0_f64.to_radians();
    let (sin_spread, cos_spread) = spread.sin_cos();
    let left_dir_x = backward_x * cos_spread - backward_y * sin_spread;
    let left_dir_y = backward_x * sin_spread + backward_y * cos_spread;
    let right_dir_x = backward_x * cos_spread + backward_y * sin_spread;
    let right_dir_y = -backward_x * sin_spread + backward_y * cos_spread;
    let left_x = f64::from(end.x) + (left_dir_x * head_length);
    let left_y = f64::from(end.y) + (left_dir_y * head_length);
    let right_x = f64::from(end.x) + (right_dir_x * head_length);
    let right_y = f64::from(end.y) + (right_dir_y * head_length);

    set_source_rgb_u8(
        context,
        style.color_r,
        style.color_g,
        style.color_b,
        style.opacity_percent,
    );
    context.set_line_width(stroke_width);
    context.set_line_cap(gtk4::cairo::LineCap::Round);
    context.set_line_join(gtk4::cairo::LineJoin::Round);
    context.move_to(f64::from(start.x), f64::from(start.y));
    context.line_to(f64::from(end.x), f64::from(end.y));
    context.move_to(f64::from(end.x), f64::from(end.y));
    context.line_to(left_x, left_y);
    context.move_to(f64::from(end.x), f64::from(end.y));
    context.line_to(right_x, right_y);
    let _ = context.stroke();
}

fn effective_rectangle_corner_radius(width: u32, height: u32, border_radius: u16) -> f64 {
    let max_radius = f64::from(width.min(height)) / 2.0;
    f64::from(border_radius).clamp(0.0, max_radius)
}

fn append_rectangle_path(
    context: &gtk4::cairo::Context,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    border_radius: u16,
) {
    let radius = effective_rectangle_corner_radius(width, height, border_radius);
    let x = f64::from(x);
    let y = f64::from(y);
    let width = f64::from(width);
    let height = f64::from(height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    if radius <= 0.0 {
        context.rectangle(x, y, width, height);
        return;
    }

    let right = x + width;
    let bottom = y + height;
    context.new_sub_path();
    context.arc(
        right - radius,
        y + radius,
        radius,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    context.arc(
        right - radius,
        bottom - radius,
        radius,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    context.arc(
        x + radius,
        bottom - radius,
        radius,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    context.arc(
        x + radius,
        y + radius,
        radius,
        std::f64::consts::PI,
        std::f64::consts::PI * 1.5,
    );
    context.close_path();
}

pub(in crate::app) fn draw_crop_mask(
    context: &gtk4::cairo::Context,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    image_width: i32,
    image_height: i32,
) {
    let x = f64::from(x);
    let y = f64::from(y);
    let width = f64::from(width);
    let height = f64::from(height);
    let image_width = f64::from(image_width.max(1));
    let image_height = f64::from(image_height.max(1));

    context.set_source_rgba(0.0, 0.0, 0.0, 0.35);
    context.rectangle(0.0, 0.0, image_width, y.max(0.0));
    context.rectangle(
        0.0,
        y + height,
        image_width,
        (image_height - (y + height)).max(0.0),
    );
    context.rectangle(0.0, y, x.max(0.0), height.max(0.0));
    context.rectangle(
        x + width,
        y,
        (image_width - (x + width)).max(0.0),
        height.max(0.0),
    );
    let _ = context.fill();
}

pub(in crate::app) fn blur_sigma_from_intensity(intensity: u8) -> f32 {
    let normalized = (f32::from(intensity.clamp(1, 100)) - 1.0) / 99.0;
    1.0 + (normalized * 15.0)
}

pub(in crate::app) fn preview_blur_downsample_factor(width: u32, height: u32, sigma: f32) -> u32 {
    let area = width.saturating_mul(height);
    if area < 32_768 || sigma < 6.0 {
        return 1;
    }
    if area >= 262_144 && sigma >= 10.0 {
        return 4;
    }
    if area >= 65_536 && sigma >= 8.0 {
        return 3;
    }
    2
}

pub(in crate::app) fn blur_region_for_preview(region: &RgbaImage, sigma: f32) -> RgbaImage {
    let width = region.width();
    let height = region.height();
    let downsample = preview_blur_downsample_factor(width, height, sigma)
        .min(width.max(1))
        .min(height.max(1));
    if downsample <= 1 {
        return imageops::blur(region, sigma);
    }

    let reduced_width = (width / downsample).max(1);
    let reduced_height = (height / downsample).max(1);
    let reduced = imageops::resize(
        region,
        reduced_width,
        reduced_height,
        imageops::FilterType::Triangle,
    );
    let reduced_sigma = (sigma / downsample as f32).max(0.8);
    let blurred = imageops::blur(&reduced, reduced_sigma);
    imageops::resize(&blurred, width, height, imageops::FilterType::Triangle)
}

pub(in crate::app) fn bounded_region_for_blur(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    source_width: i32,
    source_height: i32,
) -> Option<(i32, i32, u32, u32)> {
    if width == 0 || height == 0 || source_width <= 0 || source_height <= 0 {
        return None;
    }

    let left = x.clamp(0, source_width.saturating_sub(1));
    let top = y.clamp(0, source_height.saturating_sub(1));
    let max_width = source_width.saturating_sub(left).max(1);
    let max_height = source_height.saturating_sub(top).max(1);
    let bounded_width = width.min(u32::try_from(max_width).unwrap_or(u32::MAX));
    let bounded_height = height.min(u32::try_from(max_height).unwrap_or(u32::MAX));

    if bounded_width == 0 || bounded_height == 0 {
        return None;
    }

    Some((left, top, bounded_width, bounded_height))
}

pub(in crate::app) fn pixbuf_region_to_rgba_image(
    source: &gtk4::gdk_pixbuf::Pixbuf,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Option<RgbaImage> {
    let rowstride = usize::try_from(source.rowstride()).ok()?;
    let n_channels = usize::try_from(source.n_channels()).ok()?;
    if n_channels != 3 && n_channels != 4 {
        return None;
    }

    let source_width = usize::try_from(source.width()).ok()?;
    let source_height = usize::try_from(source.height()).ok()?;
    let x = usize::try_from(x).ok()?;
    let y = usize::try_from(y).ok()?;
    let width = usize::try_from(width).ok()?;
    let height = usize::try_from(height).ok()?;
    if width == 0 || height == 0 || x >= source_width || y >= source_height {
        return None;
    }
    if x.checked_add(width)? > source_width || y.checked_add(height)? > source_height {
        return None;
    }

    let pixels = source.read_pixel_bytes();
    let bytes = pixels.as_ref();
    let dst_row_len = width.checked_mul(4)?;
    let mut rgba_bytes = vec![0_u8; dst_row_len.checked_mul(height)?];
    let src_row_len = width.checked_mul(n_channels)?;
    let src_x_offset = x.checked_mul(n_channels)?;

    for row in 0..height {
        let src_row_offset = y
            .checked_add(row)?
            .checked_mul(rowstride)?
            .checked_add(src_x_offset)?;
        let src_row_end = src_row_offset.checked_add(src_row_len)?;
        if src_row_end > bytes.len() {
            return None;
        }

        let dst_row_offset = row.checked_mul(dst_row_len)?;
        let dst_row_end = dst_row_offset.checked_add(dst_row_len)?;
        if dst_row_end > rgba_bytes.len() {
            return None;
        }

        let src_row = &bytes[src_row_offset..src_row_end];
        let dst_row = &mut rgba_bytes[dst_row_offset..dst_row_end];

        if n_channels == 4 {
            dst_row.copy_from_slice(src_row);
            continue;
        }

        for (src_pixel, dst_pixel) in src_row.chunks_exact(3).zip(dst_row.chunks_exact_mut(4)) {
            dst_pixel[0] = src_pixel[0];
            dst_pixel[1] = src_pixel[1];
            dst_pixel[2] = src_pixel[2];
            dst_pixel[3] = 255;
        }
    }

    let width = u32::try_from(width).ok()?;
    let height = u32::try_from(height).ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    RgbaImage::from_raw(width, height, rgba_bytes)
}

pub(in crate::app) fn rgba_image_to_cairo_surface(
    image: &RgbaImage,
) -> Option<gtk4::cairo::ImageSurface> {
    let width = i32::try_from(image.width()).ok()?;
    let height = i32::try_from(image.height()).ok()?;
    let mut surface =
        gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, width, height).ok()?;
    let stride = usize::try_from(surface.stride()).ok()?;

    {
        let mut data = surface.data().ok()?;
        let image_width = usize::try_from(image.width()).ok()?;
        let image_height = usize::try_from(image.height()).ok()?;
        let src_row_len = image_width.checked_mul(4)?;
        let src = image.as_raw();

        for row in 0..image_height {
            let src_row_offset = row.checked_mul(src_row_len)?;
            let src_row_end = src_row_offset.checked_add(src_row_len)?;
            if src_row_end > src.len() {
                return None;
            }

            let dst_row_offset = row.checked_mul(stride)?;
            let dst_row_end = dst_row_offset.checked_add(src_row_len)?;
            if dst_row_end > data.len() {
                return None;
            }

            let src_row = &src[src_row_offset..src_row_end];
            let dst_row = &mut data[dst_row_offset..dst_row_end];

            for (src_pixel, dst_pixel) in src_row.chunks_exact(4).zip(dst_row.chunks_exact_mut(4)) {
                let r = src_pixel[0];
                let g = src_pixel[1];
                let b = src_pixel[2];
                let a = src_pixel[3];
                match a {
                    0 => {
                        dst_pixel[0] = 0;
                        dst_pixel[1] = 0;
                        dst_pixel[2] = 0;
                        dst_pixel[3] = 0;
                    }
                    255 => {
                        dst_pixel[0] = b;
                        dst_pixel[1] = g;
                        dst_pixel[2] = r;
                        dst_pixel[3] = 255;
                    }
                    _ => {
                        let alpha = u16::from(a);
                        let premul_r = ((u16::from(r) * alpha + 127) / 255) as u8;
                        let premul_g = ((u16::from(g) * alpha + 127) / 255) as u8;
                        let premul_b = ((u16::from(b) * alpha + 127) / 255) as u8;
                        dst_pixel[0] = premul_b;
                        dst_pixel[1] = premul_g;
                        dst_pixel[2] = premul_r;
                        dst_pixel[3] = a;
                    }
                }
            }
        }
    }

    surface.flush();
    Some(surface)
}

pub(in crate::app) fn draw_real_blur_region(
    context: &gtk4::cairo::Context,
    source_pixbuf: &gtk4::gdk_pixbuf::Pixbuf,
    blur: &editor::tools::BlurElement,
) -> bool {
    let Some((x, y, width, height)) = bounded_region_for_blur(
        blur.region.x,
        blur.region.y,
        blur.region.width,
        blur.region.height,
        source_pixbuf.width(),
        source_pixbuf.height(),
    ) else {
        return false;
    };
    let Some(region) = pixbuf_region_to_rgba_image(source_pixbuf, x, y, width, height) else {
        return false;
    };
    let sigma = blur_sigma_from_intensity(blur.options.intensity);
    let blurred = imageops::blur(&region, sigma);
    let Some(surface) = rgba_image_to_cairo_surface(&blurred) else {
        return false;
    };

    if context
        .set_source_surface(&surface, f64::from(x), f64::from(y))
        .is_err()
    {
        return false;
    }
    context.rectangle(
        f64::from(x),
        f64::from(y),
        f64::from(width),
        f64::from(height),
    );
    let _ = context.fill();
    true
}

pub(in crate::app) fn draw_real_blur_region_with_cache(
    context: &gtk4::cairo::Context,
    source_pixbuf: &gtk4::gdk_pixbuf::Pixbuf,
    blur: &editor::tools::BlurElement,
    blur_cache: &Rc<RefCell<BlurRenderCache>>,
) -> bool {
    let Some((surface, x, y, width, height)) = blur_cache
        .borrow_mut()
        .surface_for_blur(source_pixbuf, blur)
    else {
        return false;
    };

    if context
        .set_source_surface(&surface, f64::from(x), f64::from(y))
        .is_err()
    {
        return false;
    }
    context.rectangle(
        f64::from(x),
        f64::from(y),
        f64::from(width),
        f64::from(height),
    );
    let _ = context.fill();
    true
}

pub(in crate::app) fn draw_editor_tool_objects(
    context: &gtk4::cairo::Context,
    tools: &editor::EditorTools,
    render: ToolRenderContext<'_>,
) {
    let mut last_crop: Option<(i32, i32, u32, u32)> = None;
    let mut visible_blur_ids: Vec<u64> = Vec::new();

    for object in objects_in_draw_order(tools) {
        let object_id = object.id();
        let is_selected = is_object_selected(render.selected_object_ids, object_id);

        match object {
            ToolObject::Blur(blur) => {
                visible_blur_ids.push(blur.id);
                let mut rendered = false;
                if let Some(source) = render.source_pixbuf {
                    rendered = if let Some(cache) = render.blur_cache {
                        draw_real_blur_region_with_cache(context, source, blur, cache)
                    } else {
                        draw_real_blur_region(context, source, blur)
                    };
                }
                if !rendered {
                    context.set_source_rgba(0.06, 0.12, 0.22, 0.26);
                    context.rectangle(
                        f64::from(blur.region.x),
                        f64::from(blur.region.y),
                        f64::from(blur.region.width),
                        f64::from(blur.region.height),
                    );
                    let _ = context.fill();
                }
            }
            ToolObject::Pen(stroke) => {
                if stroke.points.is_empty() {
                    continue;
                }
                set_source_rgb_u8(
                    context,
                    stroke.options.color_r,
                    stroke.options.color_g,
                    stroke.options.color_b,
                    stroke.options.opacity,
                );
                context.set_line_width(f64::from(stroke.options.thickness.max(1)));
                context.set_line_cap(gtk4::cairo::LineCap::Round);
                context.set_line_join(gtk4::cairo::LineJoin::Round);
                let first = stroke.points[0];
                context.move_to(f64::from(first.x), f64::from(first.y));
                for point in &stroke.points[1..] {
                    context.line_to(f64::from(point.x), f64::from(point.y));
                }
                let _ = context.stroke();
            }
            ToolObject::Arrow(arrow) => {
                draw_arrow_segment(
                    context,
                    arrow.start,
                    arrow.end,
                    super::ArrowDrawStyle {
                        color_r: arrow.options.color_r,
                        color_g: arrow.options.color_g,
                        color_b: arrow.options.color_b,
                        opacity_percent: 100,
                        thickness: arrow.options.thickness,
                        head_size: arrow.options.head_size,
                    },
                );
            }
            ToolObject::Rectangle(rectangle) => {
                if rectangle.options.fill_enabled {
                    set_source_rgb_u8(
                        context,
                        rectangle.options.color_r,
                        rectangle.options.color_g,
                        rectangle.options.color_b,
                        24,
                    );
                    append_rectangle_path(
                        context,
                        rectangle.x,
                        rectangle.y,
                        rectangle.width,
                        rectangle.height,
                        rectangle.options.border_radius,
                    );
                    let _ = context.fill();
                }
                set_source_rgb_u8(
                    context,
                    rectangle.options.color_r,
                    rectangle.options.color_g,
                    rectangle.options.color_b,
                    100,
                );
                context.set_line_width(f64::from(rectangle.options.thickness.max(1)));
                append_rectangle_path(
                    context,
                    rectangle.x,
                    rectangle.y,
                    rectangle.width,
                    rectangle.height,
                    rectangle.options.border_radius,
                );
                let _ = context.stroke();
            }
            ToolObject::Crop(crop) => {
                last_crop = Some((crop.x, crop.y, crop.width, crop.height));
                if render.show_crop_mask {
                    context.set_source_rgba(0.98, 0.98, 0.98, 0.92);
                    context.set_line_width(2.0);
                    context.rectangle(
                        f64::from(crop.x),
                        f64::from(crop.y),
                        f64::from(crop.width),
                        f64::from(crop.height),
                    );
                    let _ = context.stroke();
                }
            }
            ToolObject::Text(text) => {
                let active_preedit = if render.active_text_id == Some(object_id) {
                    render
                        .active_text_preedit
                        .filter(|preedit| !preedit.content.is_empty())
                } else {
                    None
                };
                let render_content =
                    active_preedit.map(|preedit| text_content_with_preedit(text, preedit));
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
                set_source_rgb_u8(
                    context,
                    text.options.color_r,
                    text.options.color_g,
                    text.options.color_b,
                    100,
                );
                let line_height = text_line_height(text);
                let baseline = text_baseline_y(text);
                let lines = match render_content.as_deref() {
                    Some("") => vec![""],
                    Some(content) => content.split('\n').collect::<Vec<_>>(),
                    None => text_lines_for_render(text),
                };
                for (index, line) in lines.iter().enumerate() {
                    if !line.is_empty() {
                        let line_y = baseline + (index as f64 * line_height);
                        context.move_to(f64::from(text.x), line_y);
                        let _ = context.show_text(line);
                    }
                }
                if render.active_text_id == Some(object_id) {
                    let preedit = active_preedit;
                    let caret = text_caret_layout(context, text, preedit);

                    if preedit.is_some() {
                        let preedit_start_x = caret.preedit_start_x.unwrap_or(caret.caret_x);
                        let preedit_end_x = caret.preedit_end_x.unwrap_or(preedit_start_x);
                        context.save().ok();
                        context.set_source_rgba(0.12, 0.34, 0.92, 0.92);
                        context.set_line_width(1.0);
                        context.move_to(preedit_start_x, caret.baseline_y + 2.0);
                        context.line_to(preedit_end_x, caret.baseline_y + 2.0);
                        let _ = context.stroke();
                        context.restore().ok();
                    }

                    context.save().ok();
                    context.set_source_rgba(0.14, 0.38, 1.0, 0.95);
                    context.set_line_width(1.4);
                    context.move_to(caret.caret_x, caret.caret_top);
                    context.line_to(caret.caret_x, caret.caret_bottom);
                    let _ = context.stroke();
                    context.restore().ok();
                }
            }
        }

        if is_selected {
            if let Some((x, y, width, height)) = object_bounds(object) {
                context.save().ok();
                set_source_rgba_color(context, render.selection_palette.selected_outline);
                context.set_line_width(1.5);
                context.set_dash(&[4.0, 3.0], 0.0);
                context.rectangle(
                    f64::from(x),
                    f64::from(y),
                    f64::from(width),
                    f64::from(height),
                );
                let _ = context.stroke();
                context.restore().ok();
            }
            match object {
                ToolObject::Rectangle(rectangle) => {
                    draw_resize_handles_for_bounds(
                        context,
                        rectangle.x,
                        rectangle.y,
                        rectangle.width,
                        rectangle.height,
                        render.selection_palette.resize_handle_fill,
                    );
                }
                ToolObject::Blur(blur) => {
                    draw_resize_handles_for_bounds(
                        context,
                        blur.region.x,
                        blur.region.y,
                        blur.region.width,
                        blur.region.height,
                        render.selection_palette.resize_handle_fill,
                    );
                }
                ToolObject::Crop(crop) => {
                    draw_resize_handles_for_bounds(
                        context,
                        crop.x,
                        crop.y,
                        crop.width,
                        crop.height,
                        render.selection_palette.resize_handle_fill,
                    );
                }
                _ => {}
            }
        }
    }

    if let Some(cache) = render.blur_cache {
        cache.borrow_mut().retain_visible_blurs(&visible_blur_ids);
    }

    if render.show_crop_mask {
        if let Some((x, y, width, height)) = last_crop {
            draw_crop_mask(
                context,
                x,
                y,
                width,
                height,
                render.image_bounds.width,
                render.image_bounds.height,
            );
            context.set_source_rgba(0.98, 0.98, 0.98, 0.95);
            context.set_line_width(2.0);
            context.rectangle(
                f64::from(x),
                f64::from(y),
                f64::from(width),
                f64::from(height),
            );
            let _ = context.stroke();
        }
    }
}

pub(in crate::app) fn draw_drag_preview_overlay(
    context: &gtk4::cairo::Context,
    preview: &ToolDragPreview,
    tools: &editor::EditorTools,
    image_width: i32,
    image_height: i32,
    selection_palette: EditorSelectionPalette,
) {
    match preview.tool {
        ToolKind::Select => {
            if let Some((x, y, width, height)) = normalize_tool_box(preview.start, preview.current)
            {
                context.save().ok();
                set_source_rgba_color(context, selection_palette.drag_fill);
                context.rectangle(
                    f64::from(x),
                    f64::from(y),
                    f64::from(width),
                    f64::from(height),
                );
                let _ = context.fill();
                set_source_rgba_color(context, selection_palette.drag_stroke);
                context.set_line_width(1.0);
                context.set_dash(&[4.0, 3.0], 0.0);
                context.rectangle(
                    f64::from(x),
                    f64::from(y),
                    f64::from(width),
                    f64::from(height),
                );
                let _ = context.stroke();
                context.restore().ok();
            }
        }
        ToolKind::Pan => {}
        ToolKind::Blur => {
            if let Some((x, y, width, height)) = normalize_tool_box(preview.start, preview.current)
            {
                context.set_source_rgba(0.06, 0.12, 0.22, 0.22);
                context.rectangle(
                    f64::from(x),
                    f64::from(y),
                    f64::from(width),
                    f64::from(height),
                );
                let _ = context.fill();
                context.set_source_rgba(0.88, 0.93, 1.0, 0.95);
                context.set_line_width(1.0);
                context.rectangle(
                    f64::from(x),
                    f64::from(y),
                    f64::from(width),
                    f64::from(height),
                );
                let _ = context.stroke();
            }
        }
        ToolKind::Pen => {}
        ToolKind::Arrow => {
            let options = tools.arrow_options();
            draw_arrow_segment(
                context,
                preview.start,
                preview.current,
                super::ArrowDrawStyle {
                    color_r: options.color_r,
                    color_g: options.color_g,
                    color_b: options.color_b,
                    opacity_percent: 100,
                    thickness: options.thickness,
                    head_size: options.head_size,
                },
            );
        }
        ToolKind::Rectangle => {
            if let Some((x, y, width, height)) = normalize_tool_box(preview.start, preview.current)
            {
                let options = tools.rectangle_options();
                if options.fill_enabled {
                    set_source_rgb_u8(
                        context,
                        options.color_r,
                        options.color_g,
                        options.color_b,
                        20,
                    );
                    append_rectangle_path(context, x, y, width, height, options.border_radius);
                    let _ = context.fill();
                }
                set_source_rgb_u8(
                    context,
                    options.color_r,
                    options.color_g,
                    options.color_b,
                    95,
                );
                context.set_line_width(f64::from(options.thickness.max(1)));
                append_rectangle_path(context, x, y, width, height, options.border_radius);
                let _ = context.stroke();
            }
        }
        ToolKind::Crop => {
            if let Some((x, y, width, height)) = normalize_tool_box(preview.start, preview.current)
            {
                draw_crop_mask(context, x, y, width, height, image_width, image_height);
                context.set_source_rgba(1.0, 1.0, 1.0, 0.95);
                context.set_line_width(2.0);
                context.rectangle(
                    f64::from(x),
                    f64::from(y),
                    f64::from(width),
                    f64::from(height),
                );
                let _ = context.stroke();
            }
        }
        ToolKind::Text => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_region_for_blur_clamps_to_source_dimensions() {
        let region = bounded_region_for_blur(-5, -10, 200, 120, 64, 48).expect("expected region");
        assert_eq!(region, (0, 0, 64, 48));
    }

    #[test]
    fn preview_blur_downsample_factor_uses_larger_factor_for_heavier_regions() {
        assert_eq!(preview_blur_downsample_factor(128, 128, 5.5), 1);
        assert_eq!(preview_blur_downsample_factor(320, 240, 7.0), 2);
        assert_eq!(preview_blur_downsample_factor(320, 240, 8.2), 3);
        assert_eq!(preview_blur_downsample_factor(900, 700, 10.0), 4);
    }

    #[test]
    fn rectangle_corner_radius_clamps_to_half_of_shorter_side() {
        let radius = effective_rectangle_corner_radius(20, 10, 999);
        assert_eq!(radius, 5.0);
    }

    #[test]
    fn rectangle_corner_radius_preserves_smaller_requested_value() {
        let radius = effective_rectangle_corner_radius(120, 80, 8);
        assert_eq!(radius, 8.0);
    }

    #[test]
    fn blur_region_for_preview_preserves_original_size() {
        let mut region = RgbaImage::new(96, 72);
        for pixel in region.pixels_mut() {
            *pixel = image::Rgba([180, 120, 50, 255]);
        }
        let blurred = blur_region_for_preview(&region, 11.0);
        assert_eq!(blurred.dimensions(), region.dimensions());
    }

    #[test]
    fn preedit_cursor_char_index_handles_multibyte_positions() {
        let text = "가나다abc";
        let idx = preedit_cursor_char_index(text, 6);
        assert_eq!(idx, 2);
    }

    #[test]
    fn text_caret_layout_tracks_preedit_cursor_with_measured_width() {
        let surface = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 256, 128)
            .expect("surface");
        let context = gtk4::cairo::Context::new(&surface).expect("context");
        let mut options = editor::tools::TextOptions::default();
        options.set_size(24);
        let text = editor::tools::TextElement::with_text(
            1,
            editor::tools::ToolPoint::new(12, 16),
            "ab",
            options,
        );
        context.select_font_face(
            text.options.family.cairo_font_name(),
            gtk4::cairo::FontSlant::Normal,
            gtk4::cairo::FontWeight::Normal,
        );
        context.set_font_size(f64::from(text.options.size.max(1)));

        let preedit = super::super::TextPreeditState {
            content: "가나".to_string(),
            cursor_chars: 1,
        };
        let layout = text_caret_layout(&context, &text, Some(&preedit));
        let committed_width = measure_text_advance(&context, "ab");
        let preedit_width = measure_text_advance(&context, "가나");
        let cursor_prefix_width = measure_text_advance(&context, "가");
        let preedit_start = layout.preedit_start_x.expect("preedit start");
        let preedit_end = layout.preedit_end_x.expect("preedit end");

        assert!((preedit_start - (12.0 + committed_width)).abs() < 0.01);
        assert!((preedit_end - (preedit_start + preedit_width)).abs() < 0.01);
        assert!((layout.caret_x - (preedit_start + cursor_prefix_width)).abs() < 0.01);
    }

    #[test]
    fn text_caret_layout_uses_committed_cursor_position_not_text_end() {
        let surface = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, 256, 128)
            .expect("surface");
        let context = gtk4::cairo::Context::new(&surface).expect("context");
        let mut options = editor::tools::TextOptions::default();
        options.set_size(24);
        let mut text = editor::tools::TextElement::with_text(
            1,
            editor::tools::ToolPoint::new(12, 16),
            "abcd",
            options,
        );
        context.select_font_face(
            text.options.family.cairo_font_name(),
            gtk4::cairo::FontSlant::Normal,
            gtk4::cairo::FontWeight::Normal,
        );
        context.set_font_size(f64::from(text.options.size.max(1)));

        let _ = text.move_cursor_left();
        let _ = text.move_cursor_left();
        let layout = text_caret_layout(&context, &text, None);
        let expected = 12.0 + measure_text_advance(&context, "ab");
        assert!((layout.caret_x - expected).abs() < 0.01);
    }

    #[test]
    fn text_content_with_preedit_inserts_inline_at_cursor() {
        let options = editor::tools::TextOptions::default();
        let mut text = editor::tools::TextElement::with_text(
            1,
            editor::tools::ToolPoint::new(0, 0),
            "hello",
            options,
        );
        let _ = text.move_cursor_left();
        let _ = text.move_cursor_left();
        let preedit = super::super::TextPreeditState {
            content: "가".to_string(),
            cursor_chars: 1,
        };
        let merged = text_content_with_preedit(&text, &preedit);
        assert_eq!(merged, "hel가lo");
    }
}

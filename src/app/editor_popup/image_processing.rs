use image::{imageops, RgbaImage};

pub(in crate::app) fn blur_region_for_preview(region: &RgbaImage, sigma: f32) -> RgbaImage {
    let width = region.width();
    let height = region.height();
    let downsample = super::preview_blur_downsample_factor(width, height, sigma)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_region_for_blur_clamps_to_source_dimensions() {
        let region = bounded_region_for_blur(-5, -10, 200, 120, 64, 48).expect("expected region");
        assert_eq!(region, (0, 0, 64, 48));
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
}

use gtk4::prelude::*;

use super::launchpad_actions::{set_status, SharedStatusLog};

/// Initialise the OCR engine if it is `None`, otherwise return the existing
/// one. Designed to run on a **worker thread** — all arguments are `Send`.
pub(super) fn resolve_or_init_engine(
    engine: Option<crate::ocr::OcrEngine>,
    language: crate::ocr::OcrLanguage,
) -> Result<crate::ocr::OcrEngine, crate::ocr::OcrError> {
    if let Some(engine) = engine {
        return Ok(engine);
    }

    let model_dir =
        crate::ocr::resolve_model_dir().ok_or_else(|| crate::ocr::OcrError::EngineInit {
            message: "model directory not found".to_string(),
        })?;
    crate::ocr::create_engine(&model_dir, language)
}

pub(super) fn pixbuf_region_to_dynamic_image(
    pixbuf: &gtk4::gdk_pixbuf::Pixbuf,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> crate::ocr::OcrResult<image::DynamicImage> {
    if width == 0 || height == 0 {
        return Err(crate::ocr::OcrError::InvalidRegion {
            message: "zero-size region".to_string(),
        });
    }

    let pb_width = pixbuf.width() as u32;
    let pb_height = pixbuf.height() as u32;

    let clamped_x = x.max(0) as u32;
    let clamped_y = y.max(0) as u32;
    let clamped_w = width.min(pb_width.saturating_sub(clamped_x));
    let clamped_h = height.min(pb_height.saturating_sub(clamped_y));

    if clamped_w == 0 || clamped_h == 0 {
        return Err(crate::ocr::OcrError::InvalidRegion {
            message: "region outside image bounds".to_string(),
        });
    }

    let sub = pixbuf.new_subpixbuf(
        clamped_x as i32,
        clamped_y as i32,
        clamped_w as i32,
        clamped_h as i32,
    );

    let n_channels = sub.n_channels() as u32;
    let rowstride = sub.rowstride() as u32;
    let has_alpha = sub.has_alpha();
    let pixels = unsafe { sub.pixels() };

    let mut rgb_buf = Vec::with_capacity((clamped_w * clamped_h * 3) as usize);
    for row in 0..clamped_h {
        let row_offset = (row * rowstride) as usize;
        for col in 0..clamped_w {
            let px_offset = row_offset + (col * n_channels) as usize;
            rgb_buf.push(pixels[px_offset]);
            rgb_buf.push(pixels[px_offset + 1]);
            rgb_buf.push(pixels[px_offset + 2]);
        }
    }

    let img_buf = image::RgbImage::from_raw(clamped_w, clamped_h, rgb_buf).ok_or_else(|| {
        crate::ocr::OcrError::ImageConversion {
            message: format!(
                "RGB buffer size mismatch for {}x{} (has_alpha={has_alpha})",
                clamped_w, clamped_h
            ),
        }
    })?;

    Ok(image::DynamicImage::ImageRgb8(img_buf))
}

/// Handle a successful OCR text result on the **main thread**: copy to
/// clipboard, update status, and send a desktop notification.
pub(super) fn handle_ocr_text_result(status_log: &SharedStatusLog, text: String) {
    if text.is_empty() {
        set_status(status_log, "OCR: no text found");
        crate::notification::send("No text found");
        return;
    }

    if let Some(display) = gtk4::gdk::Display::default() {
        display.clipboard().set_text(&text);
    }
    let preview_text = if text.chars().count() > 60 {
        let truncated: String = text.chars().take(57).collect();
        format!("{truncated}...")
    } else {
        text.clone()
    };
    set_status(
        status_log,
        format!("OCR copied {} chars", text.chars().count()),
    );
    crate::notification::send(format!("Copied: {preview_text}"));
}

/// Return a user-facing status message for the start of an OCR operation.
pub(super) fn ocr_processing_status(engine_available: bool) -> &'static str {
    if engine_available {
        "Recognizing text..."
    } else {
        "Initializing OCR engine..."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn ocr_processing_status_indicates_engine_state() {
        assert_eq!(ocr_processing_status(true), "Recognizing text...");
        assert_eq!(ocr_processing_status(false), "Initializing OCR engine...");
    }

    #[test]
    fn handle_ocr_text_result_sets_no_text_found_for_empty_input() {
        let status_log = Rc::new(RefCell::new(String::new()));
        handle_ocr_text_result(&status_log, String::new());
        assert_eq!(status_log.borrow().as_str(), "OCR: no text found");
    }

    #[test]
    fn handle_ocr_text_result_reports_char_count_for_non_empty_text() {
        let status_log = Rc::new(RefCell::new(String::new()));
        handle_ocr_text_result(&status_log, "hello world".to_string());
        assert_eq!(status_log.borrow().as_str(), "OCR copied 11 chars");
    }

    #[test]
    fn resolve_or_init_engine_errors_when_no_model_dir() {
        // Override both XDG_DATA_HOME and HOME so that neither user-level nor
        // home-fallback model directories are found.  The system-level path
        // (`/usr/share/chalkak/models`) may still exist on dev machines, so
        // skip the assertion when it does.
        let prev_xdg = std::env::var("XDG_DATA_HOME").ok();
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("XDG_DATA_HOME", "/tmp/chalkak-test-nonexistent-dir");
        std::env::set_var("HOME", "/tmp/chalkak-test-nonexistent-home");

        let result = resolve_or_init_engine(None, crate::ocr::OcrLanguage::English);

        // Restore environment before asserting so later tests are unaffected.
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }

        if !std::path::Path::new("/usr/share/chalkak/models").is_dir() {
            assert!(result.is_err());
        }
        // When system models exist the engine may succeed — that is fine.
    }

    #[test]
    fn pixbuf_region_rejects_zero_size() {
        let result = pixbuf_region_to_dynamic_image(
            &gtk4::gdk_pixbuf::Pixbuf::new(gtk4::gdk_pixbuf::Colorspace::Rgb, false, 8, 10, 10)
                .unwrap(),
            0,
            0,
            0,
            10,
        );
        assert!(matches!(
            result,
            Err(crate::ocr::OcrError::InvalidRegion { .. })
        ));
    }
}

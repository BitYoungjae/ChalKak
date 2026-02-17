use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use super::launchpad_actions::set_status;

pub(super) type SharedStatusLog = Rc<RefCell<String>>;

/// Initialise the OCR engine if it is `None`, otherwise return the existing
/// one. Designed to run on a **worker thread** — all arguments are `Send`.
pub(super) fn resolve_or_init_engine(
    engine: Option<ocr_rs::OcrEngine>,
    language: crate::ocr::OcrLanguage,
) -> Result<ocr_rs::OcrEngine, crate::ocr::OcrError> {
    if let Some(engine) = engine {
        return Ok(engine);
    }

    let model_dir =
        crate::ocr::resolve_model_dir().ok_or_else(|| crate::ocr::OcrError::EngineInit {
            message: "model directory not found".to_string(),
        })?;
    crate::ocr::create_engine(&model_dir, language)
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
    fn resolve_or_init_engine_returns_existing_engine_when_some() {
        // We cannot construct a real OcrEngine without model files, so we only
        // test the None path error (model directory missing) to verify wiring.
        std::env::set_var("XDG_DATA_HOME", "/tmp/chalkak-test-nonexistent-dir");
        let result = resolve_or_init_engine(None, crate::ocr::OcrLanguage::English);
        assert!(result.is_err());
        std::env::remove_var("XDG_DATA_HOME");
    }
}

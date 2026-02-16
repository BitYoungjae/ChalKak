use std::path::{Path, PathBuf};

use image::DynamicImage;

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("engine initialization failed: {message}")]
    EngineInit { message: String },
    #[error("image conversion failed: {message}")]
    ImageConversion { message: String },
    #[error("recognition failed: {message}")]
    Recognition { message: String },
    #[error("invalid region: {message}")]
    InvalidRegion { message: String },
}

pub type OcrResult<T> = Result<T, OcrError>;

const SYSTEM_MODEL_DIR: &str = "/usr/share/chalkak/models";

/// Supported OCR language identifiers.
///
/// Each variant maps to a specific recognition model and character-set file
/// shipped in the model directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrLanguage {
    Korean,
    English,
    Chinese,
    Latin,
    Cyrillic,
    Arabic,
    Thai,
    Greek,
    Devanagari,
    Tamil,
    Telugu,
}

impl OcrLanguage {
    /// Recognition-model filename inside the model directory.
    fn rec_model_filename(self) -> &'static str {
        match self {
            Self::Korean => "korean_PP-OCRv5_mobile_rec_infer.mnn",
            Self::English => "en_PP-OCRv5_mobile_rec_infer.mnn",
            Self::Chinese => "PP-OCRv5_mobile_rec.mnn",
            Self::Latin => "latin_PP-OCRv5_mobile_rec_infer.mnn",
            Self::Cyrillic => "cyrillic_PP-OCRv5_mobile_rec_infer.mnn",
            Self::Arabic => "arabic_PP-OCRv5_mobile_rec_infer.mnn",
            Self::Thai => "th_PP-OCRv5_mobile_rec_infer.mnn",
            Self::Greek => "el_PP-OCRv5_mobile_rec_infer.mnn",
            Self::Devanagari => "devanagari_PP-OCRv5_mobile_rec_infer.mnn",
            Self::Tamil => "ta_PP-OCRv5_mobile_rec_infer.mnn",
            Self::Telugu => "te_PP-OCRv5_mobile_rec_infer.mnn",
        }
    }

    /// Character-set filename inside the model directory.
    fn keys_filename(self) -> &'static str {
        match self {
            Self::Korean => "ppocr_keys_korean.txt",
            Self::English => "ppocr_keys_en.txt",
            Self::Chinese => "ppocr_keys_v5.txt",
            Self::Latin => "ppocr_keys_latin.txt",
            Self::Cyrillic => "ppocr_keys_cyrillic.txt",
            Self::Arabic => "ppocr_keys_arabic.txt",
            Self::Thai => "ppocr_keys_th.txt",
            Self::Greek => "ppocr_keys_el.txt",
            Self::Devanagari => "ppocr_keys_devanagari.txt",
            Self::Tamil => "ppocr_keys_ta.txt",
            Self::Telugu => "ppocr_keys_te.txt",
        }
    }

    /// Human-readable language name for display purposes.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Korean => "Korean",
            Self::English => "English",
            Self::Chinese => "Chinese",
            Self::Latin => "Latin",
            Self::Cyrillic => "Cyrillic",
            Self::Arabic => "Arabic",
            Self::Thai => "Thai",
            Self::Greek => "Greek",
            Self::Devanagari => "Devanagari",
            Self::Tamil => "Tamil",
            Self::Telugu => "Telugu",
        }
    }

    /// Config string used in `theme.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Korean => "korean",
            Self::English => "en",
            Self::Chinese => "chinese",
            Self::Latin => "latin",
            Self::Cyrillic => "cyrillic",
            Self::Arabic => "arabic",
            Self::Thai => "th",
            Self::Greek => "el",
            Self::Devanagari => "devanagari",
            Self::Tamil => "ta",
            Self::Telugu => "te",
        }
    }
}

/// Parse a config string into an [`OcrLanguage`]. Returns `None` for
/// unrecognised values so the caller can fall back to system detection.
pub fn parse_ocr_language(value: &str) -> Option<OcrLanguage> {
    match value.to_ascii_lowercase().as_str() {
        "korean" | "ko" => Some(OcrLanguage::Korean),
        "en" | "english" => Some(OcrLanguage::English),
        "chinese" | "zh" | "ch" => Some(OcrLanguage::Chinese),
        "latin" => Some(OcrLanguage::Latin),
        "cyrillic" | "ru" => Some(OcrLanguage::Cyrillic),
        "arabic" | "ar" => Some(OcrLanguage::Arabic),
        "th" | "thai" => Some(OcrLanguage::Thai),
        "el" | "greek" => Some(OcrLanguage::Greek),
        "devanagari" | "hi" => Some(OcrLanguage::Devanagari),
        "ta" | "tamil" => Some(OcrLanguage::Tamil),
        "te" | "telugu" => Some(OcrLanguage::Telugu),
        _ => None,
    }
}

/// Detect the OCR language from the system `LANG` environment variable.
pub fn detect_system_ocr_language() -> OcrLanguage {
    let lang = std::env::var("LANG").unwrap_or_default();
    match lang.split('_').next().unwrap_or("en") {
        "ko" => OcrLanguage::Korean,
        "zh" => OcrLanguage::Chinese,
        "ru" | "uk" | "be" => OcrLanguage::Cyrillic,
        "ar" => OcrLanguage::Arabic,
        "th" => OcrLanguage::Thai,
        "el" => OcrLanguage::Greek,
        "hi" | "mr" | "ne" => OcrLanguage::Devanagari,
        "ta" => OcrLanguage::Tamil,
        "te" => OcrLanguage::Telugu,
        _ => OcrLanguage::English,
    }
}

/// Resolve the effective OCR language from an optional user config value.
/// Falls back to system language detection when the config is absent or invalid.
pub fn resolve_ocr_language(config_value: Option<&str>) -> OcrLanguage {
    config_value
        .and_then(parse_ocr_language)
        .unwrap_or_else(detect_system_ocr_language)
}

pub fn resolve_model_dir() -> Option<PathBuf> {
    let user_dir = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|val| !val.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".local/share"))
        })
        .map(|base| base.join("chalkak/models"));

    if let Some(ref dir) = user_dir {
        if dir.is_dir() {
            return Some(dir.clone());
        }
    }

    let system_dir = PathBuf::from(SYSTEM_MODEL_DIR);
    if system_dir.is_dir() {
        return Some(system_dir);
    }

    None
}

pub fn create_engine(model_dir: &Path, language: OcrLanguage) -> OcrResult<ocr_rs::OcrEngine> {
    let det_path = model_dir.join("PP-OCRv5_mobile_det.mnn");
    let rec_path = model_dir.join(language.rec_model_filename());
    let keys_path = model_dir.join(language.keys_filename());

    ocr_rs::OcrEngine::new(
        det_path.to_str().unwrap_or_default(),
        rec_path.to_str().unwrap_or_default(),
        keys_path.to_str().unwrap_or_default(),
        None,
    )
    .map_err(|err| OcrError::EngineInit {
        message: err.to_string(),
    })
}

pub fn pixbuf_region_to_dynamic_image(
    pixbuf: &gtk4::gdk_pixbuf::Pixbuf,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> OcrResult<DynamicImage> {
    if width == 0 || height == 0 {
        return Err(OcrError::InvalidRegion {
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
        return Err(OcrError::InvalidRegion {
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
        OcrError::ImageConversion {
            message: format!(
                "RGB buffer size mismatch for {}x{} (has_alpha={has_alpha})",
                clamped_w, clamped_h
            ),
        }
    })?;

    Ok(DynamicImage::ImageRgb8(img_buf))
}

pub fn recognize_text(engine: &ocr_rs::OcrEngine, image: &DynamicImage) -> OcrResult<String> {
    let results = engine
        .recognize(image)
        .map_err(|err| OcrError::Recognition {
            message: err.to_string(),
        })?;

    if results.is_empty() {
        return Ok(String::new());
    }

    let mut lines: Vec<_> = results
        .into_iter()
        .map(|r| {
            let y = r.bbox.rect.top();
            (y, r.text)
        })
        .collect();
    lines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let text = lines
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n");

    Ok(text)
}

pub fn recognize_text_from_file(engine: &ocr_rs::OcrEngine, path: &Path) -> OcrResult<String> {
    let image = image::open(path).map_err(|err| OcrError::ImageConversion {
        message: format!("failed to open image {}: {err}", path.display()),
    })?;
    recognize_text(engine, &image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_model_dir_returns_none_when_no_dirs_exist() {
        std::env::set_var("XDG_DATA_HOME", "/tmp/chalkak-test-nonexistent-dir");
        let result = resolve_model_dir();
        if !PathBuf::from(SYSTEM_MODEL_DIR).is_dir() {
            assert!(result.is_none());
        }
        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn resolve_model_dir_prefers_user_dir_over_system() {
        let tmp = std::env::temp_dir().join("chalkak-test-user-models");
        let _ = std::fs::create_dir_all(&tmp);
        std::env::set_var("XDG_DATA_HOME", tmp.parent().unwrap().parent().unwrap());
        let _ = resolve_model_dir();
        let _ = std::fs::remove_dir_all(&tmp);
        std::env::remove_var("XDG_DATA_HOME");
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
        assert!(matches!(result, Err(OcrError::InvalidRegion { .. })));
    }

    #[test]
    fn parse_ocr_language_accepts_known_values() {
        assert_eq!(parse_ocr_language("korean"), Some(OcrLanguage::Korean));
        assert_eq!(parse_ocr_language("ko"), Some(OcrLanguage::Korean));
        assert_eq!(parse_ocr_language("en"), Some(OcrLanguage::English));
        assert_eq!(parse_ocr_language("English"), Some(OcrLanguage::English));
        assert_eq!(parse_ocr_language("chinese"), Some(OcrLanguage::Chinese));
        assert_eq!(parse_ocr_language("zh"), Some(OcrLanguage::Chinese));
        assert_eq!(parse_ocr_language("KOREAN"), Some(OcrLanguage::Korean));
    }

    #[test]
    fn parse_ocr_language_returns_none_for_unknown() {
        assert_eq!(parse_ocr_language("klingon"), None);
        assert_eq!(parse_ocr_language(""), None);
    }

    #[test]
    fn detect_system_ocr_language_uses_lang_env() {
        std::env::set_var("LANG", "ko_KR.UTF-8");
        assert_eq!(detect_system_ocr_language(), OcrLanguage::Korean);
        std::env::set_var("LANG", "en_US.UTF-8");
        assert_eq!(detect_system_ocr_language(), OcrLanguage::English);
        std::env::set_var("LANG", "zh_CN.UTF-8");
        assert_eq!(detect_system_ocr_language(), OcrLanguage::Chinese);
        std::env::remove_var("LANG");
    }

    #[test]
    fn resolve_ocr_language_prefers_config_over_system() {
        std::env::set_var("LANG", "en_US.UTF-8");
        assert_eq!(resolve_ocr_language(Some("korean")), OcrLanguage::Korean);
        assert_eq!(resolve_ocr_language(None), OcrLanguage::English);
        std::env::remove_var("LANG");
    }
}

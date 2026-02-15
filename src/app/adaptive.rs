use crate::theme::ThemeMode;

const BASELINE_PRESET_LONG_EDGE: f64 = 1920.0;

pub(super) const TEXT_SIZE_PRESETS: [u8; 6] = [16, 20, 24, 32, 40, 56];
pub(super) const STROKE_SIZE_PRESETS: [u8; 6] = [2, 3, 4, 6, 8, 12];
pub(super) const STROKE_SIZE_BUTTON_PRESETS: [u8; 3] = [4, 8, 12];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StrokeColorPreset {
    pub(super) label: String,
    color_r: u8,
    color_g: u8,
    color_b: u8,
}

impl StrokeColorPreset {
    pub(super) fn new(label: impl Into<String>, color_r: u8, color_g: u8, color_b: u8) -> Self {
        Self {
            label: label.into(),
            color_r,
            color_g,
            color_b,
        }
    }

    pub(super) fn rgb(&self) -> (u8, u8, u8) {
        (self.color_r, self.color_g, self.color_b)
    }
}

#[derive(Debug, Clone)]
pub(super) struct StrokeColorPalette {
    presets: Vec<StrokeColorPreset>,
    default_index: usize,
}

impl StrokeColorPalette {
    pub(super) fn new(mut presets: Vec<StrokeColorPreset>, default_index: usize) -> Self {
        if presets.is_empty() {
            presets.push(StrokeColorPreset::new("Black", 18, 18, 18));
        }
        Self {
            presets,
            default_index,
        }
    }

    pub(super) fn presets(&self) -> &[StrokeColorPreset] {
        self.presets.as_slice()
    }

    pub(super) fn default_index(&self) -> usize {
        self.default_index.min(self.presets.len().saturating_sub(1))
    }

    pub(super) fn color_for_index(&self, index: usize) -> Option<(u8, u8, u8)> {
        self.presets.get(index).map(StrokeColorPreset::rgb)
    }

    pub(super) fn default_color(&self) -> (u8, u8, u8) {
        self.color_for_index(self.default_index())
            .unwrap_or((18, 18, 18))
    }
}

#[derive(Debug, Clone)]
pub(super) struct EditorToolOptionPresets {
    stroke_color_palette: StrokeColorPalette,
    stroke_width_presets: Vec<u8>,
    adaptive_stroke_width_presets: Vec<u8>,
    text_size_presets: Vec<u8>,
}

impl EditorToolOptionPresets {
    pub(super) fn defaults_for_theme(mode: ThemeMode) -> Self {
        Self {
            stroke_color_palette: stroke_color_palette_for_theme(mode),
            stroke_width_presets: STROKE_SIZE_BUTTON_PRESETS.to_vec(),
            adaptive_stroke_width_presets: STROKE_SIZE_PRESETS.to_vec(),
            text_size_presets: TEXT_SIZE_PRESETS.to_vec(),
        }
    }

    pub(super) fn with_overrides(
        mode: ThemeMode,
        stroke_color_palette_override: Option<Vec<StrokeColorPreset>>,
        stroke_width_presets_override: Option<Vec<u8>>,
        text_size_presets_override: Option<Vec<u8>>,
    ) -> Self {
        let mut presets = Self::defaults_for_theme(mode);
        if let Some(color_presets) = stroke_color_palette_override {
            if !color_presets.is_empty() {
                presets.stroke_color_palette = StrokeColorPalette::new(color_presets, 0);
            }
        }
        if let Some(stroke_widths) = stroke_width_presets_override {
            if !stroke_widths.is_empty() {
                presets.adaptive_stroke_width_presets = stroke_widths.clone();
                presets.stroke_width_presets = stroke_widths;
            }
        }
        if let Some(text_sizes) = text_size_presets_override {
            if !text_sizes.is_empty() {
                presets.text_size_presets = text_sizes;
            }
        }
        presets
    }

    pub(super) fn stroke_color_palette(&self) -> &StrokeColorPalette {
        &self.stroke_color_palette
    }

    pub(super) fn stroke_width_presets(&self) -> &[u8] {
        self.stroke_width_presets.as_slice()
    }

    pub(super) fn adaptive_stroke_width_presets(&self) -> &[u8] {
        self.adaptive_stroke_width_presets.as_slice()
    }

    pub(super) fn text_size_presets(&self) -> &[u8] {
        self.text_size_presets.as_slice()
    }
}

const LIGHT_STROKE_COLOR_PRESETS: [(&str, u8, u8, u8); 6] = [
    ("Black", 18, 18, 18),
    ("Red", 225, 64, 56),
    ("Orange", 255, 149, 0),
    ("Yellow", 255, 211, 51),
    ("Blue", 38, 125, 255),
    ("Green", 58, 179, 88),
];

const DARK_STROKE_COLOR_PRESETS: [(&str, u8, u8, u8); 6] = [
    ("White", 240, 242, 248),
    ("Red", 255, 110, 104),
    ("Orange", 255, 180, 76),
    ("Yellow", 255, 223, 120),
    ("Blue", 118, 170, 255),
    ("Green", 108, 214, 146),
];

const LIGHT_STROKE_DEFAULT_INDEX: usize = 0;
const DARK_STROKE_DEFAULT_INDEX: usize = 0;

fn color_palette_defaults_for_theme(
    mode: ThemeMode,
) -> (&'static [(&'static str, u8, u8, u8)], usize) {
    match mode {
        ThemeMode::Light => (&LIGHT_STROKE_COLOR_PRESETS, LIGHT_STROKE_DEFAULT_INDEX),
        ThemeMode::Dark | ThemeMode::System => {
            (&DARK_STROKE_COLOR_PRESETS, DARK_STROKE_DEFAULT_INDEX)
        }
    }
}

fn build_stroke_color_presets(raw: &[(&str, u8, u8, u8)]) -> Vec<StrokeColorPreset> {
    raw.iter()
        .map(|(label, red, green, blue)| StrokeColorPreset::new(*label, *red, *green, *blue))
        .collect()
}

pub(super) fn stroke_color_palette_for_theme(mode: ThemeMode) -> StrokeColorPalette {
    let (raw_presets, default_index) = color_palette_defaults_for_theme(mode);
    StrokeColorPalette::new(build_stroke_color_presets(raw_presets), default_index)
}

pub(super) fn nearest_preset_u8(target: f64, presets: &[u8]) -> u8 {
    presets
        .iter()
        .copied()
        .min_by(|left, right| {
            let left_delta = (f64::from(*left) - target).abs();
            let right_delta = (f64::from(*right) - target).abs();
            left_delta
                .partial_cmp(&right_delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or_else(|| presets.first().copied().unwrap_or(1))
}

pub(super) fn adaptive_text_size_for_image_with_presets(
    image_width: i32,
    image_height: i32,
    presets: &[u8],
) -> u8 {
    let long_edge = f64::from(image_width.max(image_height).max(1));
    let scale = (long_edge / BASELINE_PRESET_LONG_EDGE).clamp(1.0, 3.5);
    nearest_preset_u8(16.0 * scale, presets)
}

pub(super) fn adaptive_stroke_size_for_image_with_presets(
    image_width: i32,
    image_height: i32,
    presets: &[u8],
) -> u8 {
    let long_edge = f64::from(image_width.max(image_height).max(1));
    let scale = (long_edge / BASELINE_PRESET_LONG_EDGE).clamp(1.0, 3.5);
    nearest_preset_u8(3.0 * scale, presets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stroke_color_palette_differs_between_light_and_dark_modes() {
        let light = stroke_color_palette_for_theme(ThemeMode::Light);
        let dark = stroke_color_palette_for_theme(ThemeMode::Dark);

        assert_ne!(light.default_color(), dark.default_color());
        assert_ne!(light.presets(), dark.presets());
    }

    #[test]
    fn stroke_color_palette_round_trips_index_and_color() {
        let palette = stroke_color_palette_for_theme(ThemeMode::Dark);
        for (index, preset) in palette.presets().iter().enumerate() {
            assert_eq!(palette.color_for_index(index), Some(preset.rgb()));
            assert_eq!(
                palette
                    .presets()
                    .iter()
                    .position(|candidate| candidate.rgb() == preset.rgb()),
                Some(index)
            );
        }
    }

    #[test]
    fn stroke_color_palette_maps_system_to_dark_palette() {
        let system = stroke_color_palette_for_theme(ThemeMode::System);
        let dark = stroke_color_palette_for_theme(ThemeMode::Dark);

        assert_eq!(system.default_color(), dark.default_color());
        assert_eq!(system.presets(), dark.presets());
    }

    #[test]
    fn editor_tool_option_presets_apply_overrides() {
        let presets = EditorToolOptionPresets::with_overrides(
            ThemeMode::Light,
            Some(vec![
                StrokeColorPreset::new("#111111", 0x11, 0x11, 0x11),
                StrokeColorPreset::new("#223344", 0x22, 0x33, 0x44),
            ]),
            Some(vec![2, 6, 10]),
            Some(vec![14, 18, 28]),
        );

        assert_eq!(presets.stroke_color_palette().presets().len(), 2);
        assert_eq!(
            presets.stroke_color_palette().default_color(),
            (0x11, 0x11, 0x11)
        );
        assert_eq!(presets.stroke_width_presets(), [2, 6, 10]);
        assert_eq!(presets.adaptive_stroke_width_presets(), [2, 6, 10]);
        assert_eq!(presets.text_size_presets(), [14, 18, 28]);
    }

    #[test]
    fn default_stroke_adaptive_presets_keep_legacy_scale_ladder() {
        let presets = EditorToolOptionPresets::defaults_for_theme(ThemeMode::Dark);
        assert_eq!(presets.stroke_width_presets(), [4, 8, 12]);
        assert_eq!(presets.adaptive_stroke_width_presets(), [2, 3, 4, 6, 8, 12]);
        assert_eq!(
            adaptive_stroke_size_for_image_with_presets(
                1920,
                1080,
                presets.adaptive_stroke_width_presets()
            ),
            3
        );
    }
}

use crate::theme::ThemeMode;

const BASELINE_PRESET_LONG_EDGE: f64 = 1920.0;

pub(super) const TEXT_SIZE_PRESETS: [u8; 6] = [16, 20, 24, 32, 40, 56];
pub(super) const STROKE_SIZE_PRESETS: [u8; 6] = [2, 3, 4, 6, 8, 12];
pub(super) const STROKE_SIZE_BUTTON_PRESETS: [u8; 3] = [4, 8, 12];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StrokeColorPreset {
    pub(super) label: &'static str,
    color_r: u8,
    color_g: u8,
    color_b: u8,
}

impl StrokeColorPreset {
    const fn new(label: &'static str, color_r: u8, color_g: u8, color_b: u8) -> Self {
        Self {
            label,
            color_r,
            color_g,
            color_b,
        }
    }

    pub(super) const fn rgb(self) -> (u8, u8, u8) {
        (self.color_r, self.color_g, self.color_b)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct StrokeColorPalette {
    presets: &'static [StrokeColorPreset],
    default_index: usize,
}

impl StrokeColorPalette {
    pub(super) fn presets(self) -> &'static [StrokeColorPreset] {
        self.presets
    }

    pub(super) fn default_index(self) -> usize {
        self.default_index.min(self.presets.len().saturating_sub(1))
    }

    pub(super) fn color_for_index(self, index: usize) -> Option<(u8, u8, u8)> {
        self.presets.get(index).copied().map(StrokeColorPreset::rgb)
    }

    pub(super) fn default_color(self) -> (u8, u8, u8) {
        self.color_for_index(self.default_index())
            .unwrap_or((18, 18, 18))
    }
}

const LIGHT_STROKE_COLOR_PRESETS: [StrokeColorPreset; 6] = [
    StrokeColorPreset::new("Black", 18, 18, 18),
    StrokeColorPreset::new("Red", 225, 64, 56),
    StrokeColorPreset::new("Orange", 255, 149, 0),
    StrokeColorPreset::new("Yellow", 255, 211, 51),
    StrokeColorPreset::new("Blue", 38, 125, 255),
    StrokeColorPreset::new("Green", 58, 179, 88),
];

const DARK_STROKE_COLOR_PRESETS: [StrokeColorPreset; 6] = [
    StrokeColorPreset::new("White", 240, 242, 248),
    StrokeColorPreset::new("Red", 255, 110, 104),
    StrokeColorPreset::new("Orange", 255, 180, 76),
    StrokeColorPreset::new("Yellow", 255, 223, 120),
    StrokeColorPreset::new("Blue", 118, 170, 255),
    StrokeColorPreset::new("Green", 108, 214, 146),
];

const LIGHT_STROKE_DEFAULT_INDEX: usize = 0;
const DARK_STROKE_DEFAULT_INDEX: usize = 0;

pub(super) fn stroke_color_palette_for_theme(mode: ThemeMode) -> StrokeColorPalette {
    match mode {
        ThemeMode::Light => StrokeColorPalette {
            presets: &LIGHT_STROKE_COLOR_PRESETS,
            default_index: LIGHT_STROKE_DEFAULT_INDEX,
        },
        ThemeMode::Dark | ThemeMode::System => StrokeColorPalette {
            presets: &DARK_STROKE_COLOR_PRESETS,
            default_index: DARK_STROKE_DEFAULT_INDEX,
        },
    }
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

pub(super) fn adaptive_text_size_for_image(image_width: i32, image_height: i32) -> u8 {
    let long_edge = f64::from(image_width.max(image_height).max(1));
    let scale = (long_edge / BASELINE_PRESET_LONG_EDGE).clamp(1.0, 3.5);
    nearest_preset_u8(16.0 * scale, &TEXT_SIZE_PRESETS)
}

pub(super) fn adaptive_stroke_size_for_image(image_width: i32, image_height: i32) -> u8 {
    let long_edge = f64::from(image_width.max(image_height).max(1));
    let scale = (long_edge / BASELINE_PRESET_LONG_EDGE).clamp(1.0, 3.5);
    nearest_preset_u8(3.0 * scale, &STROKE_SIZE_PRESETS)
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
}

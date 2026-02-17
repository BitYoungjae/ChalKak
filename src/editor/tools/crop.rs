#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropPreset {
    Free,
    Ratio16x9,
    Ratio1x1,
    Ratio9x16,
    Original,
}

impl CropPreset {
    pub const ALL: [CropPreset; 5] = [
        Self::Free,
        Self::Ratio16x9,
        Self::Ratio1x1,
        Self::Ratio9x16,
        Self::Original,
    ];

    pub const fn is_free(self) -> bool {
        matches!(self, Self::Free)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Ratio16x9 => "16:9",
            Self::Ratio1x1 => "1:1",
            Self::Ratio9x16 => "9:16",
            Self::Original => "Original",
        }
    }

    pub const fn ratio(self) -> Option<(u32, u32)> {
        match self {
            Self::Free => None,
            Self::Ratio16x9 => Some((16, 9)),
            Self::Ratio1x1 => Some((1, 1)),
            Self::Ratio9x16 => Some((9, 16)),
            Self::Original => None,
        }
    }

    /// Returns the effective aspect ratio for this preset.
    ///
    /// For fixed-ratio presets this returns the static ratio.
    /// For `Original` this returns the image dimensions as the ratio.
    /// For `Free` this returns `None`.
    pub fn resolve_ratio(self, image_width: u32, image_height: u32) -> Option<(u32, u32)> {
        self.ratio().or_else(|| {
            if self == Self::Original {
                Some((image_width.max(1), image_height.max(1)))
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropOptions {
    pub preset: CropPreset,
}

impl Default for CropOptions {
    fn default() -> Self {
        Self {
            preset: CropPreset::Free,
        }
    }
}

impl CropOptions {
    pub fn set_preset(&mut self, preset: CropPreset) {
        self.preset = preset;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropElement {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub options: CropOptions,
}

impl CropElement {
    pub const fn new(
        id: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        options: CropOptions,
    ) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
            options,
        }
    }

    pub const fn supports_corner_handles_only(&self) -> bool {
        !self.options.preset.is_free()
    }
}

pub const CROP_MIN_SIZE: u32 = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_preset_label_returns_expected_strings() {
        assert_eq!(CropPreset::Free.label(), "Free");
        assert_eq!(CropPreset::Ratio16x9.label(), "16:9");
        assert_eq!(CropPreset::Ratio1x1.label(), "1:1");
        assert_eq!(CropPreset::Ratio9x16.label(), "9:16");
        assert_eq!(CropPreset::Original.label(), "Original");
    }

    #[test]
    fn crop_preset_all_contains_every_unique_variant() {
        let expected = [
            CropPreset::Free,
            CropPreset::Ratio16x9,
            CropPreset::Ratio1x1,
            CropPreset::Ratio9x16,
            CropPreset::Original,
        ];
        assert_eq!(CropPreset::ALL.len(), expected.len());
        for variant in &expected {
            assert!(
                CropPreset::ALL.contains(variant),
                "ALL is missing {variant:?}"
            );
        }
        // Verify no duplicates
        for (i, a) in CropPreset::ALL.iter().enumerate() {
            for (j, b) in CropPreset::ALL.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "ALL has duplicate at indices {i} and {j}");
                }
            }
        }
    }

    #[test]
    fn resolve_ratio_returns_static_ratio_for_fixed_presets() {
        assert_eq!(CropPreset::Ratio16x9.resolve_ratio(800, 600), Some((16, 9)));
        assert_eq!(CropPreset::Ratio1x1.resolve_ratio(800, 600), Some((1, 1)));
        assert_eq!(CropPreset::Ratio9x16.resolve_ratio(800, 600), Some((9, 16)));
    }

    #[test]
    fn resolve_ratio_returns_image_dims_for_original() {
        assert_eq!(
            CropPreset::Original.resolve_ratio(1920, 1080),
            Some((1920, 1080))
        );
        // Zero dimensions are clamped to 1
        assert_eq!(CropPreset::Original.resolve_ratio(0, 0), Some((1, 1)));
    }

    #[test]
    fn resolve_ratio_returns_none_for_free() {
        assert_eq!(CropPreset::Free.resolve_ratio(800, 600), None);
    }
}

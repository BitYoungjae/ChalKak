#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropPreset {
    Free,
    Ratio16x9,
    Ratio4x3,
    Ratio1x1,
    Ratio9x16,
    Original,
}

impl CropPreset {
    pub const fn is_free(self) -> bool {
        matches!(self, Self::Free)
    }

    pub const fn ratio(self) -> Option<(u32, u32)> {
        match self {
            Self::Free => None,
            Self::Ratio16x9 => Some((16, 9)),
            Self::Ratio4x3 => Some((4, 3)),
            Self::Ratio1x1 => Some((1, 1)),
            Self::Ratio9x16 => Some((9, 16)),
            Self::Original => None,
        }
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

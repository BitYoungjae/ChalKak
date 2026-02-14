use super::ToolPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlurRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl BlurRegion {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlurOptions {
    pub intensity: u8,
}

impl Default for BlurOptions {
    fn default() -> Self {
        Self::new(55)
    }
}

impl BlurOptions {
    pub fn new(intensity: u8) -> Self {
        Self {
            intensity: clamp_intensity(intensity),
        }
    }

    pub fn set_intensity(&mut self, intensity: u8) {
        self.intensity = clamp_intensity(intensity);
    }
}

const fn clamp_intensity(intensity: u8) -> u8 {
    if intensity < 1 {
        1
    } else if intensity > 100 {
        100
    } else {
        intensity
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlurElement {
    pub id: u64,
    pub region: BlurRegion,
    pub options: BlurOptions,
    pub anchor: ToolPoint,
}

impl BlurElement {
    pub fn new(id: u64, region: BlurRegion, options: BlurOptions) -> Self {
        Self {
            id,
            region,
            options,
            anchor: ToolPoint {
                x: region.x,
                y: region.y,
            },
        }
    }
}

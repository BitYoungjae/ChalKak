use super::{Color, ToolPoint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrowOptions {
    pub color: Color,
    pub thickness: u8,
    pub head_size: u8,
}

impl Default for ArrowOptions {
    fn default() -> Self {
        Self {
            color: Color::new(0, 0, 0),
            thickness: 3,
            head_size: 8,
        }
    }
}

impl ArrowOptions {
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_thickness(&mut self, thickness: u8) {
        self.thickness = clamp_u8_range(thickness, 1, 255);
    }

    pub fn set_head_size(&mut self, head_size: u8) {
        self.head_size = clamp_u8_range(head_size, 1, 255);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrowElement {
    pub id: u64,
    pub start: ToolPoint,
    pub end: ToolPoint,
    pub options: ArrowOptions,
}

impl ArrowElement {
    pub fn new(id: u64, start: ToolPoint, end: ToolPoint, options: ArrowOptions) -> Self {
        Self {
            id,
            start,
            end,
            options,
        }
    }
}

const fn clamp_u8_range(value: u8, min: u8, max: u8) -> u8 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

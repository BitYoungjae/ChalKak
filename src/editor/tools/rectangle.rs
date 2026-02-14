#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectangleOptions {
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub thickness: u8,
    pub fill_enabled: bool,
    pub border_radius: u16,
}

impl Default for RectangleOptions {
    fn default() -> Self {
        Self {
            color_r: 0,
            color_g: 0,
            color_b: 0,
            thickness: 3,
            fill_enabled: false,
            border_radius: DEFAULT_RECTANGLE_BORDER_RADIUS,
        }
    }
}

impl RectangleOptions {
    pub fn set_border_color(&mut self, color_r: u8, color_g: u8, color_b: u8) {
        self.color_r = color_r;
        self.color_g = color_g;
        self.color_b = color_b;
    }

    pub fn set_thickness(&mut self, thickness: u8) {
        self.thickness = clamp_u8_range(thickness, 1, 255);
    }

    pub fn set_fill_enabled(&mut self, fill_enabled: bool) {
        self.fill_enabled = fill_enabled;
    }

    pub fn set_border_radius(&mut self, border_radius: u16) {
        self.border_radius = border_radius;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectangleElement {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub options: RectangleOptions,
}

impl RectangleElement {
    pub fn new(
        id: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        options: RectangleOptions,
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

const DEFAULT_RECTANGLE_BORDER_RADIUS: u16 = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_options_default_uses_rounded_border_radius() {
        let options = RectangleOptions::default();
        assert_eq!(options.border_radius, DEFAULT_RECTANGLE_BORDER_RADIUS);
    }

    #[test]
    fn rectangle_options_border_radius_is_settable() {
        let mut options = RectangleOptions::default();
        options.set_border_radius(14);
        assert_eq!(options.border_radius, 14);
    }
}

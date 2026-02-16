#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PenPoint {
    pub x: i32,
    pub y: i32,
}

impl PenPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PenOptions {
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub opacity: u8,
    pub thickness: u8,
}

impl Default for PenOptions {
    fn default() -> Self {
        Self {
            color_r: 0,
            color_g: 0,
            color_b: 0,
            opacity: 100,
            thickness: 3,
        }
    }
}

impl PenOptions {
    pub fn set_color(&mut self, color_r: u8, color_g: u8, color_b: u8) {
        self.color_r = color_r;
        self.color_g = color_g;
        self.color_b = color_b;
    }

    pub fn set_opacity(&mut self, opacity: u8) {
        self.opacity = clamp_u8_range(opacity, 1, 100);
    }

    pub fn set_thickness(&mut self, thickness: u8) {
        self.thickness = clamp_u8_range(thickness, 1, 255);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenStroke {
    pub id: u64,
    pub points: Vec<PenPoint>,
    pub options: PenOptions,
    pub finalized: bool,
}

impl PenStroke {
    pub fn new(id: u64, start: PenPoint, options: PenOptions) -> Self {
        Self {
            id,
            points: vec![start],
            options,
            finalized: false,
        }
    }

    pub fn append_point(&mut self, point: PenPoint) {
        self.points.push(point);
    }

    pub fn finalize(&mut self) {
        self.finalized = true;
    }
}

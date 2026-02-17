mod arrow;
mod blur;
mod crop;
mod operations;
mod pen;
mod query;
mod rectangle;
mod selection;
mod text;

pub use crate::geometry::{Color, ImageBounds, ToolBounds, ToolPoint};
pub use arrow::{ArrowElement, ArrowOptions};
pub use blur::{BlurElement, BlurOptions, BlurRegion};
pub use crop::{CropElement, CropOptions, CropPreset, CROP_MIN_SIZE};
pub use pen::{PenOptions, PenPoint, PenStroke};
pub use rectangle::{RectangleElement, RectangleOptions};
pub use text::{TextElement, TextFontFamily, TextOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Select,
    Pan,
    Blur,
    Pen,
    Arrow,
    Rectangle,
    Crop,
    Text,
    Ocr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolObject {
    Blur(BlurElement),
    Pen(PenStroke),
    Arrow(ArrowElement),
    Rectangle(RectangleElement),
    Crop(CropElement),
    Text(TextElement),
}

impl ToolObject {
    pub const fn id(&self) -> u64 {
        match self {
            Self::Blur(blur) => blur.id,
            Self::Pen(stroke) => stroke.id,
            Self::Arrow(arrow) => arrow.id,
            Self::Rectangle(rectangle) => rectangle.id,
            Self::Crop(crop) => crop.id,
            Self::Text(text) => text.id,
        }
    }

    fn as_blur_mut(&mut self) -> Option<&mut BlurElement> {
        match self {
            Self::Blur(blur) => Some(blur),
            _ => None,
        }
    }

    fn as_pen_mut(&mut self) -> Option<&mut PenStroke> {
        match self {
            Self::Pen(stroke) => Some(stroke),
            _ => None,
        }
    }

    fn as_rectangle_mut(&mut self) -> Option<&mut RectangleElement> {
        match self {
            Self::Rectangle(rectangle) => Some(rectangle),
            _ => None,
        }
    }

    fn as_crop(&self) -> Option<&CropElement> {
        match self {
            Self::Crop(crop) => Some(crop),
            _ => None,
        }
    }

    fn as_crop_mut(&mut self) -> Option<&mut CropElement> {
        match self {
            Self::Crop(crop) => Some(crop),
            _ => None,
        }
    }

    fn as_text(&self) -> Option<&TextElement> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    fn as_text_mut(&mut self) -> Option<&mut TextElement> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    InvalidBlurRegion,
    InvalidArrowGeometry,
    InvalidRectangleGeometry,
    InvalidCropGeometry,
    EmptyPenStroke,
    PenStrokeNotFound,
    ObjectNotFound,
    ToolNotSelected,
}

#[derive(Debug, Clone)]
pub struct EditorTools {
    active_tool: ToolKind,
    blur_options: BlurOptions,
    pen_options: PenOptions,
    arrow_options: ArrowOptions,
    rectangle_options: RectangleOptions,
    crop_options: CropOptions,
    text_options: TextOptions,
    objects: Vec<ToolObject>,
    next_id: u64,
    active_pen_stroke: Option<u64>,
    active_text_box: Option<u64>,
}

impl Default for EditorTools {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorTools {
    fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    fn collect_objects<T: Clone>(&self, projector: fn(&ToolObject) -> Option<&T>) -> Vec<T> {
        self.objects
            .iter()
            .filter_map(projector)
            .cloned()
            .collect::<Vec<_>>()
    }

    fn find_object_ref<T>(&self, id: u64, projector: fn(&ToolObject) -> Option<&T>) -> Option<&T> {
        self.objects.iter().find_map(|object| {
            if object.id() == id {
                projector(object)
            } else {
                None
            }
        })
    }

    fn find_object_mut<T>(
        &mut self,
        id: u64,
        projector: fn(&mut ToolObject) -> Option<&mut T>,
    ) -> Option<&mut T> {
        self.objects.iter_mut().find_map(|object| {
            if object.id() == id {
                projector(object)
            } else {
                None
            }
        })
    }

    fn clear_active_state_for_object(&mut self, object: &ToolObject) {
        match object {
            ToolObject::Pen(stroke) => {
                if self.active_pen_stroke == Some(stroke.id) {
                    self.active_pen_stroke = None;
                }
            }
            ToolObject::Text(text) => {
                if self.active_text_box == Some(text.id) {
                    self.active_text_box = None;
                }
            }
            _ => {}
        }
    }

    pub fn new() -> Self {
        Self {
            active_tool: ToolKind::Select,
            blur_options: BlurOptions::default(),
            pen_options: PenOptions::default(),
            arrow_options: ArrowOptions::default(),
            rectangle_options: RectangleOptions::default(),
            crop_options: CropOptions::default(),
            text_options: TextOptions::default(),
            objects: Vec::new(),
            next_id: 1,
            active_pen_stroke: None,
            active_text_box: None,
        }
    }

    pub fn select_tool(&mut self, tool: ToolKind) {
        self.active_tool = tool;
    }

    pub fn arrow_options(&self) -> ArrowOptions {
        self.arrow_options
    }

    pub fn rectangle_options(&self) -> RectangleOptions {
        self.rectangle_options
    }

    pub fn crop_options(&self) -> CropOptions {
        self.crop_options
    }

    pub fn text_options(&self) -> TextOptions {
        self.text_options
    }

    fn set_pen_color(&mut self, color: Color) {
        self.pen_options.set_color(color);
    }

    fn set_pen_thickness(&mut self, thickness: u8) {
        self.pen_options.set_thickness(thickness);
    }

    fn set_arrow_color(&mut self, color: Color) {
        self.arrow_options.set_color(color);
    }

    fn set_arrow_thickness(&mut self, thickness: u8) {
        self.arrow_options.set_thickness(thickness);
    }

    pub fn set_arrow_head_size(&mut self, head_size: u8) {
        self.arrow_options.set_head_size(head_size);
    }

    fn set_rectangle_color(&mut self, color: Color) {
        self.rectangle_options.set_border_color(color);
    }

    fn set_rectangle_thickness(&mut self, thickness: u8) {
        self.rectangle_options.set_thickness(thickness);
    }

    pub fn set_rectangle_border_radius(&mut self, border_radius: u16) {
        self.rectangle_options.set_border_radius(border_radius);
    }

    pub fn set_crop_preset(&mut self, preset: CropPreset) {
        self.crop_options.set_preset(preset);
    }

    fn set_text_color(&mut self, color: Color) {
        self.text_options.set_color(color);
    }

    pub fn set_shared_stroke_color(&mut self, color: Color) {
        self.set_pen_color(color);
        self.set_arrow_color(color);
        self.set_rectangle_color(color);
        self.set_text_color(color);
    }

    pub fn set_shared_stroke_thickness(&mut self, thickness: u8) {
        self.set_pen_thickness(thickness);
        self.set_arrow_thickness(thickness);
        self.set_rectangle_thickness(thickness);
    }

    pub fn set_text_size(&mut self, size: u8) {
        self.text_options.set_size(size);
    }

    pub fn objects(&self) -> &[ToolObject] {
        &self.objects
    }
}

pub(crate) fn adjust_ratio_to_fit(
    width: u32,
    height: u32,
    ratio_x: u32,
    ratio_y: u32,
) -> (u32, u32) {
    let target_w = scale_ratio_dimension(height, ratio_x, ratio_y);
    let target_h = scale_ratio_dimension(width, ratio_y, ratio_x);

    if target_w <= width {
        (target_w, height)
    } else {
        (width, target_h)
    }
}

pub(crate) fn scale_ratio_dimension(base: u32, numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        0
    } else {
        let scaled = (u64::from(base) * u64::from(numerator)) / u64::from(denominator);
        u32::try_from(scaled).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
impl ToolObject {
    fn as_blur(&self) -> Option<&BlurElement> {
        match self {
            Self::Blur(blur) => Some(blur),
            _ => None,
        }
    }

    fn as_pen(&self) -> Option<&PenStroke> {
        match self {
            Self::Pen(stroke) => Some(stroke),
            _ => None,
        }
    }

    fn as_arrow(&self) -> Option<&ArrowElement> {
        match self {
            Self::Arrow(arrow) => Some(arrow),
            _ => None,
        }
    }

    fn as_rectangle(&self) -> Option<&RectangleElement> {
        match self {
            Self::Rectangle(rectangle) => Some(rectangle),
            _ => None,
        }
    }
}

#[cfg(test)]
impl EditorTools {
    fn count_objects(&self, matcher: fn(&ToolObject) -> bool) -> usize {
        self.objects.iter().filter(|object| matcher(object)).count()
    }

    fn active_tool(&self) -> ToolKind {
        self.active_tool
    }

    fn blur_intensity(&self) -> u8 {
        self.blur_options.intensity
    }

    fn set_blur_intensity(&mut self, intensity: u8) {
        self.blur_options.set_intensity(intensity);
    }

    fn pen_options(&self) -> PenOptions {
        self.pen_options
    }

    fn set_pen_opacity(&mut self, opacity: u8) {
        self.pen_options.set_opacity(opacity);
    }

    fn set_rectangle_fill(&mut self, fill_enabled: bool) {
        self.rectangle_options.set_fill_enabled(fill_enabled);
    }

    fn set_text_weight(&mut self, weight: u16) {
        self.text_options.set_weight(weight);
    }

    fn set_text_family(&mut self, family: TextFontFamily) {
        self.text_options.set_family(family);
    }

    fn blur_count(&self) -> usize {
        self.count_objects(|object| matches!(object, ToolObject::Blur(_)))
    }

    fn pen_stroke_count(&self) -> usize {
        self.count_objects(|object| matches!(object, ToolObject::Pen(_)))
    }

    fn arrow_count(&self) -> usize {
        self.count_objects(|object| matches!(object, ToolObject::Arrow(_)))
    }

    fn rectangle_count(&self) -> usize {
        self.count_objects(|object| matches!(object, ToolObject::Rectangle(_)))
    }

    fn crop_count(&self) -> usize {
        self.count_objects(|object| matches!(object, ToolObject::Crop(_)))
    }

    fn text_count(&self) -> usize {
        self.count_objects(|object| matches!(object, ToolObject::Text(_)))
    }

    fn get_blur(&self, id: u64) -> Option<&BlurElement> {
        self.find_object_ref(id, ToolObject::as_blur)
    }

    fn get_pen_stroke(&self, id: u64) -> Option<&PenStroke> {
        self.find_object_ref(id, ToolObject::as_pen)
    }

    fn get_arrow(&self, id: u64) -> Option<&ArrowElement> {
        self.find_object_ref(id, ToolObject::as_arrow)
    }

    fn get_rectangle(&self, id: u64) -> Option<&RectangleElement> {
        self.find_object_ref(id, ToolObject::as_rectangle)
    }
}

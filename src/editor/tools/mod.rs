mod arrow;
mod blur;
mod crop;
mod pen;
mod rectangle;
mod text;

pub use crate::geometry::{Color, ImageBounds, ToolBounds, ToolPoint};
pub use arrow::{ArrowElement, ArrowOptions};
pub use blur::{BlurElement, BlurOptions, BlurRegion};
pub use crop::{CropElement, CropOptions, CropPreset, CROP_MIN_SIZE};
pub use pen::{PenOptions, PenPoint, PenStroke};
pub use rectangle::{RectangleElement, RectangleOptions};
pub use text::{TextElement, TextFontFamily, TextOptions};

use crate::input::{resolve_text_input, TextInputAction, TextInputEvent};

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

    pub fn crops(&self) -> Vec<CropElement> {
        self.collect_objects(ToolObject::as_crop)
    }

    pub fn active_text_id(&self) -> Option<u64> {
        self.active_text_box
    }

    pub fn active_text(&self) -> Option<&TextElement> {
        self.active_text_box.and_then(|id| self.get_text(id))
    }

    pub fn active_text_focus_content(&self) -> Option<&str> {
        self.active_text().map(|text| text.content.as_str())
    }

    pub fn focus_text_box(&mut self, id: u64) -> bool {
        if let Some(text) = self.get_text_mut(id) {
            text.move_cursor_to_end();
            self.active_text_box = Some(id);
            true
        } else {
            false
        }
    }

    pub fn add_blur(&mut self, region: BlurRegion) -> Result<u64, ToolError> {
        if !region.is_valid() {
            return Err(ToolError::InvalidBlurRegion);
        }

        let id = self.allocate_id();
        let element = BlurElement::new(id, region, self.blur_options);
        self.objects.push(ToolObject::Blur(element));
        Ok(id)
    }

    pub fn add_arrow(&mut self, start: ToolPoint, end: ToolPoint) -> Result<u64, ToolError> {
        if start == end {
            return Err(ToolError::InvalidArrowGeometry);
        }

        let id = self.allocate_id();
        let element = ArrowElement::new(id, start, end, self.arrow_options);
        self.objects.push(ToolObject::Arrow(element));
        Ok(id)
    }

    pub fn add_rectangle(&mut self, start: ToolPoint, end: ToolPoint) -> Result<u64, ToolError> {
        let dx = i64::from(end.x) - i64::from(start.x);
        let dy = i64::from(end.y) - i64::from(start.y);
        let width = dx.abs();
        let height = dy.abs();

        if width == 0 || height == 0 {
            return Err(ToolError::InvalidRectangleGeometry);
        }

        let id = self.allocate_id();
        let rectangle = RectangleElement::new(
            id,
            start.x.min(end.x),
            start.y.min(end.y),
            u32::try_from(width).expect("rectangle width must fit u32"),
            u32::try_from(height).expect("rectangle height must fit u32"),
            self.rectangle_options,
        );
        self.objects.push(ToolObject::Rectangle(rectangle));
        Ok(id)
    }

    pub fn add_crop_in_bounds(
        &mut self,
        start: ToolPoint,
        end: ToolPoint,
        image_width: u32,
        image_height: u32,
    ) -> Result<u64, ToolError> {
        let (x, y, mut width, mut height) =
            normalize_crop_box(start.x, start.y, end.x, end.y, image_width, image_height)?;
        let preset = self.crop_options.preset;

        if preset == CropPreset::Original && (image_width == u32::MAX || image_height == u32::MAX) {
            return Err(ToolError::InvalidCropGeometry);
        }

        if let Some((ratio_x, ratio_y)) = preset.ratio().or_else(|| {
            if preset == CropPreset::Original {
                Some((image_width, image_height))
            } else {
                None
            }
        }) {
            let (adjusted_width, adjusted_height) =
                adjust_ratio_to_fit(width, height, ratio_x, ratio_y);
            width = adjusted_width;
            height = adjusted_height;

            if width == 0 || height == 0 {
                return Err(ToolError::InvalidCropGeometry);
            }
        }

        if width < CROP_MIN_SIZE || height < CROP_MIN_SIZE {
            return Err(ToolError::InvalidCropGeometry);
        }

        let id = self.allocate_id();
        let crop = CropElement::new(id, x, y, width, height, self.crop_options);
        self.objects.push(ToolObject::Crop(crop));
        Ok(id)
    }

    fn push_text_element(&mut self, element: TextElement) -> u64 {
        let id = element.id;
        self.objects.push(ToolObject::Text(element));
        self.active_text_box = Some(id);
        self.active_tool = ToolKind::Text;
        id
    }

    pub fn add_text_box(&mut self, at: ToolPoint) -> u64 {
        let id = self.allocate_id();
        self.push_text_element(TextElement::new(id, at, self.text_options))
    }

    pub fn finish_text_box(&mut self) -> bool {
        self.active_text_box.take().is_some()
    }

    pub fn apply_text_input(&mut self, event: TextInputEvent) -> TextInputAction {
        let action = resolve_text_input(event, self.active_text_box.is_some());

        let target_id = match self.active_text_box {
            Some(text_id) => text_id,
            None => return action,
        };

        let text = match self.get_text_mut(target_id) {
            Some(text) => text,
            None => {
                self.active_text_box = None;
                return TextInputAction::NoTextTarget;
            }
        };

        match action {
            TextInputAction::InsertCharacter(c) => {
                text.insert_char(c);
                TextInputAction::InsertCharacter(c)
            }
            TextInputAction::DeleteBackward => {
                let deleted = text.delete_backward();
                if deleted {
                    TextInputAction::DeleteBackward
                } else {
                    TextInputAction::NoAction
                }
            }
            TextInputAction::InsertLineBreak => {
                text.insert_newline();
                TextInputAction::InsertLineBreak
            }
            TextInputAction::MoveCursor => match event {
                TextInputEvent::CursorLeft => {
                    if text.move_cursor_left() {
                        TextInputAction::MoveCursor
                    } else {
                        TextInputAction::NoAction
                    }
                }
                TextInputEvent::CursorRight => {
                    if text.move_cursor_right() {
                        TextInputAction::MoveCursor
                    } else {
                        TextInputAction::NoAction
                    }
                }
                TextInputEvent::CursorUp => {
                    if text.move_cursor_up() {
                        TextInputAction::MoveCursor
                    } else {
                        TextInputAction::NoAction
                    }
                }
                TextInputEvent::CursorDown => {
                    if text.move_cursor_down() {
                        TextInputAction::MoveCursor
                    } else {
                        TextInputAction::NoAction
                    }
                }
                _ => TextInputAction::NoAction,
            },
            TextInputAction::Commit => {
                self.active_text_box = None;
                TextInputAction::Commit
            }
            TextInputAction::ExitFocus => {
                self.active_text_box = None;
                TextInputAction::ExitFocus
            }
            TextInputAction::CopyRequested => TextInputAction::CopyRequested,
            TextInputAction::NoTextTarget => TextInputAction::NoTextTarget,
            TextInputAction::NoAction => TextInputAction::NoAction,
        }
    }

    pub fn get_crop(&self, id: u64) -> Option<&CropElement> {
        self.find_object_ref(id, ToolObject::as_crop)
    }

    pub fn get_text(&self, id: u64) -> Option<&TextElement> {
        self.find_object_ref(id, ToolObject::as_text)
    }

    pub fn get_text_mut(&mut self, id: u64) -> Option<&mut TextElement> {
        self.find_object_mut(id, ToolObject::as_text_mut)
    }

    pub fn object(&self, id: u64) -> Option<&ToolObject> {
        self.objects.iter().find(|object| object.id() == id)
    }

    pub fn begin_pen_stroke(&mut self, start: ToolPoint) -> u64 {
        let id = self.allocate_id();
        let stroke = PenStroke::new(id, PenPoint::new(start.x, start.y), self.pen_options);
        self.objects.push(ToolObject::Pen(stroke));
        self.active_pen_stroke = Some(id);
        id
    }

    pub fn append_pen_point(&mut self, stroke_id: u64, point: ToolPoint) -> Result<(), ToolError> {
        if self.active_pen_stroke.is_none() {
            return Err(ToolError::ToolNotSelected);
        }

        let stroke = self
            .find_object_mut(stroke_id, ToolObject::as_pen_mut)
            .ok_or(ToolError::PenStrokeNotFound)?;
        stroke.append_point(PenPoint::new(point.x, point.y));
        Ok(())
    }

    pub fn finish_pen_stroke(&mut self, stroke_id: u64) -> Result<(), ToolError> {
        {
            let stroke = self
                .find_object_mut(stroke_id, ToolObject::as_pen_mut)
                .ok_or(ToolError::PenStrokeNotFound)?;
            stroke.finalize();
        }
        if self.active_pen_stroke == Some(stroke_id) {
            self.active_pen_stroke = None;
        }
        Ok(())
    }

    pub fn move_object_by(
        &mut self,
        id: u64,
        delta_x: i32,
        delta_y: i32,
        image_width: i32,
        image_height: i32,
    ) -> Result<(), ToolError> {
        let image_bounds = ImageBounds::new(image_width, image_height);
        let max_x = image_width.saturating_sub(1).max(0);
        let max_y = image_height.saturating_sub(1).max(0);
        let object = self
            .objects
            .iter_mut()
            .find(|object| object.id() == id)
            .ok_or(ToolError::ObjectNotFound)?;
        match object {
            ToolObject::Blur(blur) => {
                move_box_by(
                    (&mut blur.region.x, &mut blur.region.y),
                    (blur.region.width, blur.region.height),
                    (delta_x, delta_y),
                    image_bounds,
                );
                blur.anchor = ToolPoint::new(blur.region.x, blur.region.y);
            }
            ToolObject::Pen(stroke) => {
                if let Some((min_x, max_stroke_x, min_y, max_stroke_y)) =
                    pen_point_bounds(&stroke.points)
                {
                    let bounded_delta_x =
                        clamp_translation_delta(delta_x, min_x, max_stroke_x, max_x);
                    let bounded_delta_y =
                        clamp_translation_delta(delta_y, min_y, max_stroke_y, max_y);
                    translate_pen_points(&mut stroke.points, bounded_delta_x, bounded_delta_y);
                }
            }
            ToolObject::Arrow(arrow) => {
                let min_arrow_x = arrow.start.x.min(arrow.end.x);
                let max_arrow_x = arrow.start.x.max(arrow.end.x);
                let min_arrow_y = arrow.start.y.min(arrow.end.y);
                let max_arrow_y = arrow.start.y.max(arrow.end.y);
                let bounded_delta_x =
                    clamp_translation_delta(delta_x, min_arrow_x, max_arrow_x, max_x);
                let bounded_delta_y =
                    clamp_translation_delta(delta_y, min_arrow_y, max_arrow_y, max_y);

                arrow.start.x = arrow.start.x.saturating_add(bounded_delta_x);
                arrow.start.y = arrow.start.y.saturating_add(bounded_delta_y);
                arrow.end.x = arrow.end.x.saturating_add(bounded_delta_x);
                arrow.end.y = arrow.end.y.saturating_add(bounded_delta_y);
            }
            ToolObject::Rectangle(rectangle) => {
                move_box_by(
                    (&mut rectangle.x, &mut rectangle.y),
                    (rectangle.width, rectangle.height),
                    (delta_x, delta_y),
                    image_bounds,
                );
            }
            ToolObject::Crop(crop) => {
                move_box_by(
                    (&mut crop.x, &mut crop.y),
                    (crop.width, crop.height),
                    (delta_x, delta_y),
                    image_bounds,
                );
            }
            ToolObject::Text(text) => {
                text.x = text.x.saturating_add(delta_x).clamp(0, max_x);
                text.y = text.y.saturating_add(delta_y).clamp(0, max_y);
            }
        }
        Ok(())
    }

    pub fn resize_rectangle(
        &mut self,
        id: u64,
        bounds: ToolBounds,
        image_bounds: ImageBounds,
    ) -> Result<(), ToolError> {
        if bounds.width == 0 || bounds.height == 0 {
            return Err(ToolError::InvalidRectangleGeometry);
        }
        let rectangle = self
            .find_object_mut(id, ToolObject::as_rectangle_mut)
            .ok_or(ToolError::ObjectNotFound)?;
        let bounded = clamp_bounds_to_image(bounds, image_bounds);
        rectangle.x = bounded.x;
        rectangle.y = bounded.y;
        rectangle.width = bounded.width;
        rectangle.height = bounded.height;
        Ok(())
    }

    pub fn resize_blur(
        &mut self,
        id: u64,
        bounds: ToolBounds,
        image_bounds: ImageBounds,
    ) -> Result<(), ToolError> {
        if bounds.width == 0 || bounds.height == 0 {
            return Err(ToolError::InvalidBlurRegion);
        }
        let blur = self
            .find_object_mut(id, ToolObject::as_blur_mut)
            .ok_or(ToolError::ObjectNotFound)?;
        let bounded = clamp_bounds_to_image(bounds, image_bounds);
        blur.region.x = bounded.x;
        blur.region.y = bounded.y;
        blur.region.width = bounded.width;
        blur.region.height = bounded.height;
        blur.anchor = ToolPoint::new(blur.region.x, blur.region.y);
        Ok(())
    }

    pub fn resize_crop(
        &mut self,
        id: u64,
        bounds: ToolBounds,
        image_bounds: ImageBounds,
    ) -> Result<(), ToolError> {
        if bounds.width < CROP_MIN_SIZE || bounds.height < CROP_MIN_SIZE {
            return Err(ToolError::InvalidCropGeometry);
        }
        let crop = self
            .find_object_mut(id, ToolObject::as_crop_mut)
            .ok_or(ToolError::ObjectNotFound)?;
        let bounded = clamp_bounds_to_image(bounds, image_bounds);
        if bounded.width < CROP_MIN_SIZE || bounded.height < CROP_MIN_SIZE {
            return Err(ToolError::InvalidCropGeometry);
        }
        crop.x = bounded.x;
        crop.y = bounded.y;
        crop.width = bounded.width.max(CROP_MIN_SIZE);
        crop.height = bounded.height.max(CROP_MIN_SIZE);
        Ok(())
    }

    pub fn remove_object(&mut self, id: u64) -> Option<ToolObject> {
        let index = self.objects.iter().position(|object| object.id() == id)?;
        let object = self.objects.remove(index);
        self.clear_active_state_for_object(&object);
        Some(object)
    }

    pub fn replace_objects(&mut self, objects: Vec<ToolObject>) {
        self.objects = objects;
        self.next_id = self
            .objects
            .iter()
            .map(ToolObject::id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        if let Some(active_id) = self.active_pen_stroke {
            if self
                .objects
                .iter()
                .all(|object| !matches!(object, ToolObject::Pen(stroke) if stroke.id == active_id))
            {
                self.active_pen_stroke = None;
            }
        }
        if let Some(active_id) = self.active_text_box {
            if self
                .objects
                .iter()
                .all(|object| !matches!(object, ToolObject::Text(text) if text.id == active_id))
            {
                self.active_text_box = None;
            }
        }
    }
}

fn move_box_by(
    position: (&mut i32, &mut i32),
    size: (u32, u32),
    delta: (i32, i32),
    image_bounds: ImageBounds,
) {
    let (x, y) = position;
    let (width, height) = size;
    let (delta_x, delta_y) = delta;
    let bounded_width = i32::try_from(width).unwrap_or(i32::MAX);
    let bounded_height = i32::try_from(height).unwrap_or(i32::MAX);
    let limit_x = image_bounds.width.saturating_sub(bounded_width).max(0);
    let limit_y = image_bounds.height.saturating_sub(bounded_height).max(0);
    *x = x.saturating_add(delta_x).clamp(0, limit_x);
    *y = y.saturating_add(delta_y).clamp(0, limit_y);
}

fn pen_point_bounds(points: &[PenPoint]) -> Option<(i32, i32, i32, i32)> {
    let first = points.first()?;
    let mut min_x = first.x;
    let mut max_x = first.x;
    let mut min_y = first.y;
    let mut max_y = first.y;
    for point in &points[1..] {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    Some((min_x, max_x, min_y, max_y))
}

fn translate_pen_points(points: &mut [PenPoint], delta_x: i32, delta_y: i32) {
    for point in points {
        point.x = point.x.saturating_add(delta_x);
        point.y = point.y.saturating_add(delta_y);
    }
}

fn clamp_bounds_to_image(bounds: ToolBounds, image_bounds: ImageBounds) -> ToolBounds {
    let max_x = image_bounds.width.saturating_sub(1).max(0);
    let max_y = image_bounds.height.saturating_sub(1).max(0);
    let clamped_x = bounds.x.clamp(0, max_x);
    let clamped_y = bounds.y.clamp(0, max_y);
    let max_width = image_bounds.width.saturating_sub(clamped_x).max(1);
    let max_height = image_bounds.height.saturating_sub(clamped_y).max(1);
    ToolBounds::new(
        clamped_x,
        clamped_y,
        bounds
            .width
            .min(u32::try_from(max_width).unwrap_or(u32::MAX)),
        bounds
            .height
            .min(u32::try_from(max_height).unwrap_or(u32::MAX)),
    )
}

fn clamp_crop_bounds(value: i64, min: i64, max: i64) -> i64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn normalize_crop_box(
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    image_width: u32,
    image_height: u32,
) -> Result<(i32, i32, u32, u32), ToolError> {
    let mut left = i64::from(start_x.min(end_x));
    let mut right = i64::from(start_x.max(end_x));
    let mut top = i64::from(start_y.min(end_y));
    let mut bottom = i64::from(start_y.max(end_y));

    let bounded = image_width != u32::MAX && image_height != u32::MAX;
    let (max_x, max_y) = if bounded {
        (i64::from(image_width), i64::from(image_height))
    } else {
        (i64::MAX, i64::MAX)
    };

    left = clamp_crop_bounds(left, 0, max_x);
    right = clamp_crop_bounds(right, 0, max_x);
    top = clamp_crop_bounds(top, 0, max_y);
    bottom = clamp_crop_bounds(bottom, 0, max_y);

    if right <= left || bottom <= top {
        return Err(ToolError::InvalidCropGeometry);
    }

    if bounded && (image_width == 0 || image_height == 0) {
        return Err(ToolError::InvalidCropGeometry);
    }

    let width = u32::try_from(right - left).map_err(|_| ToolError::InvalidCropGeometry)?;
    let height = u32::try_from(bottom - top).map_err(|_| ToolError::InvalidCropGeometry)?;

    Ok((
        i32::try_from(left).map_err(|_| ToolError::InvalidCropGeometry)?,
        i32::try_from(top).map_err(|_| ToolError::InvalidCropGeometry)?,
        width,
        height,
    ))
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

fn clamp_translation_delta(delta: i32, min_coord: i32, max_coord: i32, axis_max: i32) -> i32 {
    let min_delta = min_coord.saturating_neg();
    let max_delta = axis_max.saturating_sub(max_coord);
    delta.clamp(min_delta, max_delta)
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

    fn add_text_box_with_text(&mut self, at: ToolPoint, text: impl Into<String>) -> u64 {
        let id = self.allocate_id();
        self.push_text_element(TextElement::with_text(id, at, text, self.text_options))
    }

    fn add_pen_stroke(&mut self, points: &[ToolPoint]) -> Result<u64, ToolError> {
        if points.is_empty() {
            return Err(ToolError::EmptyPenStroke);
        }

        let id = self.allocate_id();

        let mut points = points
            .iter()
            .copied()
            .map(|point| PenPoint::new(point.x, point.y));
        let start = points.next().expect("points is checked non-empty above");

        let mut stroke = PenStroke::new(id, start, self.pen_options);
        points.for_each(|point| stroke.append_point(point));
        stroke.finalize();

        self.objects.push(ToolObject::Pen(stroke));
        Ok(id)
    }

    fn pop_last_object(&mut self) -> Option<ToolObject> {
        let object = self.objects.pop()?;
        self.clear_active_state_for_object(&object);
        Some(object)
    }

    fn push_object(&mut self, object: ToolObject) {
        let object_id = object.id();
        self.next_id = self.next_id.max(object_id.saturating_add(1));
        self.objects.push(object);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{resolve_text_input, TextInputAction, TextInputEvent};

    fn session() -> EditorTools {
        EditorTools::new()
    }

    #[test]
    fn shared_stroke_style_updates_pen_arrow_rectangle_and_text_options() {
        let mut tools = session();

        tools.set_shared_stroke_color(Color::new(12, 24, 36));
        tools.set_shared_stroke_thickness(7);

        let pen = tools.pen_options();
        assert_eq!(pen.color, Color::new(12, 24, 36));
        assert_eq!(pen.thickness, 7);

        let arrow = tools.arrow_options();
        assert_eq!(arrow.color, Color::new(12, 24, 36));
        assert_eq!(arrow.thickness, 7);

        let rectangle = tools.rectangle_options();
        assert_eq!(rectangle.color, Color::new(12, 24, 36));
        assert_eq!(rectangle.thickness, 7);

        let text = tools.text_options();
        assert_eq!(text.color, Color::new(12, 24, 36));
    }

    #[test]
    fn tool_blur_pen_blur_options_are_sticky_across_tool_switch() {
        let mut tools = session();
        assert_eq!(tools.active_tool(), ToolKind::Select);
        assert_eq!(tools.blur_intensity(), 55);

        tools.set_blur_intensity(84);
        tools.select_tool(ToolKind::Pen);
        assert_eq!(tools.blur_intensity(), 84);

        tools.select_tool(ToolKind::Blur);
        let blur_id = tools
            .add_blur(BlurRegion::new(3, 4, 20, 12))
            .expect("blur region should be valid");
        let blur = tools.get_blur(blur_id).expect("blur should exist");
        assert_eq!(blur.options.intensity, 84);
        assert_eq!(blur.region, BlurRegion::new(3, 4, 20, 12));
    }

    #[test]
    fn tool_blur_pen_pen_stroke_records_points_and_sticky_options() {
        let mut tools = session();
        tools.set_pen_color(Color::new(12, 34, 56));
        tools.set_pen_opacity(88);
        tools.set_pen_thickness(7);

        let stroke_id = tools.begin_pen_stroke(ToolPoint::new(1, 1));
        tools
            .append_pen_point(stroke_id, ToolPoint::new(2, 2))
            .expect("pen stroke should be active");
        tools
            .append_pen_point(stroke_id, ToolPoint::new(3, 4))
            .expect("pen stroke should be active");
        tools
            .finish_pen_stroke(stroke_id)
            .expect("pen stroke should finish");

        let stroke = tools
            .get_pen_stroke(stroke_id)
            .expect("pen stroke should exist");

        assert_eq!(stroke.options.color, Color::new(12, 34, 56));
        assert_eq!(stroke.options.opacity, 88);
        assert_eq!(stroke.options.thickness, 7);
        assert_eq!(stroke.points.len(), 3);
        assert_eq!(stroke.points[1], PenPoint::new(2, 2));
        assert!(stroke.finalized);
        assert_eq!(tools.pen_stroke_count(), 1);
    }

    #[test]
    fn tool_blur_pen_rejects_invalid_blur_region() {
        let mut tools = session();

        let err = tools
            .add_blur(BlurRegion::new(0, 0, 0, 10))
            .expect_err("zero width blur region should fail");
        assert!(matches!(err, ToolError::InvalidBlurRegion));
    }

    #[test]
    fn tool_blur_pen_add_pen_stroke_from_points() {
        let mut tools = session();

        let stroke_id = tools
            .add_pen_stroke(&[
                ToolPoint::new(5, 5),
                ToolPoint::new(6, 7),
                ToolPoint::new(8, 10),
            ])
            .expect("non-empty stroke list should create pen object");

        let stroke = tools
            .get_pen_stroke(stroke_id)
            .expect("created stroke should be queryable");

        assert_eq!(stroke.points.len(), 3);
        assert_eq!(stroke.points[2], PenPoint::new(8, 10));
        assert!(stroke.finalized);
    }

    #[test]
    fn tool_arrow_rect_arrow_sticky_options_and_geometry_are_preserved() {
        let mut tools = session();
        tools.select_tool(ToolKind::Arrow);
        tools.set_arrow_color(Color::new(10, 20, 30));
        tools.set_arrow_thickness(6);
        tools.set_arrow_head_size(16);

        let arrow_id = tools
            .add_arrow(ToolPoint::new(1, 1), ToolPoint::new(13, 9))
            .expect("valid arrow should create object");
        let arrow = tools
            .get_arrow(arrow_id)
            .expect("arrow should be stored after add");

        assert_eq!(arrow.start, ToolPoint::new(1, 1));
        assert_eq!(arrow.end, ToolPoint::new(13, 9));
        assert_eq!(arrow.options.color, Color::new(10, 20, 30));
        assert_eq!(arrow.options.thickness, 6);
        assert_eq!(arrow.options.head_size, 16);
        assert_eq!(tools.arrow_count(), 1);
    }

    #[test]
    fn tool_arrow_rect_rejects_zero_length_arrow() {
        let mut tools = session();

        let err = tools
            .add_arrow(ToolPoint::new(4, 4), ToolPoint::new(4, 4))
            .expect_err("zero-length arrow should be invalid");
        assert!(matches!(err, ToolError::InvalidArrowGeometry));
    }

    #[test]
    fn tool_arrow_rect_rectangle_is_normalized_from_drag_and_keeps_style() {
        let mut tools = session();
        tools.select_tool(ToolKind::Rectangle);
        tools.set_rectangle_color(Color::new(5, 15, 25));
        tools.set_rectangle_thickness(11);
        tools.set_rectangle_fill(true);
        tools.set_rectangle_border_radius(12);

        let rect_id = tools
            .add_rectangle(ToolPoint::new(30, 40), ToolPoint::new(12, 8))
            .expect("valid rectangle drag should create object");
        let rectangle = tools
            .get_rectangle(rect_id)
            .expect("rectangle should be stored after add");

        assert_eq!(rectangle.x, 12);
        assert_eq!(rectangle.y, 8);
        assert_eq!(rectangle.width, 18);
        assert_eq!(rectangle.height, 32);
        assert_eq!(rectangle.options.color, Color::new(5, 15, 25));
        assert_eq!(rectangle.options.thickness, 11);
        assert!(rectangle.options.fill_enabled);
        assert_eq!(rectangle.options.border_radius, 12);
        assert_eq!(tools.rectangle_count(), 1);
    }

    #[test]
    fn tool_arrow_rect_rejects_flat_rectangle_drag() {
        let mut tools = session();

        let err = tools
            .add_rectangle(ToolPoint::new(0, 0), ToolPoint::new(10, 0))
            .expect_err("flat rectangle should be invalid");
        assert!(matches!(err, ToolError::InvalidRectangleGeometry));
    }

    #[test]
    fn tool_blur_resize_clamps_geometry_and_anchor_to_image_bounds() {
        let mut tools = session();
        let blur_id = tools
            .add_blur(BlurRegion::new(10, 10, 20, 20))
            .expect("blur should be inserted");
        tools
            .resize_blur(
                blur_id,
                ToolBounds::new(90, 95, 50, 40),
                ImageBounds::new(100, 100),
            )
            .expect("resize should clamp within image");
        let blur = tools.get_blur(blur_id).expect("blur should exist");
        assert_eq!(blur.region, BlurRegion::new(90, 95, 10, 5));
        assert_eq!(blur.anchor, ToolPoint::new(90, 95));
    }

    #[test]
    fn tool_crop_resize_enforces_min_size_and_bounds() {
        let mut tools = session();
        tools.select_tool(ToolKind::Crop);
        let crop_id = tools
            .add_crop_in_bounds(ToolPoint::new(10, 10), ToolPoint::new(80, 70), 100, 100)
            .expect("crop should be inserted");
        tools
            .resize_crop(
                crop_id,
                ToolBounds::new(70, 70, 40, 40),
                ImageBounds::new(100, 100),
            )
            .expect("crop resize should clamp to image bounds");
        let crop = tools.get_crop(crop_id).expect("crop should exist");
        assert_eq!(crop.x, 70);
        assert_eq!(crop.y, 70);
        assert_eq!(crop.width, 30);
        assert_eq!(crop.height, 30);

        let err = tools
            .resize_crop(
                crop_id,
                ToolBounds::new(70, 70, 10, 10),
                ImageBounds::new(100, 100),
            )
            .expect_err("crop smaller than minimum should fail");
        assert!(matches!(err, ToolError::InvalidCropGeometry));
    }

    #[test]
    fn tool_crop_fixed_ratio_reduces_size_and_keeps_boundary_minimum() {
        let mut tools = session();
        tools.select_tool(ToolKind::Crop);
        tools.set_crop_preset(CropPreset::Ratio16x9);

        let crop_id = tools
            .add_crop_in_bounds(ToolPoint::new(0, 0), ToolPoint::new(160, 160), 1920, 1080)
            .expect("crop should remain within fixed ratio");
        let crop = tools
            .get_crop(crop_id)
            .expect("crop should be stored after add");

        assert_eq!(crop.x, 0);
        assert_eq!(crop.y, 0);
        assert_eq!(crop.width, 160);
        assert_eq!(crop.height, 90);
        assert_eq!(crop.options.preset, CropPreset::Ratio16x9);
        assert!(crop.supports_corner_handles_only());
        assert_eq!(tools.crop_count(), 1);
        assert!(crop.width >= CROP_MIN_SIZE);
        assert!(crop.height >= CROP_MIN_SIZE);
    }

    #[test]
    fn tool_crop_rejects_too_small_or_invalid_after_ratio_and_keeps_bounds() {
        let mut tools = session();
        tools.select_tool(ToolKind::Crop);
        tools.set_crop_preset(CropPreset::Ratio9x16);

        let err = tools
            .add_crop_in_bounds(ToolPoint::new(0, 0), ToolPoint::new(20, 20), 100, 100)
            .expect_err("small drag should fail min-size constraint");
        assert!(matches!(err, ToolError::InvalidCropGeometry));

        tools.set_crop_preset(CropPreset::Ratio1x1);
        let crop_id = tools
            .add_crop_in_bounds(ToolPoint::new(-20, 5), ToolPoint::new(80, 40), 50, 30)
            .expect("bounded drag should be clamped and stored");
        let crop = tools
            .get_crop(crop_id)
            .expect("crop should be stored after bounded add");

        assert_eq!(crop.x, 0);
        assert_eq!(crop.y, 5);
        assert_eq!(crop.width, 25);
        assert_eq!(crop.height, 25);
        assert!(crop.width >= CROP_MIN_SIZE);
        assert!(crop.height >= CROP_MIN_SIZE);
    }

    #[test]
    fn tool_crop_original_ratio_uses_canvas_size_and_requires_bounds() {
        let mut tools = session();
        tools.select_tool(ToolKind::Crop);
        tools.set_crop_preset(CropPreset::Original);

        let crop_id = tools
            .add_crop_in_bounds(ToolPoint::new(10, 15), ToolPoint::new(250, 400), 800, 450)
            .expect("original ratio should derive from canvas bounds");
        let crop = tools.get_crop(crop_id).expect("crop should be stored");

        assert_eq!(crop.width, 240);
        assert_eq!(crop.height, 135);
        assert_eq!(crop.options.preset, CropPreset::Original);

        let err = tools
            .add_crop_in_bounds(
                ToolPoint::new(0, 0),
                ToolPoint::new(100, 100),
                u32::MAX,
                u32::MAX,
            )
            .expect_err("original ratio requires bounded canvas");
        assert!(matches!(err, ToolError::InvalidCropGeometry));
    }

    #[test]
    fn tool_move_pen_stroke_hits_edge_without_distorting_shape() {
        let mut tools = session();
        let stroke_id = tools
            .add_pen_stroke(&[
                ToolPoint::new(80, 10),
                ToolPoint::new(90, 20),
                ToolPoint::new(95, 25),
            ])
            .expect("stroke should be inserted");

        tools
            .move_object_by(stroke_id, 10, 0, 100, 100)
            .expect("move should clamp to image edge");
        let stroke = tools
            .get_pen_stroke(stroke_id)
            .expect("stroke should exist");
        assert_eq!(
            stroke.points,
            vec![
                PenPoint::new(84, 10),
                PenPoint::new(94, 20),
                PenPoint::new(99, 25),
            ]
        );

        tools
            .move_object_by(stroke_id, 10, 0, 100, 100)
            .expect("move at edge should stay in bounds");
        let stroke = tools
            .get_pen_stroke(stroke_id)
            .expect("stroke should exist");
        assert_eq!(
            stroke.points,
            vec![
                PenPoint::new(84, 10),
                PenPoint::new(94, 20),
                PenPoint::new(99, 25),
            ]
        );
    }

    #[test]
    fn tool_move_arrow_hits_edge_without_shortening() {
        let mut tools = session();
        let arrow_id = tools
            .add_arrow(ToolPoint::new(80, 40), ToolPoint::new(95, 70))
            .expect("arrow should be inserted");

        tools
            .move_object_by(arrow_id, 10, 0, 100, 100)
            .expect("move should clamp to image edge");
        let arrow = tools.get_arrow(arrow_id).expect("arrow should exist");
        assert_eq!(arrow.start, ToolPoint::new(84, 40));
        assert_eq!(arrow.end, ToolPoint::new(99, 70));
    }

    #[test]
    fn tool_move_blur_can_slide_along_edge() {
        let mut tools = session();
        let blur_id = tools
            .add_blur(BlurRegion::new(80, 10, 20, 20))
            .expect("blur should be inserted");

        tools
            .move_object_by(blur_id, 15, 12, 100, 100)
            .expect("move should clamp only blocked axis");
        let blur = tools.get_blur(blur_id).expect("blur should exist");
        assert_eq!(blur.region, BlurRegion::new(80, 22, 20, 20));
        assert_eq!(blur.anchor, ToolPoint::new(80, 22));
    }

    #[test]
    fn tool_text_input_add_text_box_creates_active_edit_target() {
        let mut tools = session();
        tools.select_tool(ToolKind::Text);
        tools.set_text_color(Color::new(1, 2, 3));
        tools.set_text_size(19);
        tools.set_text_weight(700);
        tools.set_text_family(TextFontFamily::Serif);

        let text_id = tools.add_text_box(ToolPoint::new(14, 7));
        let text = tools.get_text(text_id).expect("text box should be stored");

        assert_eq!(tools.active_text_id(), Some(1));
        assert_eq!(tools.active_text_id(), Some(1));
        assert_eq!(tools.active_tool(), ToolKind::Text);
        assert_eq!(text.content, "");
        assert_eq!(text.x, 14);
        assert_eq!(text.y, 7);
        assert_eq!(text.options.color, Color::new(1, 2, 3));
        assert_eq!(text.options.size, 19);
        assert_eq!(text.options.weight, 700);
        assert_eq!(text.options.family, TextFontFamily::Serif);
        assert_eq!(tools.text_count(), 1);
    }

    #[test]
    fn tool_text_input_handles_enter_shift_enter_and_character_input() {
        let mut tools = session();

        let text_id = tools.add_text_box(ToolPoint::new(3, 4));
        assert!(matches!(
            tools.apply_text_input(TextInputEvent::Character('h')),
            TextInputAction::InsertCharacter('h')
        ));
        assert_eq!(
            tools.apply_text_input(TextInputEvent::Enter),
            TextInputAction::InsertLineBreak
        );
        assert!(matches!(
            tools.apply_text_input(TextInputEvent::Character('i')),
            TextInputAction::InsertCharacter('i')
        ));
        assert_eq!(
            tools.apply_text_input(TextInputEvent::ShiftEnter),
            TextInputAction::InsertLineBreak
        );

        let text = tools.get_text(text_id).expect("text box should remain");
        assert_eq!(text.content, "h\ni\n");
    }

    #[test]
    fn tool_text_input_backspace_deletes_previous_character() {
        let mut tools = session();
        let text_id = tools.add_text_box(ToolPoint::new(0, 0));
        let _ = tools.apply_text_input(TextInputEvent::Character('a'));
        let _ = tools.apply_text_input(TextInputEvent::Character('b'));
        assert_eq!(
            tools.apply_text_input(TextInputEvent::Backspace),
            TextInputAction::DeleteBackward
        );
        let text = tools.get_text(text_id).expect("text box should remain");
        assert_eq!(text.content, "a");
    }

    #[test]
    fn tool_text_input_cursor_left_right_moves_caret_and_inserts_at_position() {
        let mut tools = session();
        let text_id = tools.add_text_box_with_text(ToolPoint::new(0, 0), "ab");
        assert_eq!(
            tools.apply_text_input(TextInputEvent::CursorLeft),
            TextInputAction::MoveCursor
        );
        assert_eq!(
            tools.apply_text_input(TextInputEvent::Character('X')),
            TextInputAction::InsertCharacter('X')
        );
        let text = tools.get_text(text_id).expect("text should exist");
        assert_eq!(text.content, "aXb");
    }

    #[test]
    fn tool_text_input_cursor_up_down_moves_between_lines() {
        let mut tools = session();
        let text_id = tools.add_text_box_with_text(ToolPoint::new(0, 0), "aa\nbbbb");
        assert_eq!(
            tools.apply_text_input(TextInputEvent::CursorUp),
            TextInputAction::MoveCursor
        );
        assert_eq!(
            tools.apply_text_input(TextInputEvent::Character('X')),
            TextInputAction::InsertCharacter('X')
        );
        assert_eq!(
            tools.apply_text_input(TextInputEvent::CursorDown),
            TextInputAction::MoveCursor
        );
        assert_eq!(
            tools.apply_text_input(TextInputEvent::Character('Y')),
            TextInputAction::InsertCharacter('Y')
        );
        let text = tools.get_text(text_id).expect("text should exist");
        assert_eq!(text.content, "aaX\nbbbYb");
    }

    #[test]
    fn tool_text_input_commit_or_cancel_text_mode_with_keyboard_commands() {
        let mut tools = session();

        tools.add_text_box(ToolPoint::new(1, 2));
        assert_eq!(
            resolve_text_input(TextInputEvent::CtrlEnter, true),
            TextInputAction::Commit
        );

        assert_eq!(
            tools.apply_text_input(TextInputEvent::CtrlEnter),
            TextInputAction::Commit
        );
        assert_eq!(
            tools.apply_text_input(TextInputEvent::Character('x')),
            TextInputAction::NoTextTarget
        );

        let id = tools.add_text_box(ToolPoint::new(1, 2));
        let _ = tools.apply_text_input(TextInputEvent::Character('x'));
        assert_eq!(tools.active_text_id(), Some(id));

        assert_eq!(
            tools.apply_text_input(TextInputEvent::Escape),
            TextInputAction::ExitFocus
        );
        assert_eq!(tools.active_text_id(), None);
    }

    #[test]
    fn tool_text_input_copy_shortcut_requires_text_focus() {
        let mut tools = session();

        tools.add_text_box(ToolPoint::new(0, 0));
        assert_eq!(
            tools.apply_text_input(TextInputEvent::Character('a')),
            TextInputAction::InsertCharacter('a')
        );

        assert_eq!(
            tools.apply_text_input(TextInputEvent::CtrlC),
            TextInputAction::CopyRequested
        );
        assert_eq!(tools.active_text_focus_content(), Some("a"));
    }

    #[test]
    fn tool_stack_pop_last_object_removes_most_recent_object() {
        let mut tools = session();
        let blur_id = tools
            .add_blur(BlurRegion::new(1, 2, 3, 4))
            .expect("blur should be inserted");
        assert_eq!(tools.blur_count(), 1);

        let popped = tools.pop_last_object().expect("object should be popped");
        assert!(matches!(popped, ToolObject::Blur(_)));
        assert_eq!(tools.blur_count(), 0);
        assert!(tools.get_blur(blur_id).is_none());
    }

    #[test]
    fn tool_stack_push_object_restores_and_advances_next_id() {
        let mut tools = session();
        let object = ToolObject::Rectangle(RectangleElement::new(
            42,
            10,
            10,
            20,
            30,
            RectangleOptions::default(),
        ));
        tools.push_object(object);
        assert_eq!(tools.rectangle_count(), 1);
        let next_blur = tools
            .add_blur(BlurRegion::new(0, 0, 10, 10))
            .expect("add after push should use next id");
        assert!(next_blur > 42);
    }
}

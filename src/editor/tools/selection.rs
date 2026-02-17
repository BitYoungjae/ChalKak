use super::*;

impl EditorTools {
    pub fn focus_text_box(&mut self, id: u64) -> bool {
        if let Some(text) = self.get_text_mut(id) {
            text.move_cursor_to_end();
            self.active_text_box = Some(id);
            true
        } else {
            false
        }
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

fn clamp_translation_delta(delta: i32, min_coord: i32, max_coord: i32, axis_max: i32) -> i32 {
    let min_delta = min_coord.saturating_neg();
    let max_delta = axis_max.saturating_sub(max_coord);
    delta.clamp(min_delta, max_delta)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> EditorTools {
        EditorTools::new()
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
    fn tool_move_pen_stroke_hits_edge_without_distorting_shape() {
        let mut tools = session();
        let stroke_id = tools.begin_pen_stroke(ToolPoint::new(80, 10));
        tools
            .append_pen_point(stroke_id, ToolPoint::new(90, 20))
            .expect("stroke point should append");
        tools
            .append_pen_point(stroke_id, ToolPoint::new(95, 25))
            .expect("stroke point should append");
        tools
            .finish_pen_stroke(stroke_id)
            .expect("stroke should finalize");

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
}

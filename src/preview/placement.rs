use super::geometry::{
    DEFAULT_PREVIEW_HEIGHT, DEFAULT_PREVIEW_WIDTH, MIN_PREVIEW_HEIGHT, MIN_PREVIEW_WIDTH,
};
use super::PreviewWindowGeometry;

const PREVIEW_MAX_BOUND_SCALE: f64 = 0.72;
const PREVIEW_MAX_DEFAULT_MULTIPLIER: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSourceArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSizingTokens {
    pub default_width: i32,
    pub default_height: i32,
    pub min_width: i32,
    pub min_height: i32,
}

impl Default for PreviewSizingTokens {
    fn default() -> Self {
        Self {
            default_width: DEFAULT_PREVIEW_WIDTH,
            default_height: DEFAULT_PREVIEW_HEIGHT,
            min_width: MIN_PREVIEW_WIDTH,
            min_height: MIN_PREVIEW_HEIGHT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewPlacement {
    pub geometry: PreviewWindowGeometry,
    pub min_width: i32,
    pub min_height: i32,
    pub max_width: i32,
    pub max_height: i32,
}

pub fn compute_preview_placement(
    source: PreviewSourceArea,
    bounds: PreviewBounds,
    sizing: PreviewSizingTokens,
) -> PreviewPlacement {
    let default_width = sizing.default_width.max(1);
    let default_height = sizing.default_height.max(1);
    let source_width = source.width.max(1);
    let source_height = source.height.max(1);
    let bounds_width = bounds.width.max(1);
    let bounds_height = bounds.height.max(1);

    let min_width = sizing.min_width.max(1).min(bounds_width);
    let min_height = sizing.min_height.max(1).min(bounds_height);

    let max_width_from_bounds =
        ((f64::from(bounds_width) * PREVIEW_MAX_BOUND_SCALE).round() as i32).max(min_width);
    let max_height_from_bounds =
        ((f64::from(bounds_height) * PREVIEW_MAX_BOUND_SCALE).round() as i32).max(min_height);

    let max_width_cap = default_width
        .saturating_mul(PREVIEW_MAX_DEFAULT_MULTIPLIER)
        .max(min_width);
    let max_height_cap = default_height
        .saturating_mul(PREVIEW_MAX_DEFAULT_MULTIPLIER)
        .max(min_height);

    let max_width = max_width_from_bounds
        .min(max_width_cap)
        .min(bounds_width)
        .max(min_width);
    let max_height = max_height_from_bounds
        .min(max_height_cap)
        .min(bounds_height)
        .max(min_height);

    let source_width_f = f64::from(source_width);
    let source_height_f = f64::from(source_height);
    let lower_scale =
        (f64::from(min_width) / source_width_f).max(f64::from(min_height) / source_height_f);
    let upper_scale =
        (f64::from(max_width) / source_width_f).min(f64::from(max_height) / source_height_f);

    let scale = if lower_scale <= upper_scale {
        if 1.0 < lower_scale {
            lower_scale
        } else if 1.0 > upper_scale {
            upper_scale
        } else {
            1.0
        }
    } else {
        upper_scale.max(0.01)
    };

    let width = (source_width_f * scale).round() as i32;
    let height = (source_height_f * scale).round() as i32;
    let width = width.clamp(min_width, max_width);
    let height = height.clamp(min_height, max_height);

    let source_center_x = i64::from(source.x) + i64::from(source_width) / 2;
    let source_center_y = i64::from(source.y) + i64::from(source_height) / 2;
    let desired_x = source_center_x - i64::from(width) / 2;
    let desired_y = source_center_y - i64::from(height) / 2;

    let min_x = i64::from(bounds.x);
    let min_y = i64::from(bounds.y);
    let max_x = i64::from(bounds.x) + i64::from(bounds_width.saturating_sub(width));
    let max_y = i64::from(bounds.y) + i64::from(bounds_height.saturating_sub(height));

    let x = if max_x >= min_x {
        desired_x.clamp(min_x, max_x) as i32
    } else {
        min_x as i32
    };
    let y = if max_y >= min_y {
        desired_y.clamp(min_y, max_y) as i32
    } else {
        min_y as i32
    };

    PreviewPlacement {
        geometry: PreviewWindowGeometry {
            x,
            y,
            width,
            height,
        },
        min_width,
        min_height,
        max_width,
        max_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_preview_placement_downscales_large_capture_with_center_anchor() {
        let placement = compute_preview_placement(
            PreviewSourceArea {
                x: 0,
                y: 0,
                width: 3840,
                height: 2160,
            },
            PreviewBounds {
                x: 0,
                y: 0,
                width: 3840,
                height: 2160,
            },
            PreviewSizingTokens::default(),
        );

        assert_eq!(placement.geometry.width, 1678);
        assert_eq!(placement.geometry.height, 944);
        assert_eq!(placement.geometry.x, 1081);
        assert_eq!(placement.geometry.y, 608);
        assert_eq!(placement.min_width, MIN_PREVIEW_WIDTH);
        assert_eq!(placement.min_height, MIN_PREVIEW_HEIGHT);
    }

    #[test]
    fn compute_preview_placement_upscales_tiny_capture_to_minimum_policy() {
        let placement = compute_preview_placement(
            PreviewSourceArea {
                x: 100,
                y: 120,
                width: 80,
                height: 40,
            },
            PreviewBounds {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            PreviewSizingTokens::default(),
        );

        assert_eq!(placement.geometry.width, 440);
        assert_eq!(placement.geometry.height, 220);
        assert_eq!(placement.geometry.x, 0);
        assert_eq!(placement.geometry.y, 30);
        assert_eq!(placement.min_width, MIN_PREVIEW_WIDTH);
        assert_eq!(placement.min_height, MIN_PREVIEW_HEIGHT);
    }

    #[test]
    fn compute_preview_placement_preserves_source_size_when_already_reasonable() {
        let placement = compute_preview_placement(
            PreviewSourceArea {
                x: 200,
                y: 100,
                width: 640,
                height: 360,
            },
            PreviewBounds {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            PreviewSizingTokens::default(),
        );

        assert_eq!(
            placement.geometry,
            PreviewWindowGeometry {
                x: 200,
                y: 100,
                width: 640,
                height: 360,
            }
        );
    }

    #[test]
    fn compute_preview_placement_clamps_position_to_bounds_when_near_edges() {
        let placement = compute_preview_placement(
            PreviewSourceArea {
                x: 1700,
                y: 900,
                width: 500,
                height: 300,
            },
            PreviewBounds {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            PreviewSizingTokens::default(),
        );

        assert_eq!(placement.geometry.width, 500);
        assert_eq!(placement.geometry.height, 300);
        assert_eq!(placement.geometry.x, 1420);
        assert_eq!(placement.geometry.y, 780);
    }

    #[test]
    fn compute_preview_placement_relaxes_minimum_on_very_small_bounds() {
        let placement = compute_preview_placement(
            PreviewSourceArea {
                x: 10,
                y: 10,
                width: 80,
                height: 40,
            },
            PreviewBounds {
                x: 0,
                y: 0,
                width: 300,
                height: 180,
            },
            PreviewSizingTokens::default(),
        );

        assert_eq!(placement.min_width, 300);
        assert_eq!(placement.min_height, 180);
        assert_eq!(placement.geometry.width, 300);
        assert_eq!(placement.geometry.height, 180);
        assert_eq!(placement.geometry.x, 0);
        assert_eq!(placement.geometry.y, 0);
    }
}

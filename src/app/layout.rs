use crate::capture;
use crate::preview;
use crate::state::RuntimeWindowGeometry;
use crate::ui::StyleTokens;
use gtk4::prelude::*;
use gtk4::ApplicationWindow;

fn normalize_window_dimension(value: i32, fallback: i32, minimum: i32) -> i32 {
    let normalized = if value > 0 { value } else { fallback };
    normalized.max(minimum)
}

fn monitor_bounds() -> Vec<preview::PreviewBounds> {
    let Some(display) = gtk4::gdk::Display::default() else {
        return Vec::new();
    };
    let monitors = display.monitors();
    let mut bounds_list = Vec::new();

    for index in 0..monitors.n_items() {
        let Some(item) = monitors.item(index) else {
            continue;
        };
        let Ok(monitor) = item.downcast::<gtk4::gdk::Monitor>() else {
            continue;
        };
        let geometry = monitor.geometry();
        bounds_list.push(preview::PreviewBounds {
            x: geometry.x(),
            y: geometry.y(),
            width: geometry.width().max(1),
            height: geometry.height().max(1),
        });
    }

    bounds_list
}

pub(super) fn read_window_geometry(
    window: &ApplicationWindow,
    fallback: RuntimeWindowGeometry,
    minimum: RuntimeWindowGeometry,
) -> RuntimeWindowGeometry {
    let width = if window.width() > 0 {
        window.width()
    } else if window.default_width() > 0 {
        window.default_width()
    } else {
        fallback.width
    };
    let height = if window.height() > 0 {
        window.height()
    } else if window.default_height() > 0 {
        window.default_height()
    } else {
        fallback.height
    };

    RuntimeWindowGeometry::with_position(
        fallback.x,
        fallback.y,
        normalize_window_dimension(width, fallback.width, minimum.width),
        normalize_window_dimension(height, fallback.height, minimum.height),
    )
}

fn monitor_bounds_for_point(x: i32, y: i32) -> Option<preview::PreviewBounds> {
    let bounds_list = monitor_bounds();
    let fallback = bounds_list.first().copied();

    for bounds in bounds_list {
        let max_x = bounds.x.saturating_add(bounds.width);
        let max_y = bounds.y.saturating_add(bounds.height);
        if x >= bounds.x && x < max_x && y >= bounds.y && y < max_y {
            return Some(bounds);
        }
    }

    fallback
}

fn intersection_area(a: preview::PreviewBounds, b: preview::PreviewBounds) -> i64 {
    let left = a.x.max(b.x);
    let right = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let top = a.y.max(b.y);
    let bottom =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    if right <= left || bottom <= top {
        return 0;
    }
    i64::from(right - left) * i64::from(bottom - top)
}

fn distance_sq_point_to_bounds(x: i32, y: i32, bounds: preview::PreviewBounds) -> i64 {
    let max_x = bounds.x.saturating_add(bounds.width).saturating_sub(1);
    let max_y = bounds.y.saturating_add(bounds.height).saturating_sub(1);
    let dx = if x < bounds.x {
        bounds.x - x
    } else if x > max_x {
        x - max_x
    } else {
        0
    };
    let dy = if y < bounds.y {
        bounds.y - y
    } else if y > max_y {
        y - max_y
    } else {
        0
    };
    i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
}

fn clamp_window_geometry_to_bounds(
    window_geometry: RuntimeWindowGeometry,
    bounds: &[preview::PreviewBounds],
) -> Option<RuntimeWindowGeometry> {
    let width = window_geometry.width.max(1);
    let height = window_geometry.height.max(1);
    let window_bounds = preview::PreviewBounds {
        x: window_geometry.x,
        y: window_geometry.y,
        width,
        height,
    };
    let rect_center_x = window_geometry.x.saturating_add(width / 2);
    let rect_center_y = window_geometry.y.saturating_add(height / 2);

    let target_bounds = bounds
        .iter()
        .copied()
        .max_by_key(|item| intersection_area(window_bounds, *item))
        .filter(|item| intersection_area(window_bounds, *item) > 0)
        .or_else(|| {
            bounds
                .iter()
                .copied()
                .min_by_key(|item| distance_sq_point_to_bounds(rect_center_x, rect_center_y, *item))
        })?;

    let max_x = target_bounds
        .x
        .saturating_add(target_bounds.width.saturating_sub(width));
    let max_y = target_bounds
        .y
        .saturating_add(target_bounds.height.saturating_sub(height));
    let x = if width < target_bounds.width {
        window_geometry.x.clamp(target_bounds.x, max_x)
    } else {
        target_bounds.x
    };
    let y = if height < target_bounds.height {
        window_geometry.y.clamp(target_bounds.y, max_y)
    } else {
        target_bounds.y
    };

    Some(RuntimeWindowGeometry::with_position(x, y, width, height))
}

pub(super) fn clamp_window_geometry_to_current_monitors(
    window_geometry: RuntimeWindowGeometry,
) -> Option<RuntimeWindowGeometry> {
    let bounds = monitor_bounds();
    clamp_window_geometry_to_bounds(window_geometry, &bounds)
}

fn fallback_preview_bounds(
    source: preview::PreviewSourceArea,
    style_tokens: StyleTokens,
) -> preview::PreviewBounds {
    preview::PreviewBounds {
        x: source.x,
        y: source.y,
        width: source
            .width
            .max(style_tokens.preview_default_width)
            .max(style_tokens.preview_min_width)
            .max(1),
        height: source
            .height
            .max(style_tokens.preview_default_height)
            .max(style_tokens.preview_min_height)
            .max(1),
    }
}

fn capture_source_area(
    artifact: &capture::CaptureArtifact,
    fallback_width: i32,
    fallback_height: i32,
) -> preview::PreviewSourceArea {
    let source_width = i32::try_from(artifact.screen_width)
        .ok()
        .filter(|value| *value > 0)
        .or_else(|| {
            i32::try_from(artifact.width)
                .ok()
                .filter(|value| *value > 0)
        })
        .unwrap_or(fallback_width.max(1));
    let source_height = i32::try_from(artifact.screen_height)
        .ok()
        .filter(|value| *value > 0)
        .or_else(|| {
            i32::try_from(artifact.height)
                .ok()
                .filter(|value| *value > 0)
        })
        .unwrap_or(fallback_height.max(1));

    preview::PreviewSourceArea {
        x: artifact.screen_x,
        y: artifact.screen_y,
        width: source_width,
        height: source_height,
    }
}

pub(super) fn centered_window_geometry_for_capture(
    artifact: &capture::CaptureArtifact,
    window_geometry: RuntimeWindowGeometry,
) -> (i32, i32, i32, i32) {
    let source = capture_source_area(artifact, window_geometry.width, window_geometry.height);
    let center_x = source.x.saturating_add(source.width / 2);
    let center_y = source.y.saturating_add(source.height / 2);
    centered_window_geometry_for_point(center_x, center_y, window_geometry)
}

pub(super) fn centered_window_geometry_for_point(
    anchor_x: i32,
    anchor_y: i32,
    window_geometry: RuntimeWindowGeometry,
) -> (i32, i32, i32, i32) {
    let width = window_geometry.width.max(1);
    let height = window_geometry.height.max(1);
    let bounds = monitor_bounds_for_point(anchor_x, anchor_y).unwrap_or(preview::PreviewBounds {
        x: anchor_x.saturating_sub(width / 2),
        y: anchor_y.saturating_sub(height / 2),
        width,
        height,
    });

    let x = if bounds.width > width {
        bounds.x.saturating_add((bounds.width - width) / 2)
    } else {
        bounds.x
    };
    let y = if bounds.height > height {
        bounds.y.saturating_add((bounds.height - height) / 2)
    } else {
        bounds.y
    };

    (x, y, width, height)
}

pub(super) fn compute_initial_preview_placement(
    artifact: &capture::CaptureArtifact,
    style_tokens: StyleTokens,
) -> preview::PreviewPlacement {
    let source = capture_source_area(
        artifact,
        style_tokens.preview_default_width,
        style_tokens.preview_default_height,
    );
    let center_x = source.x.saturating_add(source.width / 2);
    let center_y = source.y.saturating_add(source.height / 2);
    let bounds = monitor_bounds_for_point(center_x, center_y)
        .unwrap_or_else(|| fallback_preview_bounds(source, style_tokens));
    preview::compute_preview_placement(
        source,
        bounds,
        preview::PreviewSizingTokens {
            default_width: style_tokens.preview_default_width,
            default_height: style_tokens.preview_default_height,
            min_width: style_tokens.preview_min_width,
            min_height: style_tokens.preview_min_height,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_window_geometry_to_bounds_keeps_intersecting_window_position() {
        let bounds = [preview::PreviewBounds {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        let geometry = RuntimeWindowGeometry::with_position(120, 80, 900, 560);

        assert_eq!(
            clamp_window_geometry_to_bounds(geometry, &bounds),
            Some(geometry)
        );
    }

    #[test]
    fn clamp_window_geometry_to_bounds_moves_offscreen_window_to_nearest_monitor_edge() {
        let bounds = [preview::PreviewBounds {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        let geometry = RuntimeWindowGeometry::with_position(5000, 120, 800, 600);

        assert_eq!(
            clamp_window_geometry_to_bounds(geometry, &bounds),
            Some(RuntimeWindowGeometry::with_position(1120, 120, 800, 600))
        );
    }

    #[test]
    fn clamp_window_geometry_to_bounds_selects_nearest_monitor_when_original_monitor_missing() {
        let bounds = [
            preview::PreviewBounds {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            preview::PreviewBounds {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
            },
        ];
        let geometry = RuntimeWindowGeometry::with_position(5200, 140, 900, 700);

        assert_eq!(
            clamp_window_geometry_to_bounds(geometry, &bounds),
            Some(RuntimeWindowGeometry::with_position(3580, 140, 900, 700))
        );
    }
}

use std::cell::Cell;

use crate::editor;
use gtk4::prelude::*;
use gtk4::{DrawingArea, Label, Scale, ScrolledWindow};

pub(super) const ZOOM_SLIDER_STEPS: f64 = 1000.0;

pub(super) struct EditorViewportRuntime<'a> {
    pub(super) canvas: &'a DrawingArea,
    pub(super) scrolled: &'a ScrolledWindow,
    pub(super) status_label: &'a Label,
    pub(super) zoom_slider: &'a Scale,
    pub(super) zoom_slider_syncing: &'a Cell<bool>,
    pub(super) base_width: i32,
    pub(super) base_height: i32,
}

impl<'a> EditorViewportRuntime<'a> {
    pub(super) fn new(
        canvas: &'a DrawingArea,
        scrolled: &'a ScrolledWindow,
        status_label: &'a Label,
        zoom_slider: &'a Scale,
        zoom_slider_syncing: &'a Cell<bool>,
        base_width: i32,
        base_height: i32,
    ) -> Self {
        Self {
            canvas,
            scrolled,
            status_label,
            zoom_slider,
            zoom_slider_syncing,
            base_width,
            base_height,
        }
    }
}

pub(super) fn set_editor_viewport_status(
    label: &Label,
    viewport: &editor::EditorViewport,
    _canvas: &DrawingArea,
    _base_width: i32,
    _base_height: i32,
) {
    label.set_text(&format!(
        "Viewport: Zoom {}% | Pan ({}, {}) | Hold Space or use Pan tool + Drag",
        viewport.zoom_percent(),
        viewport.pan_x(),
        viewport.pan_y()
    ));
}

pub(super) fn zoom_slider_value_for_percent(zoom_percent: u16) -> f64 {
    let min_zoom = f64::from(editor::EditorViewport::min_zoom_percent());
    let max_zoom = f64::from(editor::EditorViewport::max_zoom_percent());
    if max_zoom <= min_zoom || min_zoom <= 0.0 {
        return 0.0;
    }

    let zoom = f64::from(zoom_percent.clamp(
        editor::EditorViewport::min_zoom_percent(),
        editor::EditorViewport::max_zoom_percent(),
    ));
    let normalized = (zoom / min_zoom).ln() / (max_zoom / min_zoom).ln();
    (normalized.clamp(0.0, 1.0) * ZOOM_SLIDER_STEPS).round()
}

pub(super) fn zoom_percent_from_slider_value(value: f64) -> u16 {
    let min_zoom = f64::from(editor::EditorViewport::min_zoom_percent());
    let max_zoom = f64::from(editor::EditorViewport::max_zoom_percent());
    if max_zoom <= min_zoom || min_zoom <= 0.0 {
        return editor::EditorViewport::min_zoom_percent();
    }

    let normalized = (value / ZOOM_SLIDER_STEPS).clamp(0.0, 1.0);
    let zoom = min_zoom * (max_zoom / min_zoom).powf(normalized);
    (zoom.round() as u16).clamp(
        editor::EditorViewport::min_zoom_percent(),
        editor::EditorViewport::max_zoom_percent(),
    )
}

pub(super) fn sync_editor_zoom_slider(
    slider: &Scale,
    syncing: &Cell<bool>,
    viewport: &editor::EditorViewport,
    _canvas: &DrawingArea,
    _base_width: i32,
    _base_height: i32,
) {
    syncing.set(true);
    slider.set_value(zoom_slider_value_for_percent(viewport.zoom_percent()));
    syncing.set(false);
}

pub(super) fn refresh_editor_viewport_ui(
    runtime: &EditorViewportRuntime<'_>,
    viewport: &editor::EditorViewport,
) {
    set_editor_viewport_status(
        runtime.status_label,
        viewport,
        runtime.canvas,
        runtime.base_width,
        runtime.base_height,
    );
    sync_editor_zoom_slider(
        runtime.zoom_slider,
        runtime.zoom_slider_syncing,
        viewport,
        runtime.canvas,
        runtime.base_width,
        runtime.base_height,
    );
}

pub(super) fn apply_editor_viewport_and_refresh(
    viewport: &mut editor::EditorViewport,
    runtime: &EditorViewportRuntime<'_>,
) {
    apply_editor_viewport_to_canvas(
        runtime.canvas,
        runtime.scrolled,
        viewport,
        runtime.base_width,
        runtime.base_height,
    );
    refresh_editor_viewport_ui(runtime, viewport);
}

fn clamp_adjustment_value(adjustment: &gtk4::Adjustment, value: f64) -> f64 {
    let min = adjustment.lower();
    let max = (adjustment.upper() - adjustment.page_size()).max(min);
    value.clamp(min, max)
}

fn viewport_extent(scrolled: &ScrolledWindow) -> (i32, i32) {
    let page_width = scrolled.hadjustment().page_size().round() as i32;
    let page_height = scrolled.vadjustment().page_size().round() as i32;
    let width = if page_width > 1 {
        page_width
    } else {
        scrolled.allocated_width()
    };
    let height = if page_height > 1 {
        page_height
    } else {
        scrolled.allocated_height()
    };
    (width.max(64), height.max(64))
}

fn fit_zoom_percent_for_extent(
    viewport_width: i32,
    viewport_height: i32,
    base_width: i32,
    base_height: i32,
) -> u16 {
    let fit_scale_w = f64::from(viewport_width.max(1)) / f64::from(base_width.max(1));
    let fit_scale_h = f64::from(viewport_height.max(1)) / f64::from(base_height.max(1));
    let fit_scale = fit_scale_w.min(fit_scale_h).max(0.01);
    let zoom_percent = (fit_scale * 100.0).round() as u16;
    zoom_percent.clamp(
        editor::EditorViewport::min_zoom_percent(),
        editor::EditorViewport::max_zoom_percent(),
    )
}

pub(super) fn fit_zoom_percent_for_window(
    scrolled: &ScrolledWindow,
    base_width: i32,
    base_height: i32,
) -> u16 {
    let (viewport_width, viewport_height) = viewport_extent(scrolled);
    let zoom_percent =
        fit_zoom_percent_for_extent(viewport_width, viewport_height, base_width, base_height);
    tracing::debug!(
        viewport_width,
        viewport_height,
        base_width,
        base_height,
        zoom_percent,
        "computed fit zoom percent for editor viewport"
    );
    zoom_percent
}

fn content_dimensions_for_viewport(
    viewport: &editor::EditorViewport,
    base_width: i32,
    base_height: i32,
) -> (i32, i32) {
    let zoom = f64::from(viewport.zoom_percent()) / 100.0;
    let width = (f64::from(base_width.max(1)) * zoom).round() as i32;
    let height = (f64::from(base_height.max(1)) * zoom).round() as i32;
    (width.max(1), height.max(1))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AxisTrack {
    margin_start: i32,
    margin_end: i32,
    scroll_range: i32,
    centered_scroll: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AxisLayout {
    margin_start: i32,
    margin_end: i32,
    scroll_range: i32,
    scroll_target: f64,
    pan: i32,
}

fn manual_overscroll_extent(viewport_extent: i32) -> i32 {
    (viewport_extent / 2).max(1)
}

fn axis_track(viewport_extent: i32, content_extent: i32) -> AxisTrack {
    let viewport_extent = viewport_extent.max(1);
    let content_extent = content_extent.max(1);
    let centered_slack = (viewport_extent - content_extent).max(0);
    let centered_margin_start = centered_slack / 2;
    let centered_margin_end = centered_slack - centered_margin_start;
    let overscroll = manual_overscroll_extent(viewport_extent);
    let margin_start = centered_margin_start.saturating_add(overscroll);
    let margin_end = centered_margin_end.saturating_add(overscroll);
    let total_extent = content_extent
        .saturating_add(margin_start)
        .saturating_add(margin_end);
    let scroll_range = total_extent.saturating_sub(viewport_extent);
    AxisTrack {
        margin_start,
        margin_end,
        scroll_range,
        centered_scroll: f64::from(scroll_range) / 2.0,
    }
}

fn axis_layout(viewport_extent: i32, content_extent: i32, pan: i32) -> AxisLayout {
    let track = axis_track(viewport_extent, content_extent);
    let scroll_target =
        (track.centered_scroll + f64::from(pan)).clamp(0.0, f64::from(track.scroll_range));
    AxisLayout {
        margin_start: track.margin_start,
        margin_end: track.margin_end,
        scroll_range: track.scroll_range,
        scroll_target,
        pan: (scroll_target - track.centered_scroll).round() as i32,
    }
}

fn anchor_ratio_for_axis(
    adjustment: &gtk4::Adjustment,
    anchor_in_viewport: f64,
    margin_start: i32,
    content_extent: i32,
) -> f64 {
    let content_extent = f64::from(content_extent.max(1));
    let margin_start = f64::from(margin_start.max(0));
    let content_anchor =
        (adjustment.value() + anchor_in_viewport - margin_start).clamp(0.0, content_extent);
    (content_anchor / content_extent).clamp(0.0, 1.0)
}

fn pan_for_anchor_axis(
    viewport_extent: i32,
    content_extent: i32,
    anchor_in_viewport: f64,
    anchor_ratio: f64,
) -> i32 {
    let anchor_in_viewport = anchor_in_viewport.clamp(0.0, f64::from(viewport_extent.max(1)));
    let content_extent = content_extent.max(1);
    let track = axis_track(viewport_extent, content_extent);
    let content_anchor = anchor_ratio.clamp(0.0, 1.0) * f64::from(content_extent);
    let desired_scroll = f64::from(track.margin_start) + content_anchor - anchor_in_viewport;
    let clamped_scroll = desired_scroll.clamp(0.0, f64::from(track.scroll_range));
    (clamped_scroll - track.centered_scroll).round() as i32
}

pub(super) fn apply_editor_viewport_to_canvas(
    canvas: &DrawingArea,
    scrolled: &ScrolledWindow,
    viewport: &mut editor::EditorViewport,
    base_width: i32,
    base_height: i32,
) {
    let (viewport_width, viewport_height) = viewport_extent(scrolled);
    let (content_width, content_height) =
        content_dimensions_for_viewport(viewport, base_width, base_height);
    canvas.set_content_width(content_width);
    canvas.set_content_height(content_height);

    let horizontal = axis_layout(viewport_width, content_width, viewport.pan_x());
    let vertical = axis_layout(viewport_height, content_height, viewport.pan_y());
    canvas.set_margin_start(horizontal.margin_start);
    canvas.set_margin_end(horizontal.margin_end);
    canvas.set_margin_top(vertical.margin_start);
    canvas.set_margin_bottom(vertical.margin_end);
    canvas.queue_resize();
    canvas.queue_draw();

    let hadjustment = scrolled.hadjustment();
    hadjustment.set_value(clamp_adjustment_value(
        &hadjustment,
        horizontal.scroll_target,
    ));
    let vadjustment = scrolled.vadjustment();
    vadjustment.set_value(clamp_adjustment_value(&vadjustment, vertical.scroll_target));

    viewport.set_pan(horizontal.pan, vertical.pan);
}

pub(super) fn apply_fit_zoom_once(
    viewport: &mut editor::EditorViewport,
    runtime: &EditorViewportRuntime<'_>,
    reason: &'static str,
) {
    tracing::debug!(
        reason,
        current_zoom = viewport.zoom_percent(),
        current_pan_x = viewport.pan_x(),
        current_pan_y = viewport.pan_y(),
        allocated_width = runtime.scrolled.allocated_width(),
        allocated_height = runtime.scrolled.allocated_height(),
        base_width = runtime.base_width,
        base_height = runtime.base_height,
        "applying one-shot fit zoom for editor viewport"
    );
    let zoom_percent =
        fit_zoom_percent_for_window(runtime.scrolled, runtime.base_width, runtime.base_height);
    viewport.set_zoom_percent(zoom_percent);
    viewport.set_pan(0, 0);
    apply_editor_viewport_to_canvas(
        runtime.canvas,
        runtime.scrolled,
        viewport,
        runtime.base_width,
        runtime.base_height,
    );
    let hadjustment = runtime.scrolled.hadjustment();
    let vadjustment = runtime.scrolled.vadjustment();
    tracing::debug!(
        reason,
        applied_zoom = viewport.zoom_percent(),
        applied_pan_x = viewport.pan_x(),
        applied_pan_y = viewport.pan_y(),
        content_width = runtime.canvas.content_width(),
        content_height = runtime.canvas.content_height(),
        margin_start = runtime.canvas.margin_start(),
        margin_end = runtime.canvas.margin_end(),
        margin_top = runtime.canvas.margin_top(),
        margin_bottom = runtime.canvas.margin_bottom(),
        hadjustment_value = hadjustment.value(),
        hadjustment_upper = hadjustment.upper(),
        hadjustment_page_size = hadjustment.page_size(),
        vadjustment_value = vadjustment.value(),
        vadjustment_upper = vadjustment.upper(),
        vadjustment_page_size = vadjustment.page_size(),
        "applied one-shot fit zoom for editor viewport"
    );
    refresh_editor_viewport_ui(runtime, viewport);
}

pub(super) fn scroller_center_anchor(scrolled: &ScrolledWindow) -> (f64, f64) {
    (
        f64::from(scrolled.allocated_width().max(1)) / 2.0,
        f64::from(scrolled.allocated_height().max(1)) / 2.0,
    )
}

fn apply_zoom_with_anchor(
    viewport: &mut editor::EditorViewport,
    runtime: &EditorViewportRuntime<'_>,
    anchor_x: f64,
    anchor_y: f64,
    mut apply_zoom: impl FnMut(&mut editor::EditorViewport),
) {
    let previous_width = runtime.canvas.content_width().max(1);
    let previous_height = runtime.canvas.content_height().max(1);
    let (viewport_width, viewport_height) = viewport_extent(runtime.scrolled);
    let hadjustment = runtime.scrolled.hadjustment();
    let vadjustment = runtime.scrolled.vadjustment();
    let anchor_x = anchor_x.clamp(0.0, f64::from(viewport_width.max(1)));
    let anchor_y = anchor_y.clamp(0.0, f64::from(viewport_height.max(1)));
    let anchor_ratio_x = anchor_ratio_for_axis(
        &hadjustment,
        anchor_x,
        runtime.canvas.margin_start(),
        previous_width,
    );
    let anchor_ratio_y = anchor_ratio_for_axis(
        &vadjustment,
        anchor_y,
        runtime.canvas.margin_top(),
        previous_height,
    );

    apply_zoom(viewport);

    let (next_width, next_height) =
        content_dimensions_for_viewport(viewport, runtime.base_width, runtime.base_height);
    let pan_x = pan_for_anchor_axis(viewport_width, next_width, anchor_x, anchor_ratio_x);
    let pan_y = pan_for_anchor_axis(viewport_height, next_height, anchor_y, anchor_ratio_y);
    viewport.set_pan(pan_x, pan_y);
    apply_editor_viewport_to_canvas(
        runtime.canvas,
        runtime.scrolled,
        viewport,
        runtime.base_width,
        runtime.base_height,
    );
}

pub(super) fn zoom_editor_viewport_with_anchor(
    viewport: &mut editor::EditorViewport,
    zoom_in: bool,
    runtime: &EditorViewportRuntime<'_>,
    anchor_x: f64,
    anchor_y: f64,
) {
    apply_zoom_with_anchor(viewport, runtime, anchor_x, anchor_y, |viewport| {
        if zoom_in {
            viewport.zoom_in();
        } else {
            viewport.zoom_out();
        }
    });
}

pub(super) fn zoom_editor_viewport_and_refresh(
    viewport: &mut editor::EditorViewport,
    zoom_in: bool,
    runtime: &EditorViewportRuntime<'_>,
    anchor_x: f64,
    anchor_y: f64,
) {
    zoom_editor_viewport_with_anchor(viewport, zoom_in, runtime, anchor_x, anchor_y);
    refresh_editor_viewport_ui(runtime, viewport);
}

pub(super) fn set_editor_zoom_percent_with_anchor(
    viewport: &mut editor::EditorViewport,
    zoom_percent: u16,
    runtime: &EditorViewportRuntime<'_>,
    anchor_x: f64,
    anchor_y: f64,
) {
    apply_zoom_with_anchor(viewport, runtime, anchor_x, anchor_y, |viewport| {
        viewport.set_zoom_percent(zoom_percent);
    });
}

pub(super) fn set_editor_zoom_percent_and_refresh(
    viewport: &mut editor::EditorViewport,
    zoom_percent: u16,
    runtime: &EditorViewportRuntime<'_>,
    anchor_x: f64,
    anchor_y: f64,
) {
    set_editor_zoom_percent_with_anchor(viewport, zoom_percent, runtime, anchor_x, anchor_y);
    refresh_editor_viewport_ui(runtime, viewport);
}

pub(super) fn set_editor_actual_size_and_refresh(
    viewport: &mut editor::EditorViewport,
    runtime: &EditorViewportRuntime<'_>,
) {
    viewport.set_actual_size();
    apply_editor_viewport_and_refresh(viewport, runtime);
}

#[cfg(test)]
mod tests {
    use super::{axis_layout, fit_zoom_percent_for_extent, pan_for_anchor_axis};

    #[test]
    fn fit_zoom_percent_for_extent_uses_smallest_axis_ratio() {
        assert_eq!(fit_zoom_percent_for_extent(800, 600, 1600, 1200), 50);
        assert_eq!(fit_zoom_percent_for_extent(400, 1200, 1600, 900), 25);
    }

    #[test]
    fn axis_layout_for_scrollable_content_allows_manual_overscroll() {
        let centered = axis_layout(100, 200, 0);
        assert_eq!(centered.margin_start, 50);
        assert_eq!(centered.margin_end, 50);
        assert_eq!(centered.scroll_target, 100.0);
        assert_eq!(centered.pan, 0);

        let right = axis_layout(100, 200, 40);
        assert_eq!(right.scroll_target, 140.0);
        assert_eq!(right.pan, 40);

        let clamped = axis_layout(100, 200, 10_000);
        assert_eq!(clamped.scroll_target, 200.0);
        assert_eq!(clamped.pan, 100);
    }

    #[test]
    fn axis_layout_for_non_scrollable_content_uses_scroll_track() {
        let layout = axis_layout(200, 100, 24);
        assert_eq!(layout.margin_start, 150);
        assert_eq!(layout.margin_end, 150);
        assert_eq!(layout.pan, 24);
        assert_eq!(layout.scroll_target, 124.0);
    }

    #[test]
    fn pan_for_anchor_axis_keeps_anchor_for_scrollable_content() {
        assert_eq!(pan_for_anchor_axis(100, 200, 50.0, 0.5), 0);
        assert_eq!(pan_for_anchor_axis(100, 200, 50.0, 0.75), 50);
    }

    #[test]
    fn pan_for_anchor_axis_keeps_anchor_for_non_scrollable_content() {
        assert_eq!(pan_for_anchor_axis(200, 100, 100.0, 0.5), 0);
        assert_eq!(pan_for_anchor_axis(200, 100, 150.0, 0.5), -50);
    }
}

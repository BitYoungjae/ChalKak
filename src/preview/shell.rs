use std::time::{Duration, Instant};

use super::geometry::{
    DEFAULT_PREVIEW_HEIGHT, DEFAULT_PREVIEW_WIDTH, DEFAULT_PREVIEW_X, DEFAULT_PREVIEW_Y,
    MIN_PREVIEW_HEIGHT, MIN_PREVIEW_WIDTH,
};
use super::PreviewWindowGeometry;

// Combined with the 120ms revealer transition, this targets ~600ms perceived hide timing.
const PREVIEW_CONTROL_HIDE_DELAY: Duration = Duration::from_millis(480);
const MIN_PREVIEW_TRANSPARENCY: f32 = 0.2;
const MAX_PREVIEW_TRANSPARENCY: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct PreviewWindowShell {
    geometry: PreviewWindowGeometry,
    controls_visible: bool,
    hover_depth: usize,
    controls_hide_at: Option<Instant>,
    transparency: f32,
}

impl PreviewWindowShell {
    pub fn with_capture_size(width: u32, height: u32) -> Self {
        let (width, height) = initial_preview_size(width, height);

        Self {
            geometry: PreviewWindowGeometry {
                x: DEFAULT_PREVIEW_X,
                y: DEFAULT_PREVIEW_Y,
                width,
                height,
            },
            controls_visible: false,
            hover_depth: 0,
            controls_hide_at: None,
            transparency: 1.0,
        }
    }

    pub fn geometry(&self) -> PreviewWindowGeometry {
        self.geometry
    }

    pub fn set_geometry(&mut self, geometry: PreviewWindowGeometry) {
        self.geometry = geometry;
    }

    pub fn controls_visible(&self) -> bool {
        self.controls_visible
    }

    pub fn transparency(&self) -> f32 {
        self.transparency
    }

    pub fn set_transparency(&mut self, value: f32) {
        self.transparency = value.clamp(MIN_PREVIEW_TRANSPARENCY, MAX_PREVIEW_TRANSPARENCY);
    }

    pub fn hover_enter(&mut self, _now: Instant) {
        self.hover_depth += 1;
        self.controls_visible = true;
        self.controls_hide_at = None;
    }

    pub fn hover_exit(&mut self, now: Instant) {
        if self.hover_depth == 0 {
            return;
        }
        self.hover_depth -= 1;
        if self.hover_depth == 0 {
            self.controls_hide_at = Some(now + PREVIEW_CONTROL_HIDE_DELAY);
        }
    }

    pub fn update_hover_controls_visibility(&mut self, now: Instant) {
        if let Some(deadline) = self.controls_hide_at {
            if now >= deadline {
                self.controls_visible = false;
                self.controls_hide_at = None;
            }
        }
    }
}

#[cfg(test)]
impl PreviewWindowShell {
    fn new() -> Self {
        Self::with_capture_size(0, 0)
    }

    fn hover_depth(&self) -> usize {
        self.hover_depth
    }
}

fn initial_preview_size(source_width: u32, source_height: u32) -> (i32, i32) {
    if source_width == 0 || source_height == 0 {
        return (DEFAULT_PREVIEW_WIDTH, DEFAULT_PREVIEW_HEIGHT);
    }

    let source_width = i32::try_from(source_width).unwrap_or(DEFAULT_PREVIEW_WIDTH);
    let source_height = i32::try_from(source_height).unwrap_or(DEFAULT_PREVIEW_HEIGHT);
    let target_width = source_width.clamp(MIN_PREVIEW_WIDTH, DEFAULT_PREVIEW_WIDTH) as f64;
    let ratio = source_height as f64 / source_width as f64;
    let target_height = (target_width * ratio)
        .round()
        .max(MIN_PREVIEW_HEIGHT as f64) as i32;
    (target_width as i32, target_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_window_shell_defaults_to_expected_geometry() {
        let shell = PreviewWindowShell::new();
        assert_eq!(
            shell.geometry(),
            PreviewWindowGeometry {
                x: DEFAULT_PREVIEW_X,
                y: DEFAULT_PREVIEW_Y,
                width: DEFAULT_PREVIEW_WIDTH,
                height: DEFAULT_PREVIEW_HEIGHT,
            }
        );
        assert!(!shell.controls_visible());
        assert_eq!(shell.hover_depth(), 0);
        assert_eq!(shell.transparency(), 1.0);
    }

    #[test]
    fn preview_hover_controls_show_and_hide_with_delay() {
        let mut shell = PreviewWindowShell::new();
        assert!(!shell.controls_visible());

        let now = Instant::now();
        shell.hover_enter(now);
        assert!(shell.controls_visible());
        assert_eq!(shell.hover_depth(), 1);

        shell.hover_exit(now);
        assert_eq!(shell.hover_depth(), 0);
        assert!(shell.controls_visible());

        shell.update_hover_controls_visibility(now + Duration::from_millis(479));
        assert!(shell.controls_visible());

        shell.update_hover_controls_visibility(now + Duration::from_millis(481));
        assert!(!shell.controls_visible());
    }

    #[test]
    fn preview_hover_controls_reappear_when_hovered_during_delay() {
        let mut shell = PreviewWindowShell::new();
        let now = Instant::now();

        shell.hover_enter(now);
        shell.hover_exit(now + Duration::from_millis(10));
        shell.update_hover_controls_visibility(now + Duration::from_millis(450));
        assert!(shell.controls_visible());

        shell.hover_enter(now + Duration::from_millis(450));
        assert!(shell.controls_visible());
        shell.update_hover_controls_visibility(now + Duration::from_millis(1_200));
        assert!(shell.controls_visible());
    }

    #[test]
    fn preview_transparency_slider_clamps_to_range() {
        let mut shell = PreviewWindowShell::new();
        shell.set_transparency(0.42);
        assert_eq!(shell.transparency(), 0.42);
        shell.set_transparency(-0.5);
        assert_eq!(shell.transparency(), MIN_PREVIEW_TRANSPARENCY);
        shell.set_transparency(1.5);
        assert_eq!(shell.transparency(), MAX_PREVIEW_TRANSPARENCY);
    }
}

//! Editor shell layout and panel behavior models.

pub mod tools;

use crate::clipboard::ClipboardError;
use crate::storage::StorageError;
use thiserror::Error;

pub use tools::{EditorTools, ToolError, ToolKind, ToolObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorViewport {
    zoom_percent: u16,
    pan_x: i32,
    pan_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EditorInputMode {
    crop_active: bool,
    text_input_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    Save,
    Copy,
    CloseRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    Save { capture_id: String },
    Copy { capture_id: String },
    CloseRequested { capture_id: String },
}

#[derive(Debug, Error)]
pub enum EditorActionError {
    #[error("storage error while {operation} {capture_id}: {source}")]
    StorageError {
        operation: &'static str,
        capture_id: String,
        #[source]
        source: StorageError,
    },

    #[error("clipboard error while {operation} {capture_id}: {source}")]
    ClipboardError {
        operation: &'static str,
        capture_id: String,
        #[source]
        source: ClipboardError,
    },
}

const VIEWPORT_ZOOM_MIN_PERCENT: u16 = 1;
const VIEWPORT_ZOOM_MAX_PERCENT: u16 = 1600;
const VIEWPORT_ZOOM_LEVELS_PERCENT: &[u16] = &[
    1, 2, 3, 4, 5, 8, 10, 12, 16, 20, 25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200,
    250, 300, 400, 500, 600, 800, 1000, 1200, 1600,
];
fn clamp_zoom_percent(zoom_percent: u16) -> u16 {
    zoom_percent.clamp(VIEWPORT_ZOOM_MIN_PERCENT, VIEWPORT_ZOOM_MAX_PERCENT)
}

fn next_zoom_in_level(current_zoom_percent: u16) -> u16 {
    for &level in VIEWPORT_ZOOM_LEVELS_PERCENT {
        if level > current_zoom_percent {
            return level;
        }
    }
    VIEWPORT_ZOOM_MAX_PERCENT
}

fn next_zoom_out_level(current_zoom_percent: u16) -> u16 {
    for &level in VIEWPORT_ZOOM_LEVELS_PERCENT.iter().rev() {
        if level < current_zoom_percent {
            return level;
        }
    }
    VIEWPORT_ZOOM_MIN_PERCENT
}

impl Default for EditorViewport {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorViewport {
    pub const fn new() -> Self {
        Self {
            zoom_percent: 100,
            pan_x: 0,
            pan_y: 0,
        }
    }

    pub const fn zoom_percent(&self) -> u16 {
        self.zoom_percent
    }

    pub const fn pan_x(&self) -> i32 {
        self.pan_x
    }

    pub const fn pan_y(&self) -> i32 {
        self.pan_y
    }

    pub const fn min_zoom_percent() -> u16 {
        VIEWPORT_ZOOM_MIN_PERCENT
    }

    pub const fn max_zoom_percent() -> u16 {
        VIEWPORT_ZOOM_MAX_PERCENT
    }

    pub fn zoom_in(&mut self) {
        self.zoom_percent = next_zoom_in_level(clamp_zoom_percent(self.zoom_percent));
    }

    pub fn zoom_out(&mut self) {
        self.zoom_percent = next_zoom_out_level(clamp_zoom_percent(self.zoom_percent));
    }

    pub fn set_zoom_percent(&mut self, zoom_percent: u16) {
        self.zoom_percent = clamp_zoom_percent(zoom_percent);
    }

    pub fn set_actual_size(&mut self) {
        self.zoom_percent = 100;
        self.pan_x = 0;
        self.pan_y = 0;
    }

    pub fn pan_by(&mut self, delta_x: i32, delta_y: i32) {
        if delta_x == 0 && delta_y == 0 {
            return;
        }
        self.pan_x = self.pan_x.saturating_add(delta_x);
        self.pan_y = self.pan_y.saturating_add(delta_y);
    }

    pub(crate) fn set_pan(&mut self, pan_x: i32, pan_y: i32) {
        self.pan_x = pan_x;
        self.pan_y = pan_y;
    }
}

#[cfg(test)]
impl EditorViewport {
    fn pan_right(&mut self) {
        self.pan_by(48, 0);
    }

    fn pan_down(&mut self) {
        self.pan_by(0, 48);
    }
}

impl EditorInputMode {
    pub const fn new() -> Self {
        Self {
            crop_active: false,
            text_input_active: false,
        }
    }

    pub fn reset(&mut self) {
        self.crop_active = false;
        self.text_input_active = false;
    }

    pub fn activate_crop(&mut self) {
        self.crop_active = true;
        self.text_input_active = false;
    }

    pub fn deactivate_crop(&mut self) {
        self.crop_active = false;
    }

    pub fn start_text_input(&mut self) {
        self.text_input_active = true;
        self.crop_active = false;
    }

    pub fn end_text_input(&mut self) {
        self.text_input_active = false;
    }

    pub const fn crop_active(&self) -> bool {
        self.crop_active
    }

    pub const fn text_input_active(&self) -> bool {
        self.text_input_active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_viewport_defaults_to_100_percent_and_origin() {
        let viewport = EditorViewport::new();
        assert_eq!(viewport.zoom_percent(), 100);
        assert_eq!(viewport.pan_x(), 0);
        assert_eq!(viewport.pan_y(), 0);
    }

    #[test]
    fn editor_viewport_zoom_controls_clamp() {
        let mut viewport = EditorViewport::new();
        viewport.zoom_out();
        assert_eq!(viewport.zoom_percent(), 90);

        for _ in 0..100 {
            viewport.zoom_out();
        }
        assert_eq!(viewport.zoom_percent(), 1);

        for _ in 0..200 {
            viewport.zoom_in();
        }
        assert_eq!(viewport.zoom_percent(), 1600);
    }

    #[test]
    fn editor_viewport_zoom_steps_follow_level_ladder_for_non_level_values() {
        let mut viewport = EditorViewport::new();
        viewport.set_zoom_percent(137);
        viewport.zoom_in();
        assert_eq!(viewport.zoom_percent(), 150);

        viewport.set_zoom_percent(137);
        viewport.zoom_out();
        assert_eq!(viewport.zoom_percent(), 125);
    }

    #[test]
    fn editor_viewport_pan_tracks_offsets() {
        let mut viewport = EditorViewport::new();
        viewport.pan_right();
        viewport.pan_down();
        assert_eq!(viewport.pan_x(), 48);
        assert_eq!(viewport.pan_y(), 48);
    }

    #[test]
    fn editor_viewport_actual_size_resets_offsets() {
        let mut viewport = EditorViewport::new();
        viewport.zoom_in();
        viewport.pan_by(120, -30);
        assert_eq!(viewport.zoom_percent(), 110);
        assert_eq!(viewport.pan_x(), 120);
        assert_eq!(viewport.pan_y(), -30);

        viewport.set_actual_size();
        assert_eq!(viewport.zoom_percent(), 100);
        assert_eq!(viewport.pan_x(), 0);
        assert_eq!(viewport.pan_y(), 0);
    }

    #[test]
    fn editor_input_mode_applies_crop_text_exclusivity_and_reset() {
        let mut mode = EditorInputMode::new();
        assert!(!mode.crop_active());
        assert!(!mode.text_input_active());

        mode.activate_crop();
        assert!(mode.crop_active());
        assert!(!mode.text_input_active());

        mode.start_text_input();
        assert!(!mode.crop_active());
        assert!(mode.text_input_active());

        mode.end_text_input();
        assert!(!mode.text_input_active());

        mode.activate_crop();
        mode.reset();
        assert!(!mode.crop_active());
        assert!(!mode.text_input_active());
    }
}

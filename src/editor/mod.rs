//! Editor shell layout and panel behavior models.

pub mod model;
pub mod tools;

use crate::capture::CaptureArtifact;
use crate::clipboard::{ClipboardBackend, ClipboardError};
use crate::storage::{CaptureStorage, StorageError};
use thiserror::Error;

pub use model::{EditorDocument, EditorOperation, EditorOperationModel, SelectionState};
pub use tools::{EditorTools, ToolError, ToolKind, ToolObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorPane {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsPanelState {
    Expanded,
    Collapsed,
}

impl OptionsPanelState {
    const fn width(self) -> u32 {
        match self {
            Self::Expanded => EDITOR_OPTIONS_PANEL_WIDTH_EXPANDED,
            Self::Collapsed => EDITOR_OPTIONS_PANEL_WIDTH_COLLAPSED,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorLayout {
    pub toolbar: EditorPane,
    pub canvas: EditorPane,
    pub options: EditorPane,
    pub options_state: OptionsPanelState,
}

#[derive(Debug)]
pub struct EditorFrame {
    options_state: OptionsPanelState,
}

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
    CopyFileReference,
    CloseRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    Save { capture_id: String },
    Copy { capture_id: String },
    CopyFileReference { capture_id: String },
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

const EDITOR_TOOLBAR_WIDTH: u32 = 68;
const EDITOR_OPTIONS_PANEL_WIDTH_EXPANDED: u32 = 320;
const EDITOR_OPTIONS_PANEL_WIDTH_COLLAPSED: u32 = 0;
const EDITOR_MIN_CANVAS_WIDTH: u32 = 320;
const VIEWPORT_ZOOM_MIN_PERCENT: u16 = 1;
const VIEWPORT_ZOOM_MAX_PERCENT: u16 = 1600;
const VIEWPORT_ZOOM_LEVELS_PERCENT: &[u16] = &[
    1, 2, 3, 4, 5, 8, 10, 12, 16, 20, 25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200,
    250, 300, 400, 500, 600, 800, 1000, 1200, 1600,
];
const VIEWPORT_PAN_STEP_PX: i32 = 48;

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

impl Default for EditorFrame {
    fn default() -> Self {
        Self {
            options_state: OptionsPanelState::Expanded,
        }
    }
}

impl Default for EditorViewport {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorFrame {
    pub const fn new() -> Self {
        Self {
            options_state: OptionsPanelState::Expanded,
        }
    }

    pub fn options_panel_state(&self) -> OptionsPanelState {
        self.options_state
    }

    pub fn is_toolbar_visible(&self) -> bool {
        true
    }

    pub fn is_canvas_visible(&self) -> bool {
        true
    }

    pub fn is_options_panel_visible(&self) -> bool {
        true
    }

    pub fn toggle_options_panel(&mut self) {
        self.options_state = match self.options_state {
            OptionsPanelState::Expanded => OptionsPanelState::Collapsed,
            OptionsPanelState::Collapsed => OptionsPanelState::Expanded,
        };
    }

    pub fn open_session(&mut self) {
        self.options_state = OptionsPanelState::Expanded;
    }

    pub fn layout(&self, window_width: u32, window_height: u32) -> EditorLayout {
        let toolbar_width = EDITOR_TOOLBAR_WIDTH.min(window_width);
        let available_for_center_and_options = window_width.saturating_sub(toolbar_width);

        let mut options_width = self.options_state.width();
        if available_for_center_and_options < EDITOR_MIN_CANVAS_WIDTH + options_width {
            options_width =
                available_for_center_and_options.saturating_sub(EDITOR_MIN_CANVAS_WIDTH);
        }
        if toolbar_width == window_width {
            options_width = 0;
        }

        let canvas_width = available_for_center_and_options.saturating_sub(options_width);

        let toolbar = EditorPane {
            x: 0,
            y: 0,
            width: toolbar_width,
            height: window_height,
        };

        let canvas = EditorPane {
            x: toolbar.width,
            y: 0,
            width: canvas_width,
            height: window_height,
        };

        let options = EditorPane {
            x: toolbar.width + canvas.width,
            y: 0,
            width: options_width,
            height: window_height,
        };

        EditorLayout {
            toolbar,
            canvas,
            options,
            options_state: self.options_state,
        }
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

    pub fn pan_left(&mut self) {
        self.pan_by(-VIEWPORT_PAN_STEP_PX, 0);
    }

    pub fn pan_right(&mut self) {
        self.pan_by(VIEWPORT_PAN_STEP_PX, 0);
    }

    pub fn pan_up(&mut self) {
        self.pan_by(0, -VIEWPORT_PAN_STEP_PX);
    }

    pub fn pan_down(&mut self) {
        self.pan_by(0, VIEWPORT_PAN_STEP_PX);
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

pub fn execute_editor_action<S: CaptureStorage, C: ClipboardBackend>(
    artifact: &CaptureArtifact,
    action: EditorAction,
    storage: &S,
    clipboard: &C,
) -> Result<EditorEvent, EditorActionError> {
    let capture_id = artifact.capture_id.clone();
    match action {
        EditorAction::Save => {
            storage
                .save_capture(artifact)
                .map_err(|err| EditorActionError::StorageError {
                    operation: "save",
                    capture_id: capture_id.clone(),
                    source: err,
                })?;
            Ok(EditorEvent::Save { capture_id })
        }
        EditorAction::Copy => {
            clipboard
                .copy_png_file(&artifact.temp_path)
                .map_err(|err| EditorActionError::ClipboardError {
                    operation: "copy",
                    capture_id: capture_id.clone(),
                    source: err,
                })?;
            Ok(EditorEvent::Copy { capture_id })
        }
        EditorAction::CopyFileReference => {
            clipboard
                .copy_file_reference(&artifact.temp_path)
                .map_err(|err| EditorActionError::ClipboardError {
                    operation: "copy file reference",
                    capture_id: capture_id.clone(),
                    source: err,
                })?;
            Ok(EditorEvent::CopyFileReference { capture_id })
        }
        EditorAction::CloseRequested => Ok(EditorEvent::CloseRequested { capture_id }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::ClipboardResult;
    use crate::storage::StorageResult;
    use std::path::PathBuf;

    #[test]
    fn editor_frame_starts_with_options_open_and_panels_visible() {
        let frame = EditorFrame::new();
        assert!(frame.is_toolbar_visible());
        assert!(frame.is_canvas_visible());
        assert!(frame.is_options_panel_visible());
        assert_eq!(frame.options_panel_state(), OptionsPanelState::Expanded);
    }

    #[test]
    fn editor_frame_options_panel_can_toggle_and_reopen_resets_expanded() {
        let mut frame = EditorFrame::new();
        frame.toggle_options_panel();
        assert_eq!(frame.options_panel_state(), OptionsPanelState::Collapsed);

        frame.open_session();
        assert_eq!(frame.options_panel_state(), OptionsPanelState::Expanded);
    }

    #[test]
    fn editor_frame_layout_orders_left_toolbar_canvas_right_options() {
        let mut frame = EditorFrame::new();
        let layout = frame.layout(1280, 720);

        assert_eq!(layout.toolbar.x, 0);
        assert_eq!(layout.canvas.x, EDITOR_TOOLBAR_WIDTH);
        assert_eq!(layout.options.x, EDITOR_TOOLBAR_WIDTH + layout.canvas.width);
        assert_eq!(layout.toolbar.y, 0);
        assert_eq!(layout.options.y, 0);
        assert_eq!(layout.toolbar.height, 720);
        assert_eq!(layout.options.height, 720);
        assert_eq!(
            layout.toolbar.width + layout.canvas.width + layout.options.width,
            1280
        );

        frame.toggle_options_panel();
        let collapsed = frame.layout(1280, 720);
        assert!(collapsed.options.width < layout.options.width);
        assert_eq!(
            collapsed.toolbar.width + collapsed.canvas.width + collapsed.options.width,
            1280
        );
    }

    #[test]
    fn editor_frame_forces_options_open_on_reopen_without_disabling_panels() {
        let mut frame = EditorFrame::new();
        frame.toggle_options_panel();
        frame.open_session();
        let layout = frame.layout(640, 360);

        assert!(layout.options.width > 0);
        assert!(layout.canvas.width > 0);
        assert_eq!(layout.options_state, OptionsPanelState::Expanded);
    }

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

    struct MockStorage;
    impl CaptureStorage for MockStorage {
        fn save_capture(&self, artifact: &CaptureArtifact) -> StorageResult<PathBuf> {
            Ok(PathBuf::from(format!("/tmp/{}.png", artifact.capture_id)))
        }

        fn discard_session_artifacts(&self, _capture_id: &str) -> StorageResult<()> {
            Ok(())
        }
    }

    struct MockClipboard;
    impl ClipboardBackend for MockClipboard {
        fn copy_png_file(&self, _path: &std::path::Path) -> ClipboardResult<()> {
            Ok(())
        }

        fn copy_file_reference(&self, _path: &std::path::Path) -> ClipboardResult<()> {
            Ok(())
        }
    }

    fn test_artifact(capture_id: &str) -> CaptureArtifact {
        CaptureArtifact {
            capture_id: capture_id.to_string(),
            temp_path: PathBuf::from(format!("/tmp/{capture_id}.png")),
            width: 320,
            height: 180,
            screen_x: 0,
            screen_y: 0,
            screen_width: 320,
            screen_height: 180,
            created_at: 0,
        }
    }

    #[test]
    fn editor_execute_action_save_maps_to_save_event() {
        let artifact = test_artifact("one");
        let event =
            execute_editor_action(&artifact, EditorAction::Save, &MockStorage, &MockClipboard)
                .unwrap();
        assert_eq!(
            event,
            EditorEvent::Save {
                capture_id: "one".to_string()
            }
        );
    }

    #[test]
    fn editor_execute_action_copy_maps_to_copy_event() {
        let artifact = test_artifact("one");
        let event =
            execute_editor_action(&artifact, EditorAction::Copy, &MockStorage, &MockClipboard)
                .unwrap();
        assert_eq!(
            event,
            EditorEvent::Copy {
                capture_id: "one".to_string()
            }
        );
    }

    #[test]
    fn editor_execute_action_copy_file_reference_maps_to_event() {
        let artifact = test_artifact("one");
        let event = execute_editor_action(
            &artifact,
            EditorAction::CopyFileReference,
            &MockStorage,
            &MockClipboard,
        )
        .unwrap();
        assert_eq!(
            event,
            EditorEvent::CopyFileReference {
                capture_id: "one".to_string()
            }
        );
    }

    #[test]
    fn editor_execute_action_close_requested_maps_to_close_event() {
        let artifact = test_artifact("one");
        let event = execute_editor_action(
            &artifact,
            EditorAction::CloseRequested,
            &MockStorage,
            &MockClipboard,
        )
        .unwrap();
        assert_eq!(
            event,
            EditorEvent::CloseRequested {
                capture_id: "one".to_string()
            }
        );
    }
}

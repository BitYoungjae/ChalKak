use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::capture;
use crate::editor::tools::{CropElement, ImageBounds, ToolPoint};
use crate::editor::{self, EditorAction, ToolKind};
use crate::storage::StorageService;

use super::ToastRuntime;

mod geometry;
mod interaction;
mod output;
mod render;

pub(super) use geometry::*;
pub(super) use interaction::*;
pub(super) use output::*;
pub(super) use render::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ToolDragPreview {
    pub(super) tool: ToolKind,
    pub(super) start: ToolPoint,
    pub(super) current: ToolPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RectangleHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResizableObjectKind {
    Rectangle,
    Blur,
    Crop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ObjectDragState {
    Move {
        object_ids: Vec<u64>,
        last: ToolPoint,
    },
    ResizeObject {
        object_id: u64,
        kind: ResizableObjectKind,
        handle: RectangleHandle,
    },
    MovePendingCrop {
        start: ToolPoint,
        origin: CropElement,
    },
    ResizePendingCrop {
        handle: RectangleHandle,
        origin: CropElement,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct TextPreeditState {
    pub(super) content: String,
    pub(super) cursor_chars: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TextCaretLayout {
    pub(super) caret_x: f64,
    pub(super) caret_top: f64,
    pub(super) caret_bottom: f64,
    pub(super) baseline_y: f64,
    pub(super) preedit_start_x: Option<f64>,
    pub(super) preedit_end_x: Option<f64>,
}

impl TextCaretLayout {
    pub(super) fn caret_height(self) -> f64 {
        (self.caret_bottom - self.caret_top).max(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BlurRenderKey {
    source_width: i32,
    source_height: i32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    intensity: u8,
}

#[derive(Debug, Clone)]
pub(super) struct BlurRenderEntry {
    key: BlurRenderKey,
    surface: gtk4::cairo::ImageSurface,
}

#[derive(Debug, Default)]
pub(super) struct BlurRenderCache {
    entries: HashMap<u64, BlurRenderEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ArrowDrawStyle {
    pub(super) color_r: u8,
    pub(super) color_g: u8,
    pub(super) color_b: u8,
    pub(super) opacity_percent: u8,
    pub(super) thickness: u8,
    pub(super) head_size: u8,
}

pub(super) struct ToolRenderContext<'a> {
    pub(super) image_bounds: ImageBounds,
    pub(super) show_crop_mask: bool,
    pub(super) selected_object_ids: &'a [u64],
    pub(super) source_pixbuf: Option<&'a gtk4::gdk_pixbuf::Pixbuf>,
    pub(super) active_text_id: Option<u64>,
    pub(super) active_text_preedit: Option<&'a TextPreeditState>,
    pub(super) blur_cache: Option<&'a Rc<RefCell<BlurRenderCache>>>,
}

pub(super) struct EditorOutputActionContext<'a> {
    pub(super) action: EditorAction,
    pub(super) active_capture: &'a capture::CaptureArtifact,
    pub(super) editor_tools: &'a editor::EditorTools,
    pub(super) pending_crop: Option<CropElement>,
    pub(super) source_pixbuf: &'a gtk4::gdk_pixbuf::Pixbuf,
    pub(super) storage_service: &'a StorageService,
    pub(super) status_log: &'a Rc<RefCell<String>>,
    pub(super) editor_toast: &'a ToastRuntime,
    pub(super) toast_duration_ms: u32,
    pub(super) editor_has_unsaved_changes: &'a Rc<RefCell<bool>>,
}

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::capture;
use crate::editor::tools::{CropElement, ImageBounds, ToolPoint};
use crate::editor::{self, EditorAction, ToolKind};
use crate::storage::StorageService;
use crate::theme::ThemeMode;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RgbaColor {
    pub(super) red: u8,
    pub(super) green: u8,
    pub(super) blue: u8,
    pub(super) alpha: u8,
}

impl RgbaColor {
    pub(super) const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub(super) fn to_cairo_rgba(self) -> (f64, f64, f64, f64) {
        (
            f64::from(self.red) / 255.0,
            f64::from(self.green) / 255.0,
            f64::from(self.blue) / 255.0,
            f64::from(self.alpha) / 255.0,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EditorSelectionPalette {
    pub(super) drag_fill: RgbaColor,
    pub(super) drag_stroke: RgbaColor,
    pub(super) selected_outline: RgbaColor,
    pub(super) resize_handle_fill: RgbaColor,
}

impl Default for EditorSelectionPalette {
    fn default() -> Self {
        Self::for_theme_mode(ThemeMode::Dark)
    }
}

impl EditorSelectionPalette {
    pub(super) const fn for_theme_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self {
                drag_fill: RgbaColor::new(0x18, 0x18, 0x1B, 0x1A),
                drag_stroke: RgbaColor::new(0x18, 0x18, 0x1B, 0xC4),
                selected_outline: RgbaColor::new(0x18, 0x18, 0x1B, 0xD9),
                resize_handle_fill: RgbaColor::new(0x18, 0x18, 0x1B, 0xE6),
            },
            ThemeMode::Dark | ThemeMode::System => Self {
                drag_fill: RgbaColor::new(0xE4, 0xE4, 0xE7, 0x1F),
                drag_stroke: RgbaColor::new(0xE4, 0xE4, 0xE7, 0xDE),
                selected_outline: RgbaColor::new(0xE4, 0xE4, 0xE7, 0xE6),
                resize_handle_fill: RgbaColor::new(0xF4, 0xF4, 0xF5, 0xF2),
            },
        }
    }
}

pub(super) struct ToolRenderContext<'a> {
    pub(super) image_bounds: ImageBounds,
    pub(super) show_crop_mask: bool,
    pub(super) selected_object_ids: &'a [u64],
    pub(super) selection_palette: EditorSelectionPalette,
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

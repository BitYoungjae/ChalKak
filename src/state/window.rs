use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeWindowKind {
    Preview,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeWindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl RuntimeWindowGeometry {
    pub const fn new(width: i32, height: i32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    pub const fn with_position(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeWindowState {
    preview_geometry: Option<RuntimeWindowGeometry>,
    editor_geometry: Option<RuntimeWindowGeometry>,
    preview_geometry_by_capture: HashMap<String, RuntimeWindowGeometry>,
}

impl RuntimeWindowState {
    pub fn geometry_for(&self, kind: RuntimeWindowKind) -> Option<RuntimeWindowGeometry> {
        match kind {
            RuntimeWindowKind::Preview => self.preview_geometry,
            RuntimeWindowKind::Editor => self.editor_geometry,
        }
    }

    pub fn set_geometry(&mut self, kind: RuntimeWindowKind, geometry: RuntimeWindowGeometry) {
        match kind {
            RuntimeWindowKind::Preview => self.preview_geometry = Some(geometry),
            RuntimeWindowKind::Editor => self.editor_geometry = Some(geometry),
        }
    }

    pub fn preview_geometry_for_capture(&self, capture_id: &str) -> Option<RuntimeWindowGeometry> {
        self.preview_geometry_by_capture.get(capture_id).copied()
    }

    pub fn set_preview_geometry_for_capture(
        &mut self,
        capture_id: impl Into<String>,
        geometry: RuntimeWindowGeometry,
    ) {
        self.preview_geometry_by_capture
            .insert(capture_id.into(), geometry);
    }

    pub fn remove_preview_geometry_for_capture(&mut self, capture_id: &str) {
        self.preview_geometry_by_capture.remove(capture_id);
    }

    pub fn preview_geometry_capture_ids(&self) -> Vec<String> {
        self.preview_geometry_by_capture.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_window_state_tracks_each_window_kind_independently() {
        let mut state = RuntimeWindowState::default();
        let preview_geometry = RuntimeWindowGeometry::new(800, 450);
        let editor_geometry = RuntimeWindowGeometry::new(1200, 760);

        state.set_geometry(RuntimeWindowKind::Preview, preview_geometry);
        state.set_geometry(RuntimeWindowKind::Editor, editor_geometry);

        assert_eq!(
            state.geometry_for(RuntimeWindowKind::Preview),
            Some(preview_geometry)
        );
        assert_eq!(
            state.geometry_for(RuntimeWindowKind::Editor),
            Some(editor_geometry)
        );
    }

    #[test]
    fn runtime_window_state_tracks_preview_geometry_by_capture_id() {
        let mut state = RuntimeWindowState::default();
        let geometry = RuntimeWindowGeometry::with_position(120, 44, 760, 480);

        state.set_preview_geometry_for_capture("capture-1", geometry);

        assert_eq!(
            state.preview_geometry_for_capture("capture-1"),
            Some(geometry)
        );
        assert!(state
            .preview_geometry_capture_ids()
            .contains(&"capture-1".to_string()));

        state.remove_preview_geometry_for_capture("capture-1");
        assert_eq!(state.preview_geometry_for_capture("capture-1"), None);
    }
}

use crate::capture;

#[derive(Default)]
pub(crate) struct RuntimeSession {
    captures: Vec<capture::CaptureArtifact>,
    active_capture_id: Option<String>,
}

impl RuntimeSession {
    pub(crate) fn push_capture(&mut self, artifact: capture::CaptureArtifact) {
        self.captures
            .retain(|capture| capture.capture_id != artifact.capture_id);
        self.active_capture_id = Some(artifact.capture_id.clone());
        self.captures.push(artifact);
        self.ensure_active_capture();
    }

    pub(crate) fn active_capture(&self) -> Option<&capture::CaptureArtifact> {
        self.active_capture_id
            .as_deref()
            .and_then(|active| {
                self.captures
                    .iter()
                    .find(|artifact| artifact.capture_id == active)
            })
            .or_else(|| self.captures.first())
    }

    pub(crate) fn remove_capture(&mut self, capture_id: &str) {
        self.captures
            .retain(|capture| capture.capture_id != capture_id);
        self.ensure_active_capture();
    }

    pub(crate) fn ids_for_display(&self) -> Vec<String> {
        self.captures
            .iter()
            .map(|artifact| artifact.capture_id.clone())
            .collect()
    }

    pub(crate) fn captures_for_display(&self) -> Vec<capture::CaptureArtifact> {
        self.captures.clone()
    }

    pub(crate) fn set_active_capture(&mut self, capture_id: &str) -> bool {
        if self
            .captures
            .iter()
            .any(|artifact| artifact.capture_id == capture_id)
        {
            self.active_capture_id = Some(capture_id.to_string());
            true
        } else {
            false
        }
    }

    pub(crate) fn latest_label_text(&self) -> String {
        self.active_capture().map_or_else(
            || "No capture yet".to_string(),
            |artifact| {
                format!(
                    "{} ({}x{})",
                    artifact.capture_id, artifact.width, artifact.height
                )
            },
        )
    }

    fn ensure_active_capture(&mut self) {
        if self
            .active_capture_id
            .as_deref()
            .is_some_and(|id| self.captures.iter().any(|capture| capture.capture_id == id))
        {
            return;
        }
        self.active_capture_id = self
            .captures
            .first()
            .map(|capture| capture.capture_id.clone());
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn artifact(id: &str) -> capture::CaptureArtifact {
        capture::CaptureArtifact {
            capture_id: id.to_string(),
            temp_path: PathBuf::from(format!("/tmp/{id}.png")),
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
    fn runtime_session_keeps_multiple_captures_and_tracks_latest_active() {
        let mut runtime = RuntimeSession::default();

        runtime.push_capture(artifact("one"));
        runtime.push_capture(artifact("two"));

        assert_eq!(runtime.ids_for_display(), vec!["one", "two"]);
        assert!(runtime
            .active_capture()
            .is_some_and(|item| item.capture_id == "two"));
    }

    #[test]
    fn runtime_session_remove_capture_clears_active_capture() {
        let mut runtime = RuntimeSession::default();
        runtime.push_capture(artifact("one"));
        runtime.remove_capture("one");
        assert!(runtime.ids_for_display().is_empty());
        assert!(runtime.active_capture().is_none());
    }

    #[test]
    fn lifecycle_cleanup_runtime_session_keeps_all_capture_ids() {
        let mut runtime = RuntimeSession::default();
        runtime.push_capture(artifact("one"));
        runtime.push_capture(artifact("two"));
        assert_eq!(runtime.ids_for_display(), vec!["one", "two"]);
    }
}

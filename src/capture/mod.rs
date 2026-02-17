use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use self::hyprland::{
    format_window_selection_regions, parse_focused_monitor, parse_selectable_windows,
};
use crate::storage::create_temp_capture;
use image::GenericImageView;
use thiserror::Error;

mod hyprland;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureArtifact {
    pub capture_id: String,
    pub temp_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub screen_x: i32,
    pub screen_y: i32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub created_at: u64,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("command failed: {command}")]
    CommandFailed { command: String, message: String },
    #[error("command io error: {command}")]
    CommandIo {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("no focused monitor found")]
    NoFocusedMonitor,
    #[error("invalid monitor metadata: {message}")]
    InvalidMonitorMetadata { message: String },
    #[error("invalid window metadata: {message}")]
    InvalidWindowMetadata { message: String },
    #[error("invalid capture artifact: {message}")]
    InvalidCaptureArtifact { message: String },
    #[error("invalid capture selection: {message}")]
    InvalidSelection { message: String },
    #[error("failed to read captured image dimensions: {message}")]
    ImageReadFailed { message: String },
}

pub trait CaptureBackend {
    fn focused_monitors_json(&self) -> Result<String, CaptureError>;
    fn clients_json(&self) -> Result<String, CaptureError>;
    fn run_full_capture(&self, monitor: &str, output: &Path) -> Result<(), CaptureError>;
    fn run_region_selection(&self) -> Result<String, CaptureError>;
    fn run_window_selection(&self, window_regions: &str) -> Result<String, CaptureError>;
    fn run_region_capture(&self, geometry: &str, output: &Path) -> Result<(), CaptureError>;
    fn image_dimensions(&self, output: &Path) -> Result<(u32, u32), CaptureError>;
}

#[derive(Default)]
pub struct SystemCaptureBackend;

impl CaptureBackend for SystemCaptureBackend {
    fn focused_monitors_json(&self) -> Result<String, CaptureError> {
        run_command_output("hyprctl", &["monitors", "-j"])
    }

    fn clients_json(&self) -> Result<String, CaptureError> {
        run_command_output("hyprctl", &["clients", "-j"])
    }

    fn run_full_capture(&self, monitor: &str, output: &Path) -> Result<(), CaptureError> {
        run_command_status("grim", &["-o", monitor], output)
    }

    fn run_region_selection(&self) -> Result<String, CaptureError> {
        run_command_output("slurp", &[])
    }

    fn run_window_selection(&self, window_regions: &str) -> Result<String, CaptureError> {
        run_command_output_with_stdin("slurp", &["-r"], window_regions)
    }

    fn run_region_capture(&self, geometry: &str, output: &Path) -> Result<(), CaptureError> {
        run_command_status("grim", &["-g", geometry], output)
    }

    fn image_dimensions(&self, output: &Path) -> Result<(u32, u32), CaptureError> {
        let image = image::open(output).map_err(|err| CaptureError::ImageReadFailed {
            message: err.to_string(),
        })?;
        Ok(image.dimensions())
    }
}

pub fn capture_full() -> Result<CaptureArtifact, CaptureError> {
    capture_full_with(&SystemCaptureBackend)
}

pub fn capture_region() -> Result<CaptureArtifact, CaptureError> {
    capture_region_with(&SystemCaptureBackend)
}

pub fn capture_window() -> Result<CaptureArtifact, CaptureError> {
    capture_window_with(&SystemCaptureBackend)
}

pub fn capture_full_with<B: CaptureBackend>(backend: &B) -> Result<CaptureArtifact, CaptureError> {
    let monitor_json = backend.focused_monitors_json()?;
    let monitor = parse_focused_monitor(&monitor_json)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| CaptureError::InvalidCaptureArtifact {
            message: format!("system time before unix epoch: {err}"),
        })?;

    let capture_id = format!("capture-{}", now.as_nanos());
    let temp_path = build_capture_path(&capture_id);

    if let Err(err) = backend.run_full_capture(&monitor.name, &temp_path) {
        cleanup_temp_capture_file(&temp_path, "full capture command failure");
        return Err(err);
    }

    let (width, height) = match backend.image_dimensions(&temp_path) {
        Ok(size) => size,
        Err(err) => {
            cleanup_temp_capture_file(&temp_path, "full capture image dimension read failure");
            return Err(err);
        }
    };

    Ok(CaptureArtifact {
        capture_id,
        temp_path,
        width,
        height,
        screen_x: monitor.x,
        screen_y: monitor.y,
        screen_width: monitor.width.unwrap_or(width),
        screen_height: monitor.height.unwrap_or(height),
        created_at: now.as_millis() as u64,
    })
}

pub fn capture_region_with<B: CaptureBackend>(
    backend: &B,
) -> Result<CaptureArtifact, CaptureError> {
    let monitor_json = backend.focused_monitors_json()?;
    let _ = parse_focused_monitor(&monitor_json)?;
    let raw_geometry = backend.run_region_selection()?;
    let geometry = raw_geometry.trim();
    if geometry.is_empty() {
        return Err(CaptureError::InvalidSelection {
            message: "region selection returned no geometry".to_string(),
        });
    }
    let selection = parse_region_selection(geometry)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| CaptureError::InvalidCaptureArtifact {
            message: format!("system time before unix epoch: {err}"),
        })?;

    let capture_id = format!("capture-{}", now.as_nanos());
    let temp_path = build_capture_path(&capture_id);

    if let Err(err) = backend.run_region_capture(geometry, &temp_path) {
        cleanup_temp_capture_file(&temp_path, "region capture command failure");
        return Err(err);
    }

    let (width, height) = match backend.image_dimensions(&temp_path) {
        Ok(size) => size,
        Err(err) => {
            cleanup_temp_capture_file(&temp_path, "region capture image dimension read failure");
            return Err(err);
        }
    };

    Ok(CaptureArtifact {
        capture_id,
        temp_path,
        width,
        height,
        screen_x: selection.x,
        screen_y: selection.y,
        screen_width: selection.width,
        screen_height: selection.height,
        created_at: now.as_millis() as u64,
    })
}

pub fn capture_window_with<B: CaptureBackend>(
    backend: &B,
) -> Result<CaptureArtifact, CaptureError> {
    let monitor_json = backend.focused_monitors_json()?;
    let focused_monitor = parse_focused_monitor(&monitor_json)?;
    let focused_workspace_id =
        focused_monitor
            .active_workspace_id
            .ok_or(CaptureError::InvalidMonitorMetadata {
                message: "focused monitor missing active workspace id".to_string(),
            })?;

    let clients_json = backend.clients_json()?;
    let window_candidates = parse_selectable_windows(&clients_json, focused_workspace_id)?;
    if window_candidates.is_empty() {
        return Err(CaptureError::InvalidSelection {
            message: "window selection has no selectable windows".to_string(),
        });
    }
    let window_regions = format_window_selection_regions(&window_candidates);
    let raw_geometry = backend.run_window_selection(&window_regions)?;
    let geometry = raw_geometry.trim();
    if geometry.is_empty() {
        return Err(CaptureError::InvalidSelection {
            message: "window selection returned no geometry".to_string(),
        });
    }
    let selection = parse_region_selection(geometry)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| CaptureError::InvalidCaptureArtifact {
            message: format!("system time before unix epoch: {err}"),
        })?;

    let capture_id = format!("capture-{}", now.as_nanos());
    let temp_path = build_capture_path(&capture_id);

    if let Err(err) = backend.run_region_capture(geometry, &temp_path) {
        cleanup_temp_capture_file(&temp_path, "window capture command failure");
        return Err(err);
    }

    let (width, height) = match backend.image_dimensions(&temp_path) {
        Ok(size) => size,
        Err(err) => {
            cleanup_temp_capture_file(&temp_path, "window capture image dimension read failure");
            return Err(err);
        }
    };

    Ok(CaptureArtifact {
        capture_id,
        temp_path,
        width,
        height,
        screen_x: selection.x,
        screen_y: selection.y,
        screen_width: selection.width,
        screen_height: selection.height,
        created_at: now.as_millis() as u64,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CaptureSelection {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn parse_region_selection(geometry: &str) -> Result<CaptureSelection, CaptureError> {
    let mut parts = geometry.split_whitespace();
    let Some(position) = parts.next() else {
        return Err(CaptureError::InvalidSelection {
            message: format!("invalid region geometry: {geometry}"),
        });
    };
    let Some(size) = parts.next() else {
        return Err(CaptureError::InvalidSelection {
            message: format!("invalid region geometry: {geometry}"),
        });
    };
    if parts.next().is_some() {
        return Err(CaptureError::InvalidSelection {
            message: format!("invalid region geometry: {geometry}"),
        });
    }

    let Some((x, y)) = position.split_once(',') else {
        return Err(CaptureError::InvalidSelection {
            message: format!("invalid region position: {position}"),
        });
    };
    let Some((width, height)) = size.split_once('x') else {
        return Err(CaptureError::InvalidSelection {
            message: format!("invalid region size: {size}"),
        });
    };

    let x = x
        .parse::<i32>()
        .map_err(|err| CaptureError::InvalidSelection {
            message: format!("invalid x coordinate '{x}': {err}"),
        })?;
    let y = y
        .parse::<i32>()
        .map_err(|err| CaptureError::InvalidSelection {
            message: format!("invalid y coordinate '{y}': {err}"),
        })?;
    let width = width
        .parse::<u32>()
        .map_err(|err| CaptureError::InvalidSelection {
            message: format!("invalid width '{width}': {err}"),
        })?;
    let height = height
        .parse::<u32>()
        .map_err(|err| CaptureError::InvalidSelection {
            message: format!("invalid height '{height}': {err}"),
        })?;
    if width == 0 || height == 0 {
        return Err(CaptureError::InvalidSelection {
            message: format!("selection must be positive, got {width}x{height}"),
        });
    }

    Ok(CaptureSelection {
        x,
        y,
        width,
        height,
    })
}

fn build_capture_path(capture_id: &str) -> PathBuf {
    create_temp_capture(capture_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TempCaptureCleanupOutcome {
    Removed,
    NotFound,
    Failed,
}

fn cleanup_temp_capture_file(temp_path: &Path, stage: &str) -> TempCaptureCleanupOutcome {
    cleanup_temp_capture_file_with(temp_path, stage, |path| std::fs::remove_file(path))
}

fn cleanup_temp_capture_file_with<F>(
    temp_path: &Path,
    stage: &str,
    remove_file: F,
) -> TempCaptureCleanupOutcome
where
    F: FnOnce(&Path) -> std::io::Result<()>,
{
    match remove_file(temp_path) {
        Ok(()) => TempCaptureCleanupOutcome::Removed,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!(
                stage = stage,
                path = %temp_path.display(),
                ?err,
                "temporary capture file was already missing during cleanup"
            );
            TempCaptureCleanupOutcome::NotFound
        }
        Err(err) => {
            tracing::warn!(
                stage = stage,
                path = %temp_path.display(),
                ?err,
                "failed to cleanup temporary capture file"
            );
            TempCaptureCleanupOutcome::Failed
        }
    }
}

fn run_command_output(command: &str, args: &[&str]) -> Result<String, CaptureError> {
    run_command_output_internal(command, args, None)
}

fn run_command_output_with_stdin(
    command: &str,
    args: &[&str],
    stdin_payload: &str,
) -> Result<String, CaptureError> {
    run_command_output_internal(command, args, Some(stdin_payload))
}

fn run_command_output_internal(
    command: &str,
    args: &[&str],
    stdin_payload: Option<&str>,
) -> Result<String, CaptureError> {
    let mut process = Command::new(command);
    process
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stdin_payload.is_some() {
        process.stdin(Stdio::piped());
    }

    let mut child = process.spawn().map_err(|err| CaptureError::CommandIo {
        command: command.to_string(),
        source: err,
    })?;

    if let Some(payload) = stdin_payload {
        let mut stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                cleanup_command_child(&mut child);
                return Err(CaptureError::CommandFailed {
                    command: command.to_string(),
                    message: "command stdin unavailable".to_string(),
                });
            }
        };
        if let Err(err) = stdin.write_all(payload.as_bytes()) {
            cleanup_command_child(&mut child);
            return Err(CaptureError::CommandIo {
                command: command.to_string(),
                source: err,
            });
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|err| CaptureError::CommandIo {
            command: command.to_string(),
            source: err,
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        let message = format!("exit status: {}; stderr: {stderr}", output.status);
        return Err(CaptureError::CommandFailed {
            command: command.to_string(),
            message,
        });
    }

    if stdout.is_empty() {
        return Err(CaptureError::CommandFailed {
            command: command.to_string(),
            message: "command produced no stdout output".to_string(),
        });
    }

    Ok(stdout)
}

fn cleanup_command_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn run_command_status(command: &str, args: &[&str], output: &Path) -> Result<(), CaptureError> {
    let status = Command::new(command)
        .args(args)
        .arg(output)
        .status()
        .map_err(|err| CaptureError::CommandIo {
            command: command.to_string(),
            source: err,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(CaptureError::CommandFailed {
            command: command.to_string(),
            message: format!("command exited with status: {status}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::PathBuf;

    struct FakeCaptureBackend {
        monitor_json: String,
        clients_json: String,
        dimensions: (u32, u32),
        region_selection: String,
        window_selection: String,
        window_selection_regions_input: RefCell<Option<String>>,
        fail_full_capture: bool,
        fail_region_capture: bool,
        fail_image_dimensions: bool,
        calls: RefCell<Vec<String>>,
    }

    const FOCUSED_MONITOR_WITH_WORKSPACE_JSON: &str =
        r#"[{"name":"DP-1","focused":true,"activeWorkspace":{"id":1,"name":"1"}}]"#;

    impl FakeCaptureBackend {
        fn new(monitors_json: &str, dimensions: (u32, u32), region_selection: &str) -> Self {
            Self {
                monitor_json: monitors_json.to_string(),
                clients_json: "[]".to_string(),
                dimensions,
                region_selection: region_selection.to_string(),
                window_selection: region_selection.to_string(),
                window_selection_regions_input: RefCell::new(None),
                fail_full_capture: false,
                fail_region_capture: false,
                fail_image_dimensions: false,
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl CaptureBackend for FakeCaptureBackend {
        fn focused_monitors_json(&self) -> Result<String, CaptureError> {
            self.calls
                .borrow_mut()
                .push("hyprctl monitors -j".to_string());
            Ok(self.monitor_json.clone())
        }

        fn clients_json(&self) -> Result<String, CaptureError> {
            self.calls
                .borrow_mut()
                .push("hyprctl clients -j".to_string());
            Ok(self.clients_json.clone())
        }

        fn run_full_capture(&self, monitor: &str, output: &Path) -> Result<(), CaptureError> {
            self.calls
                .borrow_mut()
                .push(format!("grim -o {monitor} {}", output.display()));
            if self.fail_full_capture {
                return Err(CaptureError::CommandFailed {
                    command: "grim".to_string(),
                    message: "simulated full capture failure".to_string(),
                });
            }
            std::fs::write(output, b"capture-data").map_err(|err| CaptureError::CommandIo {
                command: "grim".to_string(),
                source: err,
            })?;
            Ok(())
        }

        fn run_region_selection(&self) -> Result<String, CaptureError> {
            self.calls.borrow_mut().push("slurp".to_string());
            Ok(self.region_selection.clone())
        }

        fn run_window_selection(&self, window_regions: &str) -> Result<String, CaptureError> {
            self.calls.borrow_mut().push("slurp -r".to_string());
            *self.window_selection_regions_input.borrow_mut() = Some(window_regions.to_string());
            Ok(self.window_selection.clone())
        }

        fn run_region_capture(&self, geometry: &str, output: &Path) -> Result<(), CaptureError> {
            self.calls
                .borrow_mut()
                .push(format!("grim -g {geometry} {}", output.display()));
            if self.fail_region_capture {
                return Err(CaptureError::CommandFailed {
                    command: "grim".to_string(),
                    message: "simulated region capture failure".to_string(),
                });
            }
            std::fs::write(output, b"capture-data").map_err(|err| CaptureError::CommandIo {
                command: "grim".to_string(),
                source: err,
            })?;
            Ok(())
        }

        fn image_dimensions(&self, _output: &Path) -> Result<(u32, u32), CaptureError> {
            if self.fail_image_dimensions {
                return Err(CaptureError::ImageReadFailed {
                    message: "simulated dimension read failure".to_string(),
                });
            }
            Ok(self.dimensions)
        }
    }

    #[test]
    fn capture_full_uses_focused_monitor_and_creates_artifact() {
        let backend =
            FakeCaptureBackend::new(r#"[{"name":"DP-1","focused":true}]"#, (1920, 1080), "");
        let artifact = capture_full_with(&backend).expect("fake adapter should produce artifact");
        assert_eq!(artifact.width, 1920);
        assert_eq!(artifact.height, 1080);
        assert_eq!(artifact.screen_x, 0);
        assert_eq!(artifact.screen_y, 0);
        assert_eq!(artifact.screen_width, 1920);
        assert_eq!(artifact.screen_height, 1080);
        assert!(artifact.temp_path.exists());
        assert!(!artifact.capture_id.is_empty());
        assert!(artifact.created_at > 0);

        let calls = backend.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], "hyprctl monitors -j");
        assert!(calls[1].contains("grim -o "));

        let _ = PathBuf::from(&artifact.temp_path).exists();
        let _ = std::fs::remove_file(artifact.temp_path);
    }

    #[test]
    fn capture_region_uses_region_selection_and_creates_artifact() {
        let backend = FakeCaptureBackend::new(
            r#"[{"name":"DP-1","focused":true}]"#,
            (800, 600),
            "30,40 300x200",
        );
        let artifact = capture_region_with(&backend).expect("fake adapter should capture region");
        assert_eq!(artifact.width, 800);
        assert_eq!(artifact.height, 600);
        assert_eq!(artifact.screen_x, 30);
        assert_eq!(artifact.screen_y, 40);
        assert_eq!(artifact.screen_width, 300);
        assert_eq!(artifact.screen_height, 200);
        assert!(artifact.temp_path.exists());
        assert!(!artifact.capture_id.is_empty());
        assert!(artifact.created_at > 0);

        let calls = backend.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0], "hyprctl monitors -j");
        assert_eq!(calls[1], "slurp");
        assert_eq!(
            calls[2],
            format!("grim -g 30,40 300x200 {}", artifact.temp_path.display())
        );

        let _ = std::fs::remove_file(artifact.temp_path);
    }

    #[test]
    fn capture_region_errors_when_selection_empty() {
        let backend =
            FakeCaptureBackend::new(r#"[{"name":"DP-1","focused":true}]"#, (1920, 1080), "\n");
        let err = capture_region_with(&backend).expect_err("selection empty should be invalid");
        assert!(matches!(err, CaptureError::InvalidSelection { message: _ }));

        let calls = backend.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], "hyprctl monitors -j");
        assert_eq!(calls[1], "slurp");
    }

    #[test]
    fn capture_window_uses_window_selection_and_creates_artifact() {
        let mut backend =
            FakeCaptureBackend::new(FOCUSED_MONITOR_WITH_WORKSPACE_JSON, (800, 600), "");
        backend.clients_json = r#"[{"title":"Browser","class":"firefox","mapped":true,"hidden":false,"workspace":{"id":1,"name":"1"},"at":[30,40],"size":[300,200]}]"#.to_string();
        backend.window_selection = "30,40 300x200".to_string();
        let artifact = capture_window_with(&backend).expect("fake adapter should capture window");

        assert_eq!(artifact.width, 800);
        assert_eq!(artifact.height, 600);
        assert_eq!(artifact.screen_x, 30);
        assert_eq!(artifact.screen_y, 40);
        assert_eq!(artifact.screen_width, 300);
        assert_eq!(artifact.screen_height, 200);
        assert!(artifact.temp_path.exists());

        let calls = backend.calls();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0], "hyprctl monitors -j");
        assert_eq!(calls[1], "hyprctl clients -j");
        assert_eq!(calls[2], "slurp -r");
        assert_eq!(
            calls[3],
            format!("grim -g 30,40 300x200 {}", artifact.temp_path.display())
        );
        assert_eq!(
            backend.window_selection_regions_input.borrow().as_deref(),
            Some("30,40 300x200 01. Browser [firefox]")
        );

        let _ = std::fs::remove_file(artifact.temp_path);
    }

    #[test]
    fn capture_window_errors_when_no_selectable_window_exists() {
        let mut backend =
            FakeCaptureBackend::new(FOCUSED_MONITOR_WITH_WORKSPACE_JSON, (800, 600), "");
        backend.clients_json =
            r#"[{"title":"hidden","hidden":true,"workspace":{"id":1,"name":"1"},"at":[1,2],"size":[3,4]}]"#.to_string();

        let err =
            capture_window_with(&backend).expect_err("missing selectable windows should error");
        assert!(matches!(err, CaptureError::InvalidSelection { message: _ }));

        let calls = backend.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], "hyprctl monitors -j");
        assert_eq!(calls[1], "hyprctl clients -j");
    }

    #[test]
    fn capture_window_errors_when_focused_workspace_metadata_is_missing() {
        let mut backend =
            FakeCaptureBackend::new(r#"[{"name":"DP-1","focused":true}]"#, (400, 300), "");
        backend.clients_json =
            r#"[{"title":"Editor","mapped":true,"hidden":false,"workspace":{"id":1,"name":"1"},"at":[9,11],"size":[120,70]}]"#
                .to_string();
        backend.window_selection = "9,11 120x70".to_string();

        let err = capture_window_with(&backend)
            .expect_err("window capture should fail when focused workspace id is missing");
        assert!(matches!(
            err,
            CaptureError::InvalidMonitorMetadata { message: _ }
        ));

        assert_eq!(backend.calls(), vec!["hyprctl monitors -j".to_string()]);
    }

    #[test]
    fn capture_full_cleans_up_temp_file_when_dimension_read_fails() {
        let mut backend =
            FakeCaptureBackend::new(r#"[{"name":"DP-1","focused":true}]"#, (1920, 1080), "");
        backend.fail_image_dimensions = true;

        let err = capture_full_with(&backend).expect_err("dimension read failure should bubble");
        assert!(matches!(err, CaptureError::ImageReadFailed { message: _ }));

        let capture_call = backend
            .calls()
            .into_iter()
            .find(|call| call.starts_with("grim -o "))
            .expect("expected full capture command");
        let temp_path = PathBuf::from(
            capture_call
                .split_whitespace()
                .last()
                .expect("capture command should include output path"),
        );
        assert!(!temp_path.exists());
    }

    #[test]
    fn capture_region_cleans_up_temp_file_when_dimension_read_fails() {
        let mut backend = FakeCaptureBackend::new(
            r#"[{"name":"DP-1","focused":true}]"#,
            (1920, 1080),
            "10,20 80x60",
        );
        backend.fail_image_dimensions = true;

        let err = capture_region_with(&backend).expect_err("dimension read failure should bubble");
        assert!(matches!(err, CaptureError::ImageReadFailed { message: _ }));

        let capture_call = backend
            .calls()
            .into_iter()
            .find(|call| call.starts_with("grim -g "))
            .expect("expected region capture command");
        let temp_path = PathBuf::from(
            capture_call
                .split_whitespace()
                .last()
                .expect("capture command should include output path"),
        );
        assert!(!temp_path.exists());
    }

    #[test]
    fn capture_window_cleans_up_temp_file_when_dimension_read_fails() {
        let mut backend =
            FakeCaptureBackend::new(FOCUSED_MONITOR_WITH_WORKSPACE_JSON, (1920, 1080), "");
        backend.clients_json =
            r#"[{"title":"window","mapped":true,"hidden":false,"workspace":{"id":1,"name":"1"},"at":[10,20],"size":[80,60]}]"#.to_string();
        backend.window_selection = "10,20 80x60".to_string();
        backend.fail_image_dimensions = true;

        let err = capture_window_with(&backend).expect_err("dimension read failure should bubble");
        assert!(matches!(err, CaptureError::ImageReadFailed { message: _ }));

        let capture_call = backend
            .calls()
            .into_iter()
            .find(|call| call.starts_with("grim -g "))
            .expect("expected window capture command");
        let temp_path = PathBuf::from(
            capture_call
                .split_whitespace()
                .last()
                .expect("capture command should include output path"),
        );
        assert!(!temp_path.exists());
    }

    #[test]
    fn cleanup_temp_capture_file_with_reports_failed_when_remove_errors() {
        let path = PathBuf::from("/tmp/chalkak-nonexistent-file.png");
        let outcome = cleanup_temp_capture_file_with(&path, "test cleanup", |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "simulated cleanup failure",
            ))
        });

        assert_eq!(outcome, TempCaptureCleanupOutcome::Failed);
    }

    #[test]
    fn parse_region_selection_parses_expected_format() {
        let selection = parse_region_selection("320,240 640x360").expect("selection should parse");
        assert_eq!(
            selection,
            CaptureSelection {
                x: 320,
                y: 240,
                width: 640,
                height: 360,
            }
        );
    }

    #[test]
    fn parse_region_selection_rejects_invalid_formats() {
        let err =
            parse_region_selection("oops").expect_err("invalid geometry should produce an error");
        assert!(matches!(err, CaptureError::InvalidSelection { message: _ }));
    }
}

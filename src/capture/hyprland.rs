use serde::Deserialize;

use super::CaptureError;

#[derive(Deserialize)]
struct MonitorStatus {
    focused: bool,
    name: Option<String>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<i32>,
    height: Option<i32>,
    #[serde(default, rename = "activeWorkspace")]
    active_workspace: Option<WorkspaceStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FocusedMonitor {
    pub(super) name: String,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: Option<u32>,
    pub(super) height: Option<u32>,
    pub(super) active_workspace_id: Option<i32>,
}

#[derive(Deserialize)]
struct WindowClientStatus {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    class: Option<String>,
    #[serde(default)]
    mapped: Option<bool>,
    #[serde(default)]
    hidden: Option<bool>,
    #[serde(default)]
    at: Option<[i32; 2]>,
    #[serde(default)]
    size: Option<[i32; 2]>,
    #[serde(default)]
    workspace: Option<WorkspaceStatus>,
}

#[derive(Deserialize)]
struct WorkspaceStatus {
    #[serde(default)]
    id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WindowSelectionCandidate {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    label: String,
}

fn normalize_monitor_dimension(value: Option<i32>) -> Option<u32> {
    value
        .and_then(|size| u32::try_from(size).ok())
        .filter(|size| *size > 0)
}

pub(super) fn parse_focused_monitor(monitors_json: &str) -> Result<FocusedMonitor, CaptureError> {
    let monitors: Vec<MonitorStatus> = serde_json::from_str(monitors_json).map_err(|err| {
        CaptureError::InvalidMonitorMetadata {
            message: err.to_string(),
        }
    })?;
    let monitor = monitors
        .into_iter()
        .find(|monitor| monitor.focused)
        .ok_or(CaptureError::NoFocusedMonitor)?;
    let name = monitor
        .name
        .filter(|item| !item.is_empty())
        .ok_or(CaptureError::NoFocusedMonitor)?;

    Ok(FocusedMonitor {
        name,
        x: monitor.x.unwrap_or(0),
        y: monitor.y.unwrap_or(0),
        width: normalize_monitor_dimension(monitor.width),
        height: normalize_monitor_dimension(monitor.height),
        active_workspace_id: monitor
            .active_workspace
            .and_then(|workspace| workspace.id)
            .filter(|id| *id != 0),
    })
}

pub(super) fn parse_selectable_windows(
    clients_json: &str,
    focused_workspace_id: i32,
) -> Result<Vec<WindowSelectionCandidate>, CaptureError> {
    let clients: Vec<WindowClientStatus> =
        serde_json::from_str(clients_json).map_err(|err| CaptureError::InvalidWindowMetadata {
            message: err.to_string(),
        })?;

    let mut candidates = Vec::new();
    for client in clients {
        if client.hidden.unwrap_or(false) || matches!(client.mapped, Some(false)) {
            continue;
        }
        if client.workspace.as_ref().and_then(|workspace| workspace.id)
            != Some(focused_workspace_id)
        {
            continue;
        }

        let Some([x, y]) = client.at else {
            continue;
        };
        let Some([width, height]) = client.size else {
            continue;
        };
        let Some(width) = u32::try_from(width).ok().filter(|value| *value > 0) else {
            continue;
        };
        let Some(height) = u32::try_from(height).ok().filter(|value| *value > 0) else {
            continue;
        };

        candidates.push(WindowSelectionCandidate {
            x,
            y,
            width,
            height,
            label: format_window_label(candidates.len() + 1, &client),
        });
    }

    Ok(candidates)
}

fn format_window_label(index: usize, client: &WindowClientStatus) -> String {
    let title = client
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let class = client
        .class
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let body = match (title, class) {
        (Some(title), Some(class)) => format!("{title} [{class}]"),
        (Some(title), None) => title.to_string(),
        (None, Some(class)) => class.to_string(),
        (None, None) => "window".to_string(),
    };
    let sanitized = body.replace(['\n', '\r'], " ");
    format!("{index:02}. {sanitized}")
}

pub(super) fn format_window_selection_regions(candidates: &[WindowSelectionCandidate]) -> String {
    candidates
        .iter()
        .map(|candidate| {
            format!(
                "{},{} {}x{} {}",
                candidate.x, candidate.y, candidate.width, candidate.height, candidate.label
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_focused_monitor_prefers_focused_monitor() {
        let json = r#"[{"name":"DP-1","focused":false},{"name":"HDMI-A-1","focused":true,"x":100,"y":200,"width":2560,"height":1440,"activeWorkspace":{"id":3,"name":"3"}}]"#;
        assert_eq!(
            parse_focused_monitor(json).expect("focused monitor should parse"),
            FocusedMonitor {
                name: "HDMI-A-1".to_string(),
                x: 100,
                y: 200,
                width: Some(2560),
                height: Some(1440),
                active_workspace_id: Some(3),
            }
        );
    }

    #[test]
    fn parse_focused_monitor_errors_without_focused() {
        let json = r#"[{"name":"DP-1","focused":false}]"#;
        assert!(matches!(
            parse_focused_monitor(json).expect_err("must error without focused monitor"),
            CaptureError::NoFocusedMonitor
        ));
    }

    #[test]
    fn parse_selectable_windows_filters_invalid_hidden_and_out_of_workspace_clients() {
        let clients_json = r#"
[
  {"title":"Browser","class":"firefox","mapped":true,"hidden":false,"workspace":{"id":1,"name":"1"},"at":[5,10],"size":[400,300]},
  {"title":"OtherWorkspace","mapped":true,"hidden":false,"workspace":{"id":2,"name":"2"},"at":[8,9],"size":[50,60]},
  {"title":"Hidden","mapped":true,"hidden":true,"workspace":{"id":1,"name":"1"},"at":[0,0],"size":[10,10]},
  {"title":"Unmapped","mapped":false,"hidden":false,"workspace":{"id":1,"name":"1"},"at":[0,0],"size":[10,10]},
  {"title":"InvalidSize","mapped":true,"hidden":false,"workspace":{"id":1,"name":"1"},"at":[0,0],"size":[0,10]},
  {"title":"MissingGeometry","mapped":true,"hidden":false}
]
"#;
        let candidates = parse_selectable_windows(clients_json, 1).expect("clients should parse");
        assert_eq!(
            candidates,
            vec![WindowSelectionCandidate {
                x: 5,
                y: 10,
                width: 400,
                height: 300,
                label: "01. Browser [firefox]".to_string(),
            }]
        );
    }
}

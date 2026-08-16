use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

const HYPR_FLOAT_RETRY_COUNT: u8 = 40;
const HYPR_FLOAT_RETRY_DELAY: Duration = Duration::from_millis(50);
const HYPR_PIN_EVENT_TIMEOUT: Duration = Duration::from_millis(350);

const HYPR_DIALECT_UNKNOWN: u8 = 0;
const HYPR_DIALECT_LEGACY: u8 = 1;
const HYPR_DIALECT_LUA: u8 = 2;

/// Hyprland 0.56 evaluates `hyprctl dispatch` arguments as Lua, so the legacy
/// `setfloating address:0x...` form fails to parse there. Remember which dialect the running
/// compositor accepted so later dispatches skip the variant it rejects.
static HYPR_DISPATCH_DIALECT: AtomicU8 = AtomicU8::new(HYPR_DIALECT_UNKNOWN);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HyprPropValue {
    Toggle(bool),
    Number(i32),
}

impl HyprPropValue {
    fn legacy(self) -> String {
        match self {
            Self::Toggle(true) => "on".to_string(),
            Self::Toggle(false) => "off".to_string(),
            Self::Number(value) => value.to_string(),
        }
    }

    /// Toggles are quoted `"on"`/`"off"` rather than Lua booleans: `hl.window.set_prop` rejects a
    /// boolean `value` with "'value' is required", while the legacy on/off spelling is accepted.
    fn lua(self) -> String {
        match self {
            Self::Toggle(true) => "\"on\"".to_string(),
            Self::Toggle(false) => "\"off\"".to_string(),
            Self::Number(value) => value.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum WindowDispatch<'a> {
    Float {
        address: &'a str,
    },
    Pin {
        address: &'a str,
    },
    SetProp {
        address: &'a str,
        property: &'a str,
        value: HyprPropValue,
    },
    ResizeExact {
        address: &'a str,
        width: i32,
        height: i32,
    },
    MoveExact {
        address: &'a str,
        x: i32,
        y: i32,
    },
}

impl WindowDispatch<'_> {
    fn label(&self) -> &'static str {
        match self {
            Self::Float { .. } => "float",
            Self::Pin { .. } => "pin",
            Self::SetProp { .. } => "setprop",
            Self::ResizeExact { .. } => "resize",
            Self::MoveExact { .. } => "move",
        }
    }

    fn address(&self) -> &str {
        match self {
            Self::Float { address }
            | Self::Pin { address }
            | Self::SetProp { address, .. }
            | Self::ResizeExact { address, .. }
            | Self::MoveExact { address, .. } => address,
        }
    }

    fn legacy_args(&self) -> Vec<String> {
        let selector = format!("address:{}", self.address());
        match *self {
            Self::Float { .. } => vec!["dispatch".into(), "setfloating".into(), selector],
            Self::Pin { .. } => vec!["dispatch".into(), "pin".into(), selector],
            Self::SetProp {
                property, value, ..
            } => vec![
                "dispatch".into(),
                "setprop".into(),
                selector,
                property.into(),
                value.legacy(),
            ],
            Self::ResizeExact { width, height, .. } => vec![
                "dispatch".into(),
                "resizewindowpixel".into(),
                format!("exact {} {},{selector}", width.max(1), height.max(1)),
            ],
            Self::MoveExact { x, y, .. } => vec![
                "dispatch".into(),
                "movewindowpixel".into(),
                format!("exact {x} {y},{selector}"),
            ],
        }
    }

    fn lua_args(&self) -> Vec<String> {
        let window = format!("hl.get_window(\"address:{}\")", self.address());
        let call = match *self {
            Self::Float { .. } => {
                format!("hl.dsp.window.float({{ action = \"on\", window = {window} }})")
            }
            Self::Pin { .. } => format!("hl.dsp.window.pin({{ window = {window} }})"),
            Self::SetProp {
                property, value, ..
            } => format!(
                "hl.dsp.window.set_prop({{ prop = \"{property}\", value = {}, window = {window} }})",
                value.lua()
            ),
            Self::ResizeExact { width, height, .. } => format!(
                "hl.dsp.window.resize({{ x = {}, y = {}, window = {window} }})",
                width.max(1),
                height.max(1)
            ),
            Self::MoveExact { x, y, .. } => {
                format!("hl.dsp.window.move({{ x = {x}, y = {y}, window = {window} }})")
            }
        };
        vec!["dispatch".into(), call]
    }

    fn args(&self, dialect: u8) -> Vec<String> {
        if dialect == HYPR_DIALECT_LUA {
            self.lua_args()
        } else {
            self.legacy_args()
        }
    }
}

/// Legacy syntax is tried first while the dialect is unknown: a Lua-mode `hyprctl` rejects it with
/// a non-zero exit, so the fallback stays detectable. Older `hyprctl` builds can report an unknown
/// dispatcher with a zero exit, which would make the reverse order look successful.
fn dispatch_dialect_order() -> [u8; 2] {
    match HYPR_DISPATCH_DIALECT.load(Ordering::Relaxed) {
        HYPR_DIALECT_LUA => [HYPR_DIALECT_LUA, HYPR_DIALECT_LEGACY],
        _ => [HYPR_DIALECT_LEGACY, HYPR_DIALECT_LUA],
    }
}

fn run_window_dispatch(window_name: &str, dispatch: WindowDispatch<'_>) -> bool {
    for dialect in dispatch_dialect_order() {
        let args = dispatch.args(dialect);
        match Command::new("hyprctl").args(&args).output() {
            Ok(result) if result.status.success() => {
                HYPR_DISPATCH_DIALECT.store(dialect, Ordering::Relaxed);
                tracing::debug!(
                    window = window_name,
                    dispatch = dispatch.label(),
                    ?args,
                    "hyprctl dispatch applied"
                );
                return true;
            }
            Ok(result) => {
                tracing::debug!(
                    window = window_name,
                    dispatch = dispatch.label(),
                    ?args,
                    status = result.status.code(),
                    stdout = String::from_utf8_lossy(&result.stdout).trim(),
                    stderr = String::from_utf8_lossy(&result.stderr).trim(),
                    "hyprctl dispatch returned non-zero status"
                );
            }
            Err(err) => {
                tracing::debug!(
                    window = window_name,
                    dispatch = dispatch.label(),
                    ?args,
                    ?err,
                    "hyprctl dispatch failed"
                );
            }
        }
    }

    false
}

#[derive(Debug, Clone)]
pub(super) struct HyprClientMatch {
    pub(super) address: String,
    pub(super) pinned: bool,
    pub(super) center: Option<(i32, i32)>,
    pub(super) geometry: Option<(i32, i32, i32, i32)>,
}

fn parse_client_geometry(client: &serde_json::Value) -> Option<(i32, i32, i32, i32)> {
    let at = client.get("at")?.as_array()?;
    let size = client.get("size")?.as_array()?;
    if at.len() != 2 || size.len() != 2 {
        return None;
    }
    let x = i32::try_from(at[0].as_i64()?).ok()?;
    let y = i32::try_from(at[1].as_i64()?).ok()?;
    let width = i32::try_from(size[0].as_i64()?).ok()?;
    let height = i32::try_from(size[1].as_i64()?).ok()?;
    if width <= 0 || height <= 0 {
        return None;
    }
    Some((x, y, width, height))
}

fn parse_client_center(client: &serde_json::Value) -> Option<(i32, i32)> {
    let (x, y, width, height) = parse_client_geometry(client)?;
    Some((x.saturating_add(width / 2), y.saturating_add(height / 2)))
}

pub(super) fn hypr_client_match_from_json(
    stdout: &[u8],
    expected_title: &str,
) -> Option<HyprClientMatch> {
    let parsed: serde_json::Value = serde_json::from_slice(stdout).ok()?;
    let clients = parsed.as_array()?;
    for client in clients {
        let Some(title) = client.get("title").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if title != expected_title {
            continue;
        }
        let Some(address) = client.get("address").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let pinned = client
            .get("pinned")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        return Some(HyprClientMatch {
            address: address.to_string(),
            pinned,
            center: parse_client_center(client),
            geometry: parse_client_geometry(client),
        });
    }
    None
}

fn find_hypr_window_match(expected_title: &str) -> Option<HyprClientMatch> {
    let outcome = Command::new("hyprctl")
        .args(["-j", "clients"])
        .output()
        .ok()?;
    if !outcome.status.success() {
        return None;
    }
    hypr_client_match_from_json(&outcome.stdout, expected_title)
}

fn retry_until_some<T, F, S>(
    retry_count: u8,
    retry_delay: Duration,
    mut action: F,
    mut sleep: S,
) -> Option<T>
where
    F: FnMut(u8) -> Option<T>,
    S: FnMut(Duration),
{
    if retry_count == 0 {
        return None;
    }

    for attempt in 1..=retry_count {
        if let Some(value) = action(attempt) {
            return Some(value);
        }

        if attempt < retry_count {
            sleep(retry_delay);
        }
    }

    None
}

fn socket2_path() -> Option<PathBuf> {
    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
    Some(
        PathBuf::from(runtime_dir)
            .join("hypr")
            .join(signature)
            .join(".socket2.sock"),
    )
}

fn open_socket2_reader() -> Option<BufReader<UnixStream>> {
    let path = socket2_path()?;
    let stream = UnixStream::connect(path).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(80)));
    Some(BufReader::new(stream))
}

fn parse_pin_event(line: &str) -> Option<(&str, bool)> {
    let payload = line.trim();
    let event_data = payload.strip_prefix("pin>>")?;
    let (address, pin_state) = event_data.split_once(',')?;
    let pinned = match pin_state.trim() {
        "1" => true,
        "0" => false,
        _ => return None,
    };
    Some((address.trim(), pinned))
}

fn wait_for_pin_event(
    reader: &mut BufReader<UnixStream>,
    window_address: &str,
    expected_pinned: bool,
    timeout: Duration,
) -> Option<bool> {
    let deadline = Instant::now() + timeout;
    let mut line = String::new();
    while Instant::now() < deadline {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => continue,
            Ok(_) => {
                if let Some((event_address, pinned)) = parse_pin_event(&line) {
                    if event_address == window_address {
                        return Some(pinned == expected_pinned);
                    }
                }
            }
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => return None,
        }
    }
    None
}

fn apply_hypr_window_surface_props(window_name: &str, address: &str) {
    for (property, value) in [
        ("decorate", HyprPropValue::Toggle(false)),
        ("border_size", HyprPropValue::Number(0)),
        ("rounding", HyprPropValue::Number(0)),
        ("no_blur", HyprPropValue::Toggle(true)),
        ("no_dim", HyprPropValue::Toggle(true)),
        ("no_shadow", HyprPropValue::Toggle(true)),
    ] {
        if !run_window_dispatch(
            window_name,
            WindowDispatch::SetProp {
                address,
                property,
                value,
            },
        ) {
            tracing::debug!(
                window = window_name,
                address = address,
                property = property,
                "hyprctl setprop could not be applied"
            );
        }
    }
}

pub(super) fn request_window_floating_with_geometry(
    window_name: &str,
    expected_title: &str,
    strip_surface: bool,
    geometry: Option<(i32, i32, i32, i32)>,
) {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        tracing::debug!(
            window = window_name,
            "skipping floating dispatch outside Hyprland"
        );
        return;
    }

    let window_name = window_name.to_string();
    let expected_title = expected_title.to_string();
    std::thread::spawn(move || {
        let Some(matched) = retry_until_some(
            HYPR_FLOAT_RETRY_COUNT,
            HYPR_FLOAT_RETRY_DELAY,
            |_| find_hypr_window_match(&expected_title),
            std::thread::sleep,
        ) else {
            tracing::debug!(
                window = window_name,
                title = expected_title,
                "hypr window address lookup failed for floating request"
            );
            return;
        };

        let address = matched.address.as_str();
        if !run_window_dispatch(&window_name, WindowDispatch::Float { address }) {
            tracing::warn!(
                window = window_name,
                address = address,
                title = expected_title,
                "failed to request Hyprland floating for exact window"
            );
            return;
        }

        tracing::debug!(
            window = window_name,
            address = address,
            title = expected_title,
            "requested Hyprland floating for exact window"
        );

        if strip_surface {
            apply_hypr_window_surface_props(&window_name, address);
        }

        if let Some((x, y, width, height)) = geometry {
            for dispatch in [
                WindowDispatch::ResizeExact {
                    address,
                    width,
                    height,
                },
                WindowDispatch::MoveExact { address, x, y },
            ] {
                if !run_window_dispatch(&window_name, dispatch) {
                    tracing::warn!(
                        window = window_name,
                        address = address,
                        dispatch = dispatch.label(),
                        "window geometry dispatch failed"
                    );
                }
            }
        }
    });
}

pub(super) fn current_window_pin_state(expected_title: &str) -> Option<bool> {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    find_hypr_window_match(expected_title).map(|matched| matched.pinned)
}

pub(super) fn current_window_center(expected_title: &str) -> Option<(i32, i32)> {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    find_hypr_window_match(expected_title).and_then(|matched| matched.center)
}

pub(super) fn current_window_geometry(expected_title: &str) -> Option<(i32, i32, i32, i32)> {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    find_hypr_window_match(expected_title).and_then(|matched| matched.geometry)
}

pub(super) fn request_window_pin(window_name: &str, expected_title: &str, pinned: bool) -> bool {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        tracing::debug!(
            window = window_name,
            "skipping pin dispatch outside Hyprland"
        );
        return true;
    }

    let window_name = window_name.to_string();
    let expected_title = expected_title.to_string();
    let applied = retry_until_some(
        HYPR_FLOAT_RETRY_COUNT,
        HYPR_FLOAT_RETRY_DELAY,
        |_| {
            let matched = find_hypr_window_match(&expected_title)?;
            if matched.pinned == pinned {
                tracing::debug!(
                    window = window_name,
                    title = expected_title,
                    pinned = pinned,
                    "pin request already satisfied"
                );
                return Some(true);
            }

            let mut event_reader = open_socket2_reader();
            if run_window_dispatch(
                &window_name,
                WindowDispatch::Pin {
                    address: &matched.address,
                },
            ) {
                tracing::debug!(
                    window = window_name,
                    address = matched.address,
                    pinned = pinned,
                    "requested Hyprland pin toggle for exact window"
                );
                if let Some(reader) = event_reader.as_mut() {
                    let confirmed = wait_for_pin_event(
                        reader,
                        &matched.address,
                        pinned,
                        HYPR_PIN_EVENT_TIMEOUT,
                    )
                    .unwrap_or(false);
                    if confirmed {
                        return Some(true);
                    }
                }
            } else {
                tracing::debug!(
                    window = window_name,
                    address = matched.address,
                    pinned = pinned,
                    "hyprctl pin could not be applied"
                );
            }

            if let Some(verified) = find_hypr_window_match(&expected_title) {
                if verified.pinned == pinned {
                    tracing::debug!(
                        window = window_name,
                        title = expected_title,
                        pinned = pinned,
                        "pin request verified"
                    );
                    return Some(true);
                }
            }
            None
        },
        std::thread::sleep,
    )
    .unwrap_or(false);

    if applied {
        return true;
    }

    tracing::warn!(
        window = window_name,
        title = expected_title,
        pinned = pinned,
        "failed to apply requested pin state after retries"
    );
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn hypr_client_address_from_json(stdout: &[u8], expected_title: &str) -> Option<String> {
        hypr_client_match_from_json(stdout, expected_title).map(|item| item.address)
    }

    fn rendered(dispatch: WindowDispatch<'_>, dialect: u8) -> Vec<String> {
        dispatch.args(dialect)
    }

    #[test]
    fn window_dispatch_renders_legacy_arguments() {
        let address = "0x1234";
        assert_eq!(
            rendered(WindowDispatch::Float { address }, HYPR_DIALECT_LEGACY),
            vec!["dispatch", "setfloating", "address:0x1234"]
        );
        assert_eq!(
            rendered(WindowDispatch::Pin { address }, HYPR_DIALECT_LEGACY),
            vec!["dispatch", "pin", "address:0x1234"]
        );
        assert_eq!(
            rendered(
                WindowDispatch::SetProp {
                    address,
                    property: "decorate",
                    value: HyprPropValue::Toggle(false),
                },
                HYPR_DIALECT_LEGACY
            ),
            vec!["dispatch", "setprop", "address:0x1234", "decorate", "off"]
        );
        assert_eq!(
            rendered(
                WindowDispatch::ResizeExact {
                    address,
                    width: 700,
                    height: 500,
                },
                HYPR_DIALECT_LEGACY
            ),
            vec![
                "dispatch",
                "resizewindowpixel",
                "exact 700 500,address:0x1234"
            ]
        );
        assert_eq!(
            rendered(
                WindowDispatch::MoveExact {
                    address,
                    x: 300,
                    y: 200,
                },
                HYPR_DIALECT_LEGACY
            ),
            vec![
                "dispatch",
                "movewindowpixel",
                "exact 300 200,address:0x1234"
            ]
        );
    }

    #[test]
    fn window_dispatch_renders_lua_arguments() {
        let address = "0x1234";
        assert_eq!(
            rendered(WindowDispatch::Float { address }, HYPR_DIALECT_LUA),
            vec![
                "dispatch",
                r#"hl.dsp.window.float({ action = "on", window = hl.get_window("address:0x1234") })"#
            ]
        );
        assert_eq!(
            rendered(WindowDispatch::Pin { address }, HYPR_DIALECT_LUA),
            vec![
                "dispatch",
                r#"hl.dsp.window.pin({ window = hl.get_window("address:0x1234") })"#
            ]
        );
        assert_eq!(
            rendered(
                WindowDispatch::SetProp {
                    address,
                    property: "no_blur",
                    value: HyprPropValue::Toggle(true),
                },
                HYPR_DIALECT_LUA
            ),
            vec![
                "dispatch",
                r#"hl.dsp.window.set_prop({ prop = "no_blur", value = "on", window = hl.get_window("address:0x1234") })"#
            ]
        );
        assert_eq!(
            rendered(
                WindowDispatch::SetProp {
                    address,
                    property: "decorate",
                    value: HyprPropValue::Toggle(false),
                },
                HYPR_DIALECT_LUA
            ),
            vec![
                "dispatch",
                r#"hl.dsp.window.set_prop({ prop = "decorate", value = "off", window = hl.get_window("address:0x1234") })"#
            ]
        );
        assert_eq!(
            rendered(
                WindowDispatch::SetProp {
                    address,
                    property: "rounding",
                    value: HyprPropValue::Number(0),
                },
                HYPR_DIALECT_LUA
            ),
            vec![
                "dispatch",
                r#"hl.dsp.window.set_prop({ prop = "rounding", value = 0, window = hl.get_window("address:0x1234") })"#
            ]
        );
        assert_eq!(
            rendered(
                WindowDispatch::ResizeExact {
                    address,
                    width: 700,
                    height: 500,
                },
                HYPR_DIALECT_LUA
            ),
            vec![
                "dispatch",
                r#"hl.dsp.window.resize({ x = 700, y = 500, window = hl.get_window("address:0x1234") })"#
            ]
        );
        assert_eq!(
            rendered(
                WindowDispatch::MoveExact {
                    address,
                    x: 300,
                    y: 200,
                },
                HYPR_DIALECT_LUA
            ),
            vec![
                "dispatch",
                r#"hl.dsp.window.move({ x = 300, y = 200, window = hl.get_window("address:0x1234") })"#
            ]
        );
    }

    #[test]
    fn window_dispatch_clamps_non_positive_size_in_both_dialects() {
        let dispatch = WindowDispatch::ResizeExact {
            address: "0x1234",
            width: 0,
            height: -10,
        };
        assert_eq!(
            rendered(dispatch, HYPR_DIALECT_LEGACY),
            vec!["dispatch", "resizewindowpixel", "exact 1 1,address:0x1234"]
        );
        assert_eq!(
            rendered(dispatch, HYPR_DIALECT_LUA),
            vec![
                "dispatch",
                r#"hl.dsp.window.resize({ x = 1, y = 1, window = hl.get_window("address:0x1234") })"#
            ]
        );
    }

    #[test]
    fn dispatch_dialect_order_tries_legacy_first_until_lua_is_detected() {
        HYPR_DISPATCH_DIALECT.store(HYPR_DIALECT_UNKNOWN, Ordering::Relaxed);
        assert_eq!(
            dispatch_dialect_order(),
            [HYPR_DIALECT_LEGACY, HYPR_DIALECT_LUA]
        );

        HYPR_DISPATCH_DIALECT.store(HYPR_DIALECT_LUA, Ordering::Relaxed);
        assert_eq!(
            dispatch_dialect_order(),
            [HYPR_DIALECT_LUA, HYPR_DIALECT_LEGACY]
        );

        HYPR_DISPATCH_DIALECT.store(HYPR_DIALECT_LEGACY, Ordering::Relaxed);
        assert_eq!(
            dispatch_dialect_order(),
            [HYPR_DIALECT_LEGACY, HYPR_DIALECT_LUA]
        );

        HYPR_DISPATCH_DIALECT.store(HYPR_DIALECT_UNKNOWN, Ordering::Relaxed);
    }

    #[test]
    fn hypr_client_address_from_json_matches_exact_title() {
        let payload = br#"
[
  {"address":"0x100","title":"Preview - first"},
  {"address":"0x200","title":"Preview - second"}
]
"#;
        let address = hypr_client_address_from_json(payload, "Preview - second");
        assert_eq!(address.as_deref(), Some("0x200"));
    }

    #[test]
    fn hypr_client_address_from_json_ignores_non_object_entries() {
        let payload = br#"
[
  "ok",
  {"address":"0x300","title":"Preview - stable"}
]
"#;
        let address = hypr_client_address_from_json(payload, "Preview - stable");
        assert_eq!(address.as_deref(), Some("0x300"));
    }

    #[test]
    fn hypr_client_match_from_json_parses_center_when_available() {
        let stdout = br#"[
            {"title":"Preview - a","address":"0x1","pinned":false,"at":[100,200],"size":[600,400]}
        ]"#;
        let item = hypr_client_match_from_json(stdout, "Preview - a").expect("match");
        assert_eq!(item.address, "0x1");
        assert_eq!(item.center, Some((400, 400)));
        assert_eq!(item.geometry, Some((100, 200, 600, 400)));
    }

    #[test]
    fn retry_until_some_returns_value_without_extra_retries() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let sleeps = Rc::new(RefCell::new(Vec::new()));

        let result = retry_until_some(
            5,
            Duration::from_millis(10),
            {
                let calls = calls.clone();
                move |attempt| {
                    calls.borrow_mut().push(attempt);
                    (attempt == 3).then_some("matched")
                }
            },
            {
                let sleeps = sleeps.clone();
                move |duration| sleeps.borrow_mut().push(duration)
            },
        );

        assert_eq!(result, Some("matched"));
        assert_eq!(*calls.borrow(), vec![1, 2, 3]);
        assert_eq!(
            *sleeps.borrow(),
            vec![Duration::from_millis(10), Duration::from_millis(10)]
        );
    }

    #[test]
    fn retry_until_some_stops_after_max_attempts() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let sleeps = Rc::new(RefCell::new(Vec::new()));

        let result = retry_until_some(
            4,
            Duration::from_millis(5),
            {
                let calls = calls.clone();
                move |attempt| {
                    calls.borrow_mut().push(attempt);
                    None::<u8>
                }
            },
            {
                let sleeps = sleeps.clone();
                move |duration| sleeps.borrow_mut().push(duration)
            },
        );

        assert_eq!(result, None);
        assert_eq!(*calls.borrow(), vec![1, 2, 3, 4]);
        assert_eq!(
            *sleeps.borrow(),
            vec![
                Duration::from_millis(5),
                Duration::from_millis(5),
                Duration::from_millis(5)
            ]
        );
    }
}

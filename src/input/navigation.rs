use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{app_config_path, config_env_dirs, ConfigPathError};

const KEYBINDING_APP_DIR: &str = "chalkak";
const KEYBINDING_CONFIG_FILE: &str = "keybindings.json";
const DEFAULT_PAN_HOLD_KEY: &str = "space";
const DEFAULT_ZOOM_IN_SHORTCUTS: &[&str] = &["ctrl+plus", "ctrl+equal", "ctrl+kp_add"];
const DEFAULT_ZOOM_OUT_SHORTCUTS: &[&str] = &["ctrl+minus", "ctrl+underscore", "ctrl+kp_subtract"];
const DEFAULT_ACTUAL_SIZE_SHORTCUTS: &[&str] = &["ctrl+0", "ctrl+kp_0"];
const DEFAULT_FIT_SHORTCUTS: &[&str] = &["shift+1"];

pub type KeybindingResult<T> = std::result::Result<T, KeybindingError>;

#[derive(Debug, Error)]
pub enum KeybindingError {
    #[error("missing HOME environment variable")]
    MissingHomeDirectory,
    #[error("failed to read keybinding config: {path}")]
    ReadConfig { path: PathBuf, source: io::Error },
    #[error("failed to parse keybinding config")]
    ParseConfig(#[from] serde_json::Error),
    #[error("invalid editor pan hold key binding: {value}")]
    InvalidPanHoldKey { value: String },
    #[error("editor navigation shortcut list cannot be empty: {field}")]
    EmptyEditorNavigationShortcutList { field: &'static str },
    #[error("invalid editor navigation shortcut binding for {field}: {value}")]
    InvalidEditorNavigationShortcut { field: &'static str, value: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifierState {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyChord {
    key_name: String,
    ctrl: bool,
    shift: bool,
    alt: bool,
    super_key: bool,
}

impl KeyChord {
    fn parse(raw: &str) -> Option<Self> {
        let mut chord = Self {
            key_name: String::new(),
            ctrl: false,
            shift: false,
            alt: false,
            super_key: false,
        };
        let mut seen_key = false;
        for part in raw.split('+') {
            let token = normalize_key_name(part)?;
            match token.as_str() {
                "ctrl" | "control" => chord.ctrl = true,
                "shift" => chord.shift = true,
                "alt" | "option" => chord.alt = true,
                "super" | "meta" | "cmd" | "command" | "win" => chord.super_key = true,
                _ => {
                    if seen_key {
                        return None;
                    }
                    chord.key_name = token;
                    seen_key = true;
                }
            }
        }
        if !seen_key {
            return None;
        }
        Some(chord)
    }

    fn matches(&self, key_name: Option<&str>, state: ModifierState) -> bool {
        let Some(normalized_key_name) = key_name.and_then(normalize_key_name) else {
            return false;
        };
        if normalized_key_name != self.key_name {
            return false;
        }
        // Shift is permissive unless explicitly required to tolerate keyboard layouts where
        // symbol keys (e.g. `+`) emit shifted keyvals.
        let shift_matches = !self.shift || state.shift;
        self.ctrl == state.ctrl
            && self.alt == state.alt
            && self.super_key == state.super_key
            && shift_matches
    }

    fn as_string(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("ctrl");
        }
        if self.shift {
            parts.push("shift");
        }
        if self.alt {
            parts.push("alt");
        }
        if self.super_key {
            parts.push("super");
        }
        parts.push(self.key_name.as_str());
        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoomScrollModifier {
    None,
    #[default]
    Control,
    Shift,
    Alt,
    Super,
}

impl ZoomScrollModifier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Control => "control",
            Self::Shift => "shift",
            Self::Alt => "alt",
            Self::Super => "super",
        }
    }

    pub const fn matches(self, state: ModifierState) -> bool {
        match self {
            Self::None => true,
            Self::Control => state.ctrl,
            Self::Shift => state.shift,
            Self::Alt => state.alt,
            Self::Super => state.super_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorNavigationBindings {
    pan_hold_key: String,
    zoom_scroll_modifier: ZoomScrollModifier,
    zoom_in_shortcuts: Vec<KeyChord>,
    zoom_out_shortcuts: Vec<KeyChord>,
    actual_size_shortcuts: Vec<KeyChord>,
    fit_shortcuts: Vec<KeyChord>,
}

impl Default for EditorNavigationBindings {
    fn default() -> Self {
        fn parse_defaults(values: &[&str]) -> Vec<KeyChord> {
            values
                .iter()
                .filter_map(|raw| KeyChord::parse(raw))
                .collect()
        }

        Self {
            pan_hold_key: DEFAULT_PAN_HOLD_KEY.to_string(),
            zoom_scroll_modifier: ZoomScrollModifier::Control,
            zoom_in_shortcuts: parse_defaults(DEFAULT_ZOOM_IN_SHORTCUTS),
            zoom_out_shortcuts: parse_defaults(DEFAULT_ZOOM_OUT_SHORTCUTS),
            actual_size_shortcuts: parse_defaults(DEFAULT_ACTUAL_SIZE_SHORTCUTS),
            fit_shortcuts: parse_defaults(DEFAULT_FIT_SHORTCUTS),
        }
    }
}

impl EditorNavigationBindings {
    fn parse_shortcuts(field: &'static str, values: &[String]) -> KeybindingResult<Vec<KeyChord>> {
        if values.is_empty() {
            return Err(KeybindingError::EmptyEditorNavigationShortcutList { field });
        }

        let mut parsed = Vec::with_capacity(values.len());
        for value in values {
            let Some(chord) = KeyChord::parse(value) else {
                return Err(KeybindingError::InvalidEditorNavigationShortcut {
                    field,
                    value: value.clone(),
                });
            };
            parsed.push(chord);
        }
        Ok(parsed)
    }

    fn from_file_config(config: EditorNavigationBindingConfigFile) -> KeybindingResult<Self> {
        let Some(pan_hold_key) = normalize_key_name(&config.pan_hold_key) else {
            return Err(KeybindingError::InvalidPanHoldKey {
                value: config.pan_hold_key,
            });
        };
        let zoom_in_shortcuts =
            Self::parse_shortcuts("zoom_in_shortcuts", &config.zoom_in_shortcuts)?;
        let zoom_out_shortcuts =
            Self::parse_shortcuts("zoom_out_shortcuts", &config.zoom_out_shortcuts)?;
        let actual_size_shortcuts =
            Self::parse_shortcuts("actual_size_shortcuts", &config.actual_size_shortcuts)?;
        let fit_shortcuts = Self::parse_shortcuts("fit_shortcuts", &config.fit_shortcuts)?;

        Ok(Self {
            pan_hold_key,
            zoom_scroll_modifier: config.zoom_scroll_modifier,
            zoom_in_shortcuts,
            zoom_out_shortcuts,
            actual_size_shortcuts,
            fit_shortcuts,
        })
    }

    pub fn pan_hold_key_name(&self) -> &str {
        &self.pan_hold_key
    }

    pub const fn zoom_scroll_modifier(&self) -> ZoomScrollModifier {
        self.zoom_scroll_modifier
    }

    pub fn matches_pan_hold_key_name(&self, key_name: Option<&str>) -> bool {
        key_name
            .and_then(normalize_key_name)
            .is_some_and(|value| value == self.pan_hold_key)
    }

    pub fn matches_zoom_scroll_modifier(&self, state: ModifierState) -> bool {
        self.zoom_scroll_modifier.matches(state)
    }

    pub fn matches_zoom_in_shortcut(&self, key_name: Option<&str>, state: ModifierState) -> bool {
        self.zoom_in_shortcuts
            .iter()
            .any(|shortcut| shortcut.matches(key_name, state))
    }

    pub fn matches_zoom_out_shortcut(&self, key_name: Option<&str>, state: ModifierState) -> bool {
        self.zoom_out_shortcuts
            .iter()
            .any(|shortcut| shortcut.matches(key_name, state))
    }

    pub fn matches_actual_size_shortcut(
        &self,
        key_name: Option<&str>,
        state: ModifierState,
    ) -> bool {
        self.actual_size_shortcuts
            .iter()
            .any(|shortcut| shortcut.matches(key_name, state))
    }

    pub fn matches_fit_shortcut(&self, key_name: Option<&str>, state: ModifierState) -> bool {
        self.fit_shortcuts
            .iter()
            .any(|shortcut| shortcut.matches(key_name, state))
    }

    pub fn zoom_in_shortcuts(&self) -> String {
        self.zoom_in_shortcuts
            .iter()
            .map(KeyChord::as_string)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn zoom_out_shortcuts(&self) -> String {
        self.zoom_out_shortcuts
            .iter()
            .map(KeyChord::as_string)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn actual_size_shortcuts(&self) -> String {
        self.actual_size_shortcuts
            .iter()
            .map(KeyChord::as_string)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn fit_shortcuts(&self) -> String {
        self.fit_shortcuts
            .iter()
            .map(KeyChord::as_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct KeybindingConfigFile {
    editor_navigation: EditorNavigationBindingConfigFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct EditorNavigationBindingConfigFile {
    pan_hold_key: String,
    zoom_scroll_modifier: ZoomScrollModifier,
    zoom_in_shortcuts: Vec<String>,
    zoom_out_shortcuts: Vec<String>,
    actual_size_shortcuts: Vec<String>,
    fit_shortcuts: Vec<String>,
}

impl Default for EditorNavigationBindingConfigFile {
    fn default() -> Self {
        fn defaults(items: &[&str]) -> Vec<String> {
            items.iter().map(|value| (*value).to_string()).collect()
        }

        Self {
            pan_hold_key: DEFAULT_PAN_HOLD_KEY.to_string(),
            zoom_scroll_modifier: ZoomScrollModifier::Control,
            zoom_in_shortcuts: defaults(DEFAULT_ZOOM_IN_SHORTCUTS),
            zoom_out_shortcuts: defaults(DEFAULT_ZOOM_OUT_SHORTCUTS),
            actual_size_shortcuts: defaults(DEFAULT_ACTUAL_SIZE_SHORTCUTS),
            fit_shortcuts: defaults(DEFAULT_FIT_SHORTCUTS),
        }
    }
}

fn normalize_key_name(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    let canonical = match normalized.as_str() {
        "ctrl" | "control" | "control_l" | "control_r" => "control",
        "shift_l" | "shift_r" => "shift",
        "alt_l" | "alt_r" | "option" | "option_l" | "option_r" => "alt",
        "super_l" | "super_r" | "meta" | "meta_l" | "meta_r" | "cmd" | "command" | "win" => "super",
        "exclam" => "1",
        "at" => "2",
        "numbersign" => "3",
        "dollar" => "4",
        "percent" => "5",
        "asciicircum" => "6",
        "ampersand" => "7",
        "asterisk" => "8",
        "parenleft" => "9",
        "parenright" => "0",
        _ => normalized.as_str(),
    };
    Some(canonical.to_string())
}

pub fn load_editor_navigation_bindings() -> KeybindingResult<EditorNavigationBindings> {
    let (xdg_config_home, home) = config_env_dirs();
    load_editor_navigation_bindings_with(xdg_config_home.as_deref(), home.as_deref())
}

fn load_editor_navigation_bindings_with(
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> KeybindingResult<EditorNavigationBindings> {
    let path = keybinding_config_path_with(xdg_config_home, home)?;
    if !path.exists() {
        return Ok(EditorNavigationBindings::default());
    }

    let serialized = fs::read_to_string(&path).map_err(|source| KeybindingError::ReadConfig {
        path: path.clone(),
        source,
    })?;
    let config: KeybindingConfigFile = serde_json::from_str(&serialized)?;
    EditorNavigationBindings::from_file_config(config.editor_navigation)
}

fn keybinding_config_path_with(
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> KeybindingResult<PathBuf> {
    app_config_path(
        KEYBINDING_APP_DIR,
        KEYBINDING_CONFIG_FILE,
        xdg_config_home,
        home,
    )
    .map_err(|error| match error {
        ConfigPathError::MissingHomeDirectory => KeybindingError::MissingHomeDirectory,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let pid = std::process::id();
        path.push(format!("chalkak-keybindings-{pid}-{nanos}"));
        path
    }

    fn with_temp_root<F: FnOnce(&Path)>(f: F) {
        let root = fixture_root();
        fs::create_dir_all(&root).unwrap();
        f(&root);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn editor_navigation_keybindings_default_when_missing() {
        with_temp_root(|root| {
            let bindings = load_editor_navigation_bindings_with(Some(root), None).unwrap();
            assert_eq!(bindings.pan_hold_key_name(), "space");
            assert_eq!(bindings.zoom_scroll_modifier(), ZoomScrollModifier::Control);
            assert!(bindings.matches_zoom_in_shortcut(
                Some("plus"),
                ModifierState {
                    ctrl: true,
                    ..Default::default()
                }
            ));
            assert!(bindings.matches_zoom_out_shortcut(
                Some("minus"),
                ModifierState {
                    ctrl: true,
                    ..Default::default()
                }
            ));
            assert!(bindings.matches_actual_size_shortcut(
                Some("0"),
                ModifierState {
                    ctrl: true,
                    ..Default::default()
                }
            ));
            assert!(bindings.matches_fit_shortcut(
                Some("1"),
                ModifierState {
                    shift: true,
                    ..Default::default()
                }
            ));
            assert!(bindings.matches_fit_shortcut(
                Some("exclam"),
                ModifierState {
                    shift: true,
                    ..Default::default()
                }
            ));
        });
    }

    #[test]
    fn editor_navigation_keybindings_load_custom_values() {
        with_temp_root(|root| {
            let path = keybinding_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(
                &path,
                r#"{
  "editor_navigation": {
    "pan_hold_key": "h",
    "zoom_scroll_modifier": "alt",
    "zoom_in_shortcuts": ["alt+kp_add"],
    "zoom_out_shortcuts": ["alt+kp_subtract"],
    "actual_size_shortcuts": ["alt+0"],
    "fit_shortcuts": ["alt+1"]
  }
}"#,
            )
            .unwrap();

            let bindings = load_editor_navigation_bindings_with(Some(root), None).unwrap();
            assert_eq!(bindings.pan_hold_key_name(), "h");
            assert_eq!(bindings.zoom_scroll_modifier(), ZoomScrollModifier::Alt);
            assert!(bindings.matches_pan_hold_key_name(Some("H")));
            assert!(bindings.matches_zoom_scroll_modifier(ModifierState {
                alt: true,
                ..Default::default()
            }));
            assert!(bindings.matches_zoom_in_shortcut(
                Some("kp_add"),
                ModifierState {
                    alt: true,
                    ..Default::default()
                }
            ));
            assert!(bindings.matches_zoom_out_shortcut(
                Some("kp_subtract"),
                ModifierState {
                    alt: true,
                    ..Default::default()
                }
            ));
            assert!(bindings.matches_actual_size_shortcut(
                Some("0"),
                ModifierState {
                    alt: true,
                    ..Default::default()
                }
            ));
            assert!(bindings.matches_fit_shortcut(
                Some("1"),
                ModifierState {
                    alt: true,
                    ..Default::default()
                }
            ));
        });
    }

    #[test]
    fn editor_navigation_keybindings_normalize_pan_hold_modifier_aliases() {
        with_temp_root(|root| {
            let path = keybinding_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(
                &path,
                r#"{
  "editor_navigation": {
    "pan_hold_key": "ctrl"
  }
}"#,
            )
            .unwrap();

            let bindings = load_editor_navigation_bindings_with(Some(root), None).unwrap();
            assert_eq!(bindings.pan_hold_key_name(), "control");
            assert!(bindings.matches_pan_hold_key_name(Some("control_l")));
            assert!(bindings.matches_pan_hold_key_name(Some("control_r")));
            assert!(bindings.matches_pan_hold_key_name(Some("ctrl")));
        });
    }

    #[test]
    fn editor_navigation_keybindings_reject_empty_pan_hold_key() {
        with_temp_root(|root| {
            let path = keybinding_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(
                &path,
                r#"{
  "editor_navigation": {
    "pan_hold_key": "   "
  }
}"#,
            )
            .unwrap();

            let err = load_editor_navigation_bindings_with(Some(root), None).unwrap_err();
            assert!(matches!(err, KeybindingError::InvalidPanHoldKey { .. }));
        });
    }

    #[test]
    fn editor_navigation_keybindings_reject_invalid_shortcut() {
        with_temp_root(|root| {
            let path = keybinding_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(
                &path,
                r#"{
  "editor_navigation": {
    "zoom_in_shortcuts": ["ctrl+"]
  }
}"#,
            )
            .unwrap();

            let err = load_editor_navigation_bindings_with(Some(root), None).unwrap_err();
            assert!(matches!(
                err,
                KeybindingError::InvalidEditorNavigationShortcut { .. }
            ));
        });
    }
}

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{app_config_path, config_env_dirs, ConfigPathError};
use crate::ui::style::{default_color_tokens, ColorTokens};

const THEME_APP_DIR: &str = "chalkak";
const THEME_CONFIG_FILE: &str = "theme.json";

pub type ThemeResult<T> = std::result::Result<T, ThemeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("missing HOME environment variable")]
    MissingHomeDirectory,
    #[error("failed to read theme config: {path}")]
    ReadConfig { path: PathBuf, source: io::Error },
    #[error("failed to write theme config: {path}")]
    WriteConfig { path: PathBuf, source: io::Error },
    #[error("failed to parse theme config")]
    ParseConfig(#[from] serde_json::Error),
}

/// Per-mode color overrides — all fields optional for partial override
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColorOverrides {
    pub focus_ring_color: Option<String>,
    pub focus_ring_glow: Option<String>,
    pub border_color: Option<String>,
    pub panel_background: Option<String>,
    pub canvas_background: Option<String>,
    pub text_color: Option<String>,
    pub accent_gradient: Option<String>,
    pub accent_text_color: Option<String>,
}

/// Color overrides with optional shared defaults + per-mode overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeColors {
    #[serde(default)]
    pub common: ColorOverrides,
    #[serde(default)]
    pub dark: ColorOverrides,
    #[serde(default)]
    pub light: ColorOverrides,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorDefaults {
    #[serde(default)]
    pub rectangle_border_radius: Option<u16>,
    #[serde(default)]
    pub selection_drag_fill_color: Option<String>,
    #[serde(default)]
    pub selection_drag_stroke_color: Option<String>,
    #[serde(default)]
    pub selection_outline_color: Option<String>,
    #[serde(default)]
    pub selection_handle_color: Option<String>,
    #[serde(default)]
    pub default_tool_color: Option<String>,
    #[serde(default)]
    pub default_text_size: Option<u8>,
    #[serde(default)]
    pub default_stroke_width: Option<u8>,
    #[serde(default)]
    pub tool_color_palette: Option<Vec<String>>,
    #[serde(default)]
    pub stroke_width_presets: Option<Vec<i64>>,
    #[serde(default)]
    pub text_size_presets: Option<Vec<i64>>,
}

impl EditorDefaults {
    fn merged_with(&self, overrides: &EditorDefaults) -> EditorDefaults {
        EditorDefaults {
            rectangle_border_radius: overrides
                .rectangle_border_radius
                .or(self.rectangle_border_radius),
            selection_drag_fill_color: overrides
                .selection_drag_fill_color
                .clone()
                .or_else(|| self.selection_drag_fill_color.clone()),
            selection_drag_stroke_color: overrides
                .selection_drag_stroke_color
                .clone()
                .or_else(|| self.selection_drag_stroke_color.clone()),
            selection_outline_color: overrides
                .selection_outline_color
                .clone()
                .or_else(|| self.selection_outline_color.clone()),
            selection_handle_color: overrides
                .selection_handle_color
                .clone()
                .or_else(|| self.selection_handle_color.clone()),
            default_tool_color: overrides
                .default_tool_color
                .clone()
                .or_else(|| self.default_tool_color.clone()),
            default_text_size: overrides.default_text_size.or(self.default_text_size),
            default_stroke_width: overrides.default_stroke_width.or(self.default_stroke_width),
            tool_color_palette: overrides
                .tool_color_palette
                .clone()
                .or_else(|| self.tool_color_palette.clone()),
            stroke_width_presets: overrides
                .stroke_width_presets
                .clone()
                .or_else(|| self.stroke_width_presets.clone()),
            text_size_presets: overrides
                .text_size_presets
                .clone()
                .or_else(|| self.text_size_presets.clone()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorModeDefaults {
    #[serde(default)]
    pub dark: EditorDefaults,
    #[serde(default)]
    pub light: EditorDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub mode: ThemeMode,
    #[serde(default)]
    pub colors: Option<ThemeColors>,
    #[serde(default)]
    pub editor: EditorDefaults,
    #[serde(default)]
    pub editor_modes: Option<EditorModeDefaults>,
}

/// Resolve color tokens for a given mode, applying user overrides on top of defaults.
pub fn resolve_color_tokens(mode: ThemeMode, overrides: Option<&ThemeColors>) -> ColorTokens {
    let mut tokens = default_color_tokens(mode);

    if let Some(colors) = overrides {
        apply_overrides(&mut tokens, &colors.common);
        let mode_overrides = match mode {
            ThemeMode::Dark | ThemeMode::System => &colors.dark,
            ThemeMode::Light => &colors.light,
        };
        apply_overrides(&mut tokens, mode_overrides);
    }

    tokens
}

pub fn resolve_editor_defaults(
    mode: ThemeMode,
    defaults: &EditorDefaults,
    mode_overrides: Option<&EditorModeDefaults>,
) -> EditorDefaults {
    let Some(mode_overrides) = mode_overrides else {
        return defaults.clone();
    };
    let overrides = match mode {
        ThemeMode::Light => &mode_overrides.light,
        ThemeMode::Dark | ThemeMode::System => &mode_overrides.dark,
    };
    defaults.merged_with(overrides)
}

fn apply_overrides(tokens: &mut ColorTokens, overrides: &ColorOverrides) {
    if let Some(ref v) = overrides.focus_ring_color {
        tokens.focus_ring_color = v.clone();
    }
    if let Some(ref v) = overrides.focus_ring_glow {
        tokens.focus_ring_glow = v.clone();
    }
    if let Some(ref v) = overrides.border_color {
        tokens.border_color = v.clone();
    }
    if let Some(ref v) = overrides.panel_background {
        tokens.panel_background = v.clone();
    }
    if let Some(ref v) = overrides.canvas_background {
        tokens.canvas_background = v.clone();
    }
    if let Some(ref v) = overrides.text_color {
        tokens.text_color = v.clone();
    }
    if let Some(ref v) = overrides.accent_gradient {
        tokens.accent_gradient = v.clone();
    }
    if let Some(ref v) = overrides.accent_text_color {
        tokens.accent_text_color = v.clone();
    }
}

pub fn load_theme_config() -> ThemeResult<ThemeConfig> {
    let (xdg_config_home, home) = config_env_dirs();
    load_theme_config_with(xdg_config_home.as_deref(), home.as_deref())
}

fn load_theme_config_with(
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> ThemeResult<ThemeConfig> {
    let path = theme_config_path_with(xdg_config_home, home)?;
    if !path.exists() {
        return Ok(ThemeConfig {
            mode: ThemeMode::System,
            colors: None,
            editor: EditorDefaults::default(),
            editor_modes: None,
        });
    }

    let serialized = fs::read_to_string(&path).map_err(|source| ThemeError::ReadConfig {
        path: path.clone(),
        source,
    })?;
    let config = parse_theme_config_with_aliases(&serialized)?;
    Ok(config)
}

/// Backward-compatible: load only the mode preference
pub fn load_theme_preference() -> ThemeResult<ThemeMode> {
    load_theme_config().map(|c| c.mode)
}

pub fn save_theme_preference(mode: ThemeMode) -> ThemeResult<()> {
    let (xdg_config_home, home) = config_env_dirs();
    save_theme_preference_with(mode, xdg_config_home.as_deref(), home.as_deref())
}

fn save_theme_preference_with(
    mode: ThemeMode,
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> ThemeResult<()> {
    let path = theme_config_path_with(xdg_config_home, home)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ThemeError::WriteConfig {
            path: path.clone(),
            source,
        })?;
    }

    // Preserve existing non-mode settings if config file already exists.
    let existing_config = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| parse_theme_config_with_aliases(&s).ok())
    } else {
        None
    };
    let existing_colors = existing_config.as_ref().and_then(|c| c.colors.clone());
    let existing_editor = existing_config
        .as_ref()
        .map(|c| c.editor.clone())
        .unwrap_or_else(EditorDefaults::default);
    let existing_editor_modes = existing_config.and_then(|c| c.editor_modes);

    let config = ThemeConfig {
        mode,
        colors: existing_colors,
        editor: existing_editor,
        editor_modes: existing_editor_modes,
    };
    let serialized = serde_json::to_string_pretty(&config)?;
    fs::write(&path, serialized).map_err(|source| ThemeError::WriteConfig {
        path: path.clone(),
        source,
    })?;
    Ok(())
}

fn theme_config_path_with(
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> ThemeResult<PathBuf> {
    app_config_path(THEME_APP_DIR, THEME_CONFIG_FILE, xdg_config_home, home).map_err(|error| {
        match error {
            ConfigPathError::MissingHomeDirectory => ThemeError::MissingHomeDirectory,
        }
    })
}

fn parse_theme_config_with_aliases(
    serialized: &str,
) -> std::result::Result<ThemeConfig, serde_json::Error> {
    let raw: serde_json::Value = serde_json::from_str(serialized)?;
    let mut config: ThemeConfig = serde_json::from_value(raw.clone())?;
    apply_editor_aliases(&mut config, &raw)?;
    Ok(config)
}

fn apply_editor_aliases(
    config: &mut ThemeConfig,
    raw: &serde_json::Value,
) -> std::result::Result<(), serde_json::Error> {
    let Some(editor_obj) = raw.get("editor").and_then(serde_json::Value::as_object) else {
        return Ok(());
    };

    if let Some(common_raw) = editor_obj.get("common").filter(|v| !v.is_null()) {
        let common: EditorDefaults = serde_json::from_value(common_raw.clone())?;
        config.editor = config.editor.merged_with(&common);
    }

    let mut nested_modes = EditorModeDefaults::default();
    let mut has_nested_modes = false;

    if let Some(dark_raw) = editor_obj.get("dark").filter(|v| !v.is_null()) {
        nested_modes.dark = serde_json::from_value(dark_raw.clone())?;
        has_nested_modes = true;
    }
    if let Some(light_raw) = editor_obj.get("light").filter(|v| !v.is_null()) {
        nested_modes.light = serde_json::from_value(light_raw.clone())?;
        has_nested_modes = true;
    }

    if has_nested_modes {
        let merged_modes = match config.editor_modes.take() {
            Some(existing) => EditorModeDefaults {
                dark: existing.dark.merged_with(&nested_modes.dark),
                light: existing.light.merged_with(&nested_modes.light),
            },
            None => nested_modes,
        };
        config.editor_modes = Some(merged_modes);
    }

    Ok(())
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
        path.push(format!("chalkak-theme-{pid}-{nanos}"));
        path
    }

    fn with_temp_root<F: FnOnce(&Path)>(f: F) {
        let root = fixture_root();
        fs::create_dir_all(&root).unwrap();
        f(&root);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn theme_persistence_defaults_to_system_when_missing() {
        with_temp_root(|root| {
            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::System);
            assert!(config.colors.is_none());
            assert!(config.editor.rectangle_border_radius.is_none());
            assert!(config.editor.selection_drag_fill_color.is_none());
            assert!(config.editor.selection_drag_stroke_color.is_none());
            assert!(config.editor.selection_outline_color.is_none());
            assert!(config.editor.selection_handle_color.is_none());
            assert!(config.editor.default_tool_color.is_none());
            assert!(config.editor.default_text_size.is_none());
            assert!(config.editor.default_stroke_width.is_none());
            assert!(config.editor.tool_color_palette.is_none());
            assert!(config.editor.stroke_width_presets.is_none());
            assert!(config.editor.text_size_presets.is_none());
            assert!(config.editor_modes.is_none());
        });
    }

    #[test]
    fn theme_persistence_load_and_save_round_trip() {
        with_temp_root(|root| {
            save_theme_preference_with(ThemeMode::Light, Some(root), None).unwrap();
            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::Light);

            save_theme_preference_with(ThemeMode::Dark, Some(root), None).unwrap();
            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::Dark);
        });
    }

    #[test]
    fn theme_persistence_save_keeps_editor_defaults() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(
                &path,
                r##"{
                    "mode": "dark",
                    "editor": {
                        "rectangle_border_radius": 14,
                        "selection_drag_fill_color": "#2B63FF1F",
                        "selection_drag_stroke_color": "#2B63FFE0",
                        "selection_outline_color": "#2B63FFE6",
                        "selection_handle_color": "#2B63FFF2",
                        "default_tool_color": "#12ab34",
                        "default_text_size": 24,
                        "default_stroke_width": 8,
                        "tool_color_palette": ["#12ab34", "#55cc88"],
                        "stroke_width_presets": [2, 6, 10],
                        "text_size_presets": [14, 20, 28]
                    }
                }"##,
            )
            .unwrap();

            save_theme_preference_with(ThemeMode::Light, Some(root), None).unwrap();
            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::Light);
            assert_eq!(config.editor.rectangle_border_radius, Some(14));
            assert_eq!(
                config.editor.selection_drag_fill_color.as_deref(),
                Some("#2B63FF1F")
            );
            assert_eq!(
                config.editor.selection_drag_stroke_color.as_deref(),
                Some("#2B63FFE0")
            );
            assert_eq!(
                config.editor.selection_outline_color.as_deref(),
                Some("#2B63FFE6")
            );
            assert_eq!(
                config.editor.selection_handle_color.as_deref(),
                Some("#2B63FFF2")
            );
            assert_eq!(config.editor.default_tool_color.as_deref(), Some("#12ab34"));
            assert_eq!(config.editor.default_text_size, Some(24));
            assert_eq!(config.editor.default_stroke_width, Some(8));
            assert_eq!(
                config.editor.tool_color_palette,
                Some(vec!["#12ab34".to_string(), "#55cc88".to_string()])
            );
            assert_eq!(config.editor.stroke_width_presets, Some(vec![2, 6, 10]));
            assert_eq!(config.editor.text_size_presets, Some(vec![14, 20, 28]));
            assert!(config.editor_modes.is_none());
        });
    }

    #[test]
    fn theme_persistence_rejects_invalid_payload() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, "{ invalid ").unwrap();
            let err = load_theme_config_with(Some(root), None);
            assert!(err.is_err());
        });
    }

    #[test]
    fn theme_config_parses_color_overrides() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let json = r##"{
                "mode": "dark",
                "colors": {
                    "common": {
                        "focus_ring_color": "#444444",
                        "text_color": "#555555"
                    },
                    "dark": {
                        "canvas_background": "#111111"
                    },
                    "light": {
                        "text_color": "#222222",
                        "accent_text_color": "#333333"
                    }
                }
            }"##;
            fs::write(&path, json).unwrap();

            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::Dark);
            let colors = config.colors.unwrap();
            assert_eq!(colors.common.focus_ring_color.as_deref(), Some("#444444"));
            assert_eq!(colors.common.text_color.as_deref(), Some("#555555"));
            assert_eq!(colors.dark.canvas_background.as_deref(), Some("#111111"));
            assert_eq!(colors.light.text_color.as_deref(), Some("#222222"));
            assert_eq!(colors.light.accent_text_color.as_deref(), Some("#333333"));
        });
    }

    #[test]
    fn theme_config_parses_editor_defaults() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let json = r##"{
                "mode": "dark",
                "editor": {
                    "rectangle_border_radius": 16,
                    "selection_drag_fill_color": "#2B63FF1F",
                    "selection_drag_stroke_color": "#2B63FFE0",
                    "selection_outline_color": "#2B63FFE6",
                    "selection_handle_color": "#2B63FFF2",
                    "default_tool_color": "#ff00aa",
                    "default_text_size": 32,
                    "default_stroke_width": 12,
                    "tool_color_palette": ["#ff00aa", "#00ffaa"],
                    "stroke_width_presets": [3, 7, 11],
                    "text_size_presets": [12, 18, 26]
                }
            }"##;
            fs::write(&path, json).unwrap();

            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::Dark);
            assert_eq!(config.editor.rectangle_border_radius, Some(16));
            assert_eq!(
                config.editor.selection_drag_fill_color.as_deref(),
                Some("#2B63FF1F")
            );
            assert_eq!(
                config.editor.selection_drag_stroke_color.as_deref(),
                Some("#2B63FFE0")
            );
            assert_eq!(
                config.editor.selection_outline_color.as_deref(),
                Some("#2B63FFE6")
            );
            assert_eq!(
                config.editor.selection_handle_color.as_deref(),
                Some("#2B63FFF2")
            );
            assert_eq!(config.editor.default_tool_color.as_deref(), Some("#ff00aa"));
            assert_eq!(config.editor.default_text_size, Some(32));
            assert_eq!(config.editor.default_stroke_width, Some(12));
            assert_eq!(
                config.editor.tool_color_palette,
                Some(vec!["#ff00aa".to_string(), "#00ffaa".to_string()])
            );
            assert_eq!(config.editor.stroke_width_presets, Some(vec![3, 7, 11]));
            assert_eq!(config.editor.text_size_presets, Some(vec![12, 18, 26]));
            assert!(config.editor_modes.is_none());
        });
    }

    #[test]
    fn theme_config_parses_editor_common_and_mode_aliases() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let json = r##"{
                "mode": "dark",
                "editor": {
                    "default_stroke_width": 4,
                    "common": {
                        "default_tool_color": "#aaaaaa",
                        "default_text_size": 20
                    },
                    "dark": {
                        "default_tool_color": "#eeeeee"
                    },
                    "light": {
                        "default_tool_color": "#222222",
                        "default_stroke_width": 2
                    }
                }
            }"##;
            fs::write(&path, json).unwrap();

            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.editor.default_tool_color.as_deref(), Some("#aaaaaa"));
            assert_eq!(config.editor.default_text_size, Some(20));
            assert_eq!(config.editor.default_stroke_width, Some(4));

            let modes = config.editor_modes.expect("expected editor mode defaults");
            assert_eq!(modes.dark.default_tool_color.as_deref(), Some("#eeeeee"));
            assert_eq!(modes.light.default_tool_color.as_deref(), Some("#222222"));
            assert_eq!(modes.light.default_stroke_width, Some(2));
        });
    }

    #[test]
    fn theme_config_editor_mode_aliases_override_top_level_editor_modes() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let json = r##"{
                "mode": "dark",
                "editor_modes": {
                    "dark": {
                        "default_tool_color": "#111111"
                    }
                },
                "editor": {
                    "dark": {
                        "default_tool_color": "#eeeeee"
                    }
                }
            }"##;
            fs::write(&path, json).unwrap();

            let config = load_theme_config_with(Some(root), None).unwrap();
            let modes = config.editor_modes.expect("expected editor mode defaults");
            assert_eq!(modes.dark.default_tool_color.as_deref(), Some("#eeeeee"));
        });
    }

    #[test]
    fn theme_config_accepts_wide_integer_preset_values() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let json = r##"{
                "mode": "dark",
                "editor": {
                    "stroke_width_presets": [2, 512, -1],
                    "text_size_presets": [14, 999, -3]
                }
            }"##;
            fs::write(&path, json).unwrap();

            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.editor.stroke_width_presets, Some(vec![2, 512, -1]));
            assert_eq!(config.editor.text_size_presets, Some(vec![14, 999, -3]));
            assert!(config.editor_modes.is_none());
        });
    }

    #[test]
    fn theme_config_parses_editor_mode_defaults() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let json = r##"{
                "mode": "dark",
                "editor": {
                    "default_tool_color": "#111111",
                    "default_stroke_width": 4
                },
                "editor_modes": {
                    "dark": {
                        "default_tool_color": "#eeeeee"
                    },
                    "light": {
                        "default_tool_color": "#222222",
                        "default_stroke_width": 3
                    }
                }
            }"##;
            fs::write(&path, json).unwrap();

            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::Dark);
            let modes = config.editor_modes.expect("expected editor mode defaults");
            assert_eq!(modes.dark.default_tool_color.as_deref(), Some("#eeeeee"));
            assert_eq!(modes.light.default_tool_color.as_deref(), Some("#222222"));
            assert_eq!(modes.light.default_stroke_width, Some(3));
        });
    }

    #[test]
    fn resolve_editor_defaults_applies_mode_specific_overrides() {
        let shared = EditorDefaults {
            default_tool_color: Some("#111111".to_string()),
            default_stroke_width: Some(4),
            ..EditorDefaults::default()
        };
        let modes = EditorModeDefaults {
            dark: EditorDefaults {
                default_tool_color: Some("#eeeeee".to_string()),
                ..EditorDefaults::default()
            },
            light: EditorDefaults {
                default_stroke_width: Some(2),
                ..EditorDefaults::default()
            },
        };

        let dark = resolve_editor_defaults(ThemeMode::Dark, &shared, Some(&modes));
        assert_eq!(dark.default_tool_color.as_deref(), Some("#eeeeee"));
        assert_eq!(dark.default_stroke_width, Some(4));

        let light = resolve_editor_defaults(ThemeMode::Light, &shared, Some(&modes));
        assert_eq!(light.default_tool_color.as_deref(), Some("#111111"));
        assert_eq!(light.default_stroke_width, Some(2));

        let system = resolve_editor_defaults(ThemeMode::System, &shared, Some(&modes));
        assert_eq!(system.default_tool_color.as_deref(), Some("#eeeeee"));
        assert_eq!(system.default_stroke_width, Some(4));
    }

    #[test]
    fn theme_persistence_save_keeps_editor_mode_defaults() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(
                &path,
                r##"{
                    "mode": "dark",
                    "editor_modes": {
                        "dark": { "default_tool_color": "#eeeeee" },
                        "light": { "default_tool_color": "#111111" }
                    }
                }"##,
            )
            .unwrap();

            save_theme_preference_with(ThemeMode::Light, Some(root), None).unwrap();
            let config = load_theme_config_with(Some(root), None).unwrap();
            let modes = config.editor_modes.expect("expected editor mode defaults");
            assert_eq!(modes.dark.default_tool_color.as_deref(), Some("#eeeeee"));
            assert_eq!(modes.light.default_tool_color.as_deref(), Some("#111111"));
        });
    }

    #[test]
    fn theme_persistence_save_keeps_editor_alias_defaults() {
        with_temp_root(|root| {
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(
                &path,
                r##"{
                    "mode": "dark",
                    "editor": {
                        "common": { "default_tool_color": "#aaaaaa", "default_stroke_width": 4 },
                        "dark": { "default_tool_color": "#eeeeee" },
                        "light": { "default_tool_color": "#111111", "default_stroke_width": 2 }
                    }
                }"##,
            )
            .unwrap();

            save_theme_preference_with(ThemeMode::Light, Some(root), None).unwrap();
            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::Light);
            assert_eq!(config.editor.default_tool_color.as_deref(), Some("#aaaaaa"));
            assert_eq!(config.editor.default_stroke_width, Some(4));

            let modes = config.editor_modes.expect("expected editor mode defaults");
            assert_eq!(modes.dark.default_tool_color.as_deref(), Some("#eeeeee"));
            assert_eq!(modes.light.default_tool_color.as_deref(), Some("#111111"));
            assert_eq!(modes.light.default_stroke_width, Some(2));
        });
    }

    #[test]
    fn resolve_color_tokens_applies_overrides() {
        let overrides = ThemeColors {
            common: ColorOverrides {
                text_color: Some("#ABABAB".into()),
                ..Default::default()
            },
            dark: ColorOverrides {
                canvas_background: Some("#000000".into()),
                ..Default::default()
            },
            light: ColorOverrides {
                text_color: Some("#FFFFFF".into()),
                ..Default::default()
            },
        };

        let dark = resolve_color_tokens(ThemeMode::Dark, Some(&overrides));
        assert_eq!(dark.canvas_background, "#000000");
        assert_eq!(dark.text_color, "#ABABAB");
        // Non-overridden fields keep defaults
        assert_eq!(dark.accent_text_color, "#09090B");

        let light = resolve_color_tokens(ThemeMode::Light, Some(&overrides));
        assert_eq!(light.text_color, "#FFFFFF");
        assert_eq!(light.canvas_background, "#FAFAFA");
        assert_eq!(light.accent_text_color, "#FFFFFF");
    }

    #[test]
    fn resolve_color_tokens_without_overrides_returns_defaults() {
        let dark = resolve_color_tokens(ThemeMode::Dark, None);
        assert_eq!(dark.canvas_background, "#09090B");

        let light = resolve_color_tokens(ThemeMode::Light, None);
        assert_eq!(light.canvas_background, "#FAFAFA");
    }

    #[test]
    fn resolve_color_tokens_applies_all_override_fields() {
        let overrides = ThemeColors {
            common: ColorOverrides::default(),
            dark: ColorOverrides {
                focus_ring_color: Some("#AAAAAA".into()),
                focus_ring_glow: Some("rgba(1, 2, 3, 0.4)".into()),
                border_color: Some("rgba(4, 5, 6, 0.5)".into()),
                panel_background: Some("rgba(7, 8, 9, 0.6)".into()),
                canvas_background: Some("#101112".into()),
                text_color: Some("#131415".into()),
                accent_gradient: Some("linear-gradient(135deg, #161718 0%, #191A1B 100%)".into()),
                accent_text_color: Some("#1C1D1E".into()),
            },
            light: ColorOverrides::default(),
        };

        let resolved = resolve_color_tokens(ThemeMode::Dark, Some(&overrides));
        assert_eq!(resolved.focus_ring_color, "#AAAAAA");
        assert_eq!(resolved.focus_ring_glow, "rgba(1, 2, 3, 0.4)");
        assert_eq!(resolved.border_color, "rgba(4, 5, 6, 0.5)");
        assert_eq!(resolved.panel_background, "rgba(7, 8, 9, 0.6)");
        assert_eq!(resolved.canvas_background, "#101112");
        assert_eq!(resolved.text_color, "#131415");
        assert_eq!(
            resolved.accent_gradient,
            "linear-gradient(135deg, #161718 0%, #191A1B 100%)"
        );
        assert_eq!(resolved.accent_text_color, "#1C1D1E");
    }

    #[test]
    fn backward_compat_load_theme_preference_still_works() {
        with_temp_root(|root| {
            // Old-format config (no colors field)
            let path = theme_config_path_with(Some(root), None).unwrap();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, r#"{"mode": "light"}"#).unwrap();

            let config = load_theme_config_with(Some(root), None).unwrap();
            assert_eq!(config.mode, ThemeMode::Light);
            assert!(config.colors.is_none());
            assert!(config.editor.rectangle_border_radius.is_none());
            assert!(config.editor.selection_drag_fill_color.is_none());
            assert!(config.editor.selection_drag_stroke_color.is_none());
            assert!(config.editor.selection_outline_color.is_none());
            assert!(config.editor.selection_handle_color.is_none());
            assert!(config.editor.default_tool_color.is_none());
            assert!(config.editor.default_text_size.is_none());
            assert!(config.editor.default_stroke_width.is_none());
            assert!(config.editor.tool_color_palette.is_none());
            assert!(config.editor.stroke_width_presets.is_none());
            assert!(config.editor.text_size_presets.is_none());
            assert!(config.editor_modes.is_none());
        });
    }
}

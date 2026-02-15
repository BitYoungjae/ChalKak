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

/// Color overrides keyed by mode
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeColors {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub mode: ThemeMode,
    #[serde(default)]
    pub colors: Option<ThemeColors>,
    #[serde(default)]
    pub editor: EditorDefaults,
}

/// Convert a requested mode into the active mode used by UI.
///
/// In model-only scope, `System` is retained and resolved by runtime UI layer.
pub fn apply_theme(mode: ThemeMode) -> ThemeMode {
    mode
}

/// Resolve color tokens for a given mode, applying user overrides on top of defaults.
pub fn resolve_color_tokens(mode: ThemeMode, overrides: Option<&ThemeColors>) -> ColorTokens {
    let mut tokens = default_color_tokens(mode);

    if let Some(colors) = overrides {
        let mode_overrides = match mode {
            ThemeMode::Dark | ThemeMode::System => &colors.dark,
            ThemeMode::Light => &colors.light,
        };
        apply_overrides(&mut tokens, mode_overrides);
    }

    tokens
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
        });
    }

    let serialized = fs::read_to_string(&path).map_err(|source| ThemeError::ReadConfig {
        path: path.clone(),
        source,
    })?;
    let config: ThemeConfig = serde_json::from_str(&serialized)?;
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
            .and_then(|s| serde_json::from_str::<ThemeConfig>(&s).ok())
    } else {
        None
    };
    let existing_colors = existing_config.as_ref().and_then(|c| c.colors.clone());
    let existing_editor = existing_config
        .map(|c| c.editor)
        .unwrap_or_else(EditorDefaults::default);

    let config = ThemeConfig {
        mode,
        colors: existing_colors,
        editor: existing_editor,
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
            assert!(config.editor.default_tool_color.is_none());
            assert!(config.editor.default_text_size.is_none());
            assert!(config.editor.default_stroke_width.is_none());
            assert!(config.editor.tool_color_palette.is_none());
            assert!(config.editor.stroke_width_presets.is_none());
            assert!(config.editor.text_size_presets.is_none());
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
            assert_eq!(config.editor.default_tool_color.as_deref(), Some("#12ab34"));
            assert_eq!(config.editor.default_text_size, Some(24));
            assert_eq!(config.editor.default_stroke_width, Some(8));
            assert_eq!(
                config.editor.tool_color_palette,
                Some(vec!["#12ab34".to_string(), "#55cc88".to_string()])
            );
            assert_eq!(config.editor.stroke_width_presets, Some(vec![2, 6, 10]));
            assert_eq!(config.editor.text_size_presets, Some(vec![14, 20, 28]));
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
            assert_eq!(config.editor.default_tool_color.as_deref(), Some("#ff00aa"));
            assert_eq!(config.editor.default_text_size, Some(32));
            assert_eq!(config.editor.default_stroke_width, Some(12));
            assert_eq!(
                config.editor.tool_color_palette,
                Some(vec!["#ff00aa".to_string(), "#00ffaa".to_string()])
            );
            assert_eq!(config.editor.stroke_width_presets, Some(vec![3, 7, 11]));
            assert_eq!(config.editor.text_size_presets, Some(vec![12, 18, 26]));
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
        });
    }

    #[test]
    fn resolve_color_tokens_applies_overrides() {
        let overrides = ThemeColors {
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
        // Non-overridden fields keep defaults
        assert_eq!(dark.text_color, "#E4E4E7");
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
            assert!(config.editor.default_tool_color.is_none());
            assert!(config.editor.default_text_size.is_none());
            assert!(config.editor.default_stroke_width.is_none());
            assert!(config.editor.tool_color_palette.is_none());
            assert!(config.editor.stroke_width_presets.is_none());
            assert!(config.editor.text_size_presets.is_none());
        });
    }
}

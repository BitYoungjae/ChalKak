use crate::input::{load_editor_navigation_bindings, EditorNavigationBindings};
use crate::storage::prune_stale_temp_files;
use crate::theme::{apply_theme, load_theme_config, EditorDefaults, ThemeConfig, ThemeMode};
use crate::ui::{tokens_for, ColorTokens, StyleTokens};

use super::runtime_support::StartupConfig;

pub(super) struct AppBootstrap {
    pub(super) startup_config: StartupConfig,
    pub(super) theme_mode: ThemeMode,
    pub(super) style_tokens: StyleTokens,
    pub(super) color_tokens: ColorTokens,
    pub(super) editor_navigation_bindings: EditorNavigationBindings,
    pub(super) editor_theme_overrides: EditorThemeOverrides,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct EditorThemeOverrides {
    pub(super) rectangle_border_radius: Option<u16>,
    pub(super) default_tool_color: Option<(u8, u8, u8)>,
    pub(super) default_text_size: Option<u8>,
    pub(super) default_stroke_width: Option<u8>,
}

pub(super) fn bootstrap_app_runtime() -> AppBootstrap {
    let startup_config = StartupConfig::from_args();
    prune_stale_capture_temp_files();

    let theme_config = load_or_default_theme_config();
    let editor_theme_overrides = editor_theme_overrides_from(&theme_config.editor);
    let theme_mode = apply_theme(theme_config.mode);
    let (style_tokens, color_tokens) = tokens_for(theme_mode, theme_config.colors.as_ref());
    tracing::info!(theme_mode = ?theme_mode, "loaded theme config");

    let editor_navigation_bindings = load_editor_navigation_bindings().unwrap_or_else(|err| {
        tracing::warn!(?err, "failed to load keybinding config; using defaults");
        EditorNavigationBindings::default()
    });
    tracing::info!(
        pan_hold_key = editor_navigation_bindings.pan_hold_key_name(),
        zoom_scroll_modifier = editor_navigation_bindings.zoom_scroll_modifier().as_str(),
        zoom_in_shortcuts = editor_navigation_bindings.zoom_in_shortcuts(),
        zoom_out_shortcuts = editor_navigation_bindings.zoom_out_shortcuts(),
        actual_size_shortcuts = editor_navigation_bindings.actual_size_shortcuts(),
        fit_shortcuts = editor_navigation_bindings.fit_shortcuts(),
        "loaded editor navigation keybindings"
    );

    AppBootstrap {
        startup_config,
        theme_mode,
        style_tokens,
        color_tokens,
        editor_navigation_bindings,
        editor_theme_overrides,
    }
}

fn prune_stale_capture_temp_files() {
    match prune_stale_temp_files(24) {
        Ok(report) if report.removed_files > 0 => {
            tracing::info!(
                removed_files = report.removed_files,
                "pruned stale capture temp files"
            );
        }
        Ok(_) => {}
        Err(err) => {
            tracing::warn!(
                max_age_hours = 24,
                ?err,
                "failed to prune stale capture temp files"
            );
        }
    }
}

fn load_or_default_theme_config() -> ThemeConfig {
    load_theme_config().unwrap_or_else(|err| {
        tracing::warn!(?err, "failed to load theme config; using defaults");
        ThemeConfig {
            mode: ThemeMode::System,
            colors: None,
            editor: EditorDefaults::default(),
        }
    })
}

fn editor_theme_overrides_from(defaults: &EditorDefaults) -> EditorThemeOverrides {
    let default_tool_color = defaults
        .default_tool_color
        .as_deref()
        .and_then(parse_hex_rgb);
    if defaults.default_tool_color.is_some() && default_tool_color.is_none() {
        tracing::warn!(
            raw = ?defaults.default_tool_color,
            "invalid editor.default_tool_color value in theme config; expected #RRGGBB"
        );
    }

    EditorThemeOverrides {
        rectangle_border_radius: defaults.rectangle_border_radius,
        default_tool_color,
        default_text_size: defaults.default_text_size,
        default_stroke_width: defaults.default_stroke_width,
    }
}

fn parse_hex_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.trim();
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((red, green, blue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_rgb_accepts_hash_or_plain_six_digit_hex() {
        assert_eq!(parse_hex_rgb("#12ab34"), Some((0x12, 0xab, 0x34)));
        assert_eq!(parse_hex_rgb("12AB34"), Some((0x12, 0xab, 0x34)));
    }

    #[test]
    fn parse_hex_rgb_rejects_invalid_values() {
        assert_eq!(parse_hex_rgb("#fff"), None);
        assert_eq!(parse_hex_rgb("#zzzzzz"), None);
        assert_eq!(parse_hex_rgb(""), None);
    }

    #[test]
    fn editor_theme_overrides_parse_default_tool_color() {
        let defaults = EditorDefaults {
            default_tool_color: Some("#101112".to_string()),
            ..EditorDefaults::default()
        };

        let overrides = editor_theme_overrides_from(&defaults);

        assert_eq!(overrides.default_tool_color, Some((0x10, 0x11, 0x12)));
    }
}

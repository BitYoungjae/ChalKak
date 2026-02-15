use crate::input::{load_editor_navigation_bindings, EditorNavigationBindings};
use crate::storage::prune_stale_temp_files;
use crate::theme::{
    load_theme_config, resolve_editor_defaults, EditorDefaults, ThemeConfig, ThemeMode,
};
use crate::ui::{tokens_for, ColorTokens, StyleTokens};
use gtk4::prelude::ObjectExt;

use super::adaptive::{EditorToolOptionPresets, StrokeColorPreset};
use super::editor_popup::{EditorSelectionPalette, RgbaColor};
use super::runtime_support::StartupConfig;

pub(super) struct AppBootstrap {
    pub(super) startup_config: StartupConfig,
    pub(super) theme_config: ThemeConfig,
    pub(super) editor_navigation_bindings: EditorNavigationBindings,
}

pub(super) struct ResolvedThemeRuntime {
    pub(super) style_tokens: StyleTokens,
    pub(super) color_tokens: ColorTokens,
    pub(super) editor_theme_overrides: EditorThemeOverrides,
    pub(super) editor_tool_option_presets: EditorToolOptionPresets,
}

#[derive(Debug, Clone, Default)]
pub(super) struct EditorThemeOverrides {
    pub(super) rectangle_border_radius: Option<u16>,
    pub(super) selection_palette: EditorSelectionPalette,
    pub(super) default_tool_color: Option<(u8, u8, u8)>,
    pub(super) default_text_size: Option<u8>,
    pub(super) default_stroke_width: Option<u8>,
    pub(super) tool_color_palette: Option<Vec<StrokeColorPreset>>,
    pub(super) stroke_width_presets: Option<Vec<u8>>,
    pub(super) text_size_presets: Option<Vec<u8>>,
}

const MIN_STROKE_WIDTH_PRESET: u8 = 1;
const MAX_STROKE_WIDTH_PRESET: u8 = 64;
const MIN_TEXT_SIZE_PRESET: u8 = 8;
const MAX_TEXT_SIZE_PRESET: u8 = 160;
const MAX_TOOL_OPTION_PRESET_COUNT: usize = 6;

pub(super) fn bootstrap_app_runtime() -> AppBootstrap {
    let startup_config = StartupConfig::from_args();
    prune_stale_capture_temp_files();

    let theme_config = load_or_default_theme_config();
    tracing::info!(mode = ?theme_config.mode, "loaded theme config");

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
        theme_config,
        editor_navigation_bindings,
    }
}

pub(super) fn resolve_runtime_theme_mode(
    mode: ThemeMode,
    settings: Option<&gtk4::Settings>,
) -> ThemeMode {
    match mode {
        ThemeMode::Light => ThemeMode::Light,
        ThemeMode::Dark => ThemeMode::Dark,
        ThemeMode::System => settings
            .and_then(system_theme_mode_from_settings)
            .unwrap_or(ThemeMode::Dark),
    }
}

fn system_theme_mode_from_settings(settings: &gtk4::Settings) -> Option<ThemeMode> {
    if settings
        .list_properties()
        .iter()
        .any(|prop| prop.name() == "gtk-interface-color-scheme")
    {
        let color_scheme = settings.property_value("gtk-interface-color-scheme");
        if let Ok(raw_scheme) = color_scheme.get::<i32>() {
            return match raw_scheme {
                // GTK_INTERFACE_COLOR_SCHEME_FORCE_LIGHT
                3 => Some(ThemeMode::Light),
                // GTK_INTERFACE_COLOR_SCHEME_FORCE_DARK
                2 => Some(ThemeMode::Dark),
                // DEFAULT or PREFER_* keep fallback path
                _ => None,
            };
        }
    }

    #[allow(deprecated)]
    {
        Some(if settings.is_gtk_application_prefer_dark_theme() {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        })
    }
}

pub(super) fn resolve_theme_runtime(
    theme_config: &ThemeConfig,
    mode: ThemeMode,
) -> ResolvedThemeRuntime {
    let effective_editor_defaults = resolve_editor_defaults(
        mode,
        &theme_config.editor,
        theme_config.editor_modes.as_ref(),
    );
    let editor_theme_overrides = editor_theme_overrides_from(&effective_editor_defaults, mode);
    let editor_tool_option_presets = EditorToolOptionPresets::with_overrides(
        mode,
        editor_theme_overrides.tool_color_palette.clone(),
        editor_theme_overrides.stroke_width_presets.clone(),
        editor_theme_overrides.text_size_presets.clone(),
    );
    let (style_tokens, color_tokens) = tokens_for(mode, theme_config.colors.as_ref());

    ResolvedThemeRuntime {
        style_tokens,
        color_tokens,
        editor_theme_overrides,
        editor_tool_option_presets,
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
            editor_modes: None,
        }
    })
}

fn editor_theme_overrides_from(defaults: &EditorDefaults, mode: ThemeMode) -> EditorThemeOverrides {
    let mut selection_palette = EditorSelectionPalette::for_theme_mode(mode);
    apply_selection_color_override(
        "editor.selection_drag_fill_color",
        defaults.selection_drag_fill_color.as_deref(),
        &mut selection_palette.drag_fill,
    );
    apply_selection_color_override(
        "editor.selection_drag_stroke_color",
        defaults.selection_drag_stroke_color.as_deref(),
        &mut selection_palette.drag_stroke,
    );
    apply_selection_color_override(
        "editor.selection_outline_color",
        defaults.selection_outline_color.as_deref(),
        &mut selection_palette.selected_outline,
    );
    apply_selection_color_override(
        "editor.selection_handle_color",
        defaults.selection_handle_color.as_deref(),
        &mut selection_palette.resize_handle_fill,
    );

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
    let tool_color_palette = defaults
        .tool_color_palette
        .as_deref()
        .and_then(parse_color_palette_presets);
    let stroke_width_presets = defaults.stroke_width_presets.as_deref().and_then(|values| {
        parse_numeric_presets(
            "editor.stroke_width_presets",
            values,
            MIN_STROKE_WIDTH_PRESET,
            MAX_STROKE_WIDTH_PRESET,
        )
    });
    let text_size_presets = defaults.text_size_presets.as_deref().and_then(|values| {
        parse_numeric_presets(
            "editor.text_size_presets",
            values,
            MIN_TEXT_SIZE_PRESET,
            MAX_TEXT_SIZE_PRESET,
        )
    });

    EditorThemeOverrides {
        rectangle_border_radius: defaults.rectangle_border_radius,
        selection_palette,
        default_tool_color,
        default_text_size: defaults.default_text_size,
        default_stroke_width: defaults.default_stroke_width,
        tool_color_palette,
        stroke_width_presets,
        text_size_presets,
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

fn parse_hash_hex_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.trim();
    if !hex.starts_with('#') {
        return None;
    }
    parse_hex_rgb(hex)
}

fn parse_hash_hex_rgba(value: &str) -> Option<RgbaColor> {
    let hex = value.trim();
    if !hex.starts_with('#') {
        return None;
    }
    let digits = &hex[1..];
    let parse_pair = |index: usize| u8::from_str_radix(&digits[index..index + 2], 16).ok();
    match digits.len() {
        6 => Some(RgbaColor::new(
            parse_pair(0)?,
            parse_pair(2)?,
            parse_pair(4)?,
            0xFF,
        )),
        8 => Some(RgbaColor::new(
            parse_pair(0)?,
            parse_pair(2)?,
            parse_pair(4)?,
            parse_pair(6)?,
        )),
        _ => None,
    }
}

fn apply_selection_color_override(field: &'static str, raw: Option<&str>, target: &mut RgbaColor) {
    let Some(value) = raw else {
        return;
    };
    let Some(parsed) = parse_hash_hex_rgba(value) else {
        tracing::warn!(
            field = field,
            value = value,
            "invalid selection color override; expected #RRGGBB or #RRGGBBAA"
        );
        return;
    };
    *target = parsed;
}

fn parse_color_palette_presets(values: &[String]) -> Option<Vec<StrokeColorPreset>> {
    if values.is_empty() {
        tracing::warn!("editor.tool_color_palette is empty; ignoring override");
        return None;
    }

    let mut parsed: Vec<StrokeColorPreset> = Vec::with_capacity(values.len());
    let mut truncated = false;
    for value in values {
        if parsed.len() >= MAX_TOOL_OPTION_PRESET_COUNT {
            truncated = true;
            break;
        }
        let Some((red, green, blue)) = parse_hash_hex_rgb(value) else {
            tracing::warn!(
                value = value.as_str(),
                "invalid editor.tool_color_palette entry; expected #RRGGBB"
            );
            continue;
        };
        if parsed
            .iter()
            .any(|preset| preset.rgb() == (red, green, blue))
        {
            continue;
        }
        parsed.push(StrokeColorPreset::new(
            format!("#{red:02X}{green:02X}{blue:02X}"),
            red,
            green,
            blue,
        ));
    }
    if truncated {
        tracing::warn!(
            max = MAX_TOOL_OPTION_PRESET_COUNT,
            "editor.tool_color_palette supports up to `max` presets; extra entries were ignored"
        );
    }

    if parsed.is_empty() {
        tracing::warn!("no valid colors in editor.tool_color_palette; ignoring override");
        return None;
    }
    Some(parsed)
}

fn parse_numeric_presets(field: &'static str, values: &[i64], min: u8, max: u8) -> Option<Vec<u8>> {
    if values.is_empty() {
        tracing::warn!(field = field, "empty preset list; ignoring override");
        return None;
    }

    let mut parsed = Vec::with_capacity(values.len());
    let mut truncated = false;
    let min_bound = i64::from(min);
    let max_bound = i64::from(max);
    for value in values {
        if *value < min_bound || *value > max_bound {
            tracing::warn!(
                field = field,
                value = *value,
                min = min,
                max = max,
                "preset value out of supported range; ignoring entry"
            );
            continue;
        }
        let value = *value as u8;
        if !parsed.contains(&value) {
            if parsed.len() >= MAX_TOOL_OPTION_PRESET_COUNT {
                truncated = true;
                break;
            }
            parsed.push(value);
        }
    }
    if truncated {
        tracing::warn!(
            field = field,
            max = MAX_TOOL_OPTION_PRESET_COUNT,
            "preset list supports up to `max` items; extra entries were ignored"
        );
    }

    if parsed.is_empty() {
        tracing::warn!(
            field = field,
            "no valid preset values found; ignoring override"
        );
        return None;
    }
    Some(parsed)
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
    fn parse_hash_hex_rgb_requires_hash_prefix() {
        assert_eq!(parse_hash_hex_rgb("#12ab34"), Some((0x12, 0xab, 0x34)));
        assert_eq!(parse_hash_hex_rgb("12ab34"), None);
    }

    #[test]
    fn parse_hash_hex_rgba_accepts_six_or_eight_digit_hex() {
        assert_eq!(
            parse_hash_hex_rgba("#2B63FF"),
            Some(RgbaColor::new(0x2B, 0x63, 0xFF, 0xFF))
        );
        assert_eq!(
            parse_hash_hex_rgba("#2B63FFE0"),
            Some(RgbaColor::new(0x2B, 0x63, 0xFF, 0xE0))
        );
    }

    #[test]
    fn parse_hash_hex_rgba_rejects_invalid_values() {
        assert_eq!(parse_hash_hex_rgba("2B63FF"), None);
        assert_eq!(parse_hash_hex_rgba("#2B63F"), None);
        assert_eq!(parse_hash_hex_rgba("#GGGGGG"), None);
    }

    #[test]
    fn editor_theme_overrides_parse_default_tool_color() {
        let defaults = EditorDefaults {
            default_tool_color: Some("#101112".to_string()),
            ..EditorDefaults::default()
        };

        let overrides = editor_theme_overrides_from(&defaults, ThemeMode::Dark);

        assert_eq!(overrides.default_tool_color, Some((0x10, 0x11, 0x12)));
    }

    #[test]
    fn editor_theme_overrides_parse_selection_colors() {
        let defaults = EditorDefaults {
            selection_drag_fill_color: Some("#2B63FF1F".to_string()),
            selection_drag_stroke_color: Some("#2B63FFE0".to_string()),
            selection_outline_color: Some("#2B63FFE6".to_string()),
            selection_handle_color: Some("#2B63FFF2".to_string()),
            ..EditorDefaults::default()
        };

        let overrides = editor_theme_overrides_from(&defaults, ThemeMode::Dark);

        assert_eq!(
            overrides.selection_palette.drag_fill,
            RgbaColor::new(0x2B, 0x63, 0xFF, 0x1F)
        );
        assert_eq!(
            overrides.selection_palette.drag_stroke,
            RgbaColor::new(0x2B, 0x63, 0xFF, 0xE0)
        );
        assert_eq!(
            overrides.selection_palette.selected_outline,
            RgbaColor::new(0x2B, 0x63, 0xFF, 0xE6)
        );
        assert_eq!(
            overrides.selection_palette.resize_handle_fill,
            RgbaColor::new(0x2B, 0x63, 0xFF, 0xF2)
        );
    }

    #[test]
    fn editor_theme_overrides_parse_tool_option_presets() {
        let defaults = EditorDefaults {
            tool_color_palette: Some(vec!["#101112".to_string(), "#AABBCC".to_string()]),
            stroke_width_presets: Some(vec![2, 6, 10]),
            text_size_presets: Some(vec![14, 20, 28]),
            ..EditorDefaults::default()
        };

        let overrides = editor_theme_overrides_from(&defaults, ThemeMode::Dark);

        let palette = overrides.tool_color_palette.unwrap();
        let colors = palette
            .iter()
            .map(StrokeColorPreset::rgb)
            .collect::<Vec<_>>();
        assert_eq!(colors, vec![(0x10, 0x11, 0x12), (0xAA, 0xBB, 0xCC)]);
        assert_eq!(overrides.stroke_width_presets, Some(vec![2, 6, 10]));
        assert_eq!(overrides.text_size_presets, Some(vec![14, 20, 28]));
    }

    #[test]
    fn editor_theme_overrides_filter_invalid_tool_option_presets() {
        let defaults = EditorDefaults {
            tool_color_palette: Some(vec![
                "#invalid".to_string(),
                "#223344".to_string(),
                "123456".to_string(),
                "#223344".to_string(),
            ]),
            stroke_width_presets: Some(vec![0, 4, 4, 200]),
            text_size_presets: Some(vec![0, 16, 16, 200]),
            ..EditorDefaults::default()
        };

        let overrides = editor_theme_overrides_from(&defaults, ThemeMode::Dark);

        let palette = overrides.tool_color_palette.unwrap();
        assert_eq!(palette.len(), 1);
        assert_eq!(palette[0].rgb(), (0x22, 0x33, 0x44));
        assert_eq!(overrides.stroke_width_presets, Some(vec![4]));
        assert_eq!(overrides.text_size_presets, Some(vec![16]));
    }

    #[test]
    fn editor_theme_overrides_limit_tool_option_presets_to_six_items() {
        let defaults = EditorDefaults {
            tool_color_palette: Some(vec![
                "#111111".to_string(),
                "#222222".to_string(),
                "#333333".to_string(),
                "#444444".to_string(),
                "#555555".to_string(),
                "#666666".to_string(),
                "#777777".to_string(),
            ]),
            stroke_width_presets: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            text_size_presets: Some(vec![10, 12, 14, 16, 18, 20, 22]),
            ..EditorDefaults::default()
        };

        let overrides = editor_theme_overrides_from(&defaults, ThemeMode::Dark);

        let palette = overrides.tool_color_palette.unwrap();
        assert_eq!(palette.len(), 6);
        assert_eq!(
            palette
                .iter()
                .map(StrokeColorPreset::rgb)
                .collect::<Vec<_>>(),
            vec![
                (0x11, 0x11, 0x11),
                (0x22, 0x22, 0x22),
                (0x33, 0x33, 0x33),
                (0x44, 0x44, 0x44),
                (0x55, 0x55, 0x55),
                (0x66, 0x66, 0x66),
            ]
        );
        assert_eq!(overrides.stroke_width_presets, Some(vec![1, 2, 3, 4, 5, 6]));
        assert_eq!(
            overrides.text_size_presets,
            Some(vec![10, 12, 14, 16, 18, 20])
        );
    }

    #[test]
    fn editor_theme_overrides_ignore_wide_out_of_range_numeric_presets() {
        let defaults = EditorDefaults {
            stroke_width_presets: Some(vec![2, 512, -1]),
            text_size_presets: Some(vec![14, 999, -3]),
            ..EditorDefaults::default()
        };

        let overrides = editor_theme_overrides_from(&defaults, ThemeMode::Dark);
        assert_eq!(overrides.stroke_width_presets, Some(vec![2]));
        assert_eq!(overrides.text_size_presets, Some(vec![14]));
    }

    #[test]
    fn editor_theme_overrides_default_selection_palette_follows_mode() {
        let light = editor_theme_overrides_from(&EditorDefaults::default(), ThemeMode::Light);
        let dark = editor_theme_overrides_from(&EditorDefaults::default(), ThemeMode::Dark);
        let system = editor_theme_overrides_from(&EditorDefaults::default(), ThemeMode::System);

        assert_eq!(
            light.selection_palette.drag_fill,
            RgbaColor::new(0x18, 0x18, 0x1B, 0x1A)
        );
        assert_eq!(
            dark.selection_palette.drag_fill,
            RgbaColor::new(0xE4, 0xE4, 0xE7, 0x1F)
        );
        assert_eq!(system.selection_palette, dark.selection_palette);
    }

    #[test]
    fn editor_theme_overrides_selection_colors_override_mode_defaults() {
        let defaults = EditorDefaults {
            selection_drag_fill_color: Some("#00010203".to_string()),
            selection_drag_stroke_color: Some("#04050607".to_string()),
            selection_outline_color: Some("#08090A0B".to_string()),
            selection_handle_color: Some("#0C0D0E0F".to_string()),
            ..EditorDefaults::default()
        };

        let overrides = editor_theme_overrides_from(&defaults, ThemeMode::Light);
        assert_eq!(
            overrides.selection_palette.drag_fill,
            RgbaColor::new(0x00, 0x01, 0x02, 0x03)
        );
        assert_eq!(
            overrides.selection_palette.drag_stroke,
            RgbaColor::new(0x04, 0x05, 0x06, 0x07)
        );
        assert_eq!(
            overrides.selection_palette.selected_outline,
            RgbaColor::new(0x08, 0x09, 0x0A, 0x0B)
        );
        assert_eq!(
            overrides.selection_palette.resize_handle_fill,
            RgbaColor::new(0x0C, 0x0D, 0x0E, 0x0F)
        );
    }

    #[test]
    fn editor_theme_overrides_invalid_selection_colors_keep_mode_defaults() {
        let defaults = EditorDefaults {
            selection_drag_fill_color: Some("invalid".to_string()),
            selection_drag_stroke_color: Some("#zzzzzz".to_string()),
            selection_outline_color: Some("#12345".to_string()),
            selection_handle_color: Some("#123456789".to_string()),
            ..EditorDefaults::default()
        };
        let overrides = editor_theme_overrides_from(&defaults, ThemeMode::Light);
        assert_eq!(
            overrides.selection_palette.drag_fill,
            RgbaColor::new(0x18, 0x18, 0x1B, 0x1A)
        );
        assert_eq!(
            overrides.selection_palette.drag_stroke,
            RgbaColor::new(0x18, 0x18, 0x1B, 0xC4)
        );
        assert_eq!(
            overrides.selection_palette.selected_outline,
            RgbaColor::new(0x18, 0x18, 0x1B, 0xD9)
        );
        assert_eq!(
            overrides.selection_palette.resize_handle_fill,
            RgbaColor::new(0x18, 0x18, 0x1B, 0xE6)
        );
    }

    #[test]
    fn resolve_runtime_theme_mode_preserves_explicit_modes() {
        assert_eq!(
            resolve_runtime_theme_mode(ThemeMode::Light, None),
            ThemeMode::Light
        );
        assert_eq!(
            resolve_runtime_theme_mode(ThemeMode::Dark, None),
            ThemeMode::Dark
        );
    }

    #[test]
    fn resolve_runtime_theme_mode_system_without_settings_defaults_dark() {
        assert_eq!(
            resolve_runtime_theme_mode(ThemeMode::System, None),
            ThemeMode::Dark
        );
    }

    #[test]
    fn resolve_theme_runtime_uses_mode_specific_editor_defaults() {
        let config = ThemeConfig {
            mode: ThemeMode::System,
            colors: None,
            editor: EditorDefaults {
                default_tool_color: Some("#111111".to_string()),
                ..EditorDefaults::default()
            },
            editor_modes: Some(crate::theme::EditorModeDefaults {
                dark: EditorDefaults {
                    default_tool_color: Some("#EEEEEE".to_string()),
                    ..EditorDefaults::default()
                },
                light: EditorDefaults::default(),
            }),
        };

        let runtime = resolve_theme_runtime(&config, ThemeMode::Dark);
        assert_eq!(
            runtime.editor_theme_overrides.default_tool_color,
            Some((0xEE, 0xEE, 0xEE))
        );
    }
}

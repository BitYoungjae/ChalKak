use crate::theme::{resolve_color_tokens, ThemeColors, ThemeMode};
use serde::{Deserialize, Serialize};

/// Runtime color tokens — overridable via theme.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorTokens {
    pub focus_ring_color: String,
    pub focus_ring_glow: String,
    pub border_color: String,
    pub panel_background: String,
    pub canvas_background: String,
    pub text_color: String,
    pub accent_gradient: String,
    pub accent_text_color: String,
}

/// Compile-time layout tokens — not user-overridable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleTokens {
    pub spacing_4: i32,
    pub spacing_8: i32,
    pub spacing_12: i32,
    pub spacing_16: i32,
    pub spacing_20: i32,
    pub spacing_24: i32,
    pub card_radius: u16,
    pub panel_radius: u16,
    pub control_radius: u16,
    pub control_size: u16,
    pub icon_size: u16,
    pub border_width: u16,
    pub preview_default_width: i32,
    pub preview_default_height: i32,
    pub preview_min_width: i32,
    pub preview_min_height: i32,
    pub editor_initial_width: i32,
    pub editor_initial_height: i32,
    pub editor_min_width: i32,
    pub editor_min_height: i32,
    pub editor_toolbar_width: i32,
    pub motion_standard_ms: u32,
    pub motion_hover_ms: u32,
    pub toast_duration_ms: u32,
}

pub const LAYOUT_TOKENS: StyleTokens = StyleTokens {
    spacing_4: 4,
    spacing_8: 8,
    spacing_12: 12,
    spacing_16: 16,
    spacing_20: 20,
    spacing_24: 24,
    card_radius: 14,
    panel_radius: 18,
    control_radius: 12,
    control_size: 40,
    icon_size: 18,
    border_width: 1,
    preview_default_width: 840,
    preview_default_height: 472,
    preview_min_width: 360,
    preview_min_height: 220,
    editor_initial_width: 1280,
    editor_initial_height: 800,
    editor_min_width: 750,
    editor_min_height: 422,
    editor_toolbar_width: 68,
    motion_standard_ms: 220,
    motion_hover_ms: 160,
    toast_duration_ms: 2_000,
};

/// Mumyeong "Bright Mist" light palette
fn default_light_colors() -> ColorTokens {
    ColorTokens {
        canvas_background: "#FAFAFA".into(),
        panel_background: "rgba(255, 255, 255, 0.88)".into(),
        border_color: "rgba(9, 9, 11, 0.08)".into(),
        text_color: "#09090B".into(),
        focus_ring_color: "#18181B".into(),
        focus_ring_glow: "rgba(24, 24, 27, 0.15)".into(),
        accent_gradient: "linear-gradient(135deg, #71717A 0%, #27272A 100%)".into(),
        accent_text_color: "#FFFFFF".into(),
    }
}

/// Mumyeong dark palette (oma series)
fn default_dark_colors() -> ColorTokens {
    ColorTokens {
        canvas_background: "#09090B".into(),
        panel_background: "rgba(24, 24, 27, 0.88)".into(),
        border_color: "rgba(113, 113, 122, 0.25)".into(),
        text_color: "#E4E4E7".into(),
        focus_ring_color: "#F4F4F5".into(),
        focus_ring_glow: "rgba(244, 244, 245, 0.20)".into(),
        accent_gradient: "linear-gradient(135deg, #A1A1AA 0%, #F4F4F5 100%)".into(),
        accent_text_color: "#09090B".into(),
    }
}

pub fn default_color_tokens(mode: ThemeMode) -> ColorTokens {
    match mode {
        ThemeMode::Light => default_light_colors(),
        ThemeMode::Dark | ThemeMode::System => default_dark_colors(),
    }
}

pub fn tokens_for(mode: ThemeMode, overrides: Option<&ThemeColors>) -> (StyleTokens, ColorTokens) {
    (LAYOUT_TOKENS, resolve_color_tokens(mode, overrides))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    #[test]
    fn color_tokens_have_accent_gradient() {
        assert!(default_color_tokens(ThemeMode::Light)
            .accent_gradient
            .contains("linear-gradient"));
        assert!(default_color_tokens(ThemeMode::Dark)
            .accent_gradient
            .contains("linear-gradient"));
    }

    #[test]
    fn layout_tokens_keep_required_control_size() {
        assert_eq!(tokens_for(ThemeMode::Light, None).0.control_size, 40);
        assert_eq!(tokens_for(ThemeMode::Dark, None).0.control_size, 40);
        assert_eq!(tokens_for(ThemeMode::System, None).0.control_size, 40);
    }

    #[test]
    fn layout_tokens_match_component_spec_dimensions() {
        let tokens = tokens_for(ThemeMode::Light, None).0;
        assert_eq!(tokens.preview_min_width, 360);
        assert_eq!(tokens.preview_min_height, 220);
        assert_eq!(tokens.preview_default_width, 840);
        assert_eq!(tokens.preview_default_height, 472);
        assert_eq!(tokens.editor_initial_width, 1280);
        assert_eq!(tokens.editor_initial_height, 800);
        assert_eq!(tokens.editor_min_width, 750);
        assert_eq!(tokens.editor_min_height, 422);
        assert_eq!(tokens.editor_toolbar_width, 68);
    }

    #[test]
    fn layout_tokens_match_component_spec_motion_tokens() {
        let tokens = tokens_for(ThemeMode::Dark, None).0;
        assert_eq!(tokens.motion_standard_ms, 220);
        assert_eq!(tokens.motion_hover_ms, 160);
        assert_eq!(tokens.toast_duration_ms, 2_000);
    }

    #[test]
    fn mumyeong_dark_uses_oma_palette() {
        let colors = default_color_tokens(ThemeMode::Dark);
        assert_eq!(colors.canvas_background, "#09090B");
        assert_eq!(colors.text_color, "#E4E4E7");
        assert_eq!(colors.focus_ring_color, "#F4F4F5");
        assert_eq!(colors.accent_text_color, "#09090B");
    }

    #[test]
    fn mumyeong_light_uses_bright_mist_palette() {
        let colors = default_color_tokens(ThemeMode::Light);
        assert_eq!(colors.canvas_background, "#FAFAFA");
        assert_eq!(colors.text_color, "#09090B");
        assert_eq!(colors.focus_ring_color, "#18181B");
        assert_eq!(colors.accent_text_color, "#FFFFFF");
    }
}

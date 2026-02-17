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

#[cfg(test)]
mod tests {
    use super::LAYOUT_TOKENS;

    #[test]
    fn layout_tokens_keep_required_control_size() {
        assert_eq!(LAYOUT_TOKENS.control_size, 40);
    }

    #[test]
    fn layout_tokens_match_component_spec_dimensions() {
        let tokens = LAYOUT_TOKENS;
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
        let tokens = LAYOUT_TOKENS;
        assert_eq!(tokens.motion_standard_ms, 220);
        assert_eq!(tokens.motion_hover_ms, 160);
        assert_eq!(tokens.toast_duration_ms, 2_000);
    }
}

use crate::ui::{ColorTokens, StyleTokens};
use gtk4::CssProvider;

pub(super) fn install_runtime_css(tokens: StyleTokens, colors: &ColorTokens, motion_enabled: bool) {
    let close_icon_size = tokens.icon_size.saturating_add(2);
    let pin_icon_size = tokens.icon_size.saturating_add(1);
    let motion_standard_ms = if motion_enabled {
        tokens.motion_standard_ms
    } else {
        0
    };
    let motion_hover_ms = if motion_enabled {
        tokens.motion_hover_ms
    } else {
        0
    };
    let css = format!(
        "
window.chalkak-root {{
  background: transparent;
  color: {text_color};
}}
.chalkak-root label {{
  color: {text_color};
}}
.chalkak-root button {{
  color: {text_color};
}}
.chalkak-root button:hover,
.chalkak-root button:active {{
  color: {text_color};
}}
.chalkak-root button image {{
  -gtk-icon-style: symbolic;
  color: inherit;
}}
.chalkak-root button:hover image,
.chalkak-root button:active image {{
  color: inherit;
}}
tooltip {{
  border-radius: {control_radius}px;
  border: {border_width}px solid {border_color};
  background: {panel_background};
  color: {text_color};
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.16),
              0 1px 3px rgba(0, 0, 0, 0.10);
}}
tooltip label {{
  color: {text_color};
}}
.preview-surface,
.editor-surface {{
  border-radius: {panel_radius}px;
  border: {border_width}px solid {border_color};
  background: {panel_background};
  padding: 0;
  transition: opacity {motion_standard_ms}ms cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08),
              0 12px 40px rgba(0, 0, 0, 0.12);
}}
window.floating-preview-window,
window.floating-editor-window {{
  background: transparent;
}}
.transparent-bg {{
  background: transparent;
}}
/* ── Shared elevated panels ── */
.editor-toolbar,
.editor-action-group,
.preview-action-group {{
  border-radius: {control_radius}px;
  border: {border_width}px solid {border_color};
  background: {panel_background};
  padding: {spacing_4}px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.14),
              0 1px 3px rgba(0, 0, 0, 0.08),
              inset 0 1px 0 rgba(255, 255, 255, 0.06);
}}
.icon-button {{
  border-radius: {control_radius}px;
  min-width: {control_size}px;
  min-height: {control_size}px;
  padding: 0;
  border-color: transparent;
  background: transparent;
  box-shadow: none;
  transition: color {motion_hover_ms}ms cubic-bezier(0.4, 0, 0.2, 1),
              border-color {motion_hover_ms}ms cubic-bezier(0.4, 0, 0.2, 1),
              box-shadow {motion_hover_ms}ms cubic-bezier(0.4, 0, 0.2, 1);
}}
.icon-button:hover {{
  color: {text_color};
  background: transparent;
  border-color: transparent;
  box-shadow: 0 0 0 1.5px {focus_ring_glow},
              0 4px 12px rgba(0, 0, 0, 0.10);
}}
.icon-button:active {{
  color: {text_color};
  background: transparent;
  border-color: transparent;
  box-shadow: 0 0 0 1.5px {focus_ring_glow};
  transition: color 60ms ease,
              border-color 60ms ease,
              box-shadow 60ms ease;
}}
.editor-toolbar button.tool-active {{
  background-image: linear-gradient(
                      rgba(0, 0, 0, 0.24),
                      rgba(0, 0, 0, 0.24)
                    ),
                    {accent_gradient};
  background-origin: border-box;
  color: {accent_text_color};
  border-color: transparent;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.20),
              0 0 0 1px rgba(255, 255, 255, 0.12);
}}
.editor-toolbar button.tool-active image,
.editor-toolbar button.tool-active:hover image,
.editor-toolbar button.tool-active:active image {{
  color: {accent_text_color};
}}
.editor-toolbar button.tool-active:hover,
.editor-toolbar button.tool-active:active {{
  color: {accent_text_color};
}}

/* ── Close button base + hover ── */
button.editor-close-button {{
  border-radius: 999px;
  padding: 0;
  background: rgba(255, 80, 80, 0.10);
  color: rgba(255, 80, 80, 0.85);
}}
button.editor-close-button:hover {{
  background: rgba(255, 80, 80, 0.22);
  border-color: rgba(255, 80, 80, 0.4);
  color: rgba(255, 60, 60, 1.0);
  box-shadow: 0 0 0 1.5px rgba(255, 80, 80, 0.15);
}}
button.preview-close-button {{
  border-radius: 999px;
  padding: 0;
  background: rgba(255, 80, 80, 0.24);
  border-color: rgba(255, 80, 80, 0.46);
  color: rgba(255, 64, 64, 1.0);
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.18);
}}
button.preview-close-button:hover,
button.preview-close-button:active {{
  background: rgba(255, 80, 80, 0.36);
  border-color: rgba(255, 80, 80, 0.58);
  color: rgba(255, 45, 45, 1.0);
  box-shadow: 0 0 0 2px rgba(255, 80, 80, 0.18);
}}
button.editor-close-button image,
button.preview-close-button image {{
  -gtk-icon-size: {close_icon_size}px;
}}

/* ── Editor bottom controls ── */
.editor-bottom-controls {{
  padding: 0;
}}
.editor-options-bar,
.preview-bottom-controls {{
  border-radius: {control_radius}px;
  border: {border_width}px solid {border_color};
  background: {panel_background};
  padding: {spacing_8}px {spacing_12}px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.14),
              0 1px 3px rgba(0, 0, 0, 0.08),
              inset 0 1px 0 rgba(255, 255, 255, 0.06);
}}
.editor-options-header {{
  min-height: {control_size}px;
}}
.editor-options-collapsed .editor-options-header {{
  min-height: 30px;
}}
.editor-options-collapsed-row {{
  min-height: 30px;
}}
button.editor-options-toggle {{
  min-width: 30px;
  min-height: 30px;
  border-radius: 999px;
  padding: 0;
  border-color: transparent;
  background: transparent;
  box-shadow: none;
}}
button.editor-options-toggle image {{
  -gtk-icon-size: 16px;
}}
button.editor-options-toggle:hover {{
  border-color: {focus_ring_color};
  box-shadow: 0 0 0 1px {focus_ring_glow};
}}
.editor-options-collapsed {{
  padding: {spacing_8}px {spacing_8}px;
}}
.stroke-options-title {{
  font-size: 11px;
  opacity: 0.86;
  margin-left: 2px;
}}
button.stroke-chip-button {{
  border-radius: 999px;
  min-width: 30px;
  min-height: 30px;
  padding: 0;
  border: {border_width}px solid {border_color};
  background: rgba(0, 0, 0, 0.08);
  box-shadow: none;
  transition: border-color {motion_hover_ms}ms cubic-bezier(0.4, 0, 0.2, 1),
              box-shadow {motion_hover_ms}ms cubic-bezier(0.4, 0, 0.2, 1);
}}
button.stroke-chip-button:hover {{
  border-color: {focus_ring_color};
  box-shadow: 0 0 0 1px {focus_ring_glow};
}}
button.stroke-chip-active {{
  border-color: {focus_ring_color};
  box-shadow: 0 0 0 1.5px {focus_ring_glow},
              0 3px 10px rgba(0, 0, 0, 0.14);
}}
/* ── Shared slider style ── */
.accent-slider trough {{
  min-height: 4px;
  border-radius: 999px;
  background: rgba(127, 127, 127, 0.22);
}}
.accent-slider highlight {{
  border-radius: 999px;
  background-image: {accent_gradient};
  background-origin: border-box;
  box-shadow: 0 0 6px rgba(124, 92, 255, 0.25);
}}
.accent-slider slider {{
  min-width: 16px;
  min-height: 16px;
  border-radius: 999px;
  border: 2px solid rgba(255, 255, 255, 0.9);
  background-image: {accent_gradient};
  background-origin: border-box;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.18);
  transition: box-shadow {motion_hover_ms}ms cubic-bezier(0.4, 0, 0.2, 1);
}}
.accent-slider slider:hover {{
  box-shadow: 0 0 0 4px {focus_ring_glow},
              0 2px 6px rgba(0, 0, 0, 0.20);
}}
.editor-zoom-slider {{
  min-width: 160px;
}}
.editor-canvas {{
  border-radius: {panel_radius}px;
  border: {border_width}px solid {border_color};
  background: {canvas_background};
}}

/* ── Preview controls revealer ── */
.preview-controls-revealer {{
  transition: opacity {motion_hover_ms}ms cubic-bezier(0.4, 0, 0.2, 1);
}}
.preview-top-controls {{
  padding: 0;
}}

/* ── Icon buttons (shared base) ── */
button.preview-round-button {{
  border-radius: 999px;
  padding: 0;
}}

/* ── Pin toggle: neutral by default, emphasized when pinned ── */
button.preview-pin-toggle {{
  border-radius: 999px;
  padding: 0;
  border: {border_width}px solid {border_color};
  background: {panel_background};
  color: {text_color};
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.14),
              inset 0 1px 0 rgba(255, 255, 255, 0.06);
}}
button.preview-pin-toggle image {{
  -gtk-icon-size: {pin_icon_size}px;
}}
button.preview-pin-toggle:hover,
button.preview-pin-toggle:active {{
  border-color: {focus_ring_color};
  background: {panel_background};
  color: {text_color};
  box-shadow: 0 0 0 1.5px {focus_ring_glow},
              0 4px 12px rgba(0, 0, 0, 0.14);
}}
button.preview-pin-toggle:checked {{
  background-image: linear-gradient(
                      rgba(0, 0, 0, 0.24),
                      rgba(0, 0, 0, 0.24)
                    ),
                    {accent_gradient};
  background-origin: border-box;
  border-color: transparent;
  color: {accent_text_color};
  box-shadow: 0 0 0 1.5px {focus_ring_glow},
              0 6px 14px rgba(0, 0, 0, 0.22),
              inset 0 0 0 1px rgba(255, 255, 255, 0.24);
}}
button.preview-pin-toggle:checked:hover,
button.preview-pin-toggle:checked:active {{
  color: {accent_text_color};
  box-shadow: 0 0 0 1.5px {focus_ring_glow},
              0 8px 18px rgba(0, 0, 0, 0.24),
              inset 0 0 0 1px rgba(255, 255, 255, 0.30);
}}

/* ── Opacity slider ── */
.preview-opacity-slider {{
  min-width: 180px;
}}

/* ── Launchpad layout ── */
.launchpad-root {{
  border-radius: {card_radius}px;
  border: {border_width}px solid {border_color};
  background: {panel_background};
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.12),
              0 2px 6px rgba(0, 0, 0, 0.08);
}}
label.launchpad-title {{
  font-size: 18px;
  font-weight: 700;
}}
label.launchpad-subtitle,
label.launchpad-hint {{
  opacity: 0.8;
}}
label.launchpad-section-title {{
  font-size: 13px;
  font-weight: 650;
  opacity: 0.92;
}}
label.launchpad-key-value {{
  font-size: 13px;
}}
label.launchpad-capture-ids {{
  font-size: 12px;
  padding-top: {spacing_4}px;
  border-top: {border_width}px solid {border_color};
}}
frame.launchpad-panel {{
  border-radius: {control_radius}px;
  border: {border_width}px solid {border_color};
  background: rgba(0, 0, 0, 0.04);
  padding: {spacing_12}px;
}}
frame.launchpad-panel > border {{
  border: none;
}}
button.launchpad-primary-button {{
  background-image: linear-gradient(
                      rgba(0, 0, 0, 0.20),
                      rgba(0, 0, 0, 0.20)
                    ),
                    {accent_gradient};
  color: {accent_text_color};
  border-color: transparent;
  font-weight: 600;
}}
button.launchpad-primary-button:hover,
button.launchpad-primary-button:active {{
  color: {accent_text_color};
}}
button.launchpad-danger-button {{
  background: rgba(255, 80, 80, 0.08);
  color: rgba(255, 80, 80, 0.92);
  border-color: rgba(255, 80, 80, 0.22);
}}
button.launchpad-danger-button:hover,
button.launchpad-danger-button:active {{
  color: rgba(255, 65, 65, 1.0);
  border-color: rgba(255, 80, 80, 0.34);
}}

/* ── Toast badge ── */
.toast-badge {{
  border-radius: {control_radius}px;
  border: {border_width}px solid {border_color};
  background: {panel_background};
  color: {text_color};
  padding: {spacing_8}px {spacing_16}px;
  font-size: 13px;
  font-weight: 500;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.14),
              0 1px 3px rgba(0, 0, 0, 0.08);
}}

/* ── Focus visible ── */
.chalkak-root button:focus-visible,
.chalkak-root scale:focus-visible {{
  border-color: {focus_ring_color};
  box-shadow: 0 0 0 2px {focus_ring_glow};
}}
",
        card_radius = tokens.card_radius,
        panel_radius = tokens.panel_radius,
        control_radius = tokens.control_radius,
        border_width = tokens.border_width,
        border_color = colors.border_color,
        panel_background = colors.panel_background,
        accent_gradient = colors.accent_gradient,
        accent_text_color = colors.accent_text_color,
        canvas_background = colors.canvas_background,
        text_color = colors.text_color,
        focus_ring_color = colors.focus_ring_color,
        focus_ring_glow = colors.focus_ring_glow,
        spacing_8 = tokens.spacing_8,
        spacing_4 = tokens.spacing_4,
        spacing_12 = tokens.spacing_12,
        spacing_16 = tokens.spacing_16,
        control_size = tokens.control_size,
        close_icon_size = close_icon_size,
        pin_icon_size = pin_icon_size,
        motion_standard_ms = motion_standard_ms,
        motion_hover_ms = motion_hover_ms,
    );

    let provider = CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

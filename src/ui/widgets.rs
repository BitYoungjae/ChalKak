use std::sync::Once;

use gtk4::prelude::*;
use gtk4::{gio, Button, ToggleButton};

const LUCIDE_ICON_RESOURCE_PATH: &str = "/com/github/bityoungjae/chalkak/icons/hicolor";

pub fn icon_button(
    icon_name: &str,
    tooltip: &str,
    control_size: i32,
    extra_classes: &[&str],
) -> Button {
    let button = Button::from_icon_name(icon_name);
    button.set_focus_on_click(false);
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("flat");
    button.add_css_class("icon-button");
    for css_class in extra_classes {
        button.add_css_class(css_class);
    }
    button.set_size_request(control_size, control_size);
    button
}

pub fn install_lucide_icon_theme() {
    static ICON_THEME_SETUP: Once = Once::new();

    ICON_THEME_SETUP.call_once(|| {
        if let Err(err) = gio::resources_register_include!("chalkak.gresource") {
            tracing::error!(?err, "failed to register bundled Lucide icon resources");
            return;
        }

        let Some(display) = gtk4::gdk::Display::default() else {
            tracing::warn!("failed to initialize Lucide icon theme; no display available");
            return;
        };

        let icon_theme = gtk4::IconTheme::for_display(&display);
        icon_theme.add_resource_path(LUCIDE_ICON_RESOURCE_PATH);
        tracing::debug!(
            pin = icon_theme.has_icon("pin-symbolic"),
            copy = icon_theme.has_icon("copy-symbolic"),
            save = icon_theme.has_icon("save-symbolic"),
            "registered bundled Lucide icon resource path"
        );
    });
}

pub fn icon_toggle_button(
    icon_name: &str,
    tooltip: &str,
    control_size: i32,
    extra_classes: &[&str],
) -> ToggleButton {
    let button = ToggleButton::new();
    button.set_icon_name(icon_name);
    button.set_focus_on_click(false);
    button.set_active(false);
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("flat");
    button.add_css_class("icon-button");
    for css_class in extra_classes {
        button.add_css_class(css_class);
    }
    button.set_size_request(control_size, control_size);
    button
}

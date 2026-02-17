use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::editor::tools::CropPreset;
use crate::editor::{self, ToolKind};

use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, DrawingArea, Label, Orientation, Revealer, RevealerTransitionType,
};

use crate::app::adaptive::{nearest_preset_u8, EditorToolOptionPresets};
use crate::app::{EditorToolSwitchContext, SharedToolOptionsRefresh};
use crate::ui::{icon_button, StyleTokens};

use super::wiring::{connect_tool_button_selection, EDITOR_TOOLBAR_ENTRIES};

const STROKE_PREVIEW_ENDPOINT_MARGIN: f64 = 4.5;
const STROKE_PREVIEW_STROKE_PADDING: i32 = 2;

pub(super) struct ToolOptionsBuildContext {
    pub(super) style_tokens: StyleTokens,
    pub(super) theme_mode: crate::theme::ThemeMode,
    pub(super) motion_hover_ms: u32,
    pub(super) editor_tools: Rc<RefCell<editor::EditorTools>>,
    pub(super) editor_canvas: DrawingArea,
    pub(super) active_editor_tool: Rc<Cell<ToolKind>>,
    pub(super) tool_option_presets: EditorToolOptionPresets,
    pub(super) refresh_tool_options: SharedToolOptionsRefresh,
    pub(super) status_log_for_render: Rc<RefCell<String>>,
}

pub(super) struct ToolOptionsRuntime {
    pub(super) tool_options_bar: GtkBox,
    pub(super) tool_options_toggle: Button,
}

pub(super) fn build_top_toolbar_row(
    style_tokens: StyleTokens,
    editor_tool_switch_context: &EditorToolSwitchContext,
    status_log_for_render: &Rc<RefCell<String>>,
    tool_buttons: &Rc<RefCell<Vec<(ToolKind, Button)>>>,
) -> GtkBox {
    let top_toolbar_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
    top_toolbar_row.add_css_class("editor-toolbar");
    for (tool_kind, icon_name, tooltip) in EDITOR_TOOLBAR_ENTRIES {
        let button = icon_button(
            icon_name,
            tooltip,
            style_tokens.control_size as i32,
            &["editor-tool-button"],
        );
        connect_tool_button_selection(
            &button,
            tool_kind,
            editor_tool_switch_context,
            status_log_for_render,
        );
        if tool_kind == ToolKind::Select {
            button.add_css_class("tool-active");
        }
        tool_buttons.borrow_mut().push((tool_kind, button.clone()));
        top_toolbar_row.append(&button);
    }
    top_toolbar_row
}

pub(super) fn build_tool_options_runtime(context: ToolOptionsBuildContext) -> ToolOptionsRuntime {
    let ToolOptionsBuildContext {
        style_tokens,
        theme_mode,
        motion_hover_ms,
        editor_tools,
        editor_canvas,
        active_editor_tool,
        tool_option_presets,
        refresh_tool_options,
        status_log_for_render,
    } = context;

    let tool_options_bar = GtkBox::new(Orientation::Vertical, style_tokens.spacing_8);
    tool_options_bar.add_css_class("editor-options-bar");
    tool_options_bar.add_css_class("stroke-options-panel");
    tool_options_bar.set_hexpand(false);
    tool_options_bar.set_halign(Align::Start);
    let tool_options_collapsed = Rc::new(Cell::new(false));
    let thickness_preview_rgb = match theme_mode {
        crate::theme::ThemeMode::Light => (24_u8, 26_u8, 32_u8),
        crate::theme::ThemeMode::Dark | crate::theme::ThemeMode::System => (236_u8, 238_u8, 244_u8),
    };
    let stroke_color_palette = tool_option_presets.stroke_color_palette().clone();
    let stroke_width_presets = tool_option_presets.stroke_width_presets().to_vec();
    let text_size_presets = tool_option_presets.text_size_presets().to_vec();

    let tool_options_header = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
    tool_options_header.add_css_class("editor-options-header");
    tool_options_header.set_valign(Align::Center);
    let tool_options_collapsed_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_8);
    tool_options_collapsed_row.add_css_class("stroke-chip-row");
    tool_options_collapsed_row.add_css_class("editor-options-collapsed-row");
    tool_options_collapsed_row.set_hexpand(true);
    tool_options_collapsed_row.set_valign(Align::Center);
    tool_options_collapsed_row.set_vexpand(false);
    tool_options_collapsed_row.set_visible(false);

    let collapsed_color_rgb = Rc::new(Cell::new(editor::tools::Color::new(0, 0, 0)));
    let collapsed_thickness = Rc::new(Cell::new(1_u8));
    {
        let tools = editor_tools.borrow();
        let arrow_options = tools.arrow_options();
        collapsed_color_rgb.set(arrow_options.color);
        collapsed_thickness.set(nearest_preset_u8(
            f64::from(arrow_options.thickness),
            &stroke_width_presets,
        ));
    }

    let collapsed_color_chip = Button::new();
    collapsed_color_chip.set_focus_on_click(false);
    collapsed_color_chip.set_can_target(false);
    collapsed_color_chip.set_valign(Align::Center);
    collapsed_color_chip.add_css_class("flat");
    collapsed_color_chip.add_css_class("stroke-chip-button");
    collapsed_color_chip.add_css_class("stroke-chip-active");
    collapsed_color_chip.set_size_request(30, 30);
    let collapsed_color_swatch = DrawingArea::new();
    collapsed_color_swatch.set_content_width(18);
    collapsed_color_swatch.set_content_height(18);
    collapsed_color_swatch.set_can_target(false);
    {
        let collapsed_color_rgb = collapsed_color_rgb.clone();
        collapsed_color_swatch.set_draw_func(move |_, context, width, height| {
            let (r, g, b) = collapsed_color_rgb.get().rgb();
            let radius = (f64::from(width.min(height)) / 2.0) - 1.2;
            let center_x = f64::from(width) / 2.0;
            let center_y = f64::from(height) / 2.0;
            context.save().ok();
            context.arc(
                center_x,
                center_y,
                radius.max(1.0),
                0.0,
                std::f64::consts::TAU,
            );
            context.set_source_rgb(
                f64::from(r) / 255.0,
                f64::from(g) / 255.0,
                f64::from(b) / 255.0,
            );
            let _ = context.fill_preserve();
            context.set_source_rgba(1.0, 1.0, 1.0, 0.35);
            context.set_line_width(1.0);
            let _ = context.stroke();
            context.restore().ok();
        });
    }
    collapsed_color_chip.set_child(Some(&collapsed_color_swatch));
    tool_options_collapsed_row.append(&collapsed_color_chip);

    let collapsed_thickness_chip = Button::new();
    collapsed_thickness_chip.set_focus_on_click(false);
    collapsed_thickness_chip.set_can_target(false);
    collapsed_thickness_chip.set_valign(Align::Center);
    collapsed_thickness_chip.add_css_class("flat");
    collapsed_thickness_chip.add_css_class("stroke-chip-button");
    collapsed_thickness_chip.add_css_class("stroke-chip-active");
    collapsed_thickness_chip.set_size_request(30, 30);
    let collapsed_thickness_preview = DrawingArea::new();
    collapsed_thickness_preview.set_content_width(18);
    collapsed_thickness_preview.set_content_height(18);
    collapsed_thickness_preview.set_can_target(false);
    {
        let collapsed_thickness = collapsed_thickness.clone();
        let (line_r, line_g, line_b) = thickness_preview_rgb;
        collapsed_thickness_preview.set_draw_func(move |_, context, width, height| {
            let y = f64::from(height) / 2.0;
            context.save().ok();
            context.set_source_rgb(
                f64::from(line_r) / 255.0,
                f64::from(line_g) / 255.0,
                f64::from(line_b) / 255.0,
            );
            context.set_line_width(stroke_preview_line_width(
                collapsed_thickness.get(),
                width,
                height,
            ));
            context.set_line_cap(gtk4::cairo::LineCap::Round);
            context.move_to(STROKE_PREVIEW_ENDPOINT_MARGIN, y);
            context.line_to(f64::from(width) - STROKE_PREVIEW_ENDPOINT_MARGIN, y);
            let _ = context.stroke();
            context.restore().ok();
        });
    }
    collapsed_thickness_chip.set_child(Some(&collapsed_thickness_preview));
    tool_options_collapsed_row.append(&collapsed_thickness_chip);

    let collapsed_text_size = {
        let text_size = editor_tools.borrow().text_options().size;
        nearest_preset_u8(f64::from(text_size), &text_size_presets)
    };
    let collapsed_text_size_chip = Button::with_label(collapsed_text_size.to_string().as_str());
    collapsed_text_size_chip.set_focus_on_click(false);
    collapsed_text_size_chip.set_can_target(false);
    collapsed_text_size_chip.set_valign(Align::Center);
    collapsed_text_size_chip.add_css_class("flat");
    collapsed_text_size_chip.add_css_class("stroke-chip-button");
    collapsed_text_size_chip.add_css_class("stroke-chip-active");
    collapsed_text_size_chip.set_size_request(34, 30);
    tool_options_collapsed_row.append(&collapsed_text_size_chip);

    let initial_crop_preset = editor_tools.borrow().crop_options().preset;
    let collapsed_crop_preset_chip = Button::with_label(initial_crop_preset.label());
    collapsed_crop_preset_chip.set_focus_on_click(false);
    collapsed_crop_preset_chip.set_can_target(false);
    collapsed_crop_preset_chip.set_valign(Align::Center);
    collapsed_crop_preset_chip.add_css_class("flat");
    collapsed_crop_preset_chip.add_css_class("stroke-chip-button");
    collapsed_crop_preset_chip.add_css_class("stroke-chip-active");
    collapsed_crop_preset_chip.set_size_request(-1, 30);
    collapsed_crop_preset_chip.set_visible(false);
    tool_options_collapsed_row.append(&collapsed_crop_preset_chip);

    let tool_options_toggle = Button::from_icon_name("chevron-down-symbolic");
    tool_options_toggle.set_focus_on_click(false);
    tool_options_toggle.set_size_request(30, 30);
    tool_options_toggle.set_hexpand(false);
    tool_options_toggle.set_vexpand(false);
    tool_options_toggle.set_halign(Align::Center);
    tool_options_toggle.set_valign(Align::Center);
    tool_options_toggle.set_tooltip_text(Some("Collapse tool options (O)"));
    tool_options_toggle.add_css_class("flat");
    tool_options_toggle.add_css_class("editor-options-toggle");
    tool_options_header.append(&tool_options_collapsed_row);
    tool_options_header.append(&tool_options_toggle);
    tool_options_bar.append(&tool_options_header);

    let tool_options_content = GtkBox::new(Orientation::Vertical, style_tokens.spacing_8);
    let tool_options_content_revealer = Revealer::new();
    tool_options_content_revealer.set_transition_duration(motion_hover_ms);
    tool_options_content_revealer.set_transition_type(RevealerTransitionType::SlideDown);
    tool_options_content_revealer.set_reveal_child(true);
    tool_options_content_revealer.set_visible(true);
    tool_options_content_revealer.set_child(Some(&tool_options_content));
    tool_options_bar.append(&tool_options_content_revealer);

    let refresh_collapsed_option_chips = Rc::new({
        let editor_tools = editor_tools.clone();
        let collapsed_color_rgb = collapsed_color_rgb.clone();
        let collapsed_color_swatch = collapsed_color_swatch.clone();
        let collapsed_thickness = collapsed_thickness.clone();
        let collapsed_thickness_preview = collapsed_thickness_preview.clone();
        let collapsed_text_size_chip = collapsed_text_size_chip.clone();
        let collapsed_crop_preset_chip = collapsed_crop_preset_chip.clone();
        let stroke_width_presets = stroke_width_presets.clone();
        let text_size_presets = text_size_presets.clone();
        move || {
            let Ok(tools) = editor_tools.try_borrow() else {
                return;
            };
            let arrow_options = tools.arrow_options();
            collapsed_color_rgb.set(arrow_options.color);
            collapsed_color_swatch.queue_draw();
            collapsed_thickness.set(nearest_preset_u8(
                f64::from(arrow_options.thickness),
                &stroke_width_presets,
            ));
            collapsed_thickness_preview.queue_draw();
            let text_size =
                nearest_preset_u8(f64::from(tools.text_options().size), &text_size_presets);
            collapsed_text_size_chip.set_label(text_size.to_string().as_str());
            let crop_label = tools.crop_options().preset.label();
            collapsed_crop_preset_chip.set_label(crop_label);
        }
    });
    (refresh_collapsed_option_chips.as_ref())();

    let color_group = GtkBox::new(Orientation::Vertical, 2);
    color_group.add_css_class("stroke-options-section");
    let color_title = Label::new(Some("Color Palette"));
    color_title.add_css_class("stroke-options-title");
    color_title.set_xalign(0.0);
    let color_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
    color_row.add_css_class("stroke-chip-row");
    let color_chip_buttons = Rc::new(RefCell::new(Vec::<(usize, Button)>::new()));
    let initial_color_index = {
        let options = editor_tools.borrow().arrow_options();
        stroke_color_palette
            .presets()
            .iter()
            .position(|preset| preset.rgb() == options.color.rgb())
    };
    for (index, preset) in stroke_color_palette.presets().iter().cloned().enumerate() {
        let chip = Button::new();
        chip.set_focus_on_click(false);
        chip.set_tooltip_text(Some(preset.label.as_str()));
        chip.add_css_class("flat");
        chip.add_css_class("stroke-chip-button");
        chip.set_size_request(30, 30);
        let swatch = DrawingArea::new();
        swatch.set_content_width(18);
        swatch.set_content_height(18);
        swatch.set_can_target(false);
        let (preset_r, preset_g, preset_b) = preset.rgb();
        swatch.set_draw_func(move |_, context, width, height| {
            let radius = (f64::from(width.min(height)) / 2.0) - 1.2;
            let center_x = f64::from(width) / 2.0;
            let center_y = f64::from(height) / 2.0;
            context.save().ok();
            context.arc(
                center_x,
                center_y,
                radius.max(1.0),
                0.0,
                std::f64::consts::TAU,
            );
            context.set_source_rgb(
                f64::from(preset_r) / 255.0,
                f64::from(preset_g) / 255.0,
                f64::from(preset_b) / 255.0,
            );
            let _ = context.fill_preserve();
            context.set_source_rgba(1.0, 1.0, 1.0, 0.35);
            context.set_line_width(1.0);
            let _ = context.stroke();
            context.restore().ok();
        });
        chip.set_child(Some(&swatch));
        color_row.append(&chip);
        color_chip_buttons.borrow_mut().push((index, chip.clone()));
        let color_chip_buttons = color_chip_buttons.clone();
        let editor_tools = editor_tools.clone();
        let editor_canvas = editor_canvas.clone();
        let status_log_for_render = status_log_for_render.clone();
        let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
        chip.connect_clicked(move |_| {
            for (candidate_index, candidate_button) in color_chip_buttons.borrow().iter() {
                if *candidate_index == index {
                    candidate_button.add_css_class("stroke-chip-active");
                } else {
                    candidate_button.remove_css_class("stroke-chip-active");
                }
            }
            {
                let mut tools = editor_tools.borrow_mut();
                tools.set_shared_stroke_color(editor::tools::Color::new(
                    preset_r, preset_g, preset_b,
                ));
            }
            *status_log_for_render.borrow_mut() =
                format!("stroke color preset: {preset_r},{preset_g},{preset_b}");
            editor_canvas.queue_draw();
            (refresh_collapsed_option_chips.as_ref())();
        });
    }
    for (index, chip) in color_chip_buttons.borrow().iter() {
        if Some(*index) == initial_color_index {
            chip.add_css_class("stroke-chip-active");
        }
    }
    color_group.append(&color_title);
    color_group.append(&color_row);
    tool_options_content.append(&color_group);

    let thickness_group = GtkBox::new(Orientation::Vertical, 2);
    thickness_group.add_css_class("stroke-options-section");
    thickness_group.set_margin_top(style_tokens.spacing_8);
    let thickness_title = Label::new(Some("Stroke Width"));
    thickness_title.add_css_class("stroke-options-title");
    thickness_title.set_xalign(0.0);
    let thickness_row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
    thickness_row.add_css_class("stroke-chip-row");
    let thickness_chip_buttons = Rc::new(RefCell::new(Vec::<(u8, Button)>::new()));
    let initial_thickness = {
        let options = editor_tools.borrow().arrow_options();
        nearest_preset_u8(f64::from(options.thickness), &stroke_width_presets)
    };
    for thickness in stroke_width_presets.iter().copied() {
        let chip = Button::new();
        chip.set_focus_on_click(false);
        chip.set_tooltip_text(Some(format!("Thickness {thickness}").as_str()));
        chip.add_css_class("flat");
        chip.add_css_class("stroke-chip-button");
        chip.set_size_request(30, 30);
        let preview = DrawingArea::new();
        preview.set_content_width(18);
        preview.set_content_height(18);
        preview.set_can_target(false);
        let (line_r, line_g, line_b) = thickness_preview_rgb;
        preview.set_draw_func(move |_, context, width, height| {
            let y = f64::from(height) / 2.0;
            context.save().ok();
            context.set_source_rgb(
                f64::from(line_r) / 255.0,
                f64::from(line_g) / 255.0,
                f64::from(line_b) / 255.0,
            );
            context.set_line_width(stroke_preview_line_width(thickness, width, height));
            context.set_line_cap(gtk4::cairo::LineCap::Round);
            context.move_to(STROKE_PREVIEW_ENDPOINT_MARGIN, y);
            context.line_to(f64::from(width) - STROKE_PREVIEW_ENDPOINT_MARGIN, y);
            let _ = context.stroke();
            context.restore().ok();
        });
        chip.set_child(Some(&preview));
        thickness_row.append(&chip);
        thickness_chip_buttons
            .borrow_mut()
            .push((thickness, chip.clone()));
        let thickness_chip_buttons = thickness_chip_buttons.clone();
        let editor_tools = editor_tools.clone();
        let editor_canvas = editor_canvas.clone();
        let status_log_for_render = status_log_for_render.clone();
        let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
        chip.connect_clicked(move |_| {
            for (candidate_thickness, candidate_button) in thickness_chip_buttons.borrow().iter() {
                if *candidate_thickness == thickness {
                    candidate_button.add_css_class("stroke-chip-active");
                } else {
                    candidate_button.remove_css_class("stroke-chip-active");
                }
            }
            {
                let mut tools = editor_tools.borrow_mut();
                tools.set_shared_stroke_thickness(thickness);
            }
            *status_log_for_render.borrow_mut() = format!("stroke thickness preset: {thickness}");
            editor_canvas.queue_draw();
            (refresh_collapsed_option_chips.as_ref())();
        });
    }
    for (thickness, chip) in thickness_chip_buttons.borrow().iter() {
        if *thickness == initial_thickness {
            chip.add_css_class("stroke-chip-active");
        }
    }
    thickness_group.append(&thickness_title);
    thickness_group.append(&thickness_row);
    tool_options_content.append(&thickness_group);

    let initial_text_size = {
        let options = editor_tools.borrow().text_options();
        nearest_preset_u8(f64::from(options.size), &text_size_presets)
    };
    let text_size_group = build_label_chip_group(
        style_tokens,
        "Text Size",
        &text_size_presets,
        initial_text_size,
        34,
        0,
        |size: u8| size.to_string(),
        {
            let editor_tools = editor_tools.clone();
            let editor_canvas = editor_canvas.clone();
            let status_log_for_render = status_log_for_render.clone();
            let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
            Rc::new(move |text_size: u8| {
                editor_tools.borrow_mut().set_text_size(text_size);
                *status_log_for_render.borrow_mut() = format!("text size preset: {text_size}");
                editor_canvas.queue_draw();
                (refresh_collapsed_option_chips.as_ref())();
            })
        },
    );
    tool_options_content.append(&text_size_group);

    let initial_crop_preset_active = editor_tools.borrow().crop_options().preset;
    let crop_preset_group = build_label_chip_group(
        style_tokens,
        "Aspect Ratio",
        &CropPreset::ALL,
        initial_crop_preset_active,
        -1,
        4,
        |preset: CropPreset| preset.label().to_string(),
        {
            let editor_tools = editor_tools.clone();
            let editor_canvas = editor_canvas.clone();
            let status_log_for_render = status_log_for_render.clone();
            let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
            Rc::new(move |preset: CropPreset| {
                editor_tools.borrow_mut().set_crop_preset(preset);
                *status_log_for_render.borrow_mut() = format!("crop preset: {}", preset.label());
                editor_canvas.queue_draw();
                (refresh_collapsed_option_chips.as_ref())();
            })
        },
    );
    tool_options_content.append(&crop_preset_group);

    let refresh_tool_options_with_bottom = Rc::new({
        let tool_options_bar = tool_options_bar.clone();
        let tool_options_collapsed = tool_options_collapsed.clone();
        let tool_options_collapsed_row = tool_options_collapsed_row.clone();
        let tool_options_content_revealer = tool_options_content_revealer.clone();
        let tool_options_toggle = tool_options_toggle.clone();
        let color_group = color_group.clone();
        let thickness_group = thickness_group.clone();
        let text_size_group = text_size_group.clone();
        let crop_preset_group = crop_preset_group.clone();
        let collapsed_color_chip = collapsed_color_chip.clone();
        let collapsed_thickness_chip = collapsed_thickness_chip.clone();
        let collapsed_text_size_chip = collapsed_text_size_chip.clone();
        let collapsed_crop_preset_chip = collapsed_crop_preset_chip.clone();
        let refresh_collapsed_option_chips = refresh_collapsed_option_chips.clone();
        move |tool: ToolKind| {
            let vis = tool.option_visibility();

            tool_options_bar.set_visible(vis.has_any());
            if !vis.has_any() {
                return;
            }

            // Expanded section group visibility
            color_group.set_visible(vis.has_color);
            thickness_group.set_visible(vis.has_stroke_width);
            text_size_group.set_visible(vis.has_text_size);
            crop_preset_group.set_visible(vis.has_crop_preset);

            // Collapsed chip visibility
            collapsed_color_chip.set_visible(vis.has_color);
            collapsed_thickness_chip.set_visible(vis.has_stroke_width);
            collapsed_text_size_chip.set_visible(vis.has_text_size);
            collapsed_crop_preset_chip.set_visible(vis.has_crop_preset);

            // Restore collapsed/expanded state
            let collapsed = tool_options_collapsed.get();
            tool_options_content_revealer.set_visible(!collapsed);
            tool_options_content_revealer.set_reveal_child(!collapsed);
            tool_options_collapsed_row.set_visible(collapsed);
            if collapsed {
                tool_options_bar.add_css_class("editor-options-collapsed");
                tool_options_toggle.set_icon_name("chevron-up-symbolic");
                tool_options_toggle.set_tooltip_text(Some("Expand tool options (O)"));
            } else {
                tool_options_bar.remove_css_class("editor-options-collapsed");
                tool_options_toggle.set_icon_name("chevron-down-symbolic");
                tool_options_toggle.set_tooltip_text(Some("Collapse tool options (O)"));
            }

            (refresh_collapsed_option_chips.as_ref())();
        }
    });
    refresh_tool_options_with_bottom(active_editor_tool.get());
    let refresh_tool_options_dyn: Rc<dyn Fn(ToolKind)> = refresh_tool_options_with_bottom.clone();
    *refresh_tool_options.borrow_mut() = Some(refresh_tool_options_dyn);

    {
        let tool_options_collapsed = tool_options_collapsed.clone();
        let active_editor_tool = active_editor_tool.clone();
        let refresh = refresh_tool_options_with_bottom;
        let status_log_for_render = status_log_for_render.clone();
        tool_options_toggle.connect_clicked(move |_| {
            let collapsed = !tool_options_collapsed.get();
            tool_options_collapsed.set(collapsed);
            if collapsed {
                *status_log_for_render.borrow_mut() = "editor tool options collapsed".to_string();
            } else {
                *status_log_for_render.borrow_mut() = "editor tool options expanded".to_string();
            }
            refresh(active_editor_tool.get());
        });
    }

    ToolOptionsRuntime {
        tool_options_bar,
        tool_options_toggle,
    }
}

fn build_label_chip_group<K: Copy + PartialEq + 'static>(
    style_tokens: StyleTokens,
    title: &str,
    presets: &[K],
    initial_active: K,
    chip_width: i32,
    chip_label_padding: i32,
    label_fn: impl Fn(K) -> String,
    on_select: Rc<dyn Fn(K)>,
) -> GtkBox {
    let group = GtkBox::new(Orientation::Vertical, 2);
    group.add_css_class("stroke-options-section");
    group.set_margin_top(style_tokens.spacing_8);
    let title_label = Label::new(Some(title));
    title_label.add_css_class("stroke-options-title");
    title_label.set_xalign(0.0);
    let row = GtkBox::new(Orientation::Horizontal, style_tokens.spacing_4);
    row.add_css_class("stroke-chip-row");
    let chip_buttons = Rc::new(RefCell::new(Vec::<(K, Button)>::new()));
    for &preset in presets {
        let label = label_fn(preset);
        let chip = Button::with_label(&label);
        chip.set_focus_on_click(false);
        chip.set_tooltip_text(Some(&label));
        chip.add_css_class("flat");
        chip.add_css_class("stroke-chip-button");
        chip.set_size_request(chip_width, 30);
        if chip_label_padding > 0 {
            if let Some(child) = chip.child() {
                child.set_margin_start(chip_label_padding);
                child.set_margin_end(chip_label_padding);
            }
        }
        row.append(&chip);
        chip_buttons.borrow_mut().push((preset, chip.clone()));
        let chip_buttons = chip_buttons.clone();
        let on_select = on_select.clone();
        chip.connect_clicked(move |_| {
            for (candidate, candidate_button) in chip_buttons.borrow().iter() {
                if *candidate == preset {
                    candidate_button.add_css_class("stroke-chip-active");
                } else {
                    candidate_button.remove_css_class("stroke-chip-active");
                }
            }
            on_select(preset);
        });
    }
    for (preset, chip) in chip_buttons.borrow().iter() {
        if *preset == initial_active {
            chip.add_css_class("stroke-chip-active");
        }
    }
    group.append(&title_label);
    group.append(&row);
    group
}

fn stroke_preview_line_width(thickness: u8, preview_width: i32, preview_height: i32) -> f64 {
    let requested = f64::from(thickness.max(1));
    let vertical_limit = f64::from(
        preview_height
            .max(1)
            .saturating_sub(STROKE_PREVIEW_STROKE_PADDING),
    );
    let cap_limit = (STROKE_PREVIEW_ENDPOINT_MARGIN * 2.0).max(1.0);
    let width_limit = f64::from(preview_width.max(1));
    requested
        .min(vertical_limit.max(1.0))
        .min(cap_limit)
        .min(width_limit)
        .max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stroke_preview_line_width_clamps_large_thickness_to_chip_bounds() {
        let clamped = stroke_preview_line_width(64, 18, 18);
        assert!((clamped - 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stroke_preview_line_width_keeps_minimum_visible_width() {
        let clamped = stroke_preview_line_width(0, 18, 18);
        assert!((clamped - 1.0).abs() < f64::EPSILON);
    }
}

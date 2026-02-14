# Chalkak User Guide

[한국어 가이드](USER_GUIDE.ko.md)

This guide is for general users who want a reliable screenshot workflow on Wayland + Hyprland.

## Demo Video

<https://github.com/user-attachments/assets/4e3a4de2-10b0-4131-ab49-983f3b0ceb50>

## 1. What Chalkak Is Best For

Chalkak is designed for a preview-first screenshot flow:

1. Capture the screen (full, region, or window).
2. Check the result in Preview.
3. Save, copy, delete, or open Editor.
4. Annotate in Editor, then save/copy.

If you want quick screenshots with optional annotation and strong keyboard control, this is the intended flow.

## 2. Requirements

Chalkak expects a Wayland + Hyprland session.

Required runtime commands:

- `hyprctl`
- `grim`
- `slurp`
- `wl-copy` (from `wl-clipboard`)

Environment assumptions:

- `HOME` must be set.
- `XDG_RUNTIME_DIR` is strongly recommended.

Quick checks:

```bash
hyprctl version
grim -h
slurp -h
wl-copy --help
echo "$HOME"
echo "$XDG_RUNTIME_DIR"
```

## 3. Install and Start

### Build from source

```bash
git clone <repo-url> chalkak
cd chalkak
cargo run -- --launchpad
```

`--` passes flags to Chalkak (not Cargo).

### Startup modes

Use one of these patterns depending on how you work:

- `chalkak --launchpad`: open the launchpad window first.
- `chalkak --full`: capture fullscreen immediately.
- `chalkak --region`: capture selected region immediately.
- `chalkak --window`: capture selected window immediately.

Aliases also work:

- `--capture-full`
- `--capture-region`
- `--capture-window`

If multiple capture flags are given, the last one wins.

## 4. First Screenshot (Recommended Onboarding)

Use this path for your first run:

1. Start with `chalkak --launchpad`.
2. Trigger a capture from launchpad or keybinding.
3. In Preview, verify content and decide next action.
4. Press `e` to open Editor if you need annotation.
5. Save with `Ctrl+S` or copy with `Ctrl+C`.

## 5. Preview Stage

Preview is where you confirm the capture before final output.

Default preview keys:

- `s`: save image to file.
- `c`: copy image to clipboard.
- `e`: open Editor.
- `Delete`: discard capture.
- `Esc`: close preview.

Use Preview as a safety gate to avoid saving wrong shots.

## 6. Editor Basics

Default editor keys:

- `Ctrl+S`: save output image.
- `Ctrl+C`: copy output image.
- `Ctrl+Z`: undo.
- `Ctrl+Shift+Z`: redo.
- `Delete` / `Backspace`: delete selected object.
- `o`: toggle tool options panel.
- `Esc`: return to Select tool, or close editor when already in Select.

Tool shortcuts:

- `v`: select
- `h`: pan
- `b`: blur
- `p`: pen
- `a`: arrow
- `r`: rectangle
- `c`: crop
- `t`: text

Text editing keys:

- `Enter`: newline
- `Ctrl+Enter`: commit text
- `Ctrl+C`: copy selected text
- `Esc`: exit text editing focus

## 7. Tool-by-Tool Usage Tips

### Select (`v`)

- Click an object to select and move/resize it.
- Drag on empty canvas to make a selection box.
- Use `Delete` to remove current selection.

### Pan (`h` or hold Space)

- Hold pan key (`Space` by default) and drag to move viewport.
- Useful when zoomed in for precise annotation.

### Blur (`b`)

- Drag to define blur area.
- Very small/zero-area drags are ignored.
- Blur intensity is currently fixed in UI.

### Pen (`p`)

- Drag to draw freehand strokes.
- Color/opacity/thickness stay sticky for next strokes.

### Arrow (`a`)

- Drag from start to end point.
- Best for directional callouts.
- Thickness and head size are configurable.

### Rectangle (`r`)

- Drag to create a rectangle.
- Can be outline or filled.
- Corner radius can be adjusted.

### Crop (`c`)

- Drag crop frame to define output area.
- Crop is applied on final output render (save/copy), not by destructively trimming the source canvas immediately.
- `Esc` cancels crop and returns to Select.

### Text (`t`)

- Click to create/select text boxes.
- Double-click existing text to edit.
- Style options currently exposed in UI are color and text size.

## 8. Navigation and Zoom

Default editor navigation bindings:

- Pan hold key: `Space`
- Zoom in: `Ctrl++`, `Ctrl+=`, `Ctrl+KP_Add`
- Zoom out: `Ctrl+-`, `Ctrl+_`, `Ctrl+KP_Subtract`
- Actual size: `Ctrl+0`, `Ctrl+KP_0`
- Fit to view: `Shift+1`

## 9. Configuration

Config directory:

- `$XDG_CONFIG_HOME/chalkak/`
- fallback: `$HOME/.config/chalkak/`

Files:

- `theme.json`
- `keybindings.json`

### 9.1 `theme.json`

Minimal example:

```json
{
  "mode": "system"
}
```

Extended example:

```json
{
  "mode": "dark",
  "colors": {
    "dark": {
      "focus_ring_color": "#8cc2ff",
      "border_color": "#2e3a46",
      "panel_background": "#10151b",
      "canvas_background": "#0b0f14",
      "text_color": "#e7edf5",
      "accent_gradient": "linear-gradient(135deg, #6aa3ff, #8ee3ff)",
      "accent_text_color": "#07121f"
    }
  },
  "editor": {
    "rectangle_border_radius": 10,
    "default_tool_color": "#ff6b6b",
    "default_text_size": 18,
    "default_stroke_width": 3
  }
}
```

Notes:

- `mode` values: `system`, `light`, `dark`.
- `colors.light` and `colors.dark` can be partial.
- Missing values are filled from built-in defaults.

### 9.2 `keybindings.json`

Example:

```json
{
  "editor_navigation": {
    "pan_hold_key": "space",
    "zoom_scroll_modifier": "control",
    "zoom_in_shortcuts": ["ctrl+plus", "ctrl+equal", "ctrl+kp_add"],
    "zoom_out_shortcuts": ["ctrl+minus", "ctrl+underscore", "ctrl+kp_subtract"],
    "actual_size_shortcuts": ["ctrl+0", "ctrl+kp_0"],
    "fit_shortcuts": ["shift+1"]
  }
}
```

Notes:

- `zoom_scroll_modifier` values: `none`, `control`, `shift`, `alt`, `super`.
- Do not set shortcut arrays to empty lists.
- Key names are normalized, so common modifier aliases (`ctrl`, `control`, `cmd`, `super`) are accepted.

## 10. Where Files Go

Temporary captures:

- `$XDG_RUNTIME_DIR/` (files like `capture_<id>.png`)
- fallback: `/tmp/chalkak/`

Saved screenshots:

- `$HOME/Pictures/`

Chalkak creates these directories when needed.

## 11. Troubleshooting

### Symptom: capture does not start

Likely causes:

- Missing dependency command (`hyprctl`, `grim`, `slurp`).
- Not running inside Hyprland session.

What to do:

1. Run command checks in Section 2.
2. Ensure `HYPRLAND_INSTANCE_SIGNATURE` exists.
3. Retry with `chalkak --region` and make a valid selection.

### Symptom: copy to clipboard fails

Likely cause:

- `wl-copy` missing or failing.

What to do:

1. Check `wl-copy --help`.
2. Verify `wl-clipboard` package is installed.

### Symptom: save fails

Likely causes:

- `HOME` unset.
- No write permission to `$HOME/Pictures`.

What to do:

1. Check `echo "$HOME"`.
2. Confirm write permission on `~/Pictures`.

### Symptom: temp files pile up

Likely cause:

- `XDG_RUNTIME_DIR` missing (so fallback path `/tmp/chalkak/` is used), or stale temp files in your runtime directory.

What to do:

1. Set `XDG_RUNTIME_DIR` in your login environment.
2. Remove stale `capture_*.png` files from `$XDG_RUNTIME_DIR` (or `/tmp/chalkak` if fallback is active).

## 12. Practical Workflow Presets

### Fast one-shot screenshot

1. Run `chalkak --region`.
2. Select area.
3. Press `c` in Preview.

### Documentation screenshot with annotation

1. Run `chalkak --window`.
2. Open Editor with `e`.
3. Use `r` (rectangle), `a` (arrow), `t` (text).
4. Save with `Ctrl+S`.

### Privacy-safe sharing

1. Run `chalkak --full`.
2. Open Editor.
3. Blur sensitive sections with `b`.
4. Copy with `Ctrl+C`.

## 13. Quick Command Cheat Sheet

```bash
# launch UI first
chalkak --launchpad

# instant capture modes
chalkak --full
chalkak --region
chalkak --window
```

If your goal is everyday screenshot productivity, start with launchpad mode, then keep `--region` and `--window` for speed-focused one-shot workflows.

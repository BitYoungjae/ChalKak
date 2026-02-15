# ChalKak User Guide

[한국어 가이드](USER_GUIDE.ko.md)

This guide is for general users who want a reliable screenshot workflow on Wayland + Hyprland.

## Demo Video

<https://github.com/user-attachments/assets/4e3a4de2-10b0-4131-ab49-983f3b0ceb50>

## Start Here (Most Users)

If you are setting up ChalKak for the first time, do only this:

1. Install and run `which chalkak` to confirm the executable path.
2. Apply the Print-key Hyprland preset in Section 9.3 (copy and paste).
3. Reload and verify with Section 9.5 commands.
4. Start with `chalkak --launchpad` and use Sections 5 and 6 for daily usage.

You can skip advanced customization for now.

- Optional editor navigation overrides: Section 14.2 (`keybindings.json`)
- Optional theme customization: Section 14.1 (`theme.json`)
- Optional non-Print presets: Section 9.4

## 1. What ChalKak Is Best For

ChalKak is designed for a preview-first screenshot flow:

1. Capture the screen (full, region, or window).
2. Check the result in Preview.
3. Save, copy, delete, or open Editor.
4. Annotate in Editor, then save/copy.

If you want quick screenshots with optional annotation and strong keyboard control, this is the intended flow.

A practical use case is agentic coding workflows: capture a specific UI region, annotate it, then paste directly into tools that accept clipboard images (for example Codex CLI, Claude Code, or other coding agents, depending on client support). Many screenshot tools force file-save or manual attachment first.

## 2. Requirements

ChalKak expects a Wayland + Hyprland session.

Runtime commands used by capture/clipboard features:

- `hyprctl`
- `grim`
- `slurp`
- `wl-copy` (from `wl-clipboard`, used by image-byte clipboard paths)

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

`--` passes flags to ChalKak (not Cargo).

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

### Quick setup checklist (recommended)

If you want a low-friction setup, use this path:

1. Confirm runtime tools and ChalKak path:

```bash
hyprctl version
grim -h
slurp -h
wl-copy --help
which chalkak
```

1. Optional: create `keybindings.json` only if you want to customize editor navigation keys.

If you keep defaults, skip this step. (When the file is missing, ChalKak uses built-in defaults.)

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak"
cat > "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak/keybindings.json" <<'JSON'
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
JSON
```

1. Keep Hyprland binds in a dedicated drop-in file (`~/.config/hypr/chalkak.conf`) and ensure your main config sources it once:

```conf
source = ~/.config/hypr/chalkak.conf
```

1. Apply the recommended Print-key preset (no manual key-syntax editing):

```bash
CHALKAK_BIN="$(command -v chalkak)"
mkdir -p "$HOME/.config/hypr"
cat > "$HOME/.config/hypr/chalkak.conf" <<EOF
# ChalKak screenshot bindings (recommended: Print-based)
unbind = , Print
unbind = SHIFT, Print
unbind = CTRL, Print
bindd = , Print, ChalKak region capture, exec, ${CHALKAK_BIN} --capture-region
bindd = SHIFT, Print, ChalKak window capture, exec, ${CHALKAK_BIN} --capture-window
bindd = CTRL, Print, ChalKak full capture, exec, ${CHALKAK_BIN} --capture-full
EOF
```

1. Validate and reload:

```bash
if [ -f "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak/keybindings.json" ]; then
  jq empty "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak/keybindings.json"
fi
hyprctl reload
hyprctl binds -j | jq -r '.[] | select(.description|test("ChalKak")) | [.description,.arg] | @tsv'
```

## 4. First Screenshot (Recommended Onboarding)

Use this path for your first run:

1. Start with `chalkak --launchpad`.
2. Trigger a capture from launchpad or keybinding.
3. In Preview, verify content and decide next action.
4. Press `e` to open Editor if you need annotation.
5. Save from Preview with `s`, or open Editor and then use `Ctrl+S` / `Ctrl+C`.

## 5. Preview Stage

Preview is where you confirm the capture before final output.

Default preview keys:

- `s`: save image to file.
- `c`: copy to clipboard (`image/png` + file path/link; paste result depends on target app). Useful when sending image context to coding agents that support clipboard image paste.
- `u`: alias of `c`.
- `e`: open Editor.
- `Delete`: discard capture.
- `Esc`: close preview.

Use Preview as a safety gate to avoid saving wrong shots.

## 6. Editor Basics

Default editor keys:

- `Ctrl+S`: save output image.
- `Ctrl+C`: copy to clipboard (`image/png` + file path/link; paste result depends on target app). Useful when sending image context to coding agents that support clipboard image paste.
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

- `Enter` / `Shift+Enter`: newline
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

## 9. Wire ChalKak to Hyprland Keybindings (Recommended Setup)

For most users, this is the only setup you need after installation.

For fast capture workflows on Omarchy/Hyprland, bind ChalKak commands directly in Hyprland.

### 9.1 Check the binary path first

```bash
which chalkak
```

- AUR install is usually `/usr/bin/chalkak`
- Older `cargo install` setups may still use `~/.cargo/bin/chalkak`

Your Hyprland binding must point to the currently valid path.

### 9.2 Add one-time `source` line

In your main Hyprland config (often `~/.config/hypr/hyprland.conf`), keep this one line:

```conf
source = ~/.config/hypr/chalkak.conf
```

If you already source a file like `bindings.conf`, you can put the same `source` line there instead.

### 9.3 Quick start: use the recommended preset

If key syntax feels high-friction, start here and paste as-is into `~/.config/hypr/chalkak.conf`:

```conf
# ChalKak screenshot bindings (recommended: Print-based)
unbind = , Print
unbind = SHIFT, Print
unbind = CTRL, Print
bindd = , Print, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
bindd = SHIFT, Print, ChalKak window capture, exec, /usr/bin/chalkak --capture-window
bindd = CTRL, Print, ChalKak full capture, exec, /usr/bin/chalkak --capture-full
```

Notes:

- `unbind` helps avoid conflicts with existing bindings.
- Replace `/usr/bin/chalkak` if your executable path is different.

This keeps all ChalKak keybind edits in one file and avoids repeatedly touching `hyprland.conf`.

### 9.4 Optional: other ready-to-use presets

If you prefer non-Print keys, copy one of these blocks.

Mnemonic letters (`Alt+Shift+R/W/F`):

```conf
unbind = ALT SHIFT, R
unbind = ALT SHIFT, W
unbind = ALT SHIFT, F
bindd = ALT SHIFT, R, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
bindd = ALT SHIFT, W, ChalKak window capture, exec, /usr/bin/chalkak --capture-window
bindd = ALT SHIFT, F, ChalKak full capture, exec, /usr/bin/chalkak --capture-full
```

Number row (`Alt+Shift+2/3/4`):

```conf
unbind = ALT SHIFT, 2
unbind = ALT SHIFT, 3
unbind = ALT SHIFT, 4
bindd = ALT SHIFT, 2, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
bindd = ALT SHIFT, 3, ChalKak window capture, exec, /usr/bin/chalkak --capture-window
bindd = ALT SHIFT, 4, ChalKak full capture, exec, /usr/bin/chalkak --capture-full
```

Minimum setup (region only):

```conf
unbind = , Print
bindd = , Print, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
```

### 9.5 Reload and verify

```bash
hyprctl reload
hyprctl binds -j | jq -r '.[] | select(.description|test("ChalKak")) | [.description,.arg] | @tsv'
```

If you see `ChalKak ... capture` entries with the expected path, bindings are active.

### 9.6 Omarchy-specific note

Omarchy loads multiple files via `source = ...` in `hyprland.conf`. Ensure your `source = ~/.config/hypr/chalkak.conf` line is active.

- If you manage Hypr files via symlinked dotfiles, edit the link target.
- If keybindings stopped working after moving from Cargo to AUR, check for stale `~/.cargo/bin/chalkak` paths.

## 10. Where Files Go

Temporary captures:

- `$XDG_RUNTIME_DIR/` (files like `capture_<id>.png`)
- fallback: `/tmp/chalkak/`

Saved screenshots:

- `$HOME/Pictures/`

ChalKak creates these directories when needed.

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

- For image-byte copy paths: `wl-copy` missing or failing.
- For multi-format copy paths (Preview `c`/`u`, Editor `Ctrl+C`): Wayland/GTK clipboard display unavailable, unreadable temp file, or file-URI conversion failure.

What to do:

1. Check `wl-copy --help`.
2. Verify `wl-clipboard` package is installed.
3. Ensure you are in a live Wayland GUI session and retry.

### Symptom: save fails

Likely causes:

- `HOME` unset.
- No write permission to `$HOME/Pictures`.

What to do:

1. Check `echo "$HOME"`.
2. Confirm write permission on `~/Pictures`.

### Symptom: temp files pile up

Likely cause:

- Session cleanup did not run (for example force-kill/crash), or stale temp files remained from previous runs.
- `XDG_RUNTIME_DIR` missing (so fallback path `/tmp/chalkak/` is used).

What to do:

1. Set `XDG_RUNTIME_DIR` in your login environment.
2. Close previews/editors normally when possible (ChalKak removes per-capture temp files on close/delete and also prunes stale `capture_*.png` files at startup).
3. If stale files still remain, remove `capture_*.png` files from `$XDG_RUNTIME_DIR` (or `/tmp/chalkak` if fallback is active).

## 12. Practical Workflow Presets

### Fast one-shot screenshot

1. Run `chalkak --region`.
2. Select area.
3. Press `c` in Preview to copy to clipboard.

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

## 14. Advanced Configuration (Optional)

Most users can skip this section.

Use this only when you want to customize theme or editor navigation behavior beyond defaults.

Config directory:

- `$XDG_CONFIG_HOME/chalkak/`
- fallback: `$HOME/.config/chalkak/`

Files:

- `theme.json`
- `keybindings.json`

### 14.1 `theme.json`

`theme.json` controls runtime theme mode, app UI colors, and editor defaults.

Minimal example:

```json
{
  "mode": "system"
}
```

Recommended structure (shared `common` + per-mode overrides for both `colors` and `editor`):

```json
{
  "mode": "system",
  "colors": {
    "common": {
      "focus_ring_color": "#8cc2ff",
      "border_color": "#2e3a46",
      "text_color": "#e7edf5"
    },
    "dark": {
      "panel_background": "#10151b",
      "canvas_background": "#0b0f14",
      "accent_gradient": "linear-gradient(135deg, #6aa3ff, #8ee3ff)",
      "accent_text_color": "#07121f"
    },
    "light": {
      "panel_background": "#f7fafc",
      "canvas_background": "#ffffff",
      "accent_gradient": "linear-gradient(135deg, #3b82f6, #67e8f9)",
      "accent_text_color": "#0f172a"
    }
  },
  "editor": {
    "common": {
      "rectangle_border_radius": 10,
      "selection_drag_fill_color": "#2B63FF1F",
      "selection_drag_stroke_color": "#2B63FFE0",
      "selection_outline_color": "#2B63FFE6",
      "selection_handle_color": "#2B63FFF2",
      "default_tool_color": "#ff6b6b",
      "default_text_size": 18,
      "default_stroke_width": 3,
      "tool_color_palette": ["#ff6b6b", "#ffd166", "#3a86ff", "#06d6a0"],
      "stroke_width_presets": [2, 4, 8, 12],
      "text_size_presets": [14, 18, 24, 32]
    },
    "dark": {
      "default_tool_color": "#f4f4f5",
      "selection_drag_fill_color": "#2B63FF33"
    },
    "light": {
      "default_tool_color": "#18181b",
      "selection_drag_fill_color": "#2B63FF14"
    }
  }
}
```

Notes:

- `mode` values: `system`, `light`, `dark`.
- `system` follows your runtime desktop/theme preference. If the platform does not expose that preference, ChalKak falls back to dark mode.
- You can use the same pattern for both sections:
- `colors.common` + `colors.dark/light`
- `editor.common` + `editor.dark/light`
- Each object can be partial. Missing keys are filled from built-in defaults.
- Merge order is `built-in defaults -> common -> current mode`.
- `colors` keys:
- `focus_ring_color`, `focus_ring_glow`, `border_color`, `panel_background`, `canvas_background`, `text_color`, `accent_gradient`, `accent_text_color`
- `editor` keys (all allowed in `common`, `dark`, `light`):
- `rectangle_border_radius`, `selection_drag_fill_color`, `selection_drag_stroke_color`, `selection_outline_color`, `selection_handle_color`, `default_tool_color`, `default_text_size`, `default_stroke_width`, `tool_color_palette`, `stroke_width_presets`, `text_size_presets`
- `default_tool_color` accepts `RRGGBB` or `#RRGGBB`.
- `tool_color_palette` accepts strict `#RRGGBB` items only (`RRGGBB` without `#` is ignored).
- Selection color fields accept strict `#RRGGBB` or `#RRGGBBAA`.
- `stroke_width_presets` range: `1..=64`.
- `text_size_presets` range: `8..=160`.
- Each preset list accepts up to 6 unique items.
- Invalid preset or color values are ignored with warnings in logs.

Legacy compatibility:

- Older schema is still supported:
- shared defaults in flat `editor`
- per-mode overrides in `editor_modes.dark/light`
- If both legacy and new schema are present, precedence is:
- `editor` flat -> `editor.common` -> `editor_modes.<mode>` -> `editor.<mode>`

### 14.2 `keybindings.json`

Only use `keybindings.json` when you want to override editor navigation defaults.

If this file is missing, built-in defaults are used.

A safe starter template:

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
- `pan_hold_key` and shortcut key names are normalized, so aliases like `ctrl`/`control`, `cmd`/`command`/`win` (for `super`) are accepted.
- Each shortcut chord must include exactly one non-modifier key (for example `ctrl+plus`).
- Keep all shortcut arrays non-empty.
- Validate JSON after editing:

```bash
jq empty "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak/keybindings.json"
```

- If `keybindings.json` is invalid, ChalKak logs a warning and falls back to built-in defaults.

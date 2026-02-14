# Chalkak

English | [한국어](README.ko.md)

A Hyprland-focused screenshot utility for Wayland with a preview-first workflow and a lightweight annotation editor.

## Demo Video

- [Watch the demo](https://github.com/user-attachments/assets/4e3a4de2-10b0-4131-ab49-983f3b0ceb50)

## User Guides

- [English User Guide](docs/USER_GUIDE.md)
- [한국어 사용자 가이드](docs/USER_GUIDE.ko.md)

## Name Origin

`Chalkak` is inspired by the Korean onomatopoeia `찰칵!`, the camera shutter click sound.

## Highlights

- Capture modes: fullscreen, region, and window.
- Preview stage before final action (save, copy, edit, delete).
- Built-in editor tools: select, pan, blur, pen, arrow, rectangle, crop, text.
- Keyboard-centric workflow across preview and editor.
- Configurable theme and editor navigation keybindings.
- Startup cleanup for stale temporary captures.

## Requirements

Runtime dependencies:

- `hyprctl` (from Hyprland)
- `grim`
- `slurp`
- `wl-copy` (from `wl-clipboard`)
- GTK4 runtime libraries

Environment assumptions:

- Wayland + Hyprland session
- `HOME` is set
- `XDG_RUNTIME_DIR` is recommended (fallback: `/tmp/chalkak`)

## Install

### AUR (planned)

This project is prepared for an AUR package named `chalkak`.

When published, install with your AUR helper, for example:

```bash
yay -S chalkak
```

### Build from source

```bash
git clone https://github.com/BitYoungjae/ChalKak.git chalkak
cd chalkak
cargo run
```

## Usage

Basic launch:

```bash
chalkak
```

Startup flags:

- `--full` or `--capture-full`
- `--region` or `--capture-region`
- `--window` or `--capture-window`
- `--launchpad`

Typical flow:

1. Capture (`full`, `region`, `window`).
2. Preview the capture.
3. Save/copy/delete, or open editor.
4. Annotate in editor, then save/copy.

## Keybindings

Preview:

- `s`: save
- `c`: copy
- `e`: open editor
- `Delete`: delete capture
- `Esc`: close preview

Editor:

- `Ctrl+S`: save
- `Ctrl+C`: copy image
- `Ctrl+Z`: undo
- `Ctrl+Shift+Z`: redo
- `Delete` / `Backspace`: delete selection
- `o`: toggle tool options panel
- `Esc`: select tool, or close editor when already in select mode

Tool shortcuts:

- `v` select
- `h` pan
- `b` blur
- `p` pen
- `a` arrow
- `r` rectangle
- `c` crop
- `t` text

Text editing:

- `Enter`: line break
- `Ctrl+Enter`: commit text
- `Ctrl+C`: copy selected text
- `Esc`: exit text focus

Default editor navigation:

- Pan hold key: `Space`
- Zoom in: `Ctrl++`, `Ctrl+=`, `Ctrl+KP_Add`
- Zoom out: `Ctrl+-`, `Ctrl+_`, `Ctrl+KP_Subtract`
- Actual size: `Ctrl+0`, `Ctrl+KP_0`
- Fit: `Shift+1`

## Configuration

Config directory:

- `$XDG_CONFIG_HOME/chalkak/`
- fallback: `$HOME/.config/chalkak/`

Files:

- `theme.json`
- `keybindings.json`

Temporary captures:

- `$XDG_RUNTIME_DIR/`
- fallback: `/tmp/chalkak/`

Saved screenshots:

- `$HOME/Pictures/`

## Development

Common commands:

```bash
cargo check
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -D warnings
```

Current module layout:

- `src/app`: runtime orchestration and GTK lifecycle
- `src/capture`: Hyprland/grim/slurp capture backends
- `src/preview`: preview window behavior
- `src/editor`: editor model and tool behavior
- `src/input`: shortcut and navigation handling
- `src/storage`: temp/save lifecycle and cleanup
- `src/theme`, `src/ui`: theme/config + shared style tokens
- `src/state`: app state machine
- `src/clipboard`: clipboard integration (`wl-copy`)
- `src/config`: config/keybinding/theme path helpers
- `src/error`: application-level error/result types
- `src/logging`: tracing subscriber setup

## AUR Packaging Notes (for maintainers)

Suggested `PKGBUILD` dependency baseline:

- `depends=('gtk4' 'hyprland' 'grim' 'slurp' 'wl-clipboard')`
- `makedepends=('rust' 'cargo' 'pkgconf' 'gtk4')`

Package name target: `chalkak`.

## Maintainer

- Name: `BitYoungjae`
- Email: `bityoungjae@gmail.com`

## License

`chalkak` is dual-licensed under:

- MIT
- Apache-2.0

SPDX expression: `MIT OR Apache-2.0`

This matches the dependency landscape (mostly MIT and Apache-2.0-family permissive licenses) and keeps AUR/distribution reuse straightforward.

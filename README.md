# ChalKak

<p align="center">
  <img src="./assets/banner.jpeg" alt="ChalKak banner" width="100%" />
</p>

English | [한국어](README.ko.md)

A Hyprland-focused screenshot utility for Wayland with a preview-first workflow and a lightweight annotation editor.

## Demo Video

<https://github.com/user-attachments/assets/2d2ed794-f86e-4216-b5f1-7dcb513791d4>

## User Guides

- [English User Guide](docs/USER_GUIDE.md)
- [한국어 사용자 가이드](docs/USER_GUIDE.ko.md)

## Name Origin

`ChalKak` is inspired by the Korean onomatopoeia `찰칵!`, the camera shutter click sound.

## Highlights

- Capture modes: fullscreen, region, and window.
- Preview stage before final action (save, copy, edit, delete).
- Built-in editor tools: select, pan, blur, pen, arrow, rectangle, crop, text, OCR.
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

### AUR

This repository includes AUR packaging metadata for `chalkak` in `PKGBUILD` and `.SRCINFO`.

Install with your AUR helper, for example:

```bash
yay -S chalkak
```

For OCR text recognition support, also install the model files:

```bash
yay -S chalkak-ocr-models
```

If the published AUR package is behind the current crate release, use the source build path below.

### Build from source

```bash
git clone https://github.com/BitYoungjae/ChalKak.git chalkak
cd chalkak
cargo run
```

## Usage

Launchpad UI:

```bash
chalkak --launchpad
```

Running `chalkak` with no flags starts and exits immediately.

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
- `c`: copy image
- `e`: open editor
- `o`: OCR (extract text from entire image)
- `Delete`: delete capture
- `Esc`: close preview

Editor:

- `Ctrl+S`: save
- `Ctrl+C`: copy image
- `Ctrl+Z`: undo
- `Ctrl+Shift+Z`: redo
- `Delete` / `Backspace`: delete selection
- `Tab`: toggle tool options panel
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
- `o` OCR

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
- `config.json`

`theme.json` (summary):

- `mode`: `system`, `light`, `dark`
- `config.json`: application settings (e.g. `ocr_language`)
- `colors`: supports shared + per-mode overrides
- `colors.common` + `colors.dark` + `colors.light`
- `editor`: supports shared + per-mode overrides
- `editor.common` + `editor.dark` + `editor.light`
- all objects can be partial; missing fields fall back to built-in defaults
- merge order:
- `built-in defaults -> common -> current mode`
- `system` follows runtime desktop preference and falls back to dark when unavailable
- legacy schema is still supported:
- shared flat `editor` + `editor_modes.dark/light`
- if both legacy and new keys are present, precedence is:
- `editor(flat) -> editor.common -> editor_modes.<mode> -> editor.<mode>`
- editor preset constraints:
- `stroke_width_presets`: `1..=64`
- `text_size_presets`: `8..=160`
- each preset list: up to 6 unique items

For full examples and field-by-field details, see:

- `docs/USER_GUIDE.md`
- `docs/USER_GUIDE.ko.md`

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
- `src/ocr`: OCR text recognition (PaddleOCR v5 / MNN)
- `src/config`: config/keybinding/theme path helpers
- `src/error`: application-level error/result types
- `src/logging`: tracing subscriber setup

## AUR Packaging Notes (for maintainers)

`PKGBUILD` and `.SRCINFO` are committed in this repository.

When releasing a new version:

1. Match `PKGBUILD` `pkgver` to `Cargo.toml` `version`.
2. Reset `pkgrel=1` when `pkgver` changes.
3. Update `source` to `.../archive/refs/tags/vX.Y.Z.tar.gz`.
4. Refresh checksums with `updpkgsums`.
5. Regenerate `.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO`.

Dependency baseline:

- `depends=('gtk4' 'hyprland' 'grim' 'slurp' 'wl-clipboard')`
- `makedepends=('rust' 'cargo' 'pkgconf' 'gtk4' 'cmake')`
- `optdepends=('chalkak-ocr-models: OCR text recognition support')`

Package name target: `chalkak`.

A separate AUR package `chalkak-ocr-models` provides PaddleOCR v5 model files for OCR. Its packaging metadata lives in `aur/chalkak-ocr-models/`.

## Maintainer

- Name: `BitYoungjae`
- Email: `bityoungjae@gmail.com`

## License

`chalkak` is dual-licensed under:

- MIT
- Apache-2.0

SPDX expression: `MIT OR Apache-2.0`

This matches the dependency landscape (mostly MIT and Apache-2.0-family permissive licenses) and keeps AUR/distribution reuse straightforward.

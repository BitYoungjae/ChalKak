# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` is a thin CLI entrypoint that calls `chalkak::run()`.
- `src/lib.rs` wires top-level modules and shared result/error exports.
- Core domains are split by module: `src/app/` (runtime orchestration), `src/capture/`, `src/preview/`, `src/editor/`, `src/input/`, `src/state/`, `src/storage/`, `src/theme/`, `src/ui/`, `src/clipboard/`, and `src/logging/`.
- Tests are colocated with implementation using `#[cfg(test)]`; there is no separate `tests/` directory currently.

## Build, Test, and Development Commands
- `cargo check` : fast compile/type validation before commits.
- `cargo test` : runs module unit tests.
- `cargo fmt --check` : enforces formatting used in CI/PR checks.
- `cargo run` : launches the app locally (requires Hyprland-compatible runtime).
- `cargo clippy --all-targets --all-features -D warnings` : optional but recommended lint gate.

## Coding Style & Naming Conventions
- Follow Rust 2021 defaults and `rustfmt` output (4-space indentation, standard wrapping).
- Naming: `snake_case` for files/functions/modules, `UpperCamelCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants.
- Keep logic in focused modules instead of growing `mod.rs` files; prefer small, testable helpers.
- Use typed errors (`thiserror`) and module result aliases instead of `unwrap()` in production paths.

## Testing Guidelines
- Add or update unit tests next to changed code with behavior-focused names (example: `capture_region_errors_when_selection_empty`).
- Run `cargo test` locally before opening a PR.
- For UI/runtime behavior changes, manually verify capture/peek flow, preview/editor interactions, keyboard navigation, and temp-file cleanup behavior.

## Commit & Pull Request Guidelines
- Git history uses concise gitmoji-style subjects (examples: `♻️ extract editor runtime module`, `🧩 add collapsible editor options panel`).
- Keep commits single-purpose and imperative.
- Follow `.github/PULL_REQUEST_TEMPLATE.md`: include summary, change checklist, explicit test plan (`cargo check`, `cargo test`, `cargo fmt --check`), and logs/screenshots when UI changes.
- Note runtime dependency impacts in PRs (`grim`, `slurp`, `wl-clipboard`, `gtk4-layer-shell`).

## Configuration & Runtime Notes
- User config files live at `$XDG_CONFIG_HOME/chalkak/` (fallback `$HOME/.config/chalkak/`), including `theme.json` and `keybindings.json`.
- Temporary captures are created under `$XDG_RUNTIME_DIR/chalkak/`; preserve cleanup behavior when changing storage/capture code.

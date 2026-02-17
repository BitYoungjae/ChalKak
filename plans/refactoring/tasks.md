# Master Task Checklist

> Track progress across sessions. Mark tasks `[x]` when completed.

## Session 01: Fix Circular Dependencies
_Branch: `refactor/fix-circular-deps`_

- [x] **T01-1** Move `ColorTokens`, `default_color_tokens()`, `tokens_for()` from `ui/style.rs` to `theme/mod.rs`
- [x] **T01-2** Update `ui/mod.rs` re-exports to point at `crate::theme` for moved items
- [x] **T01-3** Strip `ui/style.rs` down to `StyleTokens` + `LAYOUT_TOKENS` only
- [x] **T01-4** Move color-related tests from `ui/style.rs` to `theme/mod.rs`
- [x] **T01-5** Create `state/error.rs` with `StateError` and `StateResult`
- [x] **T01-6** Update `state/machine.rs` to use `StateError`/`StateResult`
- [x] **T01-7** Replace `AppError::InvalidStateTransition` with `AppError::State(StateError)` wrapper
- [x] **T01-8** Update `state/machine.rs` tests to match on `StateError`
- [x] **T01-V** Validate: `cargo check && cargo test && cargo fmt --check`

## Session 02: Relocate Misplaced Types
_Branch: `refactor/relocate-types`_

- [x] **T02-1** Move `state/window.rs` → `app/window_state.rs`
- [x] **T02-2** Remove `mod window` + re-exports from `state/mod.rs`
- [x] **T02-3** Add `mod window_state` to `app/mod.rs`
- [x] **T02-4** Update 5 consumer files to use new import path for `RuntimeWindow*`
- [x] **T02-5** Create `src/geometry.rs` with `ToolPoint`, `ToolBounds`, `ImageBounds`, `Color`
- [x] **T02-6** Add `pub mod geometry;` to `src/lib.rs`
- [x] **T02-7** Replace type definitions in `editor/tools/mod.rs` with `pub use crate::geometry::*` re-exports
- [x] **T02-8** Verify all `super::Color`, `super::ToolPoint` etc. still resolve in tool subfiles
- [x] **T02-V** Validate: `cargo check && cargo test && cargo fmt --check`

## Session 03: OCR Abstraction Cleanup
_Branch: `refactor/ocr-abstraction`_

- [x] **T03-1** Add `pub use ocr_rs::OcrEngine;` to `src/ocr/mod.rs`
- [x] **T03-2** Replace all `ocr_rs::OcrEngine` → `crate::ocr::OcrEngine` in `app/` (9 sites across 6 files)
- [x] **T03-3** Verify no direct `ocr_rs` imports remain in `src/app/`
- [x] **T03-4** Move `pixbuf_region_to_dynamic_image()` from `ocr/mod.rs` → `app/ocr_support.rs`
- [x] **T03-5** Update call sites for moved function
- [x] **T03-6** Remove duplicate `SharedStatusLog` from `ocr_support.rs`, import from `launchpad_actions`
- [x] **T03-V** Validate: `cargo check && cargo test && cargo fmt --check`

## Session 04: Extract Launchpad from `app/mod.rs`
_Branch: `refactor/extract-launchpad`_

- [ ] **T04-1** Create `src/ui/widgets.rs` with `icon_button`, `icon_toggle_button`, `install_lucide_icon_theme`
- [ ] **T04-2** Update `ui/mod.rs` to declare and re-export `widgets`
- [ ] **T04-3** Create `src/app/launchpad.rs` with `LaunchpadUi`, `build_launchpad_ui`, and all helper types/functions
- [ ] **T04-4** Add `mod launchpad;` + `use self::launchpad::*;` to `app/mod.rs`
- [ ] **T04-5** Remove moved items from `app/mod.rs`
- [ ] **T04-6** Verify `app/` submodule `use super::*` globs still resolve all needed types
- [ ] **T04-V** Validate: `cargo check && cargo test && cargo fmt --check`

## Session 05: Clean Domain Boundaries
_Branch: `refactor/domain-boundaries`_

- [ ] **T05-1** Create `src/app/actions.rs` with `execute_editor_action` + `execute_preview_action`
- [ ] **T05-2** Remove `execute_editor_action` from `editor/mod.rs`
- [ ] **T05-3** Remove `execute_preview_action` from `preview/actions.rs` + `preview/mod.rs` re-export
- [ ] **T05-4** Update call sites: `editor_popup/output.rs` and `launchpad_actions.rs`
- [ ] **T05-5** Move related tests to `app/actions.rs` or keep with adjusted imports
- [ ] **T05-6** Create `src/capture/hyprland.rs` — extract Hyprland JSON types and parsing
- [ ] **T05-7** Update `capture/mod.rs` to use `mod hyprland;`
- [ ] **T05-V** Validate: `cargo check && cargo test && cargo fmt --check`

## Session 06: Split Large Files
_Branch: `refactor/split-large-files`_

- [ ] **T06-1** Split `editor/tools/mod.rs` → `operations.rs`, `selection.rs`, `query.rs`
- [ ] **T06-2** Split `app/editor_runtime.rs` → `canvas.rs`, `toolbar.rs`, `window.rs`
- [ ] **T06-3** Extract image processing from `editor_popup/render.rs` → `image_processing.rs`
- [ ] **T06-4** Move tests to their respective new files
- [ ] **T06-V** Validate: `cargo check && cargo test && cargo fmt --check`

---

## Dependencies

```
Session 01  (no deps)
    ↓
Session 02  (after 01)
    ↓
Session 03  (after 01, independent of 02)
    ↓
Session 04  (after 01-03)
    ↓
Session 05  (after 01-04)
    ↓
Session 06  (after 01-05)
```

Note: Sessions 02 and 03 are technically independent and could be parallelized if working on separate branches, but sequential execution is safer.

## Summary

| Session | Tasks | Status |
|---------|-------|--------|
| 01 — Circular Dependencies | 9 | Not Started |
| 02 — Relocate Types | 9 | Not Started |
| 03 — OCR Abstraction | 7 | Not Started |
| 04 — Extract Launchpad | 7 | Not Started |
| 05 — Domain Boundaries | 8 | Not Started |
| 06 — Split Large Files | 5 | Not Started |
| **Total** | **45** | |

---
name: chalkak-ocr-models-release
description: Release OCR model files to GitHub and update the AUR package. Use when publishing new or updated PaddleOCR model files for chalkak-ocr-models.
argument-hint: "[version]"
disable-model-invocation: true
allowed-tools: Bash, Read, Glob
---

# Chalkak OCR Models Release Workflow

Package PaddleOCR v5 model files as a GitHub release asset and update the `chalkak-ocr-models` AUR package.

## Guardrails

- Confirm model source files exist before proceeding.
- Never overwrite an existing GitHub release tag without user approval.
- Always verify the tarball contents before uploading.
- Update PKGBUILD sha256sum from the actual tarball, never use SKIP in a published package.

## Prerequisites

Required tools: `git`, `gh` (GitHub CLI, authenticated), `makepkg`, `updpkgsums` (`pacman-contrib`).

## Inputs

- Optional version argument via `$ARGUMENTS` (e.g., `2`). Defaults to reading `pkgver` from `aur/chalkak-ocr-models/PKGBUILD`.

## Model Source

Model files are expected at one of these locations (checked in order):
1. Path provided by the user
2. `$HOME/References/rust-paddle-ocr/models/`

Required files (shared detection model + 11 language-specific recognition models and charset files):
- `PP-OCRv5_mobile_det.mnn` (shared detection)
- `korean_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_korean.txt`
- `en_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_en.txt`
- `PP-OCRv5_mobile_rec.mnn` + `ppocr_keys_v5.txt` (Chinese)
- `latin_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_latin.txt`
- `cyrillic_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_cyrillic.txt`
- `arabic_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_arabic.txt`
- `th_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_th.txt`
- `el_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_el.txt`
- `devanagari_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_devanagari.txt`
- `ta_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_ta.txt`
- `te_PP-OCRv5_mobile_rec_infer.mnn` + `ppocr_keys_te.txt`

The full file list can also be read from `aur/chalkak-ocr-models/PKGBUILD`.

## Workflow

### 1. Resolve version and locate model files

- If user passed version via `$ARGUMENTS`, use it. Otherwise read from `aur/chalkak-ocr-models/PKGBUILD`:

```bash
sed -n 's/^pkgver=\(.*\)/\1/p' aur/chalkak-ocr-models/PKGBUILD
```

- Locate model files and verify all required files exist (1 detection + 11 recognition + 11 charset = 23 files). If not found, ask the user for the path.

### 2. Create tarball

```bash
# Include all model and charset files from the model directory
tar czf /tmp/chalkak-ocr-models-v${VERSION}.tar.gz \
  -C "${MODEL_DIR}" \
  PP-OCRv5_mobile_det.mnn \
  korean_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_korean.txt \
  en_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_en.txt \
  PP-OCRv5_mobile_rec.mnn ppocr_keys_v5.txt \
  latin_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_latin.txt \
  cyrillic_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_cyrillic.txt \
  arabic_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_arabic.txt \
  th_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_th.txt \
  el_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_el.txt \
  devanagari_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_devanagari.txt \
  ta_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_ta.txt \
  te_PP-OCRv5_mobile_rec_infer.mnn ppocr_keys_te.txt
```

Verify contents:

```bash
tar tzf /tmp/chalkak-ocr-models-v${VERSION}.tar.gz
```

### 3. Check for existing release tag

```bash
gh release view "ocr-models-v${VERSION}" --repo bityoungjae/chalkak 2>&1
```

- If it exists, ask the user whether to overwrite (delete and recreate) or abort.

### 4. Create GitHub release and upload asset

```bash
gh release create "ocr-models-v${VERSION}" \
  "/tmp/chalkak-ocr-models-v${VERSION}.tar.gz" \
  --repo bityoungjae/chalkak \
  --title "OCR Models v${VERSION}" \
  --notes "PaddleOCR v5 model files for ChalKak OCR feature.

Contents:
- PP-OCRv5_mobile_det.mnn (shared detection model)
- 11 language-specific recognition models (.mnn) and charset files (.txt)
- Supported: Korean, English, Chinese, Latin, Cyrillic, Arabic, Thai, Greek, Devanagari, Tamil, Telugu

License: Apache-2.0 (PaddleOCR)"
```

### 5. Verify upload

```bash
gh release view "ocr-models-v${VERSION}" --repo bityoungjae/chalkak
```

Confirm the asset is listed and the download URL works.

### 6. Update PKGBUILD

Update version in `aur/chalkak-ocr-models/PKGBUILD`:

```bash
sed -i "s/^pkgver=.*/pkgver=${VERSION}/" aur/chalkak-ocr-models/PKGBUILD
sed -i "s/^pkgrel=.*/pkgrel=1/" aur/chalkak-ocr-models/PKGBUILD
```

Update sha256sum from the uploaded tarball:

```bash
sha256sum /tmp/chalkak-ocr-models-v${VERSION}.tar.gz
```

Replace the `sha256sums` line in PKGBUILD with the actual hash.

Generate .SRCINFO:

```bash
cd aur/chalkak-ocr-models && makepkg --printsrcinfo > .SRCINFO && cd -
```

### 7. Commit PKGBUILD changes

```bash
git add aur/chalkak-ocr-models/PKGBUILD aur/chalkak-ocr-models/.SRCINFO
git commit -m "chore: update chalkak-ocr-models to v${VERSION}"
```

Ask the user whether to push to origin.

### 8. Report result

Include:
- GitHub release tag: `ocr-models-v${VERSION}`
- Asset download URL
- PKGBUILD updated with correct sha256sum
- Next step: push `chalkak-ocr-models` to AUR if applicable

## Error Handling

- **Model files not found**: ask user for the correct path.
- **gh not authenticated**: show `gh auth login` and stop.
- **Release tag exists**: ask user whether to overwrite or abort.
- **Upload failure**: retry once, then report error.
- **makepkg --printsrcinfo fails**: report and continue (PKGBUILD still updated).

## Output Template

```
OCR Models v${VERSION} released.

GitHub:
- Tag: ocr-models-v${VERSION}
- Asset: chalkak-ocr-models-v${VERSION}.tar.gz
- URL: https://github.com/bityoungjae/chalkak/releases/tag/ocr-models-v${VERSION}

Packaging:
- aur/chalkak-ocr-models/PKGBUILD updated (pkgver=${VERSION}, sha256sum refreshed)
- .SRCINFO regenerated

Next steps:
- Push chalkak-ocr-models to AUR (if applicable)
```

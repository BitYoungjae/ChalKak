---
name: chalkak-ocr-models-release
description: Release and sync the `chalkak-ocr-models` AUR package by bumping `aur/chalkak-ocr-models/PKGBUILD` and `.SRCINFO`, refreshing checksums against the GitHub asset `ocr-models-vN`, and pushing packaging-only metadata to AUR. Use when publishing OCR model files to AUR, updating model package version/checksum, or fixing `chalkak-ocr-models` AUR metadata.
argument-hint: "[version]"
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob
---

# Chalkak OCR Models Release

Execute OCR model package release tasks with a safe sequence for source validation,
checksum refresh, and AUR sync.

## Guardrails

- Treat this as independent from app release.
- Do not create or push app tags like `vX.Y.Z` from this skill.
- Do not modify top-level `PKGBUILD` or top-level `.SRCINFO`.
- Modify only:
  - `aur/chalkak-ocr-models/PKGBUILD`
  - `aur/chalkak-ocr-models/.SRCINFO`
- Run from `main` and refuse if the working tree is dirty unless the user explicitly approves.
- Never force-push tags or branches.
- Use AUR remote `aur-ocr-models` (`ssh://aur@aur.archlinux.org/chalkak-ocr-models.git`).

## Prerequisites

- `git`
- `curl` (or `wget`) for source URL probe
- `updpkgsums` (`pacman-contrib`)
- `makepkg`

## Inputs

- Optional version argument: `$ARGUMENTS` (`N` or `vN`, models package release format).
- If no version is provided, read `pkgver` from `aur/chalkak-ocr-models/PKGBUILD`.

## Workflow

1. Verify branch safety and clean tree.

```bash
current_branch="$(git branch --show-current)"
origin_default_branch="$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
printf "current_branch=%s\norigin_default_branch=%s\n" "$current_branch" "$origin_default_branch"
git status --short
```

- Abort if `origin` default branch is missing or not `main`.
- Abort if current branch is not `main`.
- Abort if tree is dirty, unless the user explicitly approves continuing.

2. Resolve model package version.

```bash
if [ -n "$1" ]; then
  pkgver="${1#v}"
else
  pkgver="$(sed -n 's/^pkgver=//p' aur/chalkak-ocr-models/PKGBUILD | head -n1)"
fi
printf "pkgver=%s\n" "$pkgver"
```

3. Update `aur/chalkak-ocr-models/PKGBUILD`.

```bash
sed -i "s/^pkgver=.*/pkgver=$pkgver/" aur/chalkak-ocr-models/PKGBUILD
sed -i "s/^pkgrel=.*/pkgrel=1/" aur/chalkak-ocr-models/PKGBUILD
sed -i 's|^source=.*|source=("$pkgname-v$pkgver.tar.gz::$url/releases/download/ocr-models-v$pkgver/$pkgname-v$pkgver.tar.gz")|' aur/chalkak-ocr-models/PKGBUILD
```

4. Confirm the GitHub release asset exists before checksum refresh.

```bash
asset_url="https://github.com/bityoungjae/chalkak/releases/download/ocr-models-v${pkgver}/chalkak-ocr-models-v${pkgver}.tar.gz"
curl -fLI "$asset_url"
```

- Abort on 404/401 or network failure.

5. Refresh checksum and regenerate `.SRCINFO`.

```bash
(
  cd aur/chalkak-ocr-models
  updpkgsums
  makepkg --printsrcinfo > .SRCINFO
)
```

6. Commit package metadata changes on `main`.

```bash
git add aur/chalkak-ocr-models/PKGBUILD aur/chalkak-ocr-models/.SRCINFO
git commit -m "chore: update chalkak-ocr-models AUR metadata for v$pkgver"
git push origin main
```

7. Ensure AUR remote exists.

```bash
git remote get-url aur-ocr-models
```

- If missing, add it:

```bash
git remote add aur-ocr-models ssh://aur@aur.archlinux.org/chalkak-ocr-models.git
```

8. Push packaging-only content to AUR `master`.

- Use a temporary worktree so local `main` stays unchanged.
- Sync only `PKGBUILD` and `.SRCINFO` to AUR root.

```bash
repo_root="$(git rev-parse --show-toplevel)"
tmpdir="$(mktemp -d)"
git worktree add "$tmpdir" --detach
(
  cd "$tmpdir"
  if git ls-remote --exit-code aur-ocr-models refs/heads/master >/dev/null 2>&1; then
    git fetch aur-ocr-models master
    git checkout -B aur-ocr-models-pkg FETCH_HEAD
  else
    git checkout --orphan aur-ocr-models-pkg
    git rm -rf --cached . >/dev/null 2>&1 || true
    find . -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +
  fi

  git --git-dir="$repo_root/.git" show main:aur/chalkak-ocr-models/PKGBUILD > PKGBUILD
  git --git-dir="$repo_root/.git" show main:aur/chalkak-ocr-models/.SRCINFO > .SRCINFO

  git add PKGBUILD .SRCINFO
  git commit -m "Update to v$pkgver" || true
  git push aur-ocr-models HEAD:master
)
git worktree remove "$tmpdir" --force
```

9. Report completion.

- Include:
  - `pkgver`
  - Commit hash pushed to `origin/main`
  - AUR push status (`aur-ocr-models` `master`)
  - AUR URL: `https://aur.archlinux.org/packages/chalkak-ocr-models`

## Error Handling

- Source asset missing: stop and ask user to publish `ocr-models-vN` release asset first.
- `updpkgsums` failure: retry once after source check, then stop.
- AUR auth failure: report SSH key issue and stop before retrying blindly.
- Non-fast-forward AUR push: fetch `aur-ocr-models/master`, replay `PKGBUILD` and `.SRCINFO`, then push again.

---
name: chalkak-release
description: Tag release and sync AUR metadata. Use when the user asks to publish a new Chalkak version, run release packaging, update AUR metadata, or automate tag + AUR sync.
argument-hint: "[version]"
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob
---

# Chalkak Release Workflow

Execute Chalkak release tasks with a safe sequence for Git tag creation, Arch checksum refresh, and AUR package push.

## Guardrails

- Run release only from `main`. Abort if the current branch or the `origin` default branch is not `main`.
- Do not create tags or run release packaging from `develop`.
- Refuse release if the working tree is dirty unless the user explicitly approves.
- Use annotated tags only. Never force-push tags or branches.
- Confirm tag non-existence both locally and on `origin`.
- Keep AUR history isolated in `aur-pkg`, and push only `PKGBUILD` and `.SRCINFO` to AUR `master`.

## Prerequisites

Required tools: `git`, `makepkg`, `updpkgsums` (`pacman-contrib`). Recommended: `jq`.

## Inputs

- Optional version argument via `$ARGUMENTS` (e.g., `X.Y.Z` or `vX.Y.Z`).
- If no argument is given, read version from `Cargo.toml`.

## Workflow

### 1. Verify branch and clean tree

```bash
current_branch="$(git branch --show-current)"
origin_default_branch="$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
printf "current_branch=%s\norigin_default_branch=%s\n" "$current_branch" "$origin_default_branch"
git status --short
```

- If `origin` default branch cannot be detected, stop and report repository configuration issue.
- If `origin` default branch is not `main`, stop and report workflow misconfiguration.
- If current branch is not `main`, stop immediately and report that release must run from `main`.
- If the tree is dirty, ask whether to continue or stop.

### 2. Resolve release version

- If user passed version via `$ARGUMENTS`, normalize to `vX.Y.Z` for tag and `X.Y.Z` for `pkgver`.
- Else read from `Cargo.toml`:

```bash
sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1
```

### 3. Update PKGBUILD version fields

```bash
sed -i "s/^pkgver=.*/pkgver=X.Y.Z/" PKGBUILD
sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD
```

Replace `X.Y.Z` with the resolved version without `v`.

### 4. Validate tag availability

```bash
git tag -l "vX.Y.Z"
git ls-remote --tags origin "refs/tags/vX.Y.Z"
```

Abort if tag exists locally or remotely.

### 5. Create and push release tag

```bash
git pull --ff-only origin main
git tag -a "vX.Y.Z" -m "Release vX.Y.Z"
git push origin "vX.Y.Z"
```

### 6. Refresh Arch package checksum and SRCINFO

```bash
sleep 5
updpkgsums
makepkg --printsrcinfo > .SRCINFO
```

If `updpkgsums` fails due to remote tag timing, retry after a short wait.

### 7. Commit packaging update on main

```bash
git add PKGBUILD .SRCINFO
git commit -m "chore: update AUR metadata for vX.Y.Z"
git push origin main
```

### 8. Ensure AUR remote exists

```bash
git remote get-url aur
```

If missing, ask user to confirm and add:

```bash
git remote add aur ssh://aur@aur.archlinux.org/chalkak.git
```

### 9. Push AUR package branch (aur-pkg -> aur/master)

Create or reuse `aur-pkg` as packaging-only branch. Keep only `PKGBUILD` and `.SRCINFO` tracked for AUR push.

**If `aur-pkg` does not exist locally:**

Check if AUR master exists:

```bash
git ls-remote aur refs/heads/master
```

If AUR master exists:

```bash
git fetch aur master:aur-pkg
git checkout aur-pkg
git checkout main -- PKGBUILD .SRCINFO
git commit -m "Update to vX.Y.Z"
git push aur aur-pkg:master
git checkout main
```

If AUR master does not exist:

```bash
git checkout --orphan aur-pkg
git rm -rf --cached . >/dev/null 2>&1 || true
git add PKGBUILD .SRCINFO
git commit -m "Initial AUR package for chalkak vX.Y.Z"
git push aur aur-pkg:master
git checkout main
```

**If `aur-pkg` exists locally:**

```bash
git checkout aur-pkg
git checkout main -- PKGBUILD .SRCINFO
git commit -m "Update to vX.Y.Z"
git push aur aur-pkg:master
git checkout main
```

### 10. Report release result

Include:
- Tag: `vX.Y.Z`
- Main repository push status
- AUR push status
- Release URL: `https://github.com/bityoungjae/chalkak/releases/tag/vX.Y.Z`
- AUR package URL: `https://aur.archlinux.org/packages/chalkak`

## Error Handling

- **Tag already exists**: abort and notify user.
- **origin default branch missing/not main**: report workflow misconfiguration and stop before tagging.
- **Missing tools**: show install command and stop.
- **updpkgsums failure**: retry, then ask user whether to proceed manually.
- **AUR authentication failure**: report SSH/key issue and stop AUR push.
- **AUR non-fast-forward**: fetch `aur/master` into `aur-pkg` and retry.

## Chalkak-Specific Notes

- Primary package files: `PKGBUILD`, `.SRCINFO`.
- Source tarball pattern in PKGBUILD: `.../archive/refs/tags/v$pkgver.tar.gz`.
- Package name and AUR repo name: `chalkak`.
- OCR models are a separate AUR package (`chalkak-ocr-models`) and do not need updating during app releases.

## Output Template

```
Release vX.Y.Z completed.

Tag:
- vX.Y.Z pushed to origin

Packaging:
- PKGBUILD pkgver/pkgrel updated
- sha256sums refreshed
- .SRCINFO regenerated and pushed

AUR:
- aur remote: configured
- pushed aur-pkg -> master

Links:
- GitHub release: https://github.com/bityoungjae/chalkak/releases/tag/vX.Y.Z
- AUR package: https://aur.archlinux.org/packages/chalkak
```

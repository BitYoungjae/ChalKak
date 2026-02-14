---
name: chalkak-release
description: Release Chalkak to GitHub and AUR by creating a version tag, updating PKGBUILD checksums, regenerating .SRCINFO, and pushing the AUR package branch. Use when the user asks to publish a new Chalkak version, run release packaging, update AUR metadata, or automate tag + AUR sync.
---

# Chalkak Release Workflow

Execute Chalkak release tasks with a safe sequence for Git tag creation, Arch checksum refresh, and AUR package push.

## Guardrails

- Verify branch safety before tagging.
- Refuse release if the working tree is dirty unless the user explicitly approves.
- Use annotated tags only.
- Never force-push tags or branches.
- Confirm tag non-existence both locally and on `origin`.
- Keep AUR history isolated in `aur-pkg`, and push only `PKGBUILD` and `.SRCINFO` to AUR `master`.

## Prerequisites

- Required tools:
- `git`
- `makepkg`
- `updpkgsums` (`pacman-contrib`)
- Recommended: `jq`

## Inputs

- Optional version argument: `X.Y.Z` or `vX.Y.Z`.
- If no argument is given, read version from `Cargo.toml`.

## Workflow

1. Verify branch and clean tree.

```bash
git branch --show-current
git remote show origin | sed -n '/HEAD branch/s/.*: //p'
git status --short
```

- If current branch is not the origin default branch (`main`/`master`), ask for confirmation before proceeding.
- If the tree is dirty, ask whether to continue or stop.

1. Resolve release version.

- If user passed version, normalize to `vX.Y.Z` for tag and `X.Y.Z` for `pkgver`.
- Else read from `Cargo.toml`:

```bash
sed -n 's/^version = \"\\([^\"]*\\)\"/\\1/p' Cargo.toml | head -n1
```

1. Update `PKGBUILD` version fields.

```bash
sed -i "s/^pkgver=.*/pkgver=X.Y.Z/" PKGBUILD
sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD
```

- Replace `X.Y.Z` with the resolved version without `v`.

1. Validate tag availability.

```bash
git tag -l "vX.Y.Z"
git ls-remote --tags origin "refs/tags/vX.Y.Z"
```

- Abort if tag exists locally or remotely.

1. Create and push release tag.

```bash
git pull --ff-only origin "$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
git tag -a "vX.Y.Z" -m "Release vX.Y.Z"
git push origin "vX.Y.Z"
```

1. Refresh Arch package checksum and SRCINFO.

```bash
sleep 5
updpkgsums
makepkg --printsrcinfo > .SRCINFO
```

- If `updpkgsums` fails due to remote tag timing, retry after a short wait.

1. Commit packaging update on default branch.

```bash
git add PKGBUILD .SRCINFO
git commit -m "chore: update AUR metadata for vX.Y.Z"
git push origin "$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
```

1. Ensure AUR remote exists.

```bash
git remote get-url aur
```

- If missing, ask user to confirm and add:

```bash
git remote add aur ssh://aur@aur.archlinux.org/chalkak.git
```

1. Push AUR package branch (`aur-pkg` -> `aur/master`).

- Create or reuse `aur-pkg` as packaging-only branch.
- Keep only `PKGBUILD` and `.SRCINFO` tracked for AUR push.

If `aur-pkg` does not exist locally:

```bash
git ls-remote aur refs/heads/master
```

- If AUR master exists:

```bash
git fetch aur master:aur-pkg
git checkout aur-pkg
git checkout "$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')" -- PKGBUILD .SRCINFO
git commit -m "Update to vX.Y.Z"
git push aur aur-pkg:master
git checkout "$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
```

- If AUR master does not exist:

```bash
git checkout --orphan aur-pkg
git rm -rf --cached . >/dev/null 2>&1 || true
git add PKGBUILD .SRCINFO
git commit -m "Initial AUR package for chalkak vX.Y.Z"
git push aur aur-pkg:master
git checkout "$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
```

If `aur-pkg` exists locally:

```bash
git checkout aur-pkg
git checkout "$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')" -- PKGBUILD .SRCINFO
git commit -m "Update to vX.Y.Z"
git push aur aur-pkg:master
git checkout "$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
```

1. Report release result.

- Include:
- Tag: `vX.Y.Z`
- Main repository push status
- AUR push status
- Release URL and AUR package URL

## Error Handling

- Tag already exists: abort and notify user.
- Missing tools: show install command and stop.
- `updpkgsums` failure: retry, then ask user whether to proceed manually.
- AUR authentication failure: report SSH/key issue and stop AUR push.
- AUR non-fast-forward: fetch `aur/master` into `aur-pkg` and retry.

## Chalkak-Specific Notes

- Primary package files: `PKGBUILD`, `.SRCINFO`.
- Source tarball pattern in `PKGBUILD`: `.../archive/refs/tags/v$pkgver.tar.gz`.
- Package name and AUR repo name: `chalkak`.
- Expected release command intent: create Git tag, refresh checksums, and sync AUR metadata.

## Output Template

```text
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

---
name: chalkak-release
description: End-to-end Chalkak release orchestrator. Detects whether release is needed, builds English release notes from PR content, creates GitHub release first (to satisfy binary upload workflow), waits for release-binary success, and syncs all AUR packages (`chalkak`, `chalkak-bin`, `chalkak-ocr-models`) in one run.
argument-hint: "[app_version] [models_version]"
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob, Write, Edit
---

# Chalkak Unified Release Workflow

Use this skill when the user asks for a full release and wants one execution that completes GitHub + AUR delivery with verification.

## Guardrails

- Run only from `main`.
- Abort unless both current branch and `origin` default branch are `main`.
- Abort on dirty working tree unless user explicitly approves.
- Never force-push tags or branches.
- Never publish release notes in Korean. Release notes must be written in English.
- Create/update GitHub release before relying on `release-binary` upload.
- Ensure the release is tag-addressable (`GET /releases/tags/vX.Y.Z`) before running binary upload.
- Do not update AUR metadata for app packages until release binary assets exist.

## Why This Order Matters

- This repository uses `taiki-e/upload-rust-binary-action` in `.github/workflows/release-binary.yml`.
- That action uploads to an existing GitHub Release and fails with `release not found` if no release exists yet.
- Therefore, release notes + release object must be created first, then tag/workflow upload, then AUR checksum refresh.

## Prerequisites

- `git`
- `gh`
- `jq`
- `curl`
- `makepkg`
- `updpkgsums` (`pacman-contrib`)

## Inputs

- Optional app version: `$ARGUMENTS[0]` (`X.Y.Z` or `vX.Y.Z`).
- Optional OCR models version: `$ARGUMENTS[1]` (`N` or `vN`).
- If omitted:
  - app version: from `Cargo.toml`
  - OCR models version: from `aur/chalkak-ocr-models/PKGBUILD`

## Single-Run Result

When needed, one execution of this skill must finish all of the following:

1. Decide whether release/sync is required from `main` state and AUR state.
2. Generate English release notes from PR content using the provided template.
3. Create or update the GitHub release first (published, not draft), ensuring tag presence.
4. Wait for `release-binary` success and verify binary assets.
5. Update/checksum/regenerate metadata for:
   - `PKGBUILD` + `.SRCINFO` (root `chalkak`)
   - `aur/chalkak-bin/PKGBUILD` + `.SRCINFO`
   - `aur/chalkak-ocr-models/PKGBUILD` + `.SRCINFO` (if needed)
6. Push metadata commits to `origin/main`.
7. Push each AUR package to its own AUR remote.
8. Re-validate AUR RPC versions and release assets; keep release in published state.

## Additional Resources

- For the release notes template, see [templates/release-notes.en.md](templates/release-notes.en.md)

## Workflow

### 1) Branch safety and clean tree

```bash
set -euo pipefail

current_branch="$(git branch --show-current)"
origin_default_branch="$(git remote show origin | sed -n '/HEAD branch/s/.*: //p')"
printf "current_branch=%s\norigin_default_branch=%s\n" "$current_branch" "$origin_default_branch"
git status --short
```

- Stop if default branch detection fails.
- Stop if either branch is not `main`.
- Stop on dirty tree unless user explicitly allows.

### 2) Resolve target versions and current state

```bash
if [ -n "${1:-}" ]; then
  app_version="${1#v}"
else
  app_version="$(sed -n 's/^version = \"\\([^\"]*\\)\"/\\1/p' Cargo.toml | head -n1)"
fi
tag="v${app_version}"
app_expected="${app_version}-1"

if [ -n "${2:-}" ]; then
  models_version="${2#v}"
else
  models_version="$(sed -n 's/^pkgver=//p' aur/chalkak-ocr-models/PKGBUILD | head -n1)"
fi
```

```bash
aur_json="$(curl -fsSL 'https://aur.archlinux.org/rpc/?v=5&type=info&arg[]=chalkak&arg[]=chalkak-bin&arg[]=chalkak-ocr-models')"
aur_chalkak="$(jq -r '.results[] | select(.Name=="chalkak") | .Version // ""' <<<"$aur_json")"
aur_chalkak_bin="$(jq -r '.results[] | select(.Name=="chalkak-bin") | .Version // ""' <<<"$aur_json")"
aur_models="$(jq -r '.results[] | select(.Name=="chalkak-ocr-models") | .Version // ""' <<<"$aur_json")"

local_chalkak="$(awk -F= '/^pkgver=/{v=$2}/^pkgrel=/{r=$2} END{print v "-" r}' PKGBUILD)"
local_chalkak_bin="$(awk -F= '/^pkgver=/{v=$2}/^pkgrel=/{r=$2} END{print v "-" r}' aur/chalkak-bin/PKGBUILD)"
local_models="$(awk -F= '/^pkgver=/{v=$2}/^pkgrel=/{r=$2} END{print v "-" r}' aur/chalkak-ocr-models/PKGBUILD)"
```

```bash
tag_exists_remote=false
git ls-remote --exit-code --tags origin "refs/tags/${tag}" >/dev/null 2>&1 && tag_exists_remote=true

release_exists=false
gh release view "${tag}" --json tagName >/dev/null 2>&1 && release_exists=true

bin_url="https://github.com/bityoungjae/chalkak/releases/download/${tag}/chalkak-x86_64-unknown-linux-gnu.tar.gz"
sha_url="https://github.com/bityoungjae/chalkak/releases/download/${tag}/chalkak-x86_64-unknown-linux-gnu.sha256"
assets_ready=false
if curl -fsSI "$bin_url" >/dev/null 2>&1 && curl -fsSI "$sha_url" >/dev/null 2>&1; then
  assets_ready=true
fi
```

### 3) Decide if sync is required

```bash
need_app_release=false
need_models_sync=false

if [ "$local_chalkak" != "$app_expected" ] || [ "$local_chalkak_bin" != "$app_expected" ]; then
  need_app_release=true
fi
if [ "$aur_chalkak" != "$app_expected" ] || [ "$aur_chalkak_bin" != "$app_expected" ]; then
  need_app_release=true
fi
if [ "$tag_exists_remote" = false ] || [ "$release_exists" = false ] || [ "$assets_ready" = false ]; then
  need_app_release=true
fi

models_expected="$(awk -F= '/^pkgver=/{v=$2}/^pkgrel=/{r=$2} END{print v "-" r}' aur/chalkak-ocr-models/PKGBUILD)"
if [ "$aur_models" != "$models_expected" ]; then
  need_models_sync=true
fi
if [ -n "${2:-}" ]; then
  need_models_sync=true
fi

printf "need_app_release=%s\nneed_models_sync=%s\n" "$need_app_release" "$need_models_sync"
```

- If both are `false`, report "already synchronized" and stop.

### 4) Build release notes from PR content (English only, app release only)

Run this step only when `need_app_release=true`.

Use `.claude/skills/chalkak-release/templates/release-notes.en.md` as the mandatory shape.

1. Determine previous app tag:

```bash
prev_tag="$(git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | grep -vx "${tag}" | head -n1 || true)"
```

2. Collect PR numbers from commit messages in `${prev_tag}..HEAD`:

```bash
tmpdir="$(mktemp -d)"
pr_numbers_file="$tmpdir/pr_numbers.txt"
: > "$pr_numbers_file"

if [ -n "$prev_tag" ]; then
  git log --pretty='%s' "${prev_tag}..HEAD" \
    | sed -nE 's/.*\\(#([0-9]+)\\)$/\\1/p; s/^Merge pull request #([0-9]+).*/\\1/p' \
    | sort -u > "$pr_numbers_file"
fi
```

3. Fallback: query commit-associated PRs if no PR numbers were found:

```bash
if [ ! -s "$pr_numbers_file" ] && [ -n "$prev_tag" ]; then
  gh api "repos/BitYoungjae/ChalKak/compare/${prev_tag}...HEAD" --jq '.commits[].sha' \
    | while IFS= read -r sha; do
        gh api "repos/BitYoungjae/ChalKak/commits/${sha}/pulls?per_page=100" --jq '.[].number' || true
      done \
    | sort -u > "$pr_numbers_file"
fi
```

4. Fetch PR details and compose notes:

```bash
pr_jsonl="$tmpdir/prs.jsonl"
pr_merged_jsonl="$tmpdir/prs_merged.jsonl"
pr_referenced_jsonl="$tmpdir/prs_referenced_unmerged.jsonl"
: > "$pr_jsonl"
: > "$pr_merged_jsonl"
: > "$pr_referenced_jsonl"
while IFS= read -r pr; do
  [ -z "$pr" ] && continue
  pr_obj="$(gh api "repos/BitYoungjae/ChalKak/pulls/${pr}" \
    --jq '{number,title,html_url,author:.user.login,state,merged,merged_at,base:.base.ref,body}')"
  printf '%s\n' "$pr_obj" >> "$pr_jsonl"
  if [ "$(jq -r '.merged' <<<"$pr_obj")" = "true" ]; then
    printf '%s\n' "$pr_obj" >> "$pr_merged_jsonl"
  else
    printf '%s\n' "$pr_obj" >> "$pr_referenced_jsonl"
  fi
done < "$pr_numbers_file"
```

- Write `release-notes-vX.Y.Z.md` in English.
- Keep it factual and based on PR title/body and changed files.
- Use merged PRs as the primary release PR list.
- Keep unmerged/closed-but-referenced PRs in a separate "Referenced PRs" section when relevant.
- If no PRs are found, use commit summaries from `${prev_tag}..HEAD`.

### 5) Create/update GitHub release first, then run binary upload (app release only)

Run this step only when `need_app_release=true`.

Create release before asset upload to avoid `release not found`.

```bash
notes_file="$tmpdir/release-notes-${tag}.md"
# (Write the final English notes into $notes_file before commands below)

if gh release view "${tag}" --json tagName,isDraft >/dev/null 2>&1; then
  gh release edit "${tag}" \
    --draft=false \
    --title "ChalKak ${tag}" \
    --notes-file "$notes_file"
else
  if git ls-remote --exit-code --tags origin "refs/tags/${tag}" >/dev/null 2>&1; then
    gh release create "${tag}" \
      --verify-tag \
      --title "ChalKak ${tag}" \
      --notes-file "$notes_file"
  else
    gh release create "${tag}" \
      --target main \
      --title "ChalKak ${tag}" \
      --notes-file "$notes_file"
  fi
fi

# Hard gate: release must be resolvable by tag API before upload action.
gh api "repos/BitYoungjae/ChalKak/releases/tags/${tag}" >/dev/null
```

Wait for `release-binary.yml` (always select in-progress run first; otherwise dispatch a fresh run and select by dispatch timestamp):

```bash
run_id="$(gh run list --workflow release-binary.yml --limit 30 --json databaseId,headBranch,status,createdAt \
  --jq '.[] | select(.headBranch=="'"${tag}"'") | select(.status=="in_progress") | .databaseId' | head -n1 || true)"

if [ -z "$run_id" ]; then
  dispatch_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  gh workflow run release-binary.yml --ref "${tag}"
  sleep 5
  run_id="$(gh run list --workflow release-binary.yml --limit 50 --json databaseId,headBranch,createdAt \
    --jq '.[] | select(.headBranch=="'"${tag}"'") | select(.createdAt >= "'"${dispatch_at}"'") | .databaseId' | head -n1)"
fi

[ -n "$run_id" ] || { echo "failed to resolve release-binary run id"; exit 1; }

if ! gh run watch "$run_id" --exit-status; then
  # Auto-recover only for known release lookup failure.
  if gh run view "$run_id" --log-failed 2>/dev/null | rg -q 'release not found'; then
    gh release edit "${tag}" --draft=false --title "ChalKak ${tag}" --notes-file "$notes_file"
    gh api "repos/BitYoungjae/ChalKak/releases/tags/${tag}" >/dev/null

    retry_dispatch_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    gh workflow run release-binary.yml --ref "${tag}"
    sleep 5
    retry_run_id="$(gh run list --workflow release-binary.yml --limit 50 --json databaseId,headBranch,createdAt \
      --jq '.[] | select(.headBranch=="'"${tag}"'") | select(.createdAt >= "'"${retry_dispatch_at}"'") | .databaseId' | head -n1)"
    [ -n "$retry_run_id" ] || { echo "failed to resolve retry release-binary run id"; exit 1; }
    gh run watch "$retry_run_id" --exit-status
  else
    echo "release-binary failed for a non-recoverable reason"
    exit 1
  fi
fi
```

Verify release assets:

```bash
curl -fSI "$bin_url" >/dev/null
curl -fSI "$sha_url" >/dev/null
```

### 6) Update package metadata for all required packages

App source package (`chalkak`) and binary package (`chalkak-bin`) only when `need_app_release=true`:

```bash
if [ "$need_app_release" = true ]; then
  sed -i "s/^pkgver=.*/pkgver=${app_version}/" PKGBUILD
  sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD
  updpkgsums
  makepkg --printsrcinfo > .SRCINFO

  sed -i "s/^pkgver=.*/pkgver=${app_version}/" aur/chalkak-bin/PKGBUILD
  sed -i "s/^pkgrel=.*/pkgrel=1/" aur/chalkak-bin/PKGBUILD
  (
    cd aur/chalkak-bin
    updpkgsums
    makepkg --printsrcinfo > .SRCINFO
  )
fi
```

OCR models package (`chalkak-ocr-models`) only when needed:

```bash
if [ "$need_models_sync" = true ]; then
  sed -i "s/^pkgver=.*/pkgver=${models_version}/" aur/chalkak-ocr-models/PKGBUILD
  sed -i "s/^pkgrel=.*/pkgrel=1/" aur/chalkak-ocr-models/PKGBUILD
  sed -i 's|^source=.*|source=("$pkgname-v$pkgver.tar.gz::$url/releases/download/ocr-models-v$pkgver/$pkgname-v$pkgver.tar.gz")|' aur/chalkak-ocr-models/PKGBUILD

  models_asset_url="https://github.com/bityoungjae/chalkak/releases/download/ocr-models-v${models_version}/chalkak-ocr-models-v${models_version}.tar.gz"
  curl -fLI "$models_asset_url"

  (
    cd aur/chalkak-ocr-models
    updpkgsums
    makepkg --printsrcinfo > .SRCINFO
  )
fi
```

### 7) Commit and push metadata to `origin/main`

```bash
git add \
  PKGBUILD .SRCINFO \
  aur/chalkak-bin/PKGBUILD aur/chalkak-bin/.SRCINFO \
  aur/chalkak-ocr-models/PKGBUILD aur/chalkak-ocr-models/.SRCINFO

if ! git diff --cached --quiet; then
  git commit -m "chore(release): sync GitHub/AUR metadata for ${tag}"
  git push origin main
fi
```

### 8) Push to three AUR remotes

Ensure remotes:

```bash
git remote get-url aur >/dev/null 2>&1 || git remote add aur ssh://aur@aur.archlinux.org/chalkak.git
git remote get-url aur-bin >/dev/null 2>&1 || git remote add aur-bin ssh://aur@aur.archlinux.org/chalkak-bin.git
git remote get-url aur-ocr-models >/dev/null 2>&1 || git remote add aur-ocr-models ssh://aur@aur.archlinux.org/chalkak-ocr-models.git
```

Push helper (packaging-only `PKGBUILD` + `.SRCINFO`):

```bash
push_aur_pkg() {
  remote="$1"            # aur | aur-bin | aur-ocr-models
  src_prefix="$2"        # "" | "aur/chalkak-bin" | "aur/chalkak-ocr-models"
  commit_msg="$3"
  repo_root="$(git rev-parse --show-toplevel)"
  tmpdir="$(mktemp -d)"

  git worktree add "$tmpdir" --detach
  (
    cd "$tmpdir"
    if git ls-remote --exit-code "$remote" refs/heads/master >/dev/null 2>&1; then
      git fetch "$remote" master
      git checkout -B aur-sync FETCH_HEAD
    else
      git checkout --orphan aur-sync
      git rm -rf --cached . >/dev/null 2>&1 || true
      find . -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +
    fi

    if [ -z "$src_prefix" ]; then
      git --git-dir="$repo_root/.git" show main:PKGBUILD > PKGBUILD
      git --git-dir="$repo_root/.git" show main:.SRCINFO > .SRCINFO
    else
      git --git-dir="$repo_root/.git" show "main:${src_prefix}/PKGBUILD" > PKGBUILD
      git --git-dir="$repo_root/.git" show "main:${src_prefix}/.SRCINFO" > .SRCINFO
    fi

    git add PKGBUILD .SRCINFO
    git commit -m "$commit_msg" || true
    git push "$remote" HEAD:master
  )
  git worktree remove "$tmpdir" --force
}
```

Push all required packages:

```bash
if [ "$need_app_release" = true ]; then
  push_aur_pkg aur "" "Update chalkak to ${tag}"
  push_aur_pkg aur-bin "aur/chalkak-bin" "Update chalkak-bin to ${tag}"
fi
if [ "$need_models_sync" = true ]; then
  push_aur_pkg aur-ocr-models "aur/chalkak-ocr-models" "Update chalkak-ocr-models to v${models_version}"
fi
```

### 9) Finalize GitHub release metadata (app release only)

Run only when `need_app_release=true`, after binaries are attached and AUR sync is done, to ensure final title/notes/latest flags:

```bash
if [ "$need_app_release" = true ]; then
  gh release edit "${tag}" \
    --draft=false \
    --latest \
    --title "ChalKak ${tag}" \
    --notes-file "$notes_file"
fi
```

### 10) Post-verify and report

```bash
curl -fsSL 'https://aur.archlinux.org/rpc/?v=5&type=info&arg[]=chalkak&arg[]=chalkak-bin&arg[]=chalkak-ocr-models' | jq -r '.results[] | "\(.Name)\t\(.Version)"'
curl -fSI "$bin_url" >/dev/null
curl -fSI "$sha_url" >/dev/null
gh release view "${tag}" --json url,isDraft,isPrerelease,publishedAt,tagName,assets --jq '{tagName,isDraft,isPrerelease,publishedAt,url,assets:[.assets[].name]}'
```

- Report:
  - release tag and URL
  - release asset verification
  - each AUR package version after sync
  - commit hash pushed to `origin/main`

## Error Handling

- `release-binary` fails with `release not found`: force release to published, verify `GET /releases/tags/vX.Y.Z`, then re-dispatch once.
- Release assets still missing after successful workflow: stop and surface run URL for manual inspection.
- AUR auth failure: stop and report SSH key/remote issue.
- AUR non-fast-forward: fetch remote `master` and replay packaging-only files, then push again.
- OCR asset URL missing: stop model package update and ask user to publish `ocr-models-vN` asset first.
- Avoid `zsh` read-only variables like `status` for local shell vars; use names like `run_state`/`exit_code`.

# Design: release-please + commitlint for togl

**Date:** 2026-05-29
**Status:** Approved
**Reference:** mirrors `~/c/github-actions/py-launch-blueprint` (release-please + commitlint clusters), adapted to the togl Rust workspace.

## Goal

Add PR-driven, Conventional-Commit-based release automation:
1. **release-please** opens a release PR proposing the next semver bump + `CHANGELOG.md`; merging it tags `v*`, which triggers the existing `release.yml` binary build.
2. **commitlint** enforces Conventional Commits on PRs (humans/dependabot split).

## Decisions

| Decision | Choice |
|---|---|
| Versioning model | Single shared workspace version (`[workspace.package].version`); one version for all crates |
| release-type | `simple` + `extra-files` bumping `Cargo.toml` `$.workspace.package.version` (predictable for a virtual workspace) |
| Tag format | `include-component-in-tag: false` → tags stay `v0.3.0` (compat with existing `v*` tags + `release.yml`) |
| Auth | GitHub App (preferred) or fine-grained PAT; `GITHUB_TOKEN` NOT used (it won't trigger `release.yml` on the tag) |
| Lock sync | `sync-cargo-lock` job runs `cargo update --workspace` on the release-PR branch (CI is `--locked`) |
| bootstrap-sha | `22a85ad7...` (the `v0.2.3` commit) so the first RP PR proposes `v0.3.0` capturing P05/P08 |
| commitlint | ported verbatim (humans/dependabot configs, `wagoid/commitlint-github-action@v6.2.1`) |

## Files (all new; tracked on `feat/release-please`)

**release-please cluster:**
- `release-please-config.json` — package `"."`, `release-type: simple`, `package-name: togl`, `include-component-in-tag: false`, `extra-files` (Cargo.toml workspace version), `changelog-sections` (ported), `bootstrap-sha`.
- `.release-please-manifest.json` — `{ ".": "0.2.3" }`.
- `.github/workflows/release-please.yml` — push→main, `permissions: {}`, App-token-or-PAT, `release-please-action@v5`; `sync-cargo-lock` job.
- `RELEASE.md` — flow, disabling, token setup, first-release cutover (Cargo/tag adapted).

**commitlint cluster:**
- `.github/workflows/commitlint.yml` — humans/dependabot split.
- `commitlint.config.mjs` — `@commitlint/config-conventional` + 200-char body/footer.
- `commitlint.dependabot.config.mjs` — line-length relaxed.
- `package.json` — the two `@commitlint` devDeps.

## Composition

```
commitlint enforces conventional commits on PRs
  → feat:/fix: land on main
  → release-please opens a release PR (bumps Cargo.toml + CHANGELOG.md; sync-cargo-lock updates Cargo.lock)
  → merge → v0.3.0 tag (App/PAT) → existing release.yml builds multi-target binaries
```

## Out of scope
- A crates.io publish workflow (togl's `release.yml` builds binaries only; crates.io publish is a separate follow-on).
- Creating the GitHub App / adding repo secrets (manual; documented in `RELEASE.md`).

## Manual steps (post-merge, done by the maintainer)
1. Create a GitHub App (Contents + Pull requests: write) or a fine-grained PAT; add secrets `RELEASE_PLEASE_APP_ID` + `RELEASE_PLEASE_PRIVATE_KEY` (App) or `RELEASE_PLEASE_APP_TOKEN` (PAT).
2. Confirm `.release-please-manifest.json` = current version (`0.2.3`) and `bootstrap-sha` = `v0.2.3` commit.
3. Merge to main → release-please opens the first PR (proposes `v0.3.0`).

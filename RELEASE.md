# Release process

Releases are driven by **release-please** (PR-based version proposals) and the
existing tag-triggered **`release.yml`** binary build.

## Default flow (release-please)

1. Merge `feat:` / `fix:` / `perf:` commits to `main` (Conventional Commits are
   enforced by the `commitlint` workflow).
2. The `release-please` workflow opens (or updates) a release PR proposing the
   next semver bump, updating `[workspace.package].version` in `Cargo.toml`, and
   writing a `CHANGELOG.md` entry. The `sync-cargo-lock` job updates `Cargo.lock`
   on that PR branch so CI's `--locked` checks pass.
3. Merge the release PR. release-please pushes a `v*` tag.
4. The `release.yml` workflow fires on the tag, verifies tag provenance
   (tag on `main`, tag == workspace version), builds the multi-target release
   binaries, and fans out to the publish destinations below.

`feat:` → minor bump, `fix:`/`perf:` → patch, `feat!:`/`BREAKING CHANGE:` → major.

## Publish destinations

Every registry that supports OIDC trusted publishing uses it, and the
Homebrew tap push mints a short-lived App installation token in-job — no
long-lived credentials anywhere. Production publishes run in GitHub
environments; the `pypi`, `npm`, and `crates` environments have a required
reviewer as the final human gate (TestPyPI publishes automatically as the
smoke test).

| Destination | Job | Environment | Auth |
|---|---|---|---|
| GitHub Release binaries | `build-release` | — | `GITHUB_TOKEN` |
| crates.io (`togl-lib`, `togl`) | `publish-crates` | `crates` | OIDC trusted publishing |
| TestPyPI → PyPI (`togl`) | `publish-testpypi` → `publish-pypi` | `testpypi` → `pypi` | OIDC trusted publishing (wheels via maturin `bindings=bin`) |
| npm (`togl-cli` + 4 `@smorinlabs/togl-*` platform packages) | `publish-npm` | `npm` | OIDC trusted publishing |
| Homebrew (`smorinlabs/tap/togl`) | `update-homebrew` | — | Short-lived App token (`TAP_PUSH_APP_*` secrets; App installed on the tap) |

Release archives are named `togl-<target>.tar.gz` (`.zip` on Windows) and
contain both the `toggle` and `togl` binaries; the npm platform packages and
the Homebrew formula unpack them.

## Disabling release-please

If you'd rather not use PR-driven version proposals:

```bash
# Rename to disable:
mv .github/workflows/release-please.yml .github/workflows/release-please.yml.disabled
# Then bump the version manually before tagging:
$EDITOR Cargo.toml          # [workspace.package] version = "X.Y.Z"
cargo update --workspace    # sync Cargo.lock
git commit -am "release: vX.Y.Z"
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

`release.yml` still fires on the tag and builds the binaries.

## Token setup

release-please must authenticate with a token that can **both** open PRs and
trigger downstream workflows (so merging the release PR fires `release.yml`).
`GITHUB_TOKEN` does neither reliably — it never re-triggers other workflows, and
org policy can block it from opening PRs — so it is **not** used. Configure one
of these two mechanisms instead:

1. **GitHub App (preferred).** Create a GitHub App with **Contents** and
   **Pull requests: write**, install it on this repo, then add two secrets:
   - `RELEASE_PLEASE_CLIENT_ID` — the App's Client ID
   - `RELEASE_PLEASE_PRIVATE_KEY` — the App's private key (`.pem` contents)

   Both jobs mint a short-lived installation token from these.

2. **Fallback PAT.** Create a fine-grained Personal Access Token scoped to this
   repo with **Contents** and **Pull requests: write**, stored as the
   `RELEASE_PLEASE_APP_TOKEN` secret.

With neither configured, release-please fails fast rather than silently
producing a release that can't build binaries.

## First-release cutover (one-time)

This cluster is configured to cut **v0.3.0** as its first managed release,
capturing the P05 (`--insert`) and P08 (C library) work already on `main`:

1. `[workspace.package].version` in `Cargo.toml` is `0.2.3` (the last release).
2. `.release-please-manifest.json` is `{ ".": "0.2.3" }`.
3. `release-please-config.json` `bootstrap-sha` is the `v0.2.3` commit
   (`22a85ad75bba5765107574816ff3ccbcea4d02c7`), so release-please scans every
   commit since `v0.2.3` and proposes `v0.3.0` with a full changelog.
4. After this PR merges to `main` (and the token secrets exist), release-please
   opens its first release PR.

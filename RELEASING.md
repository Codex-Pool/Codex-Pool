# Releasing Codex-Pool-Personal

This repository uses a tag-driven release flow for `codex-pool-personal`.

## Release outputs

- GitHub Release attachments:
  - `codex-pool-personal` archive bundles for:
    - `x86_64-unknown-linux-gnu`
    - `aarch64-unknown-linux-gnu`
    - `x86_64-apple-darwin`
    - `aarch64-apple-darwin`
    - `x86_64-pc-windows-msvc`
- Docker images:
  - `ghcr.io/codex-pool/codex-pool-personal:edge` from `main`
  - `ghcr.io/codex-pool/codex-pool-personal:latest` from stable `vX.Y.Z` tags
  - versioned tags such as `ghcr.io/codex-pool/codex-pool-personal:v0.1.0`

## Preflight

Run these checks before tagging:

```bash
cargo test --workspace
cd frontend && npm ci --legacy-peer-deps && npm run build && npm run i18n:check
```

If you want to preview the release bump without mutating git state:

```bash
cargo release patch --workspace
```

## Cut a release

For the very first tagged release, keep the current workspace version and cut the tag directly:

```bash
cargo release 0.1.0 --workspace --execute
```

After that, choose `patch`, `minor`, or `major`, then run:

```bash
cargo release patch --workspace --execute
```

The release config will:

- bump the shared workspace version
- create a release commit
- create a git tag named `vX.Y.Z`

Pushing that tag triggers:

- `.github/workflows/release.yml`
  - builds `codex-pool-personal`
  - uploads binary archives to a GitHub Release
- `.github/workflows/docker-publish.yml`
  - publishes `latest`, `vX.Y.Z`, `X.Y.Z`, and `sha-*` Docker tags

## Edge builds

Every push to `main` publishes:

```text
ghcr.io/codex-pool/codex-pool-personal:edge
ghcr.io/codex-pool/codex-pool-personal:sha-<commit>
```

Use `edge` only if you want the newest untagged build.

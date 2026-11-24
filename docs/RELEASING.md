# Release Guide (`docs/RELEASING.md`)

This guide describes the release procedure for `mymessages`, ensuring reproducible builds, passing tests, and automated packaging.

---

## 1. Versioning Strategy

`mymessages` adheres strictly to [Semantic Versioning 2.0.0](https://semver.org/):
- **Major (`X.0.0`)**: Incompatible API, storage schema, or CLI breaking changes.
- **Minor (`0.Y.0`)**: New backward-compatible features (e.g. new adapters, new MCP tools, search capabilities).
- **Patch (`0.0.Z`)**: Backward-compatible bug fixes and performance improvements.

---

## 2. Pre-Release Checklist

Before cutting a new release, verify all automated checks pass locally via `mise`:

```bash
# 1. Run all unit and integration tests
mise run test

# 2. Enforce line coverage thresholds (>= 80% for both Rust and TypeScript)
mise run coverage

# 3. Build production web bundle and release binary
mise run build
```

---

## 3. Creating a Release

### Step 1: Bump Versions
Update the version number in:
1. `Cargo.toml`: `version = "X.Y.Z"` (then run `cargo check` / `mise run test` to update `Cargo.lock`)
2. `web/package.json`: `"version": "X.Y.Z"` (automatically injected into the Web UI badge via Vite `__APP_VERSION__` in `web/src/lib/format.ts`)
3. `src/api/openapi.rs`: `version = "X.Y.Z"`
4. `README.md`: version badge `vX.Y.Z`

### Step 2: Regenerate Screenshots & Demo GIF
Rebuild the Web UI and regenerate visual assets whenever the UI changes:
```bash
mise run screenshot   # Updates docs/assets/screenshot.png, screenshot-dark.png, screenshot-light.png
mise run gif          # Updates docs/assets/demo.gif
```

Commit the version bump and updated assets:
```bash
git add Cargo.toml Cargo.lock web/package.json src/api/openapi.rs README.md docs/assets/
git commit -m "chore(release): bump version to vX.Y.Z and update WebUI assets"
```

### Step 3: Tag and Push
Create an annotated Git tag matching the version:

```bash
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin master --tags
```

---

## 4. Automated CI/CD Execution

Pushing a `v*` tag triggers the `.github/workflows/release.yml` pipeline:

1. **Binary Compilation**: Builds an optimized Linux x86_64 binary.
2. **GitHub Release**: Packages `mymessages-vX.Y.Z-linux-x86_64.tar.gz` and attaches it to the release notes.
3. **Docker Multi-Arch Container**: Builds the ultra-minimal Alpine runtime image and pushes it to GitHub Container Registry:
   - `ghcr.io/rodvieirasilva/mymessages:vX.Y.Z`
   - `ghcr.io/rodvieirasilva/mymessages:latest`

---

## 5. Rollback Procedure

If a critical flaw is discovered post-release:
1. Yank the problematic release tag on GitHub: `git push --delete origin vX.Y.Z`.
2. Delete the GitHub release notes or mark as pre-release.
3. Re-tag the last known good commit as a patch bump `vX.Y.(Z+1)` with the fix and push.

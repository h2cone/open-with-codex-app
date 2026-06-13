# Release Workflow Design

## Goal

Publish GitHub Releases for `open-with-codex-app` when a semantic version tag such as `v0.1.0` is pushed.

The release should include only platforms where the tool provides supported desktop integration behavior:

- Windows x64
- macOS Intel
- macOS Apple Silicon

Linux should not be published as a release artifact because the current Linux build reports that desktop integrations are unsupported.

## Project Facts

- Language and build system: Rust with Cargo.
- Package name: `open-with-codex-app`.
- First release version: `0.1.0`.
- Repository remote: `h2cone/open-with-codex-app`.
- Existing state: no release workflow, no changelog, no tags.

## Workflow

Create `.github/workflows/release.yml` with a tag trigger:

```yaml
on:
  push:
    tags:
      - "v*.*.*"
```

The workflow has three jobs:

- `build`: matrix build for Windows x64, macOS Intel, and macOS Apple Silicon.
- `release-notes`: extract the matching version section from `CHANGELOG.md`.
- `release`: create a GitHub Release and attach all build archives.

The workflow needs `permissions: contents: write` so it can create releases and upload assets.

## Build Matrix

Use these Rust targets and archive names:

| Platform | Target | Runner | Archive |
| --- | --- | --- | --- |
| Windows x64 | `x86_64-pc-windows-msvc` | `windows-latest` | `open-with-codex-app-v0.1.0-x86_64-windows.zip` |
| macOS Intel | `x86_64-apple-darwin` | `macos-latest` | `open-with-codex-app-v0.1.0-x86_64-macos.tar.gz` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `macos-latest` | `open-with-codex-app-v0.1.0-aarch64-macos.tar.gz` |

The workflow should derive the archive version from `github.ref_name` so future tags do not require workflow edits.

## Changelog

Add `CHANGELOG.md` in Keep a Changelog style with:

- `[Unreleased]`
- `[0.1.0] - 2026-06-13`

The `0.1.0` entry should describe the first public release: Windows Explorer integration, macOS Finder service support, `codex app <dir>` shim installation, user-scoped install locations, uninstall support, and unsupported-platform skip behavior.

## README

Add an `Installation` section that links to latest-release assets:

- Windows x64 zip
- macOS Intel tar.gz
- macOS Apple Silicon tar.gz

The links should use `https://github.com/h2cone/open-with-codex-app/releases/latest/download/...` so they always resolve to the latest release.

## Verification

Before claiming completion, run:

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --all-targets -- -D warnings`

Also inspect the generated workflow and documentation diffs for consistent artifact names.

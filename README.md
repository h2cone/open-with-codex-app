# Open With Codex App

Register user-scoped desktop integrations for opening folders in the Codex desktop app.

This tool is useful when you want a native "Open with Codex app" entry in the file manager and a `codex app <dir>` command that launches the desktop app with a project directory.

## Features

- Registers a Windows Explorer folder and folder-background context menu.
- Registers a macOS Finder service.
- Installs a `codex app <dir>` command shim.
- Opens projects through Codex app's `--open-project <dir>` argument.
- Uses only user-scoped install locations and can uninstall its own integrations.

## Supported Platforms

- Windows
- macOS

Other platforms build the crate but report that desktop integrations are unsupported.

## Installation

Download the latest release for your platform from
[GitHub Releases](https://github.com/h2cone/open-with-codex-app/releases/latest).

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 | [open-with-codex-app-x86_64-linux.tar.gz](https://github.com/h2cone/open-with-codex-app/releases/latest/download/open-with-codex-app-x86_64-linux.tar.gz) |
| Linux | arm64 | [open-with-codex-app-aarch64-linux.tar.gz](https://github.com/h2cone/open-with-codex-app/releases/latest/download/open-with-codex-app-aarch64-linux.tar.gz) |
| macOS | Apple Silicon | [open-with-codex-app-aarch64-darwin.tar.gz](https://github.com/h2cone/open-with-codex-app/releases/latest/download/open-with-codex-app-aarch64-darwin.tar.gz) |
| macOS | Intel | [open-with-codex-app-x86_64-darwin.tar.gz](https://github.com/h2cone/open-with-codex-app/releases/latest/download/open-with-codex-app-x86_64-darwin.tar.gz) |
| Windows | x86_64 | [open-with-codex-app-x86_64-windows.zip](https://github.com/h2cone/open-with-codex-app/releases/latest/download/open-with-codex-app-x86_64-windows.zip) |

## Build

```powershell
cargo build --release
```

The release binary is written to `target/release/open-with-codex-app.exe` on Windows, or `target/release/open-with-codex-app` on macOS.

## Usage

Register integrations:

```powershell
open-with-codex-app
```

Open a directory in Codex app:

```powershell
open-with-codex-app app C:\path\to\project
codex app C:\path\to\project
```

Remove integrations:

```powershell
open-with-codex-app uninstall
```

## Environment Overrides

Windows:

- `CODEX_APP_EXECUTABLE` can point directly to `Codex.exe`.

macOS:

- `CODEX_APP_BUNDLE` can point directly to the Codex app bundle.

## Development

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

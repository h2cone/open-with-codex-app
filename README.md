# Open With Codex App

Register user-scoped desktop integrations for opening folders in the Codex desktop app.

Supports both the current ChatGPT desktop package entry point and the legacy Codex desktop entry point on Windows.

This tool is useful when you want a native "Open with Codex app" entry in the file manager and a `codex app <dir>` command that launches the desktop app with a project directory.

## Features

- Registers a Windows Explorer folder and folder-background context menu.
- Registers a macOS Finder service.
- Installs a `codex app <dir>` command shim.
- Registers a PowerShell function that routes `codex app` to this tool even when mise appears first on `PATH`, while other subcommands use the existing Codex CLI.
- Sends Windows project links to the registered desktop app through Explorer, using the desktop user's permissions even from an elevated terminal.
- Opens macOS projects through Codex app's bundled `codex app <dir>` command.
- Uses only user-scoped install locations and can uninstall its own integrations.

## Supported Platforms

- Windows
- macOS

Other platforms build the crate but report that desktop integrations are unsupported.

## Installation

Download the latest release for your platform from
[GitHub Releases](https://github.com/h2cone/open-with-codex-app/releases/latest).

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

- `CODEX_APP_EXECUTABLE` can point directly to `ChatGPT.exe` or legacy `Codex.exe`.

macOS:

- `CODEX_APP_BUNDLE` can point directly to the Codex app bundle.

## Windows Troubleshooting

Windows launches go through the running Explorer desktop shell and the registered `codex://` protocol handler. This avoids passing an elevated terminal's token to the desktop app and avoids creating a separate app profile named after an execution alias. The launcher waits for the shell handoff and reports errors from that handoff. `requested: <path>` means the request was submitted; it does not confirm that the app displayed the project. A running Explorer desktop is required.

`CODEX_APP_EXECUTABLE` overrides the protocol handler with a specific executable. That executable receives the project deep link through the same Explorer launch mechanism.

If `codex app .` prints `Opening workspace ... in the Desktop app...`, the official CLI is handling the command. Check which command takes precedence:

```powershell
Get-Command codex -All
```

Tools such as mise can put their own shim ahead of this tool on `PATH`. To use this tool's installed shim in the current PowerShell session:

```powershell
. "$env:LOCALAPPDATA\OpenAI\Codex\OpenWithCodexApp\powershell.ps1"
codex app .
```

The installer adds a managed loader to the current user's PowerShell and Windows PowerShell `profile.ps1` files, so new sessions load this function automatically. Already running terminals need the command above once. Uninstall removes these managed blocks and preserves the rest of each profile.

`Get-Command codex` should now report `Function`. `codex --version`, `codex --help`, and other CLI commands still go to the existing CLI. Sessions started with `-NoProfile` need the loader command explicitly.

You can also bypass `PATH` lookup:

```powershell
& "$env:LOCALAPPDATA\OpenAI\Codex\OpenWithCodexApp\bin\codex.exe" app .
```

## Development

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

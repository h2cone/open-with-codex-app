# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

## [0.1.1] - 2026-11-01

### Fixed

- Register a PowerShell command router so `codex app` uses this launcher even when mise or another CLI shim precedes it on `PATH`; keep other subcommands on the existing CLI.
- Submit Windows project links through the existing Explorer desktop shell and registered app protocol, instead of inheriting an elevated terminal's token or creating an alias-named app profile.
- Wait for the shell handoff, surface dispatch errors, and report `requested` instead of claiming the project was opened.
- Recognize the current `chatgpt-desktop.exe` Windows execution alias when detecting an installed app.
- Use Codex app's bundled `codex app <dir>` launcher on macOS so Finder service opens the selected folder as a Codex project.
- Write a fuller macOS Finder service workflow and upgrade existing workflow files during install.

## [0.1.0] - 2026-06-25

### Added

- Initial release.

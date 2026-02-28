# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2025-05-01

### Added

- Profile-based provisioning with Laptop, Desktop, and Manual profiles
- 92-package catalog across 10 categories, embedded at compile time
- Remote catalog fetching with 24-hour cache and fallback to embedded
- Two-column package selection screen with category toggles and search
- Review screen showing selected packages before installation
- Streamed winget installation with real-time terminal log output
- Update scanner to detect outdated packages via winget
- Bulk upgrade flow with per-package progress tracking
- Export and import package selections via native file dialogs
- Detect already-installed packages and show badges in the UI
- Winget settings screen with install mode, scope, architecture, and advanced flags
- Keyboard shortcuts: Enter to confirm, Escape to go back, Ctrl+A to select all
- Copy log button on progress screens
- Dry-run mode via `--dry` flag for testing without real installs
- Cancel button for install, upgrade, and scan operations
- Admin elevation prompt on release builds
- Dark theme with Tailwind zinc palette and Lucide icon font
- GitHub Actions release workflow for automated builds

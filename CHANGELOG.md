# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Post-install steps screen — after installation, packages with custom install commands (WSL, Bun, uv, Rust, Topping, Remove Windows AI) show a checklist of required manual actions (restart terminal, restart PC, run downloaded installer)

### Removed

- Unused `Profile::description()` method and `SystemInfo::os_version` field

## [0.4.1] - 2026-03-19

### Added

- Persist GitHub OAuth token across app restarts (saved to `%APPDATA%\provision`)
- Skip re-authentication when navigating back to Repos screen within same session

### Fixed

- Only clear saved token on auth errors (401/403), not transient network failures

## [0.4.0] - 2026-03-19

### Added

- GitHub repo cloning — authenticate via device flow, browse repos, clone to local folders
- Copy button on GitHub device code display
- Winget package search — search the full winget catalog and install directly
- PNG icon asset for GitHub OAuth app

### Changed

- Bump minimum font size to 12px for crisp rendering on Windows
- Replace update scan terminal log with centered spinner layout
- Profile cards no longer expand to fill screen height
- Clone repos into subdirectories of selected folder (not as the folder itself)

### Fixed

- Scrollbar padding missing on package select and update select screens

## [0.3.2] - 2026-03-11

### Added

- New "Tweaks" package category for system customization scripts
- Remove Windows AI package (removes Copilot, Recall, etc. via zoicware script)

## [0.3.1] - 2026-03-11

### Changed

- Deduplicate upgrade/uninstall batch streams via shared `BatchItem` trait and `run_winget_batch()`
- Deduplicate scan operations via shared `ScanEvent` enum and `run_winget_scan()`
- Add `LogBuffer` with `RefCell`-cached `joined()` to avoid per-frame string joins in view
- Cache `search_lower` and `categories` on App to avoid per-frame recomputation
- Extract `common_args()` in settings to deduplicate install/uninstall arg building
- Extract `action_card()` helper in views replacing 3 copy-pasted card blocks
- Simplify `CopyLog` to read from state instead of cloning entire log vector
- Take `InstallProgress` by value in `handle_event` to avoid unnecessary string clones

## [0.3.0] - 2026-03-11

### Added

- Uninstall support — select and remove installed packages via `winget uninstall`
- Package size display from registry `EstimatedSize` on uninstall screen
- System info banner on profile screen (hostname, OS, CPU, RAM)
- Elapsed timer on install, upgrade, and uninstall progress screens
- `Ctrl+K` hotkey to focus the search box on package/update/uninstall screens

### Fixed

- Uninstall screen visual issues (scrollbar overlap, layout alignment)

## [0.2.0] - 2026-03-11

### Added

- Animated braille dots spinner on all loading states (installed scan, install, upgrade, update scan)
- GitHub release version checker with 24-hour cache
- Update-available banner on profile select screen with link to release page
- App updates section in settings with manual "Check now" button

### Fixed

- Review screen scrollbar overlapping content

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

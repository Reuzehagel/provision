# Provision v1.0 — TODO

## Must-have (before release)

### Metadata & Legal

- [x] Add LICENSE file (MIT recommended)
- [x] Fill Cargo.toml metadata: `description`, `license`, `authors`, `repository`

### README

- [x] Fix profile names: "Personal/Work" → "Laptop/Desktop"
- [x] Remove hardcoded package count and per-category package lists — replace with a dynamic summary or just point to `packages.toml` (avoids going stale every time we add a package)

### App icon

- [ ] Design/choose an .ico file
- [ ] Add `winres` build script so the .exe gets a proper icon
- [ ] Set window icon in iced application builder

### Install label bug

- [x] Review screen: when all selected packages are already installed, button says "Install N packages" instead of "Reinstall N packages" — fix the label logic in `views.rs`

### Remote catalog fetching

- [x] On startup, fetch `packages.toml` from `https://raw.githubusercontent.com/Reuzehagel/provision/main/packages.toml`
- [x] Parse and validate the remote TOML — if valid, use it; if not, fall back
- [x] Cache fetched catalog to `%APPDATA%\provision\packages.toml` with a last-fetched timestamp
- [x] On subsequent launches: if cache < 24h old, use cache; otherwise re-fetch
- [x] Final fallback: embedded `include_str!` version (app always works offline)
- [x] Show subtle indicator in UI when using a stale/embedded catalog vs fresh remote

### Package catalog sorting

- [x] Write a `just sort-packages` script that sorts `packages.toml` entries: grouped by category (in the existing category order), alphabetical by `name` within each category
- [x] Add `just sort-packages` as a step in `just check` (or as a pre-commit hook) so it stays sorted automatically
- [x] Run the sort once to clean up the current file

## Nice-to-have (post-release or if time allows)

### CI / GitHub Actions

- [x] Add a workflow: `cargo build --release` + `clippy` + `fmt --check` on push/PR
- [x] Add a release workflow: build on tag push, upload .exe to GitHub Releases

### CHANGELOG

- [ ] Create CHANGELOG.md following [Keep a Changelog](https://keepachangelog.com/) format
- [ ] Consider an in-app changelog screen (accessible from settings or a "What's new" link)

### Screenshots

- [ ] Add 2-3 screenshots to README (profile select, package select, installing)

### Minor UX

- [ ] `topping` package opens a browser URL instead of silently installing — consider adding a visual indicator or different UX for browser-download packages
- [ ] `wsl --install` may require a restart — consider showing a note in the UI

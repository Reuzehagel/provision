# Provision v1.0 — TODO

## Must-have (before release)

### App icon

- [ ] Design/choose an .ico file
- [ ] Add `winres` build script so the .exe gets a proper icon
- [ ] Set window icon in iced application builder

## Nice-to-have (post-release or if time allows)

### CHANGELOG

- [x] Create CHANGELOG.md following [Keep a Changelog](https://keepachangelog.com/) format
- [x] In-app changelog tab in Settings screen

### Screenshots

- [ ] Add 2-3 screenshots to README (profile select, package select, installing)

### Minor UX

- [ ] `topping` package opens a browser URL instead of silently installing — consider adding a visual indicator or different UX for browser-download packages
- [ ] `wsl --install` may require a restart — consider showing a note in the UI

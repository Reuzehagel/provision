# Provision v1.0 — TODO

### Minor UX

- [ ] `topping` package opens a browser URL instead of silently installing — consider adding a visual indicator or different UX for browser-download packages
- [ ] `wsl --install` may require a restart — consider showing a note in the UI

## Later releases

- [ ] **Custom/user packages** — Let users add arbitrary winget IDs not in the catalog. Persist to a local config file (`%APPDATA%\provision\custom-packages.toml`)
- [ ] **Config file for preferences** — `%APPDATA%\provision\config.toml` for last-used profile, winget flags, window position, custom packages

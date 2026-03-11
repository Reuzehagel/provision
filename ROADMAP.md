# Roadmap

Potential future features, roughly ordered by impact-to-effort ratio:

- **Package search** — Search winget's full catalog (`winget search`) and add packages on the fly, not just from the curated `packages.toml`. Reuse existing table parser from `upgrade.rs`.
- **Export to script** — Generate a standalone `.ps1` script from the current selection (`winget install --id X` per package). Useful for sharing setups or running without the app.
- **Install timer** — Show elapsed time during install and total time on completion. Simple addition, satisfying to see.
- **Search hotkey** — `Ctrl+K` or `\` to focus the search box from anywhere. Quick quality-of-life shortcut.

## Later

- **Post-install hooks UI** — Surface `post_install` steps as a checklist after install instead of running them silently.
- **Light theme** — Toggle between dark/light mode. Low priority but straightforward with the existing `Theme::custom()` setup.

# Roadmap

Potential future features, roughly ordered by impact-to-effort ratio:

- **Package search** — Search winget's full catalog (`winget search`) and add packages on the fly, not just from the curated `packages.toml`. Reuse existing table parser from `upgrade.rs`.
- **Export to script** — Generate a standalone `.ps1` script from the current selection (`winget install --id X` per package). Useful for sharing setups or running without the app.

## Later

- **Post-install hooks UI** — Surface `post_install` steps as a checklist after install instead of running them silently.
- **Light theme** — Toggle between dark/light mode. Low priority but straightforward with the existing `Theme::custom()` setup.
- **Package categories** — Add more categories and allow filtering by them on the selection screen.

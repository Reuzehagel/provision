# Roadmap

Potential future features, roughly ordered by impact-to-effort ratio:

- **GitHub repo cloning** — Standalone tool: log in to GitHub, browse/search your repos, pick a clone location, clone. Useful for dotfiles, project repos, etc.
- **Guided setup wizard** — Dedicated walkthrough screens for packages needing manual steps (e.g. WSL restart, Topping manual download, bun shell config). Step-by-step instructions with "next" navigation instead of silent install.
- **Export to script** — Generate a standalone `.ps1` script from the current selection (`winget install --id X` per package). Useful for sharing setups or running without the app.

## Later

- **Post-install hooks UI** — Surface `post_install` steps as a checklist after install instead of running them silently.
- **Light theme** — Toggle between dark/light mode. Low priority but straightforward with the existing `Theme::custom()` setup.
- **Package categories** — Add more categories and allow filtering by them on the selection screen.

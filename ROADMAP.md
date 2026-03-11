# Roadmap

Potential future features, roughly ordered by impact-to-effort ratio:

- **Package search** — Search winget's full catalog (`winget search`) and add packages on the fly, not just from the curated `packages.toml`. Reuse existing table parser from `upgrade.rs`.
- **Export to script** — Generate a standalone `.ps1` script from the current selection (`winget install --id X` per package). Useful for sharing setups or running without the app.
- **Scheduled update checks** — Periodically check for upgrades and notify via Windows toast notifications (`winrt-toast` crate). Could be a "rerun to update" model (like Nanite) rather than a persistent background service.
- **Uninstall support** — Select installed packages to remove via `winget uninstall`. Start simple (just uninstall), potentially add leftover file/registry cleanup later — that's a much bigger scope.
- **Install timer** — Show elapsed time during install and total time on completion. Simple addition, satisfying to see.
- **Post-install hooks UI** — Surface `post_install` steps as a checklist after install instead of running them silently.
- **Drag-and-drop import** — Drop a `.json` selection file onto the window to import it, instead of going through the file dialog.
- **Package dependency hints** — Show soft recommendations like "You selected Docker Desktop — consider also adding WSL" as subtle suggestions in the review screen.
- **Light theme** — Toggle between dark/light mode. Low priority but straightforward with the existing `Theme::custom()` setup.
- **Keyboard-driven workflow** — Vim-style `j`/`k` navigation through the package list, `/` to focus search, number keys for profiles. Power user feel.

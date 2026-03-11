# Roadmap

Potential future features, roughly ordered by impact-to-effort ratio:

1. **Package search** — Search winget's full catalog (`winget search`) and add packages on the fly, not just from the curated `packages.toml`. Reuse existing table parser from `upgrade.rs`.
2. **Export to script** — Generate a standalone `.ps1` script from the current selection (`winget install --id X` per package). Useful for sharing setups or running without the app.
3. **Scheduled update checks** — Periodically check for upgrades and notify via Windows toast notifications (`winrt-toast` crate). Could be a "rerun to update" model (like Nanite) rather than a persistent background service.
4. **Uninstall support** — Select installed packages to remove via `winget uninstall`. Start simple (just uninstall), potentially add leftover file/registry cleanup later — that's a much bigger scope.
5. **Light theme** — Toggle between dark/light mode. Low priority but straightforward with the existing `Theme::custom()` setup.

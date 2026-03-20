# Guided Setup Wizard

Post-install guidance for packages that need manual steps (browser downloads, terminal restarts, reboots).

## Problem

Six packages in the catalog use custom install commands instead of winget and require user action after install. Currently the only guidance is an `is_browser_download()` external-link icon and a WSL-specific reboot warning on the review screen. Users can be surprised when a browser opens mid-install or when WSL prompts for a reboot.

## Affected Packages

| Package | Install Method | Post-Install Need |
|---------|---------------|-------------------|
| Bun | PowerShell script | Terminal restart (PATH) |
| Rust | Browser download (rustup-init.exe) | Run installer, may need VS Build Tools, terminal restart |
| uv | PowerShell script | Terminal restart (PATH) |
| WSL | `wsl --install` | System reboot, then interactive Linux user setup |
| Topping | Browser download | Run downloaded installer |
| RemoveWindowsAI | PowerShell script | System reboot |

## Design

### Package Classification

Each package is tagged with a `SetupKind` — no new fields in `packages.toml`:

- **`Silent`** — normal winget install, no guidance needed (default)
- **`TerminalRestart`** — PATH changes, user needs to restart terminal (Bun, uv)
- **`BrowserDownload`** — opens a URL, user must run the downloaded installer (Rust, Topping)
- **`Reboot`** — system reboot required (WSL, RemoveWindowsAI)

Classification logic lives in `catalog.rs` as a method on `Package`, using a combination of heuristics and id-based lookup:

- `install_command` starting with `"start http"` → `BrowserDownload`
- Package id `"wsl"` or `"remove-windows-ai"` → `Reboot` (id-based, since their install commands are PowerShell scripts indistinguishable from TerminalRestart packages)
- Remaining packages with `install_command` (PowerShell scripts like Bun, uv) → `TerminalRestart`
- Everything else (has `winget_id`, no `install_command`) → `Silent`

Priority when multiple rules could match: `Reboot` > `BrowserDownload` > `TerminalRestart` > `Silent`.

`SetupKind` derives `Ord` with weight values: `Silent = 0`, `TerminalRestart = 1`, `BrowserDownload = 2`, `Reboot = 3`.

### Checklist Instruction Text

Each special package needs user-facing checklist text. This is a `fn setup_instruction(&self) -> Option<&str>` method on `Package`, returning hard-coded text per package id:

| Package | Checklist Text |
|---------|---------------|
| Bun | "Restart your terminal for Bun to be available on PATH" |
| uv | "Restart your terminal for uv to be available on PATH" |
| Rust | "Run rustup-init.exe — follow the prompts and select defaults" |
| Topping | "Run the Topping installer you downloaded" |
| WSL | "Reboot, then open Ubuntu to set up your Linux username and password" |
| RemoveWindowsAI | "Reboot for changes to take effect" |

### Review Screen Changes

**Inline badges** next to each special package in the review list:

- `manual download` — amber (`STATUS_AMBER`) — for `BrowserDownload` packages
- `reboot` — red (`STATUS_RED`) — for `Reboot` packages
- `terminal restart` — blue (`STATUS_BLUE`) — for `TerminalRestart` packages

**Summary line** at the bottom of the review screen (same area as the current WSL warning). Adapts to context:

- Mixed types: "3 packages need manual steps — they'll be installed last."
- Single type: "1 package requires a system reboot — installed last." / "2 packages need a terminal restart — installed last."

Only shown when at least one special package is in the queue. The existing WSL-specific reboot warning is removed — it's now covered by the badge + summary system.

**No changes to review list order** — packages stay in category order on Review. Reordering is internal to the install engine.

### Install Engine Changes

**Queue reordering happens in `handle_start_install()`** (in `main.rs`), before both storing `self.install_queue` and passing the queue to `install_all()`. This ensures the indices emitted by `InstallProgress` events match the stored queue order. `install_all()` itself is unchanged.

The reordering partitions the queue: `Silent` packages first (preserving original order), then special packages sorted by `SetupKind` weight:

1. `TerminalRestart` (least disruptive)
2. `BrowserDownload`
3. `Reboot` (most disruptive, last)

A note appears at the top of the progress log: "Manual-step packages queued last." This note is suppressed when all packages are special (no "regular" packages to contrast against).

**No behavioral changes to the install itself** — browser downloads still open the browser, scripts still run. Only the ordering changes.

### Post-Install Checklist

When all packages finish installing, the progress screen transitions to a checklist. The checklist is rendered by `view_installing()` directly — not through the shared `view_progress_screen()` helper, since only the install flow needs a checklist. Other screens (Updating, Uninstalling) continue using the shared helper unchanged.

Layout:

- Install log stays scrollable above (scrolled up)
- Checklist appears below, grouped by action type (only groups with relevant packages are shown):

**Group 1: "Run downloaded installers"** (if any `BrowserDownload` packages)
- One checkbox per package with its `setup_instruction()` text

**Group 2: "Restart your terminal"** (if any `TerminalRestart` packages)
- Single checkbox covering all affected packages
- Subtext lists the packages: "For Bun, uv PATH changes to take effect"

**Group 3: "Reboot your system"** (if any `Reboot` packages)
- Single checkbox
- Subtext lists the packages: "Required for WSL, Remove Windows AI"

**"Done" button** at the bottom returns to home.

**State tracking** — checklist state is a `HashSet<SetupKind>` on `App` tracking which groups are checked. Purely cosmetic — no enforcement.

**Dry-run mode** — the checklist still appears in dry-run mode since the packages were "installed" (simulated). This lets developers test the checklist UI with `cargo run -- --dry`.

### What This Does NOT Include

- No step-by-step wizard with "Next" navigation — the checklist is scannable, not linear
- No enforcement or verification that steps were completed
- No new `packages.toml` fields — classification uses `install_command` content + id-based lookup
- No new `Screen` enum variant — the checklist is the done-state of the existing `Installing` screen

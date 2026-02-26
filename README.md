# Provision

A GUI tool that takes a fresh Windows machine from "just installed" to "ready to use" in one sitting.

## What it does

You reinstall Windows, or get a new machine. Normally you spend the next hour opening Edge, googling "Firefox download", installing it, then googling "VS Code download", and so on — twenty, thirty, forty times. You forget half the tools you used to have.

Provision fixes that. Tell it what kind of machine this is — personal or work — and it pre-selects a sensible set of packages. Tweak the list, hit go, walk away. It handles winget installs, WSL setup, and post-install steps (like installing Bun after Node).

## Install

Download the latest release from the [Releases](../../releases) page and run the executable. No installer needed.

> **Note:** Release builds request admin elevation automatically. Run as administrator for full functionality (required for winget and WSL).

## Usage

1. **Pick a profile** — Personal, Work, or Manual. Each pre-selects a curated package set.
2. **Browse & customize** — See all available packages by category. Toggle what you want.
3. **Review** — See exactly what's about to be installed.
4. **Install** — Hit go and watch the live output. Walk away when done.

## Package catalog

68 packages across 10 categories, embedded in the binary (no network needed to browse):

| Category | Packages |
|---|---|
| Browsers | Firefox, Chrome, Brave, Vivaldi, Zen |
| Communication | Discord, Slack, Teams, Zoom, Telegram, Signal, WhatsApp |
| Development | VS Code, Zed, Neovim, JetBrains, Node+Bun, Python/UV, Rust, Go, Git, Docker, Windows Terminal, Oh My Posh, GitHub CLI, AutoHotkey |
| Documents | Obsidian, Notion, LibreOffice, Adobe Reader, SumatraPDF |
| Games | Steam, Epic, GOG, EA, Prism Launcher |
| Microsoft Tools | PowerToys, Dev Home, WSL, Sysinternals, App Installer |
| Multimedia | Spotify, VLC, OBS, Audacity, GIMP, HandBrake, FFmpeg, LosslessCut, ShareX |
| Utilities | 7-Zip, Everything, Directory Opus, Bitwarden, Claude, croc, Ente Auth, AltSnap, HWiNFO, LocalSend, Nilesoft Shell, Raycast, Helium |
| Security & Privacy | Proton Drive, Proton Mail, Proton Pass, Proton VPN |
| Design | Canva |

Packages map to winget IDs. Some have post-install commands (Node installs Bun). WSL uses `wsl --install` instead of winget.

## Profiles

Profiles are curated default selections — you can always add or remove anything.

- **Personal** — browsers, communication, multimedia, utilities
- **Work** — development tools, documents, communication
- **Manual** — start from scratch, select everything yourself

## Building from source

Requires Rust 1.85+ (edition 2024).

```bash
cargo build           # debug build
cargo build --release # release build (includes admin elevation manifest)
cargo run             # launch the GUI
```

If `cargo build` fails with "Access is denied", the binary is still running — kill it with:

```
taskkill /F /IM provision.exe
```

## Tech stack

- **[Iced](https://github.com/iced-rs/iced) 0.14** — cross-platform GUI, Elm-style architecture
- **Tokio** — async runtime for spawning install processes
- **winget** — Windows package manager (ships with Windows 11)
- **TOML** — package catalog format, embedded in the binary at compile time

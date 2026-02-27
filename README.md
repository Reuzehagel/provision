# Provision

A GUI tool that takes a fresh Windows machine from "just installed" to "ready to use" in one sitting.

## What it does

You reinstall Windows, or get a new machine. Normally you spend the next hour opening Edge, googling "Firefox download", installing it, then googling "VS Code download", and so on — twenty, thirty, forty times. You forget half the tools you used to have.

Provision fixes that. Tell it what kind of machine this is — laptop or desktop — and it pre-selects a sensible set of packages. Tweak the list, hit go, walk away. It handles winget installs, WSL setup, and post-install steps.

## Install

Download the latest release from the [Releases](../../releases) page and run the executable. No installer needed.

> **Note:** Release builds request admin elevation automatically. Run as administrator for full functionality (required for winget and WSL).

## Usage

1. **Pick a profile** — Laptop, Desktop, or Manual. Each pre-selects a curated package set.
2. **Browse & customize** — See all available packages by category. Toggle what you want.
3. **Review** — See exactly what's about to be installed.
4. **Install** — Hit go and watch the live output. Walk away when done.

## Package catalog

90+ packages across 10 categories: Browsers, Communication, Development, Documents, Games, Microsoft Tools, Multimedia, Utilities, Security & Privacy, and Design. See [`packages.toml`](packages.toml) for the full list.

## Profiles

Profiles are curated default selections — you can always add or remove anything.

- **Laptop** — portable essentials
- **Desktop** — full setup
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

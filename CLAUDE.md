# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is this?

Provision is a Windows-native GUI tool that takes a fresh machine from "just installed" to "ready to use." Users pick a profile (Personal / Work / Homelab / Manual), customize a package list, and let it handle winget installs, WSL setup, and post-install steps. Built with Rust and Iced.

See `IDEA.md` for the full design vision, package catalog spec, and planned flows.

## Workflow

- Use `/commit` skill when asked to commit

## Build & Run

```bash
cargo build          # debug build
cargo build --release
cargo run            # launch the GUI in debug mode
cargo clippy         # lint
cargo fmt --check    # check formatting
```

No test suite yet. When tests exist, run with `cargo test`.

## Architecture

Iced (0.13) Elm-style architecture: **State → Message → Update → View**.

- **`src/main.rs`** — `App` struct (state), `Message` enum, `Screen` enum (`ProfileSelect` → `PackageSelect`), `update()`/`view()` entry points. Standalone functions `profile_card()` and `card_style()` handle card rendering and styling.
- **`src/profile.rs`** — `Profile` enum (Personal, Work, Homelab, Manual) with metadata methods (`title`, `description`, `icon`) and `Profile::ALL` constant.
- **`src/theme.rs`** — Theme stub, currently re-exports `Theme::Dark`. Seam for future custom theming.

Screen flow is driven by `Screen` enum variants. Each variant maps to a `view_*` method on `App`.

## Conventions

- **Iced 0.13 API** — uses `iced::application(title, update, view)` builder, NOT the older `Sandbox` trait. Context7 docs may show outdated patterns.
- Use `Element<'_, Message>` (explicit elided lifetime) in view methods to avoid `mismatched_lifetime_syntaxes` warnings
- **Rust 2024 edition** — requires Rust 1.85+
- Dark theme by default; card/button styles use explicit RGB values for contrast control (don't rely on palette values for card backgrounds — they blend with text on hover)
- `button::Style::text_color` overrides `.color()` on child text widgets — set description contrast via background color choices, not text color overrides
- Profile cards are `button` widgets wrapping `column` layouts, styled with closures passed to `.style()`
- Center content in a screen: `container(content).center_x(Length::Fill).center_y(Length::Fill)`
- `MUTED` constant (`Color::from_rgb(0.55, 0.55, 0.58)`) for secondary/subtitle text

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is this?

Windows provisioning GUI built with Rust and Iced. See `IDEA.md` for the full design vision.

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

If `cargo build` fails with "Access is denied", the binary is still running — kill with `taskkill //F //IM provision.exe`

## Architecture

Iced (0.13) Elm-style architecture: **State → Message → Update → View**.

- **`src/main.rs`** — `App` struct (state with `catalog`, `selected`, `search`), `Message` enum, `Screen` enum (`ProfileSelect` → `PackageSelect`), `update()`/`view()` entry points. Standalone functions `profile_card()`, `card_style()`, `back_button_style()`.
- **`src/catalog.rs`** — `Package` struct (derives `Deserialize`), `load_catalog()` (embeds `packages.toml` via `include_str!`), `default_selection()`, `category_display_name()`, `categories()`.
- **`src/profile.rs`** — `Profile` enum (Personal, Work, Homelab, Manual) with metadata methods (`title`, `description`, `icon`, `slug`) and `Profile::ALL` constant.
- **`src/theme.rs`** — Theme stub, currently re-exports `Theme::Dark`. Seam for future custom theming.
- **`packages.toml`** — 68-package catalog (10 categories) embedded in the binary at compile time. Each entry has `id`, `name`, `description`, `category`, `winget_id`, `profiles`, and optional `post_install`/`install_command`.

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
- Iced 0.13 `Padding` does NOT support `[_; 4]` arrays — use `padding::left(n)`, `padding::top(n)`, etc. for directional padding
- Button styles: extract to standalone functions (`card_style`, `back_button_style`) when reusable; inline closures only for one-offs
- Serde structs: derive `Deserialize` directly on runtime types (no separate DTO layer) — see `Package` in `catalog.rs`

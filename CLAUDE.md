# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is this?

Windows provisioning GUI built with Rust and Iced. See `IDEA.md` for the full design vision.

## Workflow

- Use `/commit` skill when asked to commit
- Use `/run-check` skill after making code changes — runs `cargo build`, `clippy`, and `fmt --check` in sequence

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

Iced (0.14) Elm-style architecture: **State → Message → Update → View**.

- **`src/main.rs`** — `App` struct, `Message` enum, `Screen` enum (`ProfileSelect` → `PackageSelect` → `Review` → `Installing`), `update()`/`view()` entry points. Standalone functions `profile_card()`, `screen_header()`, `card_style()`, `back_button_style()`, `continue_button_style()`, `terminal_box_style()`.
- **`src/install.rs`** — Install engine. `PackageStatus`/`InstallProgress` enums, `install_all()` returns a stream via `iced::stream::channel`. Reads raw bytes from process stdout with mini terminal emulator (handles `\r`, `\n`, ANSI escapes). Classifies output as `Log` (meaningful) vs `Activity` (transient spinners/progress).
- **`src/catalog.rs`** — `Package` struct (derives `Deserialize`), `load_catalog()` (embeds `packages.toml` via `include_str!`), `default_selection()`, `category_display_name()`, `categories()`.
- **`src/profile.rs`** — `Profile` enum (Personal, Work, Homelab, Manual) with metadata methods (`title`, `description`, `icon`, `slug`) and `Profile::ALL` constant.
- **`src/theme.rs`** — Theme stub, currently re-exports `Theme::Dark`. Seam for future custom theming.
- **`packages.toml`** — 68-package catalog (10 categories) embedded in the binary at compile time. Each entry has `id`, `name`, `description`, `category`, `winget_id`, `profiles`, and optional `post_install`/`install_command`.

Screen flow is driven by `Screen` enum variants. Each variant maps to a `view_*` method on `App`.

## Conventions

- **Iced 0.14 API** — uses `iced::application(new, update, view)` builder where `new` returns `(Self, Task<Message>)` and `update` returns `Task<Message>`. NOT the older `Sandbox` trait.
- Use `Element<'_, Message>` (explicit elided lifetime) in view methods to avoid `mismatched_lifetime_syntaxes` warnings
- **Rust 2024 edition** — requires Rust 1.85+
- Dark theme by default; card/button styles use explicit RGB values for contrast control (don't rely on palette values for card backgrounds — they blend with text on hover)
- `button::Style::text_color` overrides `.color()` on child text widgets — set description contrast via background color choices, not text color overrides
- Profile cards are `button` widgets wrapping `column` layouts, styled with closures passed to `.style()`
- Center content in a screen: `container(content).center_x(Length::Fill).center_y(Length::Fill)`
- Named color constants for repeated colors: `MUTED`, `TERMINAL_TEXT`, `STATUS_BLUE`, `STATUS_GREEN`, `STATUS_RED` — prefer constants over inline `Color::from_rgb(...)` when used more than once
- Iced `Padding` does NOT support `[_; 4]` arrays — use `padding::left(n)`, `padding::top(n)`, etc. for directional padding
- `checkbox(bool)` builder pattern — use `.label()` and `.on_toggle()`, no positional label arg
- `button::Style` requires `snap: false` field in struct literals
- **Icons**: Lucide icons via `iced_fonts` crate (feature `"lucide"`). Use `text(char).font(iced_fonts::LUCIDE_FONT)`. Codepoints in `profile.rs`. Emoji chars do NOT render in Iced — always use an icon font.
- Load icon fonts via `.font(iced_fonts::LUCIDE_FONT_BYTES)` on the application builder
- Scrollable content needs explicit `width(Length::Fill)` on inner column or it shrink-wraps
- Button styles: extract to standalone functions (`card_style`, `back_button_style`) when reusable; inline closures only for one-offs
- Serde structs: derive `Deserialize` directly on runtime types (no separate DTO layer) — see `Package` in `catalog.rs`
- **No `Task::sip` in iced 0.14** — use `Task::run(stream, mapper)` for streaming progress. Returns `Task<Message>`, call `.abortable()` for cancellation support via `task::Handle`
- **No `horizontal_space()` in iced 0.14** — use `iced::widget::Space::new().width(Length::Fill)`
- **`progress_bar().height()` is private** — don't try to set it
- **Auto-scroll**: `scrollable(content).anchor_bottom()` keeps scrollable pinned to bottom
- **`iced::stream::channel`** needs explicit sender type: `|mut sender: futures::channel::mpsc::Sender<T>|`
- **`futures` crate**: Not a direct dep — use `iced::futures` and `iced::futures::SinkExt as _` for the re-export
- **Layout stability**: Always render buttons (disabled state) rather than conditionally adding/removing — avoids layout shifts when state changes
- **Spawning processes on Windows**: Use `tokio::process::Command` with `.creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) to prevent console windows flashing
- **Winget piped output**: Winget outputs spinner frames as individual `\r\n` lines when piped. Read raw bytes and classify transient vs meaningful output — don't use `lines()` reader

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is this?

Windows provisioning GUI built with Rust and Iced. See `DESIGN.md` for design system tokens and visual spec.

## Workflow

- **After making any code changes, always run `just check`** — runs `cargo build`, `clippy`, and `fmt --check` in sequence
- **After completing a roadmap feature, update the Roadmap section** — remove the finished item (or move it to a "Done" section if context is useful to keep)

## Build & Run

```bash
cargo build          # debug build
cargo build --release
cargo run            # launch the GUI in debug mode
cargo run -- --dry   # dry-run mode (fake winget data, no real installs)
cargo clippy         # lint
cargo fmt --check    # check formatting
```

**`just` shortcuts** (preferred):

```bash
just run             # cargo run
just build           # debug build
just release         # release build
just check           # build + clippy + fmt --check (run after every code change)
just fmt             # cargo fmt (auto-fix formatting)
just sort-packages   # sort packages.toml by category (cargo run --bin sort_packages)
just kill            # taskkill provision.exe (when "Access is denied" on build)
```

No test suite yet. When tests exist, run with `cargo test`.

If `cargo build` fails with "Access is denied", the binary is still running — kill with `just kill` or `taskkill //F //IM provision.exe`

## Releases

GitHub Actions workflow (`.github/workflows/release.yml`) builds and publishes on version tags. To release:

1. Bump `version` in `Cargo.toml`
2. Commit and tag: `git tag v0.X.0`
3. Push the tag: `git push origin v0.X.0`

The workflow builds a release binary on `windows-latest`, renames it to `provision-vX.Y.Z.exe`, and creates a GitHub Release with auto-generated release notes.

## Architecture

Iced (0.14) Elm-style architecture: **State → Message → Update → View**.

- **`src/main.rs`** — `App` struct, `Message` enum, `Screen` enum, `ProgressState`, `UpdateScanState`, thin `update()` dispatcher + `handle_*()` domain methods (navigation, install, upgrade, selection, export/import, keyboard), `view()` dispatch, `subscription()` for keyboard shortcuts. Helper method `is_installed()` for checking install state. Free functions: `toggle_set()` for bulk select/deselect. No view or style code.
- **`src/views.rs`** — All `view_*` methods (as `impl App`), standalone helpers `terminal_log_box()`, `view_progress_screen()`, `profile_card()`, `package_row()`, `ProgressLabels`.
- **`src/styles.rs`** — Color constants (zinc palette: `TEXT`, `MUTED`, `MUTED_FG`, `CARD_BG`, `BORDER`, `STATUS_*`), `LUCIDE_FONT` constant, button/card/checkbox/container style functions.
- **`src/install.rs`** — Install engine. `PackageStatus`/`InstallProgress` enums, `install_all()` returns a stream via `iced::stream::channel`. Reads raw bytes from process stdout with mini terminal emulator (handles `\r`, `\n`, ANSI escapes). Classifies output as `Log` (meaningful) vs `Activity` (transient spinners/progress).
- **`src/upgrade.rs`** — Upgrade & installed-detection engine. `UpgradeablePackage`/`InstalledPackage` structs, `ScanProgress`/`InstalledScanProgress` enums, `scan_upgrades()`/`scan_installed()` stream winget output, `parse_upgrade_table()`/`parse_list_table()` parse column-aligned tables, `upgrade_all()` streams per-package upgrades.
- **`src/catalog.rs`** — `Package` struct (derives `Deserialize`), `CatalogSource` enum (Embedded/Cached/Remote). `load_catalog()` embeds `packages.toml` via `include_str!`; `fetch_remote_catalog()` tries `%APPDATA%\provision` cache (24h TTL) then GitHub raw URL, falling back to embedded on failure. Also `default_selection()`, `category_display_name()`, `categories()`, `SelectionFile` serde struct, and async `export_selection()`/`import_selection()` using `rfd::AsyncFileDialog` + `tokio::fs`.
- **`src/settings.rs`** — `WingetSettings` struct (persisted to `%APPDATA%\provision\settings.toml` via `load_settings()`/`save_settings()`). `SettingsTab` enum (`Winget`, `Changelog`). `InstallMode`, `InstallScope`, `Architecture` enums with Display impls for pick_list. `OptionalScope`/`OptionalArchitecture` newtypes showing "Default" for `None`. `install_args()` builds extra CLI flags for install/upgrade commands.
- **`src/profile.rs`** — `Profile` enum (Laptop, Desktop, Manual) with metadata methods (`title`, `description`, `icon`, `slug`) and `Profile::ALL` constant.
- **`src/theme.rs`** — Custom theme via `Theme::custom("provision", Palette { ... })` with Tailwind zinc neutrals and blue/emerald/red/amber accents.
- **`src/bin/sort_packages.rs`** — Utility binary (`just sort-packages`) that reads `packages.toml`, groups by category in a fixed display order, sorts alphabetically within each category, and rewrites the file.
- **`packages.toml`** — 92-package catalog (10 categories) embedded in the binary at compile time. Each entry has `id`, `name`, `description`, `category`, `winget_id`, `profiles`, and optional `post_install`/`install_command`.
- **`DESIGN.md`** — Design system reference (color tokens, spacing, component patterns).

Screen flow is driven by `Screen` enum variants. Each variant maps to a `view_*` method on `App`.

## Conventions

### Iced API

- **Iced 0.14 API** — uses `iced::application(new, update, view)` builder where `new` returns `(Self, Task<Message>)` and `update` returns `Task<Message>`. NOT the older `Sandbox` trait.
- Use `Element<'_, Message>` (explicit elided lifetime) in view methods to avoid `mismatched_lifetime_syntaxes` warnings
- **Rust 2024 edition** — requires Rust 1.85+
- **No `Task::sip` in iced 0.14** — use `Task::run(stream, mapper)` for streaming progress. Returns `Task<Message>`, call `.abortable()` for cancellation support via `task::Handle`
- **`iced::stream::channel`** needs explicit sender type: `|mut sender: futures::channel::mpsc::Sender<T>|`
- **`Task::perform(future, mapper)`** — one-shot async (file dialogs, file I/O). Pair with `Task::perform(async { sleep(4s) }, |_| Msg)` for auto-clearing transient UI feedback.
- **`futures` crate**: Not a direct dep — use `iced::futures` and `iced::futures::SinkExt as _` for the re-export
- **Keyboard subscriptions**: No `keyboard::on_key_press` in iced 0.14 — use `keyboard::listen()` which returns `Subscription<keyboard::Event>`. Call `.map()` to convert events to `Message` (requires a catch-all variant like `KeyIgnored` since `.map()` is total). Match on `Event::KeyPressed { key, modifiers, .. }` for key handling.
- **Subscriptions**: Wire up with `.subscription(App::subscription)` on the application builder. The `subscription()` closure is `'static` — cannot capture `&self`, so route by screen in `update()` instead.

### Layout & Widgets

- Center content in a screen: `container(content).center_x(Length::Fill).center_y(Length::Fill)`
- Iced `Padding` does NOT support `[_; 4]` arrays — use `padding::left(n)`, `padding::top(n)`, etc. for directional padding
- `checkbox(bool)` builder pattern — use `.label()` and `.on_toggle()`, no positional label arg
- Scrollable content needs explicit `width(Length::Fill)` on inner column or it shrink-wraps
- **No `horizontal_space()` in iced 0.14** — use `iced::widget::Space::new().width(Length::Fill)`
- **`progress_bar().height()` is private** — don't try to set it
- **Auto-scroll**: `scrollable(content).anchor_bottom()` keeps scrollable pinned to bottom
- **`pick_list(options, selected, on_selected)`** — `T` needs `ToString + PartialEq + Clone`. For `Option<T>` fields, wrap in a newtype with `Display` showing "Default" for `None` (see `OptionalScope`/`OptionalArchitecture` in `settings.rs`)
- **`toggler(is_checked)`** builder pattern — use `.label()`, `.on_toggle()`, `.size()`. Same pattern as checkbox.
- **Layout stability**: Always render buttons (disabled state) rather than conditionally adding/removing — avoids layout shifts when state changes
- **Tooltips are broken inside scrollable/grid layouts** — they render inline and overlap adjacent rows. Avoid `tooltip` in scrollable content.
- **`Theme::custom(name, palette)`** — don't call `.into()` on the name string, it causes ambiguous type inference. Pass `&str` directly. Palette requires all fields including `warning`.

### Styling

- Dark theme by default; card/button styles use explicit RGB values for contrast control (don't rely on palette values for card backgrounds — they blend with text on hover)
- `button::Style::text_color` overrides `.color()` on child text widgets — set description contrast via background color choices, not text color overrides
- `button::Style` requires `snap: false` field in struct literals
- Profile cards are `button` widgets wrapping `column` layouts, styled with closures passed to `.style()`
- Named color constants live in `styles.rs`: zinc palette (`TEXT`, `MUTED_FG`, `MUTED`, `TERMINAL_TEXT`, `CARD_BG`, `CARD_HOVER`, `BORDER`, `BORDER_FOCUS`) + accents (`STATUS_BLUE`, `STATUS_GREEN`, `STATUS_RED`, `STATUS_AMBER`) — prefer constants over inline `Color::from_rgb(...)` when used more than once
- Button styles: extract to standalone functions (`card_style`, `ghost_button_style`, etc.) when reusable; inline closures only for one-offs
- **Icons**: Lucide icons via `lucide-icons` crate. Use `text(char::from(Icon::ChevronLeft)).font(LUCIDE_FONT)` with the type-safe `lucide_icons::Icon` enum — never hardcode codepoints. `LUCIDE_FONT` constant is in `styles.rs`. Load font bytes via `.font(lucide_icons::LUCIDE_FONT_BYTES)` on the application builder. Emoji chars do NOT render in Iced — always use an icon font.

### Data & Serde

- Serde structs: derive `Deserialize` directly on runtime types (no separate DTO layer) — see `Package` in `catalog.rs`
- **Precomputed lowercase fields**: `Package` and `UpgradeablePackage` have `#[serde(skip)]` fields (`name_lower`, `desc_lower`, `winget_id_lower`) populated after deserialization. Use these in search filters and `is_installed()` instead of calling `.to_lowercase()` per-frame. When adding new searchable structs, follow the same pattern.

### Process & IO

- **Spawning processes on Windows**: Use `tokio::process::Command` with `.creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) to prevent console windows flashing. Use `.stderr(Stdio::null())` unless you consume stderr — piped-but-unread stderr deadlocks when the buffer fills.
- **UTF-8 safe slicing**: When slicing strings at byte offsets (e.g. parsing winget column-aligned tables), snap to char boundaries with `str::is_char_boundary()` — multi-byte chars like `…` cause panics
- **Winget piped output**: Winget outputs spinner frames as individual `\r\n` lines when piped. Read raw bytes and classify transient vs meaningful output — don't use `lines()` reader

### Rust Patterns

- **Dead code on enum fields**: Rust doesn't track enum field reads through pattern matching in other modules. Fields in message/progress enums consumed only via `..` or match arms in `main.rs` need `#[allow(dead_code)]` annotations in their defining module.
- **Shared search state**: `self.search` is reused across screens (only one visible at a time). Clear it in the `update()` handler when transitioning to any screen that uses search (see `ProfileSelected`, `StartUpdateScan`, `FinishAndReset`).
- **Background startup tasks**: Kick off non-blocking scans in `App::new()` by returning the `Task` from the constructor. Store the `task::Handle` (via `.abort_on_drop()`) to keep it alive. Handle results gracefully — if the scan fails, the app works without the data.
- **Standalone view helpers returning `Element`**: When a free function takes multiple `&str` params and returns `Element<'_, Message>`, Rust can't infer which borrow — use explicit `<'a>` lifetime on all params and the return type.
- **Threading config into streams**: Stream closures are `'static` — pass owned data (e.g. `Vec<String>` from `settings.install_args()`) into the closure. Use `.iter().cloned()` to extend args vecs inside the stream.

## Roadmap

See `TODO.md` for the full roadmap and task list.
